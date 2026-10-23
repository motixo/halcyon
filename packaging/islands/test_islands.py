#!/usr/bin/env python3
"""Exercise only extracted packages, fixture homes, and fake binaries."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
sys.path.insert(0, str(REPO / 'companion'))
import panel_launcher

PACKAGE = 'cosmic-ext-halcyon-islands'
ARTIFACTS = None


def run(command, **kwargs):
    return subprocess.run([str(part) for part in command], text=True, capture_output=True,
                          timeout=60, **kwargs)


def manifest(binary, destination):
    destination.write_text(json.dumps({
        'protocol': 1, 'upstream': 'ca20ce3185b3b4404f429d59d429c40b58a2ddef',
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
    }) + '\n')


class IslandsPackage(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='halcyon-islands-test-')
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name)
        cls.binary = cls.root / 'cosmic-panel-islands'
        # A fake executable that must never be launched, even during activation.
        cls.binary.write_text('#!/bin/sh\necho "FAKE PANEL MUST NOT RUN" >&2\nexit 99\n')
        cls.input_manifest = cls.root / 'companion-manifest.json'
        manifest(cls.binary, cls.input_manifest)
        cls.out = cls.root / 'out'
        cls.build_env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1', 'SOURCE_DATE_EPOCH': '1792627200'}
        cls.build_command = [sys.executable, HERE / 'build-deb.py', '--binary', cls.binary,
                             '--manifest', cls.input_manifest, '--version', '0.1.0', '--out', cls.out]
        result = run(cls.build_command, env=cls.build_env)
        if result.returncode:
            raise AssertionError(result.stdout + result.stderr)
        cls.deb = cls.out / f'{PACKAGE}_0.1.0_amd64.deb'
        cls.extracted = cls.root / 'extracted'
        result = run(['dpkg-deb', '-x', cls.deb, cls.extracted])
        if result.returncode:
            raise AssertionError(result.stderr)
        if ARTIFACTS:
            ARTIFACTS.mkdir(parents=True, exist_ok=True)
            shutil.copy2(cls.deb, ARTIFACTS / cls.deb.name)
            shutil.copy2(cls.deb.with_suffix('.deb.sha256'), ARTIFACTS / cls.deb.with_suffix('.deb.sha256').name)

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(dir=self.root)
        self.addCleanup(temporary.cleanup)
        self.case = Path(temporary.name)
        self.home = self.case / 'home'
        self.home.mkdir()
        self.prefix = self.case / 'package'
        shutil.copytree(self.extracted, self.prefix)
        self.library = self.prefix / 'usr/lib' / PACKAGE
        self.command = self.prefix / 'usr/bin/halcyon-islands'
        self.bundle = self.home / '.local/share/cosmic-desktop-cards/companion'
        self.fake_bin = self.case / 'bin'
        self.fake_bin.mkdir()
        self.versions = self.case / 'versions.json'
        self.versions.write_text(json.dumps(panel_launcher.PACKAGES))
        query = self.fake_bin / 'dpkg-query'
        query.write_text('#!/usr/bin/python3\nimport json, os, sys\n'
                         'versions=json.load(open(os.environ["ISLANDS_TEST_VERSIONS"]))\n'
                         'print(versions.get(sys.argv[-1], "unavailable"), end="")\n')
        query.chmod(0o755)
        self.env = {key: value for key, value in os.environ.items()
                    if not key.startswith('XDG_') and key not in (
                        'PANEL_NOTIFICATIONS_FD', 'WAYLAND_SOCKET', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS')}
        self.env.update(HOME=str(self.home), XDG_CONFIG_HOME=str(self.home / '.config'),
                        XDG_DATA_HOME=str(self.home / '.local/share'), XDG_STATE_HOME=str(self.home / '.local/state'),
                        XDG_RUNTIME_DIR=str(self.case / 'runtime'), XDG_SESSION_ID='islands-fixture-session',
                        PYTHONDONTWRITEBYTECODE='1', ISLANDS_TEST_VERSIONS=str(self.versions),
                        PATH=f'{self.home}/.local/bin:{self.fake_bin}:/usr/bin:/bin')
        # Copy the real installer implementation, adding a fixture-only invocation log.
        # Assert that no test calls the live --user path, and every restore is confined.
        install = self.library / 'install.py'
        text = install.read_text()
        guard = ('\nif __name__ == "__main__":\n'
                 '    assert "--user" not in sys.argv, "test attempted --user installation"\n'
                 '    if "--root" in sys.argv:\n'
                 '        assert Path(sys.argv[sys.argv.index("--root") + 1]).resolve() == Path.home().resolve()\n'
                 '    if "--restore" in sys.argv:\n'
                 '        assert Path(sys.argv[sys.argv.index("--restore") + 1]).resolve().is_relative_to(Path.home())\n'
                 '    with open(os.environ["ISLANDS_TEST_CALLS"], "a") as log:\n'
                 '        log.write(json.dumps(sys.argv[1:]) + "\\n")\n')
        install.write_text(text.replace("\nif __name__ == '__main__':", guard + "\nif __name__ == '__main__':"))
        self.calls = self.case / 'installer-calls.jsonl'
        self.env['ISLANDS_TEST_CALLS'] = str(self.calls)
        self.preference = self.home / '.config/cosmic/fixture-preferences'
        self.preference.parent.mkdir(parents=True)
        self.preference.write_bytes(b'untouched native preferences\n')

    def cli(self, command, expected=0, fixture=True, options=()):
        args = [self.command]
        if fixture:
            args += ['--root', self.home]
        result = run([*args, command, *options], env=self.env)
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        self.assertNotIn('FAKE PANEL MUST NOT RUN', result.stdout + result.stderr)
        self.assertEqual(self.preference.read_bytes(), b'untouched native preferences\n')
        return result

    def journals(self):
        return sorted((self.home / '.local/share/cosmic-desktop-cards/backups').glob('panel-*/install-record.json'))

    def test_structured_status_never_enabled_and_removed_payload(self):
        result = self.cli('status', options=['--json'])
        status = json.loads(result.stdout)
        self.assertEqual(status['installation'], 'installed')
        self.assertEqual(status['provenance'], 'package')
        self.assertFalse(status['enabled'])
        self.assertIsNone(status['running'])
        (self.library / 'cosmic-panel-islands').unlink()
        status = json.loads(self.cli('status', options=['--json']).stdout)
        # A missing payload is a damaged installed package. Package removal is
        # a different provenance input, covered by test_install_status.py.
        self.assertEqual(status['installation'], 'installed')
        self.assertEqual(status['provenance'], 'package')
        self.assertFalse(status['healthy'])

    def test_reviewed_source_rebuild_needs_new_probe_bundle(self):
        versions = {k: v.replace('~24.04~', '~26.04~') for k, v in panel_launcher.PACKAGES.items()}
        self.versions.write_text(json.dumps(versions))
        self.cli('enable', expected=1)
        self.assertFalse(self.bundle.exists())
        manifest_path = self.library / 'companion-manifest.json'
        record = json.loads(manifest_path.read_text())
        record['runtime_probe'] = 1
        record['source_policy'] = 'reviewed-source-v1'
        manifest_path.write_text(json.dumps(record))
        self.cli('enable')
        self.assertIn('reviewed source combination', self.cli('status').stdout)
        versions['cosmic-comp'] = '0.1~999~26.04~abcdef0'
        self.versions.write_text(json.dumps(versions))
        self.cli('enable', expected=1)
        self.cli('disable')

    def test_package_contents_control_modes_and_hash(self):
        listing = run(['dpkg-deb', '-c', self.deb])
        info = run(['dpkg-deb', '-I', self.deb])
        self.assertEqual(listing.returncode, 0, listing.stderr)
        self.assertEqual(info.returncode, 0, info.stderr)
        expected = {
            f'usr/lib/{PACKAGE}/cosmic-panel-islands': 0o755,
            f'usr/lib/{PACKAGE}/companion-manifest.json': 0o644,
            f'usr/lib/{PACKAGE}/install.py': 0o755,
            f'usr/lib/{PACKAGE}/install_status.py': 0o644,
            f'usr/lib/{PACKAGE}/panel_launcher.py': 0o755,
            'usr/bin/halcyon-islands': 0o755,
            f'usr/share/doc/{PACKAGE}/copyright': 0o644,
            f'usr/share/doc/{PACKAGE}/README': 0o644,
            f'usr/share/doc/{PACKAGE}/changelog.gz': 0o644,
            'usr/share/man/man1/halcyon-islands.1.gz': 0o644,
        }
        actual = {str(path.relative_to(self.extracted)): path.stat().st_mode & 0o777
                  for path in self.extracted.rglob('*') if path.is_file()}
        self.assertEqual(actual, expected)
        for path, mode in expected.items():
            self.assertIn('./' + path, listing.stdout)
            line = next(line for line in listing.stdout.splitlines() if line.endswith('./' + path))
            self.assertTrue(line.startswith('-rwxr-xr-x' if mode == 0o755 else '-rw-r--r--'), line)
            self.assertIn('root/root', line)
        for field in [f'Package: {PACKAGE}', 'Architecture: amd64', 'Version: 0.1.0',
                      'python3, cosmic-ext-halcyon', 'optional source-qualified']:
            self.assertIn(field, info.stdout)
        for name, pin in panel_launcher.PACKAGES.items():
            self.assertIn(name, info.stdout)
            self.assertIn(pin, (HERE / 'README.md').read_text())
        control = self.case / 'control'
        result = run(['dpkg-deb', '-e', self.deb, control])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([path.name for path in control.iterdir()], ['control'])
        checksum, filename = self.deb.with_suffix('.deb.sha256').read_text().split()
        self.assertEqual(filename, self.deb.name)
        self.assertEqual(checksum, hashlib.sha256(self.deb.read_bytes()).hexdigest())
        copyright_text = (HERE / 'copyright').read_text()
        self.assertIn('System76', copyright_text)
        self.assertIn('GPL-3.0-only', copyright_text)

    def test_cosmic_upgrades_are_not_blocked_by_package_dependencies(self):
        result = run(['dpkg-deb', '-f', self.deb, 'Depends'])
        self.assertEqual(result.returncode, 0, result.stderr)
        dependencies = [entry.strip() for entry in result.stdout.split(',')]
        for name in ('cosmic-panel', 'cosmic-comp', 'cosmic-session'):
            self.assertIn(name, dependencies,
                          'COSMIC compatibility belongs in the launcher, not an APT version constraint')

    def test_real_elf_dependencies_are_detected_without_silent_fallback(self):
        binary = self.case / 'cosmic-panel-islands'
        shutil.copyfile('/usr/bin/true', binary)
        record = self.case / 'elf-manifest.json'
        manifest(binary, record)
        out = self.case / 'elf-package'
        licenses = self.case / 'licenses.txt'
        licenses.write_text('Fixture dependency notice\n')
        command = [sys.executable, HERE / 'build-deb.py', '--binary', binary,
                   '--manifest', record, '--version', '0.1.0', '--out', out]
        missing = run(command)
        self.assertNotEqual(missing.returncode, 0, 'Native releases need their license notices')
        result = run([*command, '--licenses', licenses])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        control = run(['dpkg-deb', '-f', out / self.deb.name, 'Depends'])
        self.assertEqual(control.returncode, 0, control.stderr)
        extracted = self.case / 'elf-extracted'
        self.assertEqual(run(['dpkg-deb', '-x', out / self.deb.name, extracted]).returncode, 0)
        self.assertEqual((extracted / f'usr/share/doc/{PACKAGE}/THIRD_PARTY_LICENSES.txt').read_text(), licenses.read_text())
        self.assertIn('libc6 (>=', control.stdout,
                      'Real ELF packages must retain the ABI floor found by dpkg-shlibdeps')

    def test_reproducible_build(self):
        original = self.deb.read_bytes()
        result = run(self.build_command, env=self.build_env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.deb.read_bytes(), original)

    def test_builder_rejects_wrong_hash_protocol_revision_and_version(self):
        record = json.loads(self.input_manifest.read_text())
        for key, value in [('binary_sha256', '0' * 64), ('protocol', 2), ('upstream', 'unknown')]:
            with self.subTest(key=key):
                bad = self.case / 'bad-manifest.json'
                bad.write_text(json.dumps({**record, key: value}))
                command = list(self.build_command)
                command[command.index('--manifest') + 1] = bad
                command[command.index('--out') + 1] = self.case / key
                result = run(command, env=self.build_env)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('selector', result.stderr)
                self.assertFalse(list((self.case / key).glob('*.deb')))
        command = list(self.build_command)
        command[command.index('--version') + 1] = '1.0\nDepends: bad'
        result = run(command, env=self.build_env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Invalid Debian version', result.stderr)

    def test_enable_status_disable_and_idempotency(self):
        before = self.cli('status', expected=1)
        self.assertIn('Installed: yes', before.stdout)
        self.assertIn('Enabled for this user: no', before.stdout)
        self.cli('enable')
        selection = json.loads((self.bundle / 'selection.json').read_text())
        self.assertTrue(selection['enabled'])
        self.assertTrue(selection['defer_session'].endswith(':islands-fixture-session'))
        record = json.loads(self.journals()[0].read_text())
        self.assertEqual(record['root'], str(self.home))
        self.assertEqual(record['owner'], PACKAGE)
        self.assertTrue(all((self.home / rel).is_relative_to(self.home) for rel in record['installed']))
        self.assertIn('waiting for the next login', self.cli('status').stdout)
        self.cli('enable')
        self.assertEqual(len(self.journals()), 1)
        self.env['XDG_SESSION_ID'] = 'next-fixture-login'
        self.assertIn('eligible in this login', self.cli('status').stdout)
        self.cli('disable')
        self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())
        self.assertFalse((self.bundle / 'cosmic-panel-islands').exists())
        self.assertTrue(json.loads(self.journals()[0].read_text())['restored'])
        self.cli('disable')
        self.cli('status', expected=1)
        calls = [json.loads(line) for line in self.calls.read_text().splitlines()]
        self.assertEqual(len(calls), 2)
        self.assertIn('--root', calls[0])
        self.assertIn('--enable-next-login', calls[0])
        self.assertIn('--restore', calls[1])

    def test_each_version_mismatch_and_failed_marker_select_stock(self):
        self.cli('enable')
        for name in panel_launcher.PACKAGES:
            with self.subTest(package=name):
                self.versions.write_text(json.dumps({**panel_launcher.PACKAGES, name: 'new-version'}))
                result = self.cli('status', expected=1)
                self.assertIn('no match', result.stdout)
                self.assertIn('Enabled for this user: yes', result.stdout)
                self.assertIn('Stock panel selected', result.stdout)
        self.versions.write_text(json.dumps(panel_launcher.PACKAGES))
        failed = self.home / '.local/state/desktop-cards-panel/failed'
        failed.parent.mkdir(parents=True)
        failed.write_text('previous failure')
        self.assertIn('previous Islands failure', self.cli('status', expected=1).stdout)
        calls = self.calls.read_bytes()
        self.assertIn('previous Islands failure', self.cli('enable', expected=1).stderr)
        self.assertEqual(self.calls.read_bytes(), calls)
        self.assertEqual(failed.read_text(), 'previous failure')
        self.cli('disable')
        self.assertTrue(failed.exists())

    def test_incompatible_enable_reports_versions_without_installing(self):
        for name in panel_launcher.PACKAGES:
            with self.subTest(package=name):
                self.versions.write_text(json.dumps({**panel_launcher.PACKAGES, name: '1.10-older-build'}))
                result = self.cli('enable', expected=1)
                self.assertIn(name, result.stderr)
                self.assertIn('1.10-older-build', result.stderr)
                self.assertIn(panel_launcher.PACKAGES[name], result.stderr)
                self.assertFalse(self.calls.exists())
                self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())
                self.assertFalse(self.bundle.exists())
                self.assertEqual(self.journals(), [])
                status = self.cli('status', expected=1)
                self.assertIn('1.10-older-build', status.stdout)
                self.assertIn(panel_launcher.PACKAGES[name], status.stdout)

    def test_supported_110_can_enable_and_manifest_advertises_the_combination(self):
        versions = {
            'cosmic-panel': '1.10.0~1791315703~24.04~3ce6ce1',
            'cosmic-comp': '0.1~1791979042~24.04~742b800',
            'cosmic-session': '1.10.0~1791314577~24.04~8093b59',
        }
        self.versions.write_text(json.dumps(versions))
        self.cli('enable')
        self.assertIn('System compatibility (selector checks): match', self.cli('status').stdout)
        record = json.loads((self.bundle / 'manifest.json').read_text())
        self.assertIn(versions, record['supported_packages'])
        self.assertIn(panel_launcher.PACKAGES, record['supported_packages'])
        self.cli('disable')

    def test_mixed_release_packages_do_not_inherit_support_individually(self):
        self.versions.write_text(json.dumps({
            **panel_launcher.PACKAGES,
            'cosmic-session': '1.10.0~1791314577~24.04~8093b59',
        }))
        self.cli('enable', expected=1)
        self.assertFalse(self.calls.exists())

    def test_incompatible_reenable_preserves_existing_bundle_and_can_disable(self):
        self.cli('enable')
        before = {p: p.read_bytes() for p in self.bundle.iterdir() if p.is_file()}
        calls = self.calls.read_bytes()
        self.versions.write_text(json.dumps({**panel_launcher.PACKAGES, 'cosmic-comp': 'new-build'}))
        self.cli('enable', expected=1)
        self.assertEqual({p: p.read_bytes() for p in before}, before)
        self.assertEqual(self.calls.read_bytes(), calls)
        self.cli('disable')
        self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())

    def test_failed_package_query_refuses_enable_and_reports_other_versions(self):
        query = self.fake_bin / 'dpkg-query'
        query.write_text(query.read_text().replace(
            'versions=json.load', 'if sys.argv[-1] == "cosmic-panel": sys.exit(1)\nversions=json.load'))
        result = self.cli('enable', expected=1)
        self.assertIn('cosmic-panel', result.stderr)
        self.assertIn('unavailable', result.stderr)
        self.assertFalse(self.calls.exists())
        status = self.cli('status', expected=1)
        self.assertIn('unavailable', status.stdout)
        self.assertIn(panel_launcher.PACKAGES['cosmic-comp'], status.stdout)

    def test_upgrade_restores_all_owned_layers(self):
        self.cli('enable')
        selector = self.library / 'panel_launcher.py'
        selector.write_text(selector.read_text() + '\n# Fixture selector update.\n')
        self.assertIn('Enabled for this user: yes', self.cli('status').stdout)
        binary = self.library / 'cosmic-panel-islands'
        binary.write_text('#!/bin/sh\nexit 98\n')
        manifest(binary, self.library / 'companion-manifest.json')
        self.cli('enable')
        self.assertEqual(len(self.journals()), 2)
        self.assertEqual((self.bundle / 'cosmic-panel-islands').read_bytes(), binary.read_bytes())
        self.cli('disable')
        self.assertTrue(all(json.loads(path.read_text())['restored'] for path in self.journals()))
        self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())

    def test_interrupted_enable_can_be_restored_from_its_owned_journal(self):
        install = self.library / 'install.py'
        original = install.read_text()
        install.write_text(original.replace(
            "            place(path, data, mode)",
            "            place(path, data, mode)\n            os._exit(90)"))
        self.cli('enable', expected=1)
        self.assertEqual(len(self.journals()), 1)
        self.assertFalse(json.loads(self.journals()[0].read_text())['complete'])
        self.assertTrue((self.bundle / 'cosmic-panel-islands').exists())
        install.write_text(original)
        self.assertIn('interrupted install', self.cli('enable', expected=1).stderr)
        self.cli('disable')
        self.assertFalse((self.bundle / 'cosmic-panel-islands').exists())
        self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())
        self.assertTrue(json.loads(self.journals()[0].read_text())['restored'])

    def test_mode_edits_are_refused_before_restore(self):
        self.cli('enable')
        selector = self.home / '.local/bin/cosmic-panel'
        selector.chmod(0o700)
        self.assertIn('changed since installation', self.cli('disable', expected=1).stderr)
        self.assertEqual(selector.stat().st_mode & 0o777, 0o700)
        self.assertTrue((self.bundle / 'cosmic-panel-islands').exists())

    def test_external_edit_and_corrupt_backup_are_not_overwritten(self):
        self.cli('enable')
        binary = self.bundle / 'cosmic-panel-islands'
        binary.write_text('outside edit')
        self.cli('status', expected=1)
        self.assertIn('changed since installation', self.cli('enable', expected=1).stderr)
        self.assertIn('changed since installation', self.cli('disable', expected=1).stderr)
        self.assertEqual(binary.read_text(), 'outside edit')
        binary.write_bytes((self.library / 'cosmic-panel-islands').read_bytes())
        candidate = self.library / 'cosmic-panel-islands'
        candidate.write_text('#!/bin/sh\nexit 97\n')
        manifest(candidate, self.library / 'companion-manifest.json')
        self.cli('enable')
        record = json.loads(self.journals()[-1].read_text())
        backup = self.home / record['before'][str(DATA / 'companion/cosmic-panel-islands')]
        backup.write_text('corrupt backup')
        self.assertIn('backup checksum', self.cli('disable', expected=1).stderr)
        self.assertEqual(binary.read_bytes(), candidate.read_bytes())

    def test_existing_selector_and_symlink_escape_are_refused(self):
        selector = self.home / '.local/bin/cosmic-panel'
        selector.parent.mkdir(parents=True)
        selector.write_text('unmanaged selector')
        self.assertIn('not managed', self.cli('enable', expected=1).stderr)
        self.assertIn('not managed', self.cli('disable', expected=1).stderr)
        self.assertEqual(selector.read_text(), 'unmanaged selector')
        shutil.rmtree(self.home / '.local')
        outside = self.case / 'outside'
        outside.mkdir()
        (self.home / '.local').symlink_to(outside, target_is_directory=True)
        self.assertIn('symlink', self.cli('enable', expected=1).stderr)
        self.assertEqual(list(outside.iterdir()), [])

    def test_missing_session_and_tampered_package_do_not_activate(self):
        self.env.pop('XDG_SESSION_ID')
        self.assertIn('COSMIC desktop session', self.cli('enable', expected=1).stderr)
        self.assertEqual(self.journals(), [])
        self.env['XDG_SESSION_ID'] = 'fixture'
        (self.library / 'cosmic-panel-islands').write_text('tampered package')
        self.assertIn('integrity', self.cli('enable', expected=1).stderr)
        self.assertFalse((self.home / '.local/bin/cosmic-panel').exists())

    def test_status_help_and_usage_are_read_only(self):
        self.cli('--help')
        self.cli('status', expected=1)
        self.cli('unknown', expected=2)
        self.assertFalse((self.home / '.local').exists())
        self.assertFalse(self.calls.exists())

    def test_fixture_ignores_inherited_xdg_paths(self):
        outside = self.case / 'outside'
        self.env['XDG_DATA_HOME'] = str(outside)
        self.env['XDG_STATE_HOME'] = str(outside)
        self.cli('enable')
        self.cli('status')
        self.cli('disable')
        self.assertFalse(outside.exists())

    def test_production_dispatch_requests_user_mode_without_running_installer(self):
        # Inspect production dispatch with a recording stub. Never invoke --user.
        from importlib.machinery import SourceFileLoader
        loader = SourceFileLoader('islands_command', str(self.command))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        module = importlib.util.module_from_spec(spec)
        loader.exec_module(module)
        from unittest.mock import patch
        installer_spec = importlib.util.spec_from_file_location('fixture_installer', REPO / 'companion/install.py')
        installer = importlib.util.module_from_spec(installer_spec)
        installer_spec.loader.exec_module(installer)
        with patch.dict(os.environ, self.env, clear=True), patch.object(module, 'run_installer') as invoke:
            module.change('enable', self.home, False, panel_launcher, installer)
        arguments, _ = invoke.call_args.args
        self.assertIn('--user', arguments)
        self.assertNotIn('--root', arguments)
        self.assertIn('--enable-next-login', arguments)

    def test_lintian(self):
        result = run(['lintian', self.deb], env={**self.env, 'XDG_CACHE_HOME': str(self.case / 'cache')})
        log = result.stdout + result.stderr
        if ARTIFACTS:
            (ARTIFACTS / 'lintian.txt').write_text(log)
        print('\nLintian output:\n' + (log or '(no findings)\n'))
        self.assertEqual(result.returncode, 0, log)
        self.assertFalse(any(line.startswith('E:') for line in log.splitlines()), log)


DATA = Path('.local/share/cosmic-desktop-cards')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path)
    args, remaining = parser.parse_known_args()
    ARTIFACTS = args.artifacts
    unittest.main(argv=[sys.argv[0], *remaining], verbosity=2)
