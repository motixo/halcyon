"""Byte-binding unit tests. Mocked native approval NEVER produces qualification.

All artifacts exist only under TemporaryDirectory; no catalog row is invented.
The unmocked negative test proves a literal acceptance claim cannot ship.
"""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import archive
import build_archive
import promotion
from test_archive import enrollment, execute, key, package


class Admission(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='halcyon-admission-fixture-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.keyhome = self.root/'keys'
        _, public = key(self.keyhome)
        self.addCleanup(lambda: execute(['gpgconf', '--homedir', self.keyhome, '--kill', 'gpg-agent']))
        self.candidate = self.root/'candidate'
        self.candidate.mkdir()
        self.artifacts = {}
        debs = [package(self.candidate, name, '0.2.1~fixture') for name in archive.PACKAGES[:2]]
        # Build trees are not release evidence; keep only final fixture artifacts.
        import shutil
        for path in self.candidate.glob('*-tree'):
            shutil.rmtree(path)
        debs += enrollment.build(public, '1.0', self.candidate, 1700000000)
        for path, kind in zip(debs, ['main-deb', 'islands-deb', 'enrollment-deb', 'enrollment-deb']):
            self.artifacts[path.name] = dict(kind=kind, sha256=archive.sha(path))
        for name, kind, data in [('source.tar', 'corresponding-source', b'UNIT FIXTURE NOT SOURCE CLOSURE'),
                                  ('recipe.txt', 'build-recipe', b'UNIT FIXTURE NOT BUILD PROVENANCE')]:
            (self.candidate/name).write_bytes(data)
            self.artifacts[name] = dict(kind=kind, sha256=archive.sha(self.candidate/name))
        self.provenance = dict(format='halcyon-build-provenance', version=1,
            packages={path.name: dict(sha256=archive.sha(path), source_commit='a'*40,
                sources=['source.tar'], recipe='recipe.txt') for path in debs})
        (self.candidate/'native-evidence').mkdir()
        (self.candidate/'native-evidence/raw.txt').write_text('NO NATIVE OBSERVATION: UNIT FIXTURE')
        (self.candidate/'approval.json').write_text('{}')
        self.manifest = dict(format='halcyon-release', version=1, release='0.2.1~fixture',
                             islands_package=debs[1].name, artifacts=self.artifacts,
                             system_updates_acceptance='passed')
        self.refresh_review()

    def refresh_review(self):
        provenance = self.candidate/'build-provenance.json'
        provenance.write_bytes(archive.canonical(self.provenance))
        self.artifacts[provenance.name] = dict(kind='build-provenance', sha256=archive.sha(provenance))
        manifest = self.candidate/'release-manifest.json'
        manifest.write_bytes(archive.canonical(self.manifest))
        review = dict(format='halcyon-distribution-review', version=1, decision='approve',
                      review_reference='UNIT MOCK ONLY: NEVER APPROVED FOR DISTRIBUTION',
                      release_manifest_sha256=archive.sha(manifest),
                      native_evidence=archive.inventory(self.candidate/'native-evidence'),
                      native_approval_sha256=archive.sha(self.candidate/'approval.json'))
        path = self.candidate/'distribution-review.json'
        path.write_bytes(archive.canonical(review))
        self.review_sha = archive.sha(path)

    def test_unmocked_gate_rejects_literal_pass_without_native_evidence(self):
        with self.assertRaises((ValueError, FileNotFoundError)):
            build_archive.admit(self.candidate, self.review_sha)
        self.assertFalse((self.root/'out').exists())

    def test_review_pins_exact_main_companion_source_and_raw_native_evidence(self):
        with patch.object(build_archive.release_gate, 'promotion') as native:
            build_archive.admit(self.candidate, self.review_sha)
            native.assert_called_once()
            for name in [*self.artifacts, 'native-evidence/raw.txt', 'approval.json']:
                path = self.candidate/name
                original = path.read_bytes()
                path.write_bytes(original+b'changed')
                with self.subTest(name=name), self.assertRaises((ValueError, json.JSONDecodeError)):
                    build_archive.admit(self.candidate, self.review_sha)
                path.write_bytes(original)

    def test_source_binding_required_even_when_native_verifier_passes(self):
        binding = next(iter(self.provenance['packages'].values()))
        for change in [dict(source_commit='short'), dict(sources=[]), dict(recipe='source.tar'), dict(sha256='b'*64)]:
            original = copy.deepcopy(binding)
            binding.update(change)
            self.refresh_review()
            with patch.object(build_archive.release_gate, 'promotion'), self.assertRaises(ValueError):
                build_archive.admit(self.candidate, self.review_sha)
            binding.clear()
            binding.update(original)

    def test_promotion_revalidates_review_and_pool_without_rebuild(self):
        import time
        out = self.root/'unsigned'
        with patch.object(build_archive.release_gate, 'promotion'):
            digest = build_archive.build(self.candidate, self.review_sha, out, int(time.time())-60)
            promotion.revalidate(out, digest, self.review_sha)
            with self.assertRaisesRegex(ValueError, 'independent_review_pin_required'):
                promotion.revalidate(out, digest, 'b'*64)
            record = json.loads((out/'archive.json').read_bytes())['packages'][0]
            (out/record['path']).write_bytes(b'rebuilt after review')
            with self.assertRaisesRegex(ValueError, 'archive_bytes_changed'):
                promotion.revalidate(out, digest, self.review_sha)

    def test_symlinked_evidence_rejected(self):
        path = self.candidate/'native-evidence/escape'
        path.symlink_to('/etc/passwd')
        with self.assertRaisesRegex(ValueError, 'nonregular_archive_path'):
            build_archive.admit(self.candidate, self.review_sha)

    def test_secret_key_not_accepted_as_enrollment_material(self):
        private = self.root/'private-key-fixture'
        private.write_bytes(archive.run(['gpg', '--batch', '--homedir', str(self.keyhome),
                                        '--export-secret-keys']))
        private.chmod(0o600)
        with self.assertRaisesRegex(ValueError, 'public_keyring_required'):
            enrollment.build(private, '1.0', self.root/'rejected', 1700000000)

    def test_armored_key_rejected_for_binary_gpg_signed_by_path(self):
        armored = self.root/'armored.gpg'
        armored.write_bytes(archive.run(['gpg', '--batch', '--homedir', str(self.keyhome),
                                        '--armor', '--export']))
        with self.assertRaisesRegex(ValueError, 'binary_public_keyring_required'):
            enrollment.build(armored, '1.0', self.root/'rejected', 1700000000)

    def test_bootstrap_lintian_errors(self):
        for path in self.candidate.glob('halcyon-*.deb'):
            result = execute(['lintian', '--fail-on', 'error', path])
            self.assertEqual(result.returncode, 0, result.stdout.decode())


if __name__ == '__main__':
    unittest.main()
