"""Final-artifact policy gate. Local repack fixtures, never native qualification."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from bundle_contract import HELPERS,canonical,digest
import practical_release as policy
import release_gate

class Release(unittest.TestCase):
    def test_explicit_final_approval_not_native_matrix_or_old_deb_reuse(self):
        receipt=os.environ.get('HALCYON_POLICY_TEST_RECEIPT')
        if not receipt:self.skipTest('Set retained integration.5 receipt; no network')
        record=json.loads(Path(receipt).read_bytes())
        with tempfile.TemporaryDirectory(prefix='halcyon-policy-package-fixture-') as work:
            root=Path(work)
            for kind in ('main','islands'):
                p=Path(record['packages'][kind]['deb_path'])
                self.assertEqual(digest(p),record['packages'][kind]['sha256'])
                subprocess.run(['dpkg-deb','-x',str(p),str(root/kind)],check=True,timeout=30)
            library=root/'islands/usr/lib/cosmic-ext-halcyon-islands'
            spec=importlib.util.spec_from_file_location('policy_builder',policy.ROOT/'packaging/islands/build-deb.py')
            builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)
            package=builder.build(library/'cosmic-panel-islands',library/'companion-manifest.json',
                '0.2.5~policy.fixture.1',root/'out',
                licenses=root/'islands/usr/share/doc/cosmic-ext-halcyon-islands/THIRD_PARTY_LICENSES.txt')
            app=root/'main/usr/lib/cosmic-ext-halcyon'
            for name in HELPERS:
                if name=='selector.py':continue
                shutil.copyfile(policy.ROOT/'companion'/name,app/('islands-status.py' if name=='install_status.py' else name))
            # Disposable packaging fixture, deliberately not a release main deb.
            control=root/'main/DEBIAN';control.mkdir()
            (control/'control').write_text('Package: cosmic-ext-halcyon\nVersion: 0.2.5~policy.fixture.1\nArchitecture: amd64\nMaintainer: Fixture <fixture@example.invalid>\nDescription: NOT RELEASE; policy gate fixture\n')
            main=root/'out/main.deb'
            subprocess.run(['dpkg-deb','--root-owner-group','--build',str(root/'main'),str(main)],check=True,timeout=30,stdout=subprocess.DEVNULL)
            root=root/'out';source=root/'source-fixture.txt';source.write_text('NOT CORRESPONDING SOURCE; SYNTHETIC POLICY GATE TEST ONLY')
            data=dict(format='halcyon-release',version=1,release='0.2.5',compatibility_policy=policy.POLICY['id'],
                islands_package=package.name,artifacts={p.name:dict(kind=k,sha256=digest(p)) for p,k in (
                    (package,'islands-deb'),(main,'main-deb'),(source,'corresponding-source'))})
            manifest=root/'release-manifest.json';manifest.write_bytes(canonical(data));expected=digest(manifest)
            request=policy.audit(root,data,expected)
            self.assertNotEqual(request['payload_sha256'],'f3bb6c779355b9fb4180a18c84dd7453e8d00b51720edc7376d8535bceb15635')
            approval=root/'approval.json';approval.write_bytes(canonical({**request,'decision':'PENDING'}))
            with self.assertRaises(ValueError):release_gate.verify(root,expected)
            accepted={**request,'decision':'approve','review_reference':'SYNTHETIC TEST ONLY'}
            approval.write_bytes(canonical(accepted));approval.chmod(0o644)
            with patch('release_gate.promotion',side_effect=AssertionError('must not manufacture full matrix')):
                self.assertEqual(release_gate.verify(root,expected),data)
            for key,value in [('artifacts',{}),('payload_sha256','0'*64),('historical_acceptance_sha256','0'*64),('limitations',[]),('release_manifest_sha256','0'*64)]:
                approval.write_bytes(canonical({**accepted,key:value}))
                with self.assertRaises(ValueError):policy.verify(root,data,expected)
            approval.write_bytes(canonical(accepted))
            # Repack outside reviewed byte inventory cannot inherit approval.
            with package.open('ab') as f:f.write(b'changed final archive')
            with self.assertRaisesRegex(ValueError,'artifact_mismatch'):release_gate.verify(root,expected)

if __name__=='__main__':unittest.main()
