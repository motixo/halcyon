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
    def test_fresh_native_acceptance_requires_reviewed_source_and_exact_artifacts(self):
        # Schema/byte-binding test only. This fixture is never native evidence.
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary); proof=root/'native.zip'; proof.write_bytes(b'SYNTHETIC TEST ONLY')
            bindings=dict(packages={'main':{'sha256':'1'*64,'elf_sha256':'2'*64},
                                    'islands':{'sha256':'3'*64,'elf_sha256':'4'*64}},
                          payload_sha256='5'*64,catalog_sha256='6'*64,helpers={'fixture.py':'7'*64})
            data=dict(release='0.3.0',artifacts={'native.zip':dict(kind='native-evidence',sha256=digest(proof))})
            record=dict(format='halcyon-final-package-native-acceptance',version=1,
                        decision='approve-focused-native',release='0.3.0',
                        policy_sha256=policy.identity(policy.POLICY),bindings=bindings,
                        limitations=policy.FINAL_LIMITATIONS,
                        scope=dict(environment='complete-login-session',cosmic_family='1.11',
                                   graphics='virtio-llvmpipe',image_sha256='8'*64),
                        observations={k:'passed' for k in ('installation','settings','startup','input','cleanup','stock-recovery','relogin')},
                        evidence=dict(file='native.zip',sha256=digest(proof)),review_reference='SYNTHETIC TEST ONLY')
            accepted=root/'maintainer-record.json'
            with patch.dict(policy.FINAL_ACCEPTANCES,{'0.3.0':accepted},clear=True):
                with self.assertRaisesRegex(ValueError,'acceptance_pending'):
                    policy.final_native_acceptance(root,data,bindings)
                accepted.write_bytes(canonical(record))
                result=policy.final_native_acceptance(root,data,bindings)
                self.assertEqual(result['final_native_acceptance_sha256'],digest(accepted))
                self.assertEqual(result['final_native_evidence_sha256'],digest(proof))
                changes=[('decision','PENDING'),('release','0.2.5'),('policy_sha256','0'*64),
                         ('bindings',{}),('limitations',[]),('scope',{}),('observations',{}),
                         ('evidence',{}),('review_reference',' ')]
                for key,value in changes:
                    with self.subTest(key=key):
                        accepted.write_bytes(canonical({**record,key:value}))
                        with self.assertRaises(ValueError):policy.final_native_acceptance(root,data,bindings)
                for name in record['observations']:
                    accepted.write_bytes(canonical({**record,'observations':{**record['observations'],name:'not-run'}}))
                    with self.assertRaisesRegex(ValueError,'observations_incomplete'):
                        policy.final_native_acceptance(root,data,bindings)
                accepted.write_bytes(canonical(record));proof.write_bytes(b'changed raw evidence')
                with self.assertRaisesRegex(ValueError,'evidence_mismatch'):
                    policy.final_native_acceptance(root,data,bindings)

    def test_auditfix_published_renewal_is_hash_bound_and_separate_from_admission(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            records = {}
            for name, kind in [('main.deb', 'main-deb'), ('islands.deb', 'islands-deb'),
                               ('source.tar', 'corresponding-source'), ('evidence.zip', 'native-evidence')]:
                (root/name).write_bytes(('fixture ' + name).encode())
                records[name] = dict(kind=kind, sha256=digest(root/name))
            data = dict(format='halcyon-release', version=1, release='0.2.5',
                        compatibility_policy='cosmic-1.10-1.11-startup-v1',
                        islands_package='islands.deb', artifacts=records)
            (root/'release-manifest.json').write_bytes(canonical(data))
            (root/'approval.json').write_bytes(b'fixture historical approval')
            expected = digest(root/'release-manifest.json')
            pins = {expected: dict(release=data['release'], policy=data['compatibility_policy'],
                                   approval_sha256=digest(root/'approval.json'))}
            with patch.object(release_gate, 'PUBLISHED_RELEASES', pins):
                self.assertEqual(release_gate.verify_published(root, expected), data)
                # Even pinned publications cannot enter new-artifact admission.
                with self.assertRaisesRegex(ValueError, 'unknown_release_policy'):
                    release_gate.verify(root, expected)
                for name in records:
                    original = (root/name).read_bytes()
                    (root/name).write_bytes(b'changed')
                    with self.assertRaisesRegex(ValueError, 'artifact_mismatch'):
                        release_gate.verify_published(root, expected)
                    (root/name).write_bytes(original)
                (root/'approval.json').write_bytes(b'changed approval')
                with self.assertRaisesRegex(ValueError, 'published_release_approval_mismatch'):
                    release_gate.verify_published(root, expected)
                data['release'] = '0.3.0'
                (root/'release-manifest.json').write_bytes(canonical(data))
                with self.assertRaisesRegex(ValueError, 'reviewed_manifest_digest_mismatch'):
                    release_gate.verify_published(root, expected)
                with self.assertRaisesRegex(ValueError, 'unknown_release_policy'):
                    release_gate.verify_published(root, digest(root/'release-manifest.json'))

    def test_exact_approval_fields_and_decision_are_required(self):
        # Approval mechanics only; this synthetic request is not native evidence.
        required = dict(format='halcyon-practical-release-approval', version=1,
                        policy_id=policy.POLICY['id'], policy_sha256='1'*64,
                        release_manifest_sha256='2'*64, payload_sha256='3'*64,
                        catalog_sha256='4'*64, historical_acceptance_sha256='5'*64,
                        artifacts={'main.deb':'6'*64,'islands.deb':'7'*64},
                        limitations=['SYNTHETIC TEST ONLY'])
        with tempfile.TemporaryDirectory() as temporary, patch.object(policy,'audit',return_value=required):
            root=Path(temporary); approval=root/'approval.json'
            accepted={**required,'decision':'approve','review_reference':'SYNTHETIC TEST ONLY'}
            approval.write_bytes(canonical(accepted))
            policy.verify(root,{},required['release_manifest_sha256'])
            for key,value in [('artifacts',{}),('policy_id','old-policy'),('policy_sha256','0'*64),
                              ('payload_sha256','0'*64),('catalog_sha256','0'*64),
                              ('historical_acceptance_sha256','0'*64),('limitations',[]),
                              ('release_manifest_sha256','0'*64),('decision','PENDING'),
                              ('review_reference',' ')]:
                with self.subTest(key=key):
                    approval.write_bytes(canonical({**accepted,key:value}))
                    with self.assertRaisesRegex(ValueError,'not_explicitly_approved'):
                        policy.verify(root,{},required['release_manifest_sha256'])

    def test_dispatch_uses_current_policy_and_checks_immutable_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifacts = {}
            for name, kind in [('main.deb', 'main-deb'), ('islands.deb', 'islands-deb'), ('source.tar', 'corresponding-source')]:
                (root/name).write_bytes(b'synthetic dispatch fixture')
                artifacts[name] = dict(kind=kind, sha256=digest(root/name))
            data = dict(format='halcyon-release', version=1, release='0.3.0',
                        compatibility_policy=policy.POLICY['id'], islands_package='islands.deb', artifacts=artifacts)
            manifest = root/'release-manifest.json'
            def write():
                manifest.write_bytes(canonical(data))
                return digest(manifest)
            expected = write()
            with patch.object(policy, 'verify') as verify, patch.object(release_gate, 'promotion', side_effect=AssertionError('wrong verifier')):
                self.assertEqual(release_gate.verify(root, expected), data)
                verify.assert_called_once_with(root, data, expected)
                verify.side_effect = ValueError('practical_release_not_explicitly_approved')
                with self.assertRaisesRegex(ValueError, 'not_explicitly_approved'):
                    release_gate.verify(root, expected)
                verify.reset_mock()
                for wrong in ('cosmic-1.10-1.11-startup-v1', 'unknown-policy'):
                    data['compatibility_policy'] = wrong
                    with self.assertRaisesRegex(ValueError, 'unknown_release_policy'):
                        release_gate.verify(root, write())
                verify.assert_not_called()
                data['compatibility_policy'] = policy.POLICY['id']; expected = write()
                (root/'islands.deb').write_bytes(b'changed archive')
                with self.assertRaisesRegex(ValueError, 'artifact_mismatch'):
                    release_gate.verify(root, expected)
                verify.assert_not_called()
                with self.assertRaisesRegex(ValueError, 'reviewed_manifest_digest_mismatch'):
                    release_gate.verify(root, '0'*64)

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
            if digest(library/'cosmic-panel-islands') != policy.POLICY['binary_sha256']:
                # integration.5 predates startup-v2's native change. It cannot
                # be repackaged under today's policy or provide positive native
                # approval. Exercise that rejection without rewriting its pins.
                with self.assertRaisesRegex(ValueError,'startup_native_source_or_interface'):
                    builder.build(library/'cosmic-panel-islands',library/'companion-manifest.json',
                        '0.2.5~policy.fixture.1',root/'out',
                        licenses=root/'islands/usr/share/doc/cosmic-ext-halcyon-islands/THIRD_PARTY_LICENSES.txt')
                self.assertEqual(list((root/'out').glob('*.deb')),[])
                return
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
