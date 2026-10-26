"""Real CLI/Rust boundary with bounded subprocesses and private HOME/XDG.
Run with HALCYON_TEST_BINARY=/absolute/path/to/halcyon python3 -m unittest
scripts.test_controller_transaction. Never invokes a desktop service or GUI.
"""
import base64
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get('HALCYON_TEST_BINARY')
BOOT = '''import importlib.machinery, importlib.util, pathlib, sys
loader=importlib.machinery.SourceFileLoader('controller',sys.argv[1])
spec=importlib.util.spec_from_loader(loader.name,loader)
m=importlib.util.module_from_spec(spec);loader.exec_module(m)
m.BINARY=pathlib.Path(sys.argv[2]);sys.argv=[sys.argv[1],*sys.argv[3:]]
m.main()
'''

@unittest.skipUnless(BINARY, 'set HALCYON_TEST_BINARY to test the real Rust helper')
class TransactionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='halcyon-cli-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.home = self.root / 'home'
        self.home.mkdir()
        self.env = {k: os.environ[k] for k in ('PATH', 'LANG') if k in os.environ}
        self.env.update(HOME=str(self.home), XDG_CONFIG_HOME=str(self.home / '.config'),
                        XDG_DATA_HOME=str(self.home / '.local/share'), XDG_STATE_HOME=str(self.home / '.local/state'),
                        XDG_CACHE_HOME=str(self.home / '.cache'), XDG_RUNTIME_DIR=str(self.root / 'runtime'),
                        XDG_DATA_DIRS=str(self.root / 'system'), HALCYON_PREFIX=str(self.root / 'prefix'),
                        DBUS_SESSION_BUS_ADDRESS='unix:path=' + str(self.root / 'no-session-bus'),
                        DBUS_SYSTEM_BUS_ADDRESS='unix:path=' + str(self.root / 'no-system-bus'))
        (self.root / 'runtime').mkdir(mode=0o700)
        self.config = self.home / '.config/halcyon/config.json'
        self.native = self.home / '.config/cosmic'
        self.manifest = self.home / '.local/share/halcyon/desktop-recovery/original/manifest.json'
        self.wallpapers = self.root / 'prefix/share/backgrounds/halcyon'
        self.wallpapers.mkdir(parents=True)
        for name in ('rose', 'teal', 'ocean'):
            shutil.copyfile(ROOT / 'assets/wallpapers' / (name + '.jpg'), self.wallpapers / (name + '.jpg'))

    def cmd(self, *args):
        return [sys.executable, '-c', BOOT, str(ROOT / 'scripts/desktop-cards'), str(Path(BINARY).resolve()), *args]

    def run_cli(self, *args, success=True):
        result = subprocess.run(self.cmd(*args), env=self.env, text=True, capture_output=True, timeout=20)
        self.assertEqual(result.returncode == 0, success, result.stderr)
        return result

    def test_fresh_palette_and_wallpaper_collection_are_captured_before_app_write(self):
        collection = self.native / 'com.system76.CosmicSettings.Wallpaper/v1/custom-images'
        collection.parent.mkdir(parents=True)
        collection.write_bytes(b'["/private fixture/wall.png",]\xff\n')
        self.run_cli('theme', 'rose')
        info = json.loads(self.manifest.read_text())
        self.assertEqual(info['kind'], 'original_desktop')
        self.assertFalse(info['app_config_present'])
        self.assertEqual(json.loads(self.config.read_text())['theme'], 'rose')
        captured = next(r for d in info['domains'] for r in d['native'] if r['namespace'] == 'com.system76.CosmicSettings.Wallpaper')
        self.assertEqual(base64.b64decode(captured['bytes']), collection.read_bytes())
        self.assertIn('rose.jpg', (self.native / 'com.system76.CosmicBackground/v1/all').read_text())

    def test_auditfix_controller_help_preserves_legacy_first_launch_migration(self):
        legacy = self.home / '.config/cosmic-desktop-cards/config.json'
        legacy.parent.mkdir(parents=True)
        original = b'{"theme":"ocean","appearance":{"opacity":71}}\n'
        legacy.write_bytes(original)
        self.run_cli('--help')
        self.assertFalse(self.config.parent.exists())
        self.run_cli('cosmic-colors', 'on')
        self.assertEqual(json.loads(self.config.read_bytes())['appearance']['opacity'], 71)
        self.assertEqual(legacy.read_bytes(), original)

    def test_auditfix_python_writer_migrates_before_creating_config(self):
        legacy = self.home / '.config/cosmic-desktop-cards/config.json'
        legacy.parent.mkdir(parents=True)
        legacy.write_text('{"theme":"rose","appearance":{"opacity":71}}')
        # Exercise the same edit path as weather without reaching its network lookup.
        boot = BOOT.rsplit('m.main()', 1)[0] + 'm.edit_config(lambda data: data.update(location={"name":"Fixture"}))\n'
        result = subprocess.run([sys.executable, '-c', boot, str(ROOT/'scripts/desktop-cards'),
                                 str(Path(BINARY).resolve())], env=self.env, capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(self.config.read_bytes())
        self.assertEqual(data['theme'], 'rose')
        self.assertEqual(data['appearance']['opacity'], 71)
        self.assertEqual(data['location']['name'], 'Fixture')

    def test_cosmic_colors_on_captures_false_preference_and_palette_only_keeps_other_leaves(self):
        self.config.parent.mkdir(parents=True)
        self.config.write_text('{"theme":"teal","appearance":{"match_wallpaper":false,"match_cosmic_colors":false,"opacity":83},"private_token":"not exported"}')
        self.run_cli('cosmic-colors', 'on')
        before = self.manifest.read_bytes()
        self.assertNotIn(b'not exported', before)
        self.run_cli('theme', 'ocean')
        self.run_cli('cosmic-colors', 'off')
        data = json.loads(self.config.read_text())
        self.assertEqual(data['theme'], 'ocean')
        self.assertEqual(data['appearance']['opacity'], 83)
        self.assertEqual(data['private_token'], 'not exported')
        self.assertFalse(data['appearance']['match_cosmic_colors'])
        self.assertEqual(self.manifest.read_bytes(), before)
        self.assertFalse((self.native / 'com.system76.CosmicBackground/v1/all').exists())

    def test_failed_capture_does_not_change_preferences_or_native_keys(self):
        self.config.parent.mkdir(parents=True)
        self.config.write_bytes(b'{"theme":"teal"}\n')
        self.manifest.parent.mkdir(parents=True)
        self.manifest.write_bytes(b'broken baseline')
        for args in [('theme', 'rose'), ('cosmic-colors', 'on')]:
            self.run_cli(*args, success=False)
            self.assertEqual(self.config.read_bytes(), b'{"theme":"teal"}\n')
            self.assertFalse(self.native.exists())
            self.assertEqual(self.manifest.read_bytes(), b'broken baseline')

    def test_concurrent_first_cli_writes_finish_without_lock_recursion(self):
        processes = [subprocess.Popen(self.cmd('theme', name), env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE) for name in ('rose', 'teal')]
        results = []
        try:
            for process in processes:
                out, err = process.communicate(timeout=20)
                results.append((process.returncode, err))
        finally:
            for process in processes:
                if process.poll() is None:
                    process.kill(); process.wait()
        self.assertTrue(any(code == 0 for code, _ in results), results)
        info = json.loads(self.manifest.read_text())
        self.assertFalse(info['app_config_present'])
        self.assertEqual(info['kind'], 'original_desktop')
        self.assertIn(json.loads(self.config.read_text())['theme'], ('rose', 'teal'))

    def test_preview_writes_only_the_alternate_config(self):
        alternate = self.root / 'preview.json'
        self.env['DESKTOP_CARDS_CONFIG'] = str(alternate)
        self.run_cli('theme', 'rose')
        self.run_cli('cosmic-colors', 'on')
        self.assertEqual(json.loads(alternate.read_text())['theme'], 'rose')
        self.assertFalse(self.config.exists())
        self.assertFalse(self.native.exists())
        self.assertFalse(self.manifest.exists())

    def test_original_wallpaper_restore_captures_before_the_restore_write(self):
        self.config.parent.mkdir(parents=True)
        self.config.write_text('{"theme":"teal"}')
        native = self.native / 'com.system76.CosmicBackground/v1/all'
        native.parent.mkdir(parents=True)
        old = b'(output:"all",source:Color(Single((0.1,0.2,0.3))),filter_by_theme:false,rotation_frequency:300)'
        saved = b'(output:"all",source:Color(Single((0.3,0.2,0.1))),filter_by_theme:false,rotation_frequency:300)'
        native.write_bytes(old)
        (self.config.parent / 'original-background.ron').write_bytes(saved)
        self.run_cli('restore-wallpaper')
        info = json.loads(self.manifest.read_text())
        captured = next(r for d in info['domains'] for r in d['native'] if r['namespace'] == 'com.system76.CosmicBackground' and base64.b64decode(r['name']) == b'all')
        self.assertEqual(base64.b64decode(captured['bytes']), old)
        self.assertEqual(native.read_bytes(), saved)
        self.assertEqual(json.loads(self.config.read_text())['theme'], 'teal')

    def test_lock_links_refuse_before_native_or_app_mutation(self):
        for which in ('config', 'desktop'):
            for kind in ('dangling-leaf', 'ancestor', 'dangling-ancestor', 'fifo'):
                with self.subTest(lock=which, kind=kind):
                    # Every subcase gets wholly separate HOME/XDG paths.
                    self.setUp()
                    self.config.parent.mkdir(parents=True)
                    original = b'{"theme":"teal","appearance":{"match_wallpaper":false}}\n'
                    self.config.write_bytes(original)
                    target = self.native / 'com.system76.CosmicPanel.Dock/v1/opacity'
                    target.parent.mkdir(parents=True)
                    lock = Path(str(self.config) + '.lock') if which == 'config' else self.manifest.parents[2] / 'desktop.lock'
                    lock.parent.mkdir(parents=True, exist_ok=True)
                    outside = self.root / 'outside'
                    outside.mkdir()
                    if kind == 'dangling-leaf':
                        lock.symlink_to(target)
                    elif kind == 'fifo':
                        os.mkfifo(lock)
                    else:
                        if which == 'config':
                            self.config.rename(outside / 'config.json')
                        lock.parent.rmdir()
                        lock.parent.symlink_to(outside if kind == 'ancestor' else outside / 'absent', target_is_directory=True)
                    def outside_tree():
                        return {p.relative_to(outside): None if p.is_dir() else p.read_bytes()
                                for p in outside.rglob('*')}
                    def native_tree():
                        return {p.relative_to(self.native): p.read_bytes()
                                for p in self.native.rglob('*') if p.is_file()}
                    before = outside_tree()
                    native_before = native_tree()
                    self.run_cli('theme', 'rose', success=False)
                    self.assertFalse(target.exists())
                    self.assertFalse(self.manifest.exists())
                    self.assertEqual(outside_tree(), before)
                    self.assertEqual(native_tree(), native_before)
                    if self.config.is_file():
                        self.assertEqual(self.config.read_bytes(), original)
