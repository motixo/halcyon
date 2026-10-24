"""Accepted real catalog integrity; no host runtime/native identities fabricated."""
import json
from pathlib import Path
import sys
import unittest
P=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(P/'companion'))
from bundle_contract import read_catalog,canonical,digest,identity,runtime_id,valid_runtime,cohort_id,HELPERS
from runtime_target import valid_target

class ReleaseCatalog(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog=read_catalog(P/'companion/compatibility/catalog.json')
        cls.row=cls.catalog['cohorts'][0]
        cls.records=P/'companion/compatibility/evidence'
        cls.accepted=json.loads((cls.records/'accepted-runtime-40071768899.json').read_bytes())

    def test_complete_cohort_and_narrow_runtime_are_both_preserved(self):
        self.assertEqual(len(self.catalog['cohorts']),1)
        row=self.row;target=valid_target(row['runtime_target'],row);valid_runtime(row['runtime'])
        self.assertEqual(row['id'],cohort_id(row))
        self.assertEqual(len(row['packages']),1783)
        self.assertEqual(sum(len(p['files']) for p in row['packages'].values()),165721)
        self.assertEqual(target['work'],dict(files=105,bytes=523221044))
        self.assertEqual(runtime_id(row['runtime']),self.accepted['profile_id'])
        self.assertEqual(identity(target['installed_files']),self.accepted['runtime_inventory_sha256'])
        self.assertEqual(identity(target['packages']),self.accepted['runtime_packages_sha256'])
        self.assertEqual(row['runtime']['configuration']['max_outputs'],1)
        device=row['runtime']['constraints']['drm_devices'][0]
        self.assertEqual((device['vendor'],device['device']),('0x1af4','0x1050'))
        self.assertEqual(len(device['virtio_features']),128)
        self.assertEqual(row['runtime']['processes']['cosmic-applets']['count'],'configured')

    def test_actual_acceptance_and_pending_history_are_distinct_bound_records(self):
        a=self.accepted;row=self.row;target=row['runtime_target']
        self.assertEqual(row['qualification'],'native-session')
        self.assertEqual(row['evidence']['record_sha256'],digest(self.records/'accepted-runtime-40071768899.json'))
        self.assertEqual(row['evidence']['id'],a['id'])
        for key in ('original_observation','original_test_authorization'):
            self.assertEqual(digest(self.records/a[key]['file']),a[key]['sha256'])
        raw=json.loads((self.records/a['original_observation']['file']).read_bytes())
        pending=json.loads((self.records/a['original_test_authorization']['file']).read_bytes())
        self.assertEqual(raw['state'],'PASSED-FOCUSED-OBSERVATIONS')
        self.assertEqual(pending['state'],'PENDING-OBSERVATION');self.assertFalse(pending['qualified'])
        self.assertEqual(a['owned_cleanup']['absent'],19);self.assertEqual(a['owned_cleanup']['live_survivors'],0)
        self.assertLess(a['timings']['prelaunch_seconds'],5)
        self.assertLess(a['timings']['production_running_seconds_after_trial_start'],15)
        self.assertEqual(target['native_scope']['review_sha256'],digest(self.records/'native-scope-40071768899.json'))
        self.assertEqual(target['dependency_review']['record_sha256'],digest(self.records/'dependency-review-afaff752.json'))
        native=json.loads((self.records/'native-scope-40071768899.json').read_bytes())
        self.assertTrue(native['acceptance']['passed'])
        self.assertFalse(native['actual_r2']['cleanup_passed'])

    def test_current_helpers_and_payload_are_exact_tested_bytes_not_dev_version_admission(self):
        a=self.accepted
        for name in HELPERS:self.assertEqual(digest(P/'companion'/name),a['helper_hashes'][name],name)
        payload={name:dict(sha256=a['helper_hashes'][name],mode=0o755 if name in ('install.py','panel_launcher.py','selector.py') else 0o644) for name in HELPERS}
        payload['cosmic-panel-islands']=dict(sha256=a['tested_packages']['islands']['elf_sha256'],mode=0o755)
        self.assertEqual(identity(payload),a['payload_sha256'])
        self.assertEqual(self.row['payload_sha256'],a['payload_sha256'])
        # Development package versions belong to the immutable observation,
        # never the runtime target's installed package set (the OS dependencies).
        self.assertNotIn('cosmic-ext-halcyon',self.row['runtime_target']['packages'])
        self.assertNotIn('cosmic-ext-halcyon-islands',self.row['runtime_target']['packages'])
        self.assertNotIn(b'dev.integration',canonical(self.row['runtime']))
        self.assertEqual(len(self.row['runtime_compatibility']),1)
        self.assertEqual(self.row['runtime_compatibility'][0]['id'],runtime_id(self.row['runtime']))

if __name__=='__main__':unittest.main()
