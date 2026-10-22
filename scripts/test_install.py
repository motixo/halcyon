import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('install', Path(__file__).with_name('install.py'))
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class InstallTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.home = self.root / 'home'
        self.binary = self.root / 'cosmic-desktop-cards'
        self.build(b'release one')
        self.calls = []

    def build(self, contents):
        self.binary.write_bytes(contents)
        self.binary.chmod(0o755)

    def run_install(self, **options):
        return installer.install(self.binary, self.home,
                                 run=lambda args, check: self.calls.append(args), **options)

    def test_first_install_places_every_part_and_starts_nothing(self):
        result = self.run_install()
        self.assertTrue(result['first'])
        self.assertIsNone(result['backup'])
        app = self.home / '.local/bin/cosmic-desktop-cards'
        self.assertEqual(app.read_bytes(), b'release one')
        for path in (app, self.home / '.local/bin/desktop-cards',
                     self.home / '.local/share/cosmic-desktop-cards/calendar-sync.py',
                     self.home / '.local/share/cosmic-desktop-cards/project-status.py'):
            self.assertEqual(path.stat().st_mode & 0o777, 0o755, path)
        source = self.home / '.local/share/cosmic-desktop-cards/source'
        self.assertTrue((source / 'scripts/install_wallpapers.py').is_file())
        self.assertTrue((source / 'assets/wallpapers/teal-4k.png').is_file())
        self.assertFalse((source / 'target').exists())
        self.assertEqual(list(source.rglob('__pycache__')), [])
        self.assertEqual(len(list((self.home / 'Pictures/Desktop Cards').glob('*.png'))), 26)
        launcher = (self.home / '.local/share/applications' / installer.LAUNCHER).read_text()
        self.assertIn(f'Exec={self.home}/.local/bin/desktop-cards settings\n', launcher)
        unit = (self.home / '.config/systemd/user' / installer.SERVICE).read_text()
        self.assertIn('ExecStart=%h/.local/bin/cosmic-desktop-cards\n', unit)
        # Only a reload: never enabled or started by installing.
        self.assertEqual(self.calls, [['systemctl', '--user', 'daemon-reload']])
        self.assertFalse((self.home / '.config/cosmic-desktop-cards').exists())

    def test_upgrade_keeps_preferences_drafts_and_the_previous_build(self):
        self.run_install()
        config = self.home / '.config/cosmic-desktop-cards/config.json'
        config.parent.mkdir(parents=True)
        config.write_text(json.dumps({'theme': 'rose', 'future': 1}))
        data = self.home / '.local/share/cosmic-desktop-cards'
        private = {name: f'{name} contents' for name in
                   ('notes.json', 'notes-draft.json', 'desktop-undo.json', 'automation.json',
                    'profiles/mine.json')}
        for name, text in private.items():
            (data / name).parent.mkdir(parents=True, exist_ok=True)
            (data / name).write_text(text)
        (data / 'source/stale.txt').write_text('from an older release')
        self.calls.clear()
        self.build(b'release two')
        result = self.run_install(restart=True)
        self.assertFalse(result['first'])
        self.assertEqual((self.home / '.local/bin/cosmic-desktop-cards').read_bytes(), b'release two')
        self.assertEqual((result['backup'] / 'cosmic-desktop-cards').read_bytes(), b'release one')
        self.assertFalse((result['backup'] / 'desktop-cards').exists(), 'unchanged controller')
        self.assertEqual(json.loads(config.read_text()), {'theme': 'rose', 'future': 1})
        for name, text in private.items():
            self.assertEqual((data / name).read_text(), text, name)
        self.assertFalse((data / 'source/stale.txt').exists())
        self.assertEqual([path.name for path in data.iterdir() if path.name.startswith('.')], [])
        # The unit is unchanged, so only a running service is restarted.
        self.assertEqual(self.calls, [['systemctl', '--user', 'try-restart', installer.SERVICE]])

    def test_a_missing_build_changes_nothing(self):
        self.binary.unlink()
        with self.assertRaises(RuntimeError):
            self.run_install()
        self.assertFalse(self.home.exists())
        self.assertEqual(self.calls, [])

    def test_publisher_client_is_sanitized_private_and_preserved_on_upgrade(self):
        client = self.root / 'google.json'
        expected = {'client_id': 'sample.apps.googleusercontent.com', 'client_secret': 'fixture'}
        client.write_text(json.dumps({'installed': {**expected,
            'token_uri': 'https://example.invalid/token'}, 'unrelated': 'omit'}))
        self.run_install(google_oauth_client=client)
        installed = self.home / '.local/share/cosmic-desktop-cards/google-calendar-client.json'
        self.assertEqual(json.loads(installed.read_text()), {'installed': expected})
        self.assertEqual(installed.stat().st_mode & 0o777, 0o600)
        self.build(b'release two')
        self.run_install()
        self.assertEqual(json.loads(installed.read_text()), {'installed': expected})

    def test_invalid_publisher_client_changes_nothing(self):
        client = self.root / 'google.json'
        for raw in ('{}', '{"web":{}}', 'not json', 'x' * (64 * 1024 + 1)):
            client.write_text(raw)
            with self.assertRaises(RuntimeError):
                self.run_install(google_oauth_client=client)
            self.assertFalse(self.home.exists())
            self.assertEqual(self.calls, [])


if __name__ == '__main__':
    unittest.main()
