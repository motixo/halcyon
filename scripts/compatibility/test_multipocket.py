"""Actual official signatures through multipocket resolution, fully offline."""
import copy
import datetime as dt
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import discover

FIXTURE=Path(__file__).parent/'fixtures/multipocket'

class MultiPocket(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.out=Path(self.tmp.name)/'candidate.json'
        self.config=json.loads((FIXTURE/'config.json').read_bytes())
        for source in self.config['archives']:source['keyring']=str(FIXTURE/source['keyring'])
        self.now=dt.datetime(2026,10,23,18,tzinfo=dt.timezone.utc)
        self.get=discover.retained_fetch(FIXTURE/'inputs.json')

    def run_discovery(self,config=None,previous=None,get=None):
        return discover.discover(config or self.config,previous or {},self.out,self.now,get or self.get)

    def test_real_signed_multipocket_retains_binary_and_source_provenance(self):
        result=self.run_discovery()
        self.assertEqual(result['state'],'blocked')
        self.assertIn('metadata_resolution_only',result['reasons'])
        self.assertEqual(result['cohort']['suite'],'noble')
        self.assertEqual({r['suite'] for r in result['indexes']},{'noble','noble-updates','noble-security'})
        package=result['cohort']['packages']['clamav-testfiles-rar']
        self.assertEqual({p['suite'] for p in package['provenance']},{'noble-updates','noble-security'})
        self.assertEqual({p['suite'] for p in package['source']['provenance']},{'noble-updates','noble-security'})
        self.assertTrue(all(p['codename']=='noble' and p['origin']=='Ubuntu' and p['index_sha256'] and p['fingerprint'] for p in package['provenance']))
        self.assertTrue(any(i['kind']=='binary' and i['package']=='clamav-testfiles-rar' and i['sha256']==package['sha256'] for i in result['missing_inputs']))
        self.assertTrue(any(i['kind']=='source-archive' for i in result['missing_inputs']))
        self.assertTrue(self.run_discovery(previous=result)['duplicate'])
        reverse=copy.deepcopy(self.config);reverse['archives'].reverse()
        self.assertEqual(self.run_discovery(reverse)['id'],result['id'])

    def test_signed_suite_scope_and_changed_index_fail_closed(self):
        bad=copy.deepcopy(self.config);bad['archives'][1]['suite']='noble'
        with self.assertRaisesRegex(ValueError,'scope_mismatch'):self.run_discovery(bad)
        def changed(url):
            data=self.get(url)
            return data+b'tamper' if url.endswith('Packages.gz') else data
        with self.assertRaisesRegex(ValueError,'hash_mismatch'):self.run_discovery(get=changed)

    def test_conflicting_binary_and_source_claims_are_retained_not_last_wins(self):
        real=discover.download_index
        # Unit injection AFTER independent real signature/hash validation tests
        # above: exercise collision policy, not an authenticated candidate.
        def conflicting(base,path,hashes,get):
            rows=real(base,path,hashes,get)
            if base.endswith('noble-updates'):
                for row in rows:
                    if row['Package']=='clamav-testfiles-rar':row['SHA256']='0'*64
                    if row['Package']=='libclamunrar' and 'Checksums-Sha256' in row:row['Checksums-Sha256']=row['Checksums-Sha256'].replace(row['Checksums-Sha256'].split()[0],'1'*64)
            return rows
        with patch.object(discover,'download_index',side_effect=conflicting):
            result=self.run_discovery()
        self.assertIn('binary_provenance_collision:clamav-testfiles-rar',result['reasons'])
        key=next(k for k in result['collisions']['binary'] if k.startswith('clamav-testfiles-rar='))
        self.assertEqual(len(result['collisions']['binary'][key]),2)
        self.assertNotIn('clamav-testfiles-rar',result['cohort']['packages'])
        # A digest pin resolves the binary identity, but differing source records
        # remain independently visible and block acceptance.
        good=self.run_discovery()['cohort']['packages']['clamav-testfiles-rar']
        config=copy.deepcopy(self.config);config['package_sha256']={'clamav-testfiles-rar':good['sha256']}
        with patch.object(discover,'download_index',side_effect=conflicting):
            result=self.run_discovery(config)
        self.assertEqual(result['cohort']['packages']['clamav-testfiles-rar']['sha256'],good['sha256'])
        self.assertIn('source_provenance_collision:clamav-testfiles-rar',result['reasons'])
        self.assertTrue(result['collisions']['source'])

if __name__=='__main__':unittest.main()
