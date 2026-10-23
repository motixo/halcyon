"""Admission tests: metadata rebuilds are not unknown compositor source."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parent))
import panel_launcher as launcher

class Compatibility(unittest.TestCase):
    def test_reviewed_sources_survive_distribution_rebuild(self):
        rebuilt = {k: v.replace('~24.04~', '~26.04~') for k, v in launcher.PACKAGES.items()}
        self.assertTrue(launcher.supported_system(rebuilt))

    def test_unknown_missing_mixed_and_malformed_sources_stay_stock(self):
        for version in ['1.99.0~123~24.04~abcdef0', 'ca20ce3', None, '1.9.0~123~24.04~ca20ce3', '1.11.0~123~24.04~ca20ce3-evil']:
            self.assertFalse(launcher.supported_system({**launcher.PACKAGES, 'cosmic-panel': version}))
        self.assertFalse(launcher.supported_system({**launcher.PACKAGES, 'cosmic-comp': launcher.SUPPORTED_SYSTEMS[1]['cosmic-comp']}))

    def test_rebuilt_system_requires_probe_contract_in_manifest(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            (root / 'cosmic-panel-islands').write_bytes(b'test binary')
            record = {'upstream':'ca20ce3185b3b4404f429d59d429c40b58a2ddef', 'protocol':1,
                      'binary_sha256':launcher.digest(root / 'cosmic-panel-islands')}
            versions = {k:v.replace('~24.04~','~26.04~') for k,v in launcher.PACKAGES.items()}
            (root / 'manifest.json').write_text(json.dumps(record))
            self.assertFalse(launcher.compatible(root, versions))
            record['runtime_probe'] = 1
            (root / 'manifest.json').write_text(json.dumps(record))
            self.assertTrue(launcher.compatible(root, versions))

class RuntimeProbe(unittest.TestCase):
    def test_probe_failure_and_malformed_output_select_stock(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            (root/'manifest.json').write_text('{"runtime_probe":1}')
            binary = root/'cosmic-panel-islands'
            for body in ['print("bad json")', 'print("[]")',
                         'print(\'{"probe":1,"ready":false}\')',
                         'print(\'{"probe":1,"ready":true}\'); raise SystemExit(1)']:
                binary.write_text('#!/usr/bin/python3\n' + body + '\n')
                binary.chmod(0o755)
                self.assertFalse(launcher.runtime_compatible(root))

    def test_probe_uses_fresh_connection_and_no_session_fds(self):
        import os
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            (root/'manifest.json').write_text('{"runtime_probe":1}')
            binary = root/'cosmic-panel-islands'
            binary.write_text("#!/usr/bin/python3\nimport os,sys,json\n"
                "assert sys.argv[1:] == ['--islands-probe']\n"
                "assert not any(k in os.environ for k in ['WAYLAND_SOCKET','PANEL_NOTIFICATIONS_FD','COSMIC_SESSION_SOCK'])\n"
                "print(json.dumps({'probe':1,'ready':True}))\n")
            binary.chmod(0o755)
            with patch.dict(os.environ, {'WAYLAND_SOCKET':'42','PANEL_NOTIFICATIONS_FD':'43','COSMIC_SESSION_SOCK':'44'}):
                self.assertTrue(launcher.runtime_compatible(root))

class StartupHealth(unittest.TestCase):
    def test_silent_host_that_never_serves_ipc_is_reaped(self):
        import os, subprocess, time
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as root, patch.dict(os.environ, {'XDG_RUNTIME_DIR':root}):
            binary=Path(root)/'panel'
            binary.write_text('#!/usr/bin/python3\nimport time\ntime.sleep(30)\n')
            binary.chmod(0o755)
            with patch.object(launcher, 'READY_SECONDS', .15):
                start=time.monotonic()
                code, stopping, wedged=launcher.run_host(binary, readiness=True)
            self.assertLess(time.monotonic()-start, 3)
            self.assertNotEqual(code, 0)
            self.assertFalse(stopping)
            self.assertTrue(wedged)

if __name__ == '__main__': unittest.main()
