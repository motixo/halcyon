"""Clock-independent restoration using real fixture files, never live hosts."""
import datetime
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import install
import journal
import refresh

loader = importlib.machinery.SourceFileLoader('journal_cli', str(
    Path(__file__).parents[1] / 'packaging/islands/halcyon-islands'))
spec = importlib.util.spec_from_loader(loader.name, loader)
cli = importlib.util.module_from_spec(spec)
loader.exec_module(cli)


class JournalOrder(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.home = Path(temporary.name)

    def layer(self, day, value, legacy=False):
        class Clock(datetime.datetime):
            @classmethod
            def now(cls, tz=None):
                return cls(2030, 1, day, tzinfo=datetime.timezone.utc)
        contents = {journal.SELECTOR: (b'fixture selector', 0o755),
                    journal.POINTER: (value, 0o600)}
        if not (self.home / journal.SELECTION).exists():
            contents[journal.SELECTION] = (b'{"enabled":true}', 0o600)
        with patch.object(install.datetime, 'datetime', Clock):
            record = install.install_contents(self.home, contents, journal.PACKAGE)
        path = Path(record['backup']) / 'install-record.json'
        if legacy:
            del record['journal_version'], record['previous']
            path.write_text(json.dumps(record))
        return path

    def assert_order(self, paths):
        active = refresh.records(self.home)
        self.assertEqual([p for p, _ in active], paths)
        self.assertEqual([p for p, _ in cli.records(self.home, install)], paths)
        refresh.verify_owned(self.home, active)

    def disable(self):
        for path, _ in cli.records(self.home, install):
            install.restore(path)
        self.assertFalse((self.home / journal.SELECTOR).exists())
        self.assertFalse((self.home / journal.POINTER).exists())

    def test_backwards_and_frozen_clock_restore_in_application_order(self):
        a = self.layer(3, b'A')
        b = self.layer(1, b'B')
        c = self.layer(1, b'C')
        self.assert_order([c, b, a])
        self.disable()
        self.assertEqual(refresh.records(self.home), [])
        d = self.layer(1, b'D')
        self.assert_order([d])
        self.disable()

    def test_legacy_dependencies_anchor_the_first_explicit_layer(self):
        a = self.layer(3, b'A', legacy=True)
        b = self.layer(1, b'B', legacy=True)
        self.assert_order([b, a])
        c = self.layer(2, b'C')
        self.assert_order([c, b, a])
        self.disable()

    def test_interrupted_restore_replays_even_after_all_files_were_restored(self):
        a = self.layer(3, b'A', legacy=True)
        b = self.layer(1, b'B', legacy=True)
        c = self.layer(2, b'C')
        original_place = install.place
        def interrupted(path, data, mode):
            if Path(path) == c:
                raise OSError('receipt not yet durable')
            return original_place(path, data, mode)
        with patch.object(install, 'place', side_effect=interrupted):
            with self.assertRaises(OSError):
                install.restore(c)
        self.assertEqual((self.home / journal.POINTER).read_bytes(), b'B')
        self.assertEqual([p for p, _ in refresh.records(self.home)], [c, b, a])
        self.disable()

    def test_broken_predecessor_refuses_before_restore_or_install_writes(self):
        self.layer(3, b'A')
        b = self.layer(1, b'B')
        record = json.loads(b.read_text())
        record['previous'] = None
        b.write_text(json.dumps(record))
        with self.assertRaises(ValueError):
            cli.records(self.home, install)
        with self.assertRaises(ValueError):
            self.layer(2, b'C')
        self.assertEqual((self.home / journal.POINTER).read_bytes(), b'B')

    def test_ambiguous_legacy_cycle_requires_repair_before_writing(self):
        self.layer(3, b'A', legacy=True)
        self.layer(1, b'B', legacy=True)
        self.layer(2, b'A', legacy=True)
        with self.assertRaisesRegex(ValueError, 'ambiguous_legacy'):
            cli.records(self.home, install)
        self.assertEqual((self.home / journal.POINTER).read_bytes(), b'A')


if __name__ == '__main__':
    unittest.main()
