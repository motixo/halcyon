"""Panel selection and restoration without using the live desktop."""
import importlib.util
import json
import os
import socket
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

COMPANION = Path(__file__).resolve().parent.parent / 'companion'
sys.path.insert(0, str(COMPANION))
import panel_launcher as launcher
spec = importlib.util.spec_from_file_location('panel_installer', COMPANION / 'install.py')
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class CompanionPackaging(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.home = self.root / 'home'
        self.binary = self.root / 'binary'
        self.binary.write_bytes(b'fixture panel')
        self.manifest = self.root / 'manifest.json'
        self.manifest.write_text(json.dumps({'protocol': 1, 'upstream': 'ca20ce3185b3b4404f429d59d429c40b58a2ddef', 'binary_sha256': launcher.digest(self.binary)}))

    def test_stage_restore_preserves_preferences_and_previous_launcher(self):
        old = self.home / '.local/bin/cosmic-panel'
        old.parent.mkdir(parents=True)
        old.write_text('previous launcher')
        prefs = self.home / '.config/cosmic/panel'
        prefs.parent.mkdir(parents=True)
        prefs.write_text('original preferences')
        result = installer.install(self.home, self.binary, self.manifest)
        bundle = self.home / '.local/share/cosmic-desktop-cards/companion'
        self.assertFalse(json.loads((bundle / 'selection.json').read_text())['enabled'])
        self.assertTrue(launcher.compatible(bundle, launcher.PACKAGES))
        self.assertFalse(launcher.compatible(bundle, {**launcher.PACKAGES, 'cosmic-panel': 'new version'}))
        installer.restore(Path(result['backup']) / 'install-record.json')
        self.assertEqual(old.read_text(), 'previous launcher')
        self.assertEqual(prefs.read_text(), 'original preferences')
        self.assertFalse((bundle / 'cosmic-panel-islands').exists())

    def test_valid_json_with_wrong_manifest_shape_falls_back(self):
        installer.install(self.home, self.binary, self.manifest)
        bundle = self.home / '.local/share/cosmic-desktop-cards/companion'
        for value in [None, [], 1, 'invalid']:
            with self.subTest(value=value):
                (bundle / 'manifest.json').write_text(json.dumps(value))
                self.assertFalse(launcher.compatible(bundle, launcher.PACKAGES))

    def test_invalid_selection_shape_starts_stock(self):
        installer.install(self.home, self.binary, self.manifest)
        bundle = self.home / '.local/share/cosmic-desktop-cards/companion'
        runtime = self.root / 'runtime'
        runtime.mkdir(mode=0o700)
        code = ('import sys; from pathlib import Path; '
                + 'sys.path.insert(0, ' + repr(str(COMPANION)) + '); '
                + 'import panel_launcher as p; p.STOCK=Path("/bin/true"); '
                + 'p.other_host=lambda: None; p.prepare_launch=lambda bundle: (lambda child: True); sys.exit(p.main())')
        env = {**os.environ, 'HOME': str(self.home), 'XDG_DATA_HOME': str(self.home / '.local/share'),
               'XDG_STATE_HOME': str(self.home / '.local/state'), 'XDG_RUNTIME_DIR': str(runtime),
               'WAYLAND_DISPLAY': str(runtime / 'isolated-test-display')}
        for name in ['PANEL_NOTIFICATIONS_FD', 'WAYLAND_SOCKET']:
            env.pop(name, None)
        for value in [None, [], 1, 'invalid']:
            with self.subTest(value=value):
                (bundle / 'selection.json').write_text(json.dumps(value))
                result = subprocess.run([sys.executable, '-c', code], env=env, capture_output=True, timeout=5)
                self.assertEqual(result.returncode, 0, result.stderr.decode())

    def test_enable_next_login_keeps_restarts_in_the_current_session_on_stock(self):
        marker = self.root / 'companion-started'
        self.binary.write_text('#!/usr/bin/python3\nfrom pathlib import Path\nPath(' + repr(str(marker)) + ').write_text("started")\n')
        self.manifest.write_text(json.dumps({'protocol': 1, 'upstream': 'ca20ce3185b3b4404f429d59d429c40b58a2ddef', 'binary_sha256': launcher.digest(self.binary)}))
        runtime = self.root / 'runtime'
        runtime.mkdir(mode=0o700)
        env = {**os.environ, 'HOME': str(self.home), 'XDG_DATA_HOME': str(self.home / '.local/share'),
               'XDG_STATE_HOME': str(self.home / '.local/state'), 'XDG_RUNTIME_DIR': str(runtime),
               'WAYLAND_DISPLAY': str(runtime / 'isolated-test-display'), 'XDG_SESSION_ID': 'review-current'}
        for name in ['PANEL_NOTIFICATIONS_FD', 'WAYLAND_SOCKET']:
            env.pop(name, None)
        subprocess.run([sys.executable, str(COMPANION / 'install.py'), '--root', str(self.home),
                        '--binary', str(self.binary), '--manifest', str(self.manifest), '--enable-next-login'],
                       env=env, capture_output=True, check=True)
        # Isolate session deferral from host package and compositor admission.
        code = ('import sys; from pathlib import Path; '
                + 'sys.path.insert(0, ' + repr(str(COMPANION)) + '); '
                + 'import panel_launcher as p; p.STOCK=Path("/bin/true"); '
                + 'p.other_host=lambda: None; p.prepare_launch=lambda bundle: (lambda child: True); '
                + 'p.runtime_compatible=lambda bundle: True; sys.exit(p.main())')
        subprocess.run([sys.executable, '-c', code], env=env, capture_output=True, check=True, timeout=5)
        self.assertFalse(marker.exists(), 'staging activated the companion in the same login')
        env['XDG_SESSION_ID'] = 'review-next'
        subprocess.run([sys.executable, '-c', code], env=env, capture_output=True, check=True, timeout=5)
        self.assertEqual(marker.read_text(), 'started')

    def test_tamper_and_restore_conflict_are_refused(self):
        result = installer.install(self.home, self.binary, self.manifest, True)
        bundle = self.home / '.local/share/cosmic-desktop-cards/companion'
        (bundle / 'cosmic-panel-islands').write_bytes(b'external change')
        self.assertFalse(launcher.compatible(bundle, launcher.PACKAGES))
        with self.assertRaises(ValueError):
            installer.restore(Path(result['backup']) / 'install-record.json')
        self.assertEqual((bundle / 'cosmic-panel-islands').read_bytes(), b'external change')

    def test_partial_install_restores_written_paths(self):
        real = installer.place
        def fail(path, data, mode):
            if path.name == 'manifest.json':
                raise OSError('injected write failure')
            return real(path, data, mode)
        with patch.object(installer, 'place', fail):
            with self.assertRaises(OSError):
                installer.install(self.home, self.binary, self.manifest)
        self.assertFalse((self.home / '.local/share/cosmic-desktop-cards/companion/cosmic-panel-islands').exists())
        self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())

    def test_interruption_after_atomic_replace_restores_the_original(self):
        original = self.home / '.local/share/cosmic-desktop-cards/companion/cosmic-panel-islands'
        original.parent.mkdir(parents=True)
        original.write_bytes(b'previous panel')
        original.chmod(0o750)
        real = installer.place
        interrupt = True
        def fail_after_replace(path, data, mode):
            nonlocal interrupt
            real(path, data, mode)
            if path == original and interrupt:
                interrupt = False
                raise KeyboardInterrupt('interrupted after replacement')
        with patch.object(installer, 'place', fail_after_replace):
            with self.assertRaises(KeyboardInterrupt):
                installer.install(self.home, self.binary, self.manifest)
        self.assertEqual(original.read_bytes(), b'previous panel')
        self.assertEqual(original.stat().st_mode & 0o777, 0o750)
        self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())

    def test_restore_refuses_mode_changes_before_touching_any_file(self):
        result = installer.install(self.home, self.binary, self.manifest)
        selector = self.home / '.local/bin/cosmic-panel'
        selector.chmod(0o700)
        with self.assertRaises(ValueError):
            installer.restore(Path(result['backup']) / 'install-record.json')
        self.assertEqual(selector.stat().st_mode & 0o777, 0o700)
        self.assertTrue((self.home / '.local/share/cosmic-desktop-cards/companion/cosmic-panel-islands').exists())

    def test_restore_checks_every_backup_before_changing_any_file(self):
        selector = self.home / '.local/bin/cosmic-panel'
        selector.parent.mkdir(parents=True)
        selector.write_text('old selector')
        result = installer.install(self.home, self.binary, self.manifest)
        saved = self.home / result['before']['.local/bin/cosmic-panel']
        saved.write_text('corrupted backup')
        current = selector.read_bytes()
        with self.assertRaises(ValueError):
            installer.restore(Path(result['backup']) / 'install-record.json')
        self.assertEqual(selector.read_bytes(), current)
        self.assertTrue((self.home / '.local/share/cosmic-desktop-cards/companion/cosmic-panel-islands').exists())

    def test_restore_accepts_original_absence_in_a_completed_install(self):
        result = installer.install(self.home, self.binary, self.manifest)
        selector = self.home / '.local/bin/cosmic-panel'
        selector.unlink()
        installer.restore(Path(result['backup']) / 'install-record.json')
        self.assertFalse((self.home / '.local/share/cosmic-desktop-cards/companion/cosmic-panel-islands').exists())

    def test_restore_resumes_after_every_file_and_final_record_failure(self):
        prefix = '.local/share/cosmic-desktop-cards/companion/'
        paths = [prefix + 'cosmic-panel-islands', prefix + 'manifest.json',
                 '.local/bin/cosmic-panel', prefix + 'selection.json']
        for originals in [paths, paths[::2], []]:
            for boundary in range(1, 6):
                with self.subTest(originals=originals, boundary=boundary), tempfile.TemporaryDirectory() as temporary:
                    home = Path(temporary)
                    for rel in originals:
                        path = home / rel
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_bytes(('original ' + rel).encode())
                        path.chmod(0o640)
                    result = installer.install(home, self.binary, self.manifest)
                    record = Path(result['backup']) / 'install-record.json'
                    real_sync, real_place = installer.sync_directory, installer.place
                    steps = 0
                    def interrupt(directory):
                        nonlocal steps
                        real_sync(directory)
                        steps += 1
                        if steps == boundary:
                            raise KeyboardInterrupt('interrupted after a restore boundary')
                    def fail_record(path, data, mode):
                        if path == record:
                            raise OSError('final record write failed')
                        real_place(path, data, mode)
                    with patch.object(installer, 'sync_directory', interrupt), \
                            patch.object(installer, 'place', fail_record if boundary == 5 else real_place):
                        with self.assertRaises(OSError if boundary == 5 else KeyboardInterrupt):
                            installer.restore(record)
                    self.assertFalse(json.loads(record.read_text())['restored'])
                    installer.restore(record)
                    for rel in paths:
                        path = home / rel
                        if rel in originals:
                            self.assertEqual(path.read_bytes(), ('original ' + rel).encode())
                            self.assertEqual(path.stat().st_mode & 0o777, 0o640)
                        else:
                            self.assertFalse(path.exists())
                    self.assertTrue(json.loads(record.read_text())['restored'])

    def test_backup_contents_are_synced_before_install_replacements(self):
        original = self.home / '.local/bin/cosmic-panel'
        original.parent.mkdir(parents=True)
        original.write_bytes(b'old launcher')
        synced = []
        real_sync, real_replace = os.fsync, os.replace
        def sync(fd):
            synced.append(Path('/proc/self/fd/' + str(fd)).resolve())
            real_sync(fd)
        def replace(source, destination):
            if destination == original:
                self.assertTrue(any(path.name == original.name and path.parent.name.startswith('panel-')
                                    for path in synced))
            real_replace(source, destination)
        with patch.object(installer.os, 'fsync', sync), patch.object(installer.os, 'replace', replace):
            installer.install(self.home, self.binary, self.manifest)

    def test_restore_retries_directory_sync_after_final_record_replacement(self):
        original = self.home / '.local/bin/cosmic-panel'
        original.parent.mkdir(parents=True)
        original.write_bytes(b'old launcher')
        result = installer.install(self.home, self.binary, self.manifest)
        record = Path(result['backup']) / 'install-record.json'
        real_sync = installer.sync_directory
        def fail(directory):
            if directory == record.parent:
                raise OSError('final record directory sync failed')
            real_sync(directory)
        with patch.object(installer, 'sync_directory', fail):
            with self.assertRaises(OSError):
                installer.restore(record)
        self.assertTrue(json.loads(record.read_text())['restored'])
        with patch.object(installer, 'sync_directory', wraps=real_sync) as sync:
            installer.restore(record)
        sync.assert_called_once_with(record.parent)
        self.assertEqual(original.read_bytes(), b'old launcher')
        for rel in result['installed']:
            if rel not in result['before']:
                self.assertFalse((self.home / rel).exists())

    def test_restore_resumes_when_each_file_directory_sync_fails(self):
        for boundary in range(1, 5):
            with self.subTest(boundary=boundary), tempfile.TemporaryDirectory() as temporary:
                home = Path(temporary)
                original = home / '.local/bin/cosmic-panel'
                original.parent.mkdir(parents=True)
                original.write_bytes(b'old launcher')
                result = installer.install(home, self.binary, self.manifest)
                record = Path(result['backup']) / 'install-record.json'
                real_sync = installer.sync_directory
                steps = 0
                def fail(directory):
                    nonlocal steps
                    steps += 1
                    if steps == boundary:
                        raise OSError('directory sync failed')
                    real_sync(directory)
                with patch.object(installer, 'sync_directory', fail):
                    with self.assertRaises(OSError):
                        installer.restore(record)
                installer.restore(record)
                self.assertEqual(original.read_bytes(), b'old launcher')
                for rel in result['installed']:
                    if rel not in result['before']:
                        self.assertFalse((home / rel).exists())
                self.assertTrue(json.loads(record.read_text())['restored'])

    def test_compositor_shutdown_does_not_disable_islands(self):
        # The compositor disappears before the selector receives SIGTERM.
        # A closed listener can leave its socket inode behind at logout.
        for stale_inode in [False, True]:
            with self.subTest(stale_inode=stale_inode):
                path = self.root / 'wayland-shutdown'
                server = socket.socket(socket.AF_UNIX)
                server.bind(str(path))
                server.listen()
                server.close()
                if not stale_inode:
                    path.unlink()
                marker = self.root / 'false-crash'
                with patch.dict(os.environ, {'WAYLAND_DISPLAY': str(path)}):
                    code = launcher.supervise('/bin/false', '/unreachable/fallback',
                        failed=lambda: marker.write_text('failed'), session_socket=True)
                self.assertEqual(code, 0)
                self.assertFalse(marker.exists(), 'logout must not disable the next login')
                path.unlink(missing_ok=True)

    def test_panel_crash_with_live_compositor_still_persists_failure(self):
        path = self.root / 'wayland-live'
        with socket.socket(socket.AF_UNIX) as server:
            server.bind(str(path))
            server.listen()
            marker = self.root / 'real-crash'
            with patch.dict(os.environ, {'WAYLAND_DISPLAY': str(path)}):
                code = launcher.supervise('/bin/false', '/unreachable/fallback',
                    failed=lambda: marker.write_text('failed'), session_socket=True)
            self.assertEqual(code, 75)
            self.assertEqual(marker.read_text(), 'failed')

    def test_stock_panel_does_not_restart_after_compositor_shutdown(self):
        path = self.root / 'gone-compositor'
        runs = self.root / 'shutdown-runs'
        child = self.fixture('shutdown', 'open(' + repr(str(runs)) + ', "a").write("run\\n")\n'
                             'sys.exit(1)\n')
        with patch.dict(os.environ, {'WAYLAND_DISPLAY': str(path)}), \
                patch.object(launcher, 'RESTART_DELAYS', (0,)):
            self.assertEqual(launcher.stock_host(child, limit=2), 0)
        self.assertEqual(runs.read_text().splitlines(), ['run'])

    def test_session_socket_requires_fresh_session_restart(self):
        # A new host cannot authenticate again on a consumed notifications
        # stream. Session supervision recreates that socket after exit 75.
        marker = []
        code = launcher.supervise('/bin/false', '/unreachable/fallback', failed=lambda: marker.append(True), session_socket=True)
        self.assertEqual(code, 75)
        self.assertEqual(marker, [True])

    def test_notification_fd_survives_supervision(self):
        reader, writer = os.pipe()
        self.addCleanup(os.close, reader)
        self.addCleanup(os.close, writer)
        executable = self.root / 'child'
        executable.write_text('#!/usr/bin/python3\nimport os\nos.write(' + str(writer) + ',b"fd survived")\n')
        executable.chmod(0o755)
        self.assertEqual(launcher.supervise(executable, '/unreachable/fallback', (writer,),
                                          failed=lambda: None, session_socket=True), 75)
        self.assertEqual(os.read(reader, 64), b'fd survived')

    def fixture(self, name, body):
        executable = self.root / name
        executable.write_text('#!/usr/bin/python3\nimport os, sys, time\n' + body)
        executable.chmod(0o755)
        return executable

    def test_a_wedged_host_is_stopped_and_its_flood_collapsed(self):
        # The live failure: the compositor rejects a popup, and the panel
        # loops on the dead connection instead of exiting.
        child = self.fixture('wedged', 'print("panel up", flush=True)\n'
                             'while True: print("Protocol error 2 on object wl_surface@149", flush=True)\n')
        read, write = os.pipe()
        with os.fdopen(read, 'rb') as journal, os.fdopen(write, 'wb') as out, \
                patch.object(launcher.sys, 'stderr', type('E', (), {'buffer': out})()):
            started = time.monotonic()
            code, stopping, wedged = launcher.run_host(child)
            out.close()
            lines = journal.read().splitlines()
        self.assertTrue(wedged)
        self.assertFalse(stopping)
        self.assertEqual(code, -9)
        self.assertLess(time.monotonic() - started, 5)
        self.assertEqual(lines[0], b'panel up')
        self.assertLessEqual(lines.count(b'Protocol error 2 on object wl_surface@149'), launcher.REPEATS_SHOWN + 1)
        self.assertIn(b'cosmic-panel stopped responding to the compositor; restarting it', lines)
        self.assertTrue(lines[-1].startswith(b'(previous line repeated '), lines[-3:])

    def test_stock_restarts_a_failed_host_without_the_used_notifications_stream(self):
        reader, writer = os.pipe()
        self.addCleanup(os.close, reader)
        self.addCleanup(os.close, writer)
        runs = self.root / 'runs'
        # Fails the first time, then exits as at logout; records whether it
        # received the notifications stream each time.
        child = self.fixture('flaky', 'p = ' + repr(str(runs)) + '\n'
                             'n = len(open(p).read().splitlines()) if os.path.exists(p) else 0\n'
                             'open(p, "a").write(os.environ.get("PANEL_NOTIFICATIONS_FD", "-") + "\\n")\n'
                             'sys.exit(1 if n == 0 else 0)\n')
        with patch.dict(os.environ, {'PANEL_NOTIFICATIONS_FD': str(writer)}), \
                patch.object(launcher, 'RESTART_DELAYS', (0,)):
            self.assertEqual(launcher.stock_host(child, (writer,)), 0)
        self.assertEqual(runs.read_text().splitlines(), [str(writer), '-'])

    def test_stock_gives_up_after_the_limit(self):
        child = self.fixture('broken', 'sys.exit(3)\n')
        with patch.object(launcher, 'RESTART_DELAYS', (0,)):
            self.assertEqual(launcher.stock_host(child, limit=3), 3)

    def test_missing_binary_runs_stock_without_a_second_host(self):
        marker = []
        result = launcher.supervise('/unreachable/panel', '/bin/true', failed=lambda: marker.append(True))
        self.assertEqual(result, 0)
        self.assertEqual(marker, [True])

    def test_marker_storage_failures_still_select_stock(self):
        installer.install(self.home, self.binary, self.manifest, True)
        runtime = self.root / 'runtime'
        runtime.mkdir()
        state = self.root / 'state'
        env = {'HOME': str(self.home), 'XDG_DATA_HOME': str(self.home / '.local/share'),
               'XDG_STATE_HOME': str(state), 'XDG_RUNTIME_DIR': str(runtime)}
        for operation, error in [('mkdir', PermissionError('unwritable marker directory')),
                                 ('write_text', OSError('full marker disk'))]:
            for started in [True, False]:
                for session_socket in [True, False]:
                    with self.subTest(operation=operation, started=started, session_socket=session_socket):
                        child_env = {**env, **({'PANEL_NOTIFICATIONS_FD': '80', 'WAYLAND_SOCKET': '81'} if session_socket else {})}
                        locks = []
                        real_open = os.open
                        def opened(path, flags, mode):
                            fd = real_open(path, flags, mode)
                            locks.append(fd)
                            return fd
                        with patch.dict(os.environ, child_env, clear=True), \
                                patch.object(launcher, 'other_host', return_value=None), \
                                patch.object(launcher, 'prepare_launch', return_value=lambda child: True), \
                                patch.object(launcher, 'runtime_compatible', return_value=True), \
                                patch.object(launcher.os, 'open', opened), \
                                patch.object(launcher.os, 'fstat'), patch.object(launcher.os, 'set_inheritable'), \
                                patch.object(Path, operation, side_effect=error), \
                                patch.object(launcher, 'run_host', return_value=(1, False, False),
                                             side_effect=None if started else OSError('cannot start')), \
                                patch.object(launcher, 'stock_host', return_value=0) as stock:
                            try:
                                self.assertEqual(launcher.main(), 0)
                            finally:
                                for fd in locks:
                                    os.close(fd)
                        args, kwargs = stock.call_args
                        self.assertEqual(args[0], launcher.STOCK)
                        self.assertTrue(kwargs['keep_running'])
                        if session_socket and started:
                            self.assertEqual(args[1], (81,))
                            self.assertNotIn('PANEL_NOTIFICATIONS_FD', kwargs['env'])
                        else:
                            self.assertEqual(args[1], (80, 81) if session_socket else ())
                            self.assertIsNone(kwargs['env'])

    def test_unpersisted_stock_choice_survives_clean_host_exits(self):
        with patch.object(launcher, 'run_host', side_effect=[(0, False, False), (0, True, False)]) as run, \
                patch.object(launcher, 'pause', return_value=False):
            self.assertEqual(launcher.stock_host('/fake/stock', keep_running=True), 0)
        self.assertEqual(run.call_count, 2)

    def test_symlinked_install_parent_is_refused(self):
        self.home.mkdir()
        (self.home / '.local').symlink_to(self.root / 'elsewhere')
        with self.assertRaises(ValueError):
            installer.install(self.home, self.binary, self.manifest)


if __name__ == '__main__':
    unittest.main()
