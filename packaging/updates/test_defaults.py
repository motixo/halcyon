"""Exercise installation scripts on private roots, including interrupted writes."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('defaults', Path(__file__).with_name('defaults.py'))
d = importlib.util.module_from_spec(spec)
spec.loader.exec_module(d)
spec = importlib.util.spec_from_file_location('bootstrap', Path(__file__).with_name('build-deb.py'))
b = importlib.util.module_from_spec(spec)
spec.loader.exec_module(b)

class DefaultsTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        source, prefs = b.configuration()
        self.payload = {'source.sources': source.encode(), 'preferences': prefs.encode(),
                        'archive-keyring.gpg': Path(__file__).with_name('halcyon-archive-keyring.gpg').read_bytes()}
        for name, value in self.payload.items():
            self.put(d.BASE + '/' + name, value)

    def put(self, name, value):
        p = d.path(self.root, name);p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(value)

    def configure(self):
        with contextlib.redirect_stdout(io.StringIO()):
            d.configure(self.root)

    def test_fresh_install_is_offline_scoped_and_rerun_keeps_admin_choices(self):
        self.configure()
        for name, target in d.TARGETS.items():
            self.assertEqual(d.path(self.root, target).read_bytes(), self.payload[name])
        state = json.loads(d.path(self.root, d.STATE).read_bytes())
        self.assertEqual(state['phase'], 'complete')
        self.assertEqual(d.path(self.root, d.STATE).stat().st_mode & 0o777, 0o644)
        self.assertTrue(d.path(self.root, d.STATE).parent.stat().st_mode & 0o005)
        source = d.path(self.root, d.SOURCE)
        source.write_bytes(source.read_bytes() + b'Enabled: no\n')
        self.configure()
        self.assertTrue(source.read_bytes().endswith(b'Enabled: no\n'))
        source.unlink();self.configure()
        self.assertFalse(source.exists())

    def test_preexisting_disabled_modified_removed_and_bootstrap_states_are_untouched(self):
        cases = [d.SOURCE, d.PREFS, d.KEY, d.SOURCE + '.removed.abc',
                 '/etc/apt/sources.list.d/admin.sources',
                 '/var/lib/dpkg/info/halcyon-update-source.list']
        for target in cases:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                for name, data in self.payload.items():
                    p=d.path(root,d.BASE+'/'+name);p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data)
                file=d.path(root,target);file.parent.mkdir(parents=True,exist_ok=True)
                file.write_bytes(b'# disabled Halcyon source\n')
                d.configure(root)
                self.assertEqual(file.read_bytes(), b'# disabled Halcyon source\n')
                self.assertEqual(json.loads(d.path(root,d.STATE).read_bytes())['phase'],'preserved')
                for path in d.TARGETS.values():
                    if path != target:self.assertFalse(d.path(root,path).exists())

    def test_interrupted_install_resumes_without_overwriting_changes(self):
        write = d.write_new
        def interrupted(file, value, mode=0o644):
            if file == d.path(self.root,d.SOURCE):raise OSError('simulated crash')
            write(file,value,mode)
        with patch.object(d,'write_new',interrupted), self.assertRaises(OSError):self.configure()
        self.assertEqual(json.loads(d.path(self.root,d.STATE).read_bytes())['phase'],'pending')
        self.assertFalse(d.path(self.root,d.SOURCE).exists())
        self.configure()
        self.assertEqual(json.loads(d.path(self.root,d.STATE).read_bytes())['phase'],'complete')

    def test_pending_interruption_with_external_edit_refuses_replacement(self):
        hashes={d.TARGETS[name]:hashlib.sha256(data).hexdigest() for name,data in self.payload.items()}
        d.receipt(self.root,dict(format=d.FORMAT,version=1,phase='pending',files=hashes))
        self.put(d.KEY,b'admin key')
        self.configure()
        self.assertEqual(json.loads(d.path(self.root,d.STATE).read_bytes())['phase'],'conflict')
        self.assertEqual(d.path(self.root,d.KEY).read_bytes(),b'admin key')
        self.assertFalse(d.path(self.root,d.SOURCE).exists())

    def test_damaged_receipt_does_not_reenroll_or_block_package_configuration(self):
        for value in [b'', b'{}', b'[]', b'broken json']:
            self.put(d.STATE,value)
            self.configure()
            self.assertEqual(d.path(self.root,d.STATE).read_bytes(),value)
            self.assertFalse(d.path(self.root,d.SOURCE).exists())

    def test_remove_disables_owned_source_and_reinstall_keeps_it_disabled(self):
        self.configure();d.remove(self.root)
        self.assertFalse(d.path(self.root,d.SOURCE).exists())
        self.assertEqual(len(list(d.path(self.root,d.SOURCE).parent.glob('halcyon.sources.removed.*'))),1)
        self.configure();self.assertFalse(d.path(self.root,d.SOURCE).exists())
        d.remove(self.root,purge=True)
        for target in [d.KEY,d.PREFS,d.STATE]:self.assertFalse(d.path(self.root,target).exists())

    def test_remove_preserves_admin_files_and_later_bootstrap_adoption(self):
        self.configure();self.put(d.SOURCE,b'admin source')
        d.remove(self.root,purge=True)
        self.assertEqual(d.path(self.root,d.SOURCE).read_bytes(),b'admin source')
        self.assertTrue(d.path(self.root,d.KEY).exists())
        d.path(self.root,d.STATE).unlink(missing_ok=True)
        self.put('/var/lib/dpkg/info/halcyon-update-source.list',d.SOURCE.encode())
        self.configure();d.remove(self.root,purge=True)
        self.assertEqual(d.path(self.root,d.SOURCE).read_bytes(),b'admin source')

    def test_existing_symlink_is_never_followed_or_replaced(self):
        outside=self.root/'outside';outside.write_bytes(b'untouched')
        p=d.path(self.root,d.SOURCE);p.parent.mkdir(parents=True);p.symlink_to(outside)
        self.configure()
        self.assertTrue(p.is_symlink());self.assertEqual(outside.read_bytes(),b'untouched')

    def test_auditfix_purge_preserves_trust_for_renamed_and_duplicate_active_sources(self):
        self.configure()
        source = d.path(self.root, d.SOURCE)
        renamed = source.with_name('administrator.sources')
        source.rename(renamed)
        d.remove(self.root, purge=True)
        self.assertEqual(renamed.read_bytes(), self.payload['source.sources'])
        for target in (d.KEY, d.PREFS):
            self.assertTrue(d.path(self.root, target).exists())
        # A one-line source also retains both dependencies after canonical removal.
        renamed.unlink()
        d.path(self.root, d.STATE).unlink(missing_ok=True)
        for target in (d.KEY, d.PREFS):
            d.path(self.root, target).unlink()
        self.configure()
        other = source.with_name('administrator.list')
        other.write_text('deb [signed-by=' + d.KEY + '] https://profilaz3.github.io/halcyon/apt stable main\n')
        d.remove(self.root, purge=True)
        self.assertTrue(other.exists())
        for target in (d.KEY, d.PREFS):
            self.assertTrue(d.path(self.root, target).exists())

    def test_auditfix_disabled_other_source_does_not_require_trust(self):
        self.configure()
        source = d.path(self.root, d.SOURCE)
        source.with_name('administrator.sources').write_bytes(self.payload['source.sources'] + b'Enabled: no\n')
        d.remove(self.root, purge=True)
        self.assertFalse(d.path(self.root, d.KEY).exists())

    def test_auditfix_pending_retry_respects_renamed_disabled_source(self):
        original = d.receipt
        def interrupted(root, state):
            if state['phase'] == 'complete':
                raise OSError('final receipt failed')
            original(root, state)
        with patch.object(d, 'receipt', interrupted), self.assertRaises(OSError):
            self.configure()
        source = d.path(self.root, d.SOURCE)
        disabled = source.with_suffix('.sources.disabled')
        source.rename(disabled)
        self.configure()
        self.assertFalse(source.exists())
        self.assertEqual(disabled.read_bytes(), self.payload['source.sources'])
        self.assertEqual(json.loads(d.path(self.root, d.STATE).read_bytes())['phase'], 'preserved')

    def test_auditfix_publication_interruption_has_no_partial_destination(self):
        source = d.path(self.root, d.SOURCE)
        link = d.os.link
        def interrupted(staging, destination, **kwargs):
            if destination == source:
                self.assertEqual(Path(staging).read_bytes(), self.payload['source.sources'])
                raise OSError('killed before publication')
            link(staging, destination, **kwargs)
        with patch.object(d.os, 'link', interrupted), self.assertRaises(OSError):
            self.configure()
        self.assertFalse(source.exists())
        # An orphan from SIGKILL is ignored and cannot prevent a safe retry.
        source.with_name('.halcyon-enroll-orphan').write_bytes(b'')
        self.configure()
        self.assertEqual(source.read_bytes(), self.payload['source.sources'])
        self.assertEqual(json.loads(d.path(self.root, d.STATE).read_bytes())['phase'], 'complete')

    def test_auditfix_publication_never_replaces_a_concurrent_admin_file(self):
        target = self.root / 'concurrent'
        target.write_bytes(b'admin configuration')
        with self.assertRaises(FileExistsError):
            d.write_new(target, b'new configuration')
        self.assertEqual(target.read_bytes(), b'admin configuration')

if __name__ == '__main__':unittest.main()
