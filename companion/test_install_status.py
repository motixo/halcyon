import importlib.util
import json
import hashlib
import tempfile
import unittest
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
import panel_launcher

class InstallationStatus(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        spec = importlib.util.spec_from_file_location('install_status', Path(__file__).with_name('install_status.py'))
        self.status = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.status)
        self.library = self.root / 'lib'
        self.home = self.root / 'home'
        self.library.mkdir(); self.home.mkdir()

    def package(self):
        binary = self.library / 'cosmic-panel-islands'
        binary.write_bytes(b'fixture never executed'); binary.chmod(0o755)
        (self.library / 'companion-manifest.json').write_text(json.dumps({'upstream':'ca20ce3185b3b4404f429d59d429c40b58a2ddef','protocol': 1, 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'packages': panel_launcher.PACKAGES}))

    def test_package_never_enabled_disabled_incompatible_and_removed_leftovers(self):
        self.package()
        s = self.status.detect(self.home, self.library, True, panel_launcher.PACKAGES)
        self.assertEqual(s['installation'], 'installed'); self.assertEqual(s['provenance'], 'package')
        self.assertFalse(s['enabled']); self.assertTrue(s['compatible']); self.assertIsNone(s['running'])
        s = self.status.detect(self.home, self.library, True, {**panel_launcher.PACKAGES, 'cosmic-panel':'future'})
        self.assertEqual(s['installation'], 'installed'); self.assertFalse(s['compatible'])
        s = self.status.detect(self.home, self.library, False, panel_launcher.PACKAGES)
        self.assertEqual(s['installation'], 'absent')

    def test_verified_manual_record_required_and_restored_record_is_not_installation(self):
        bundle = self.home / '.local/share/cosmic-desktop-cards/companion'
        bundle.mkdir(parents=True)
        binary = bundle / 'cosmic-panel-islands'; binary.write_bytes(b'manual'); binary.chmod(0o755)
        manifest = bundle / 'manifest.json'; manifest.write_text(json.dumps({'upstream':'ca20ce3185b3b4404f429d59d429c40b58a2ddef','protocol':1, 'binary_sha256':hashlib.sha256(b'manual').hexdigest()}))
        selector = self.home / '.local/bin/cosmic-panel'; selector.parent.mkdir(parents=True); selector.write_bytes(b'selector'); selector.chmod(0o755)
        args = (self.home, self.library, False, {})
        self.assertEqual(self.status.detect(*args)['installation'], 'absent')
        record = {'root':str(self.home), 'complete':True, 'restored':False,
                  'installed':{str(p.relative_to(self.home)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (binary, manifest, selector)},
                  'modes':{str(p.relative_to(self.home)):p.stat().st_mode & 0o777 for p in (binary, manifest, selector)}}
        path = self.home / '.local/share/cosmic-desktop-cards/backups/panel-test/install-record.json'
        path.parent.mkdir(parents=True); path.write_text(json.dumps(record))
        self.assertEqual(self.status.detect(*args)['provenance'], 'manual')
        record['restored'] = True; path.write_text(json.dumps(record))
        self.assertEqual(self.status.detect(*args)['installation'], 'absent')

if __name__ == '__main__': unittest.main()
