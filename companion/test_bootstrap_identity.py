"""Bootstrap fallback must recognize the same session as the verified launcher."""
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import panel_launcher
import selector


class BootstrapIdentity(unittest.TestCase):
    def test_socket_aliases_do_not_start_a_second_host(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            proc = root / 'proc'
            actor = proc / '99999999'
            actor.mkdir(parents=True)
            (actor / 'exe').symlink_to('/usr/bin/cosmic-panel')
            runtime = root / 'runtime'
            runtime.mkdir()
            relative = {'XDG_RUNTIME_DIR': str(runtime), 'WAYLAND_DISPLAY': 'wayland-1'}
            absolute = {'WAYLAND_DISPLAY': str(runtime / 'wayland-1')}
            default = {'XDG_RUNTIME_DIR': str(runtime)}
            explicit_default = {**default, 'WAYLAND_DISPLAY': 'wayland-0'}
            other = {**relative, 'WAYLAND_DISPLAY': 'wayland-2'}
            cases = [(relative, absolute, True), (absolute, relative, True),
                     (default, explicit_default, True), (explicit_default, default, True),
                     (relative, other, False)]
            def fixture_path(value):
                return proc if value == '/proc' else Path(value)
            for ours, theirs, matches in cases:
                with self.subTest(ours=ours, theirs=theirs):
                    ours = {'XDG_RUNTIME_DIR': str(runtime), **ours}
                    (actor / 'environ').write_bytes(b'\0'.join(
                        f'{key}={value}'.encode() for key, value in theirs.items()))
                    with patch.dict(os.environ, ours, clear=True), \
                         patch.object(selector, 'Path', side_effect=fixture_path), \
                         patch.object(panel_launcher, 'Path', side_effect=fixture_path), \
                         patch.object(selector, 'OwnedHost') as host:
                        host.return_value.poll.return_value = 0
                        host.return_value.finish.return_value = 0
                        self.assertEqual(panel_launcher.other_host(), 99999999 if matches else None)
                        self.assertEqual(selector.fallback(), 0)
                        self.assertEqual(host.call_count, 0 if matches else 1)


if __name__ == '__main__':
    unittest.main()
