#!/usr/bin/env python3
"""Real APT in a disposable user namespace; synthetic packages NEVER qualify.

No network except a loopback HTTP fixture, no host apt state, no real keys.
Run: python3 -B scripts/apt/test_archive.py
"""
import http.server
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest

import archive

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('enrollment', ROOT/'packaging/updates/build-deb.py')
enrollment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(enrollment)


def execute(args, **kwargs):
    return subprocess.run([str(a) for a in args], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          timeout=90, **kwargs)


def key(home, stamp=None):
    home.mkdir(mode=0o700)
    common = ['gpg', '--batch', '--pinentry-mode', 'loopback', '--passphrase', '', '--homedir', str(home)]
    if stamp:
        common += ['--faked-system-time', str(stamp)]
    for args in ([*common, '--quick-generate-key', 'APT disposable fixture <fixture@example.invalid>', 'ed25519', 'cert', '1d'],):
        result = execute(args)
        if result.returncode:
            raise RuntimeError('fixture key generation failed (private output suppressed)')
    listing = execute([*common, '--with-colons', '--list-keys']).stdout.decode()
    primary = next(x.split(':')[9] for x in listing.splitlines() if x.startswith('fpr:'))
    if execute([*common, '--quick-add-key', primary, 'ed25519', 'sign', '1d']).returncode:
        raise RuntimeError('fixture subkey generation failed (private output suppressed)')
    listing = execute([*common, '--with-colons', '--list-keys']).stdout.decode()
    subkey = [x.split(':')[9] for x in listing.splitlines() if x.startswith('fpr:')][-1]
    public = home/'public.gpg'
    public.write_bytes(archive.run([*common, '--export']))
    return subkey, public


def package(root, name, version, depends=None):
    tree = root/(name+'-'+version+'-tree')
    (tree/'DEBIAN').mkdir(parents=True)
    (tree/'DEBIAN/control').write_text(f'Package: {name}\nVersion: {version}\nArchitecture: amd64\n'
        'Maintainer: Fixture <fixture@example.invalid>\nDescription: SYNTHETIC NOT NATIVE\n' +
        (f'Depends: {depends}\n' if depends else ''))
    payload = tree/'usr/share'/name/'fixture'
    payload.parent.mkdir(parents=True)
    payload.write_text('SYNTHETIC APT ACCEPTANCE ONLY '+version)
    if name == archive.PACKAGES[0]:
        meta = tree/'usr/share/metainfo'/f'{archive.APP_ID}.metainfo.xml'
        meta.parent.mkdir(parents=True)
        shutil.copyfile(ROOT/'packaging'/meta.name, meta)
        for size in (64, 128):
            icon = tree/f'usr/share/icons/hicolor/{size}x{size}/apps'/f'{archive.APP_ID}.png'
            icon.parent.mkdir(parents=True)
            shutil.copyfile(ROOT/f'packaging/icons/{size}'/icon.name, icon)
    out = root/(name+'-'+version+'.deb')
    result = execute(['dpkg-deb', '--root-owner-group', '--build', tree, out],
                     env={**os.environ, 'SOURCE_DATE_EPOCH': '1700000000'})
    if result.returncode:
        raise RuntimeError(result.stdout.decode())
    return out


class Server:
    def __init__(self, root):
        self.root = root
        self.requests = []
        self.after_release = None
        self.closed = False
        owner = self
        class Handler(http.server.SimpleHTTPRequestHandler):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, directory=str(owner.root), **kwargs)
            def log_message(self, *args):
                pass
            def do_GET(self):
                owner.requests.append(self.path)
                # Model a generation-aware origin. HTTP date granularity alone
                # cannot distinguish two fixture deployments in one second.
                if 'If-Modified-Since' in self.headers:
                    del self.headers['If-Modified-Since']
                super().do_GET()
                if self.path.endswith('/InRelease') and owner.after_release:
                    callback, owner.after_release = owner.after_release, None
                    callback()
        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.uri = f'http://127.0.0.1:{self.server.server_port}/apt'

    def close(self):
        if self.closed:
            return
        self.closed = True
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()


class Apt:
    def __init__(self, root, server, public):
        self.root = root
        self.server = server
        for rel in ('etc/apt/apt.conf.d', 'etc/apt/sources.list.d', 'etc/apt/preferences.d',
                    'etc/apt/trusted.gpg.d', 'var/lib/apt/lists/partial', 'var/lib/dpkg/updates',
                    'var/cache/apt/archives/partial', 'var/log/apt', 'usr/share/keyrings'):
            (root/rel).mkdir(parents=True, exist_ok=True)
        (root/'var/lib/dpkg/status').touch()
        self.public = root/'usr/share/keyrings/halcyon-archive-keyring.gpg'
        shutil.copyfile(public, self.public)
        self.source = root/'etc/apt/sources.list.d/halcyon.sources'
        source, prefs = enrollment.configuration(server.uri, str(self.public))
        self.source.write_text(source)
        (root/'etc/apt/preferences.d/halcyon').write_text(prefs)
        self.config = root/'apt.conf'
        self.config.write_text(f'''
Dir "{root}";
Dir::Etc "{root}/etc/apt";
Dir::Etc::main "{root}/etc/apt/nonexistent.conf";
Dir::Etc::parts "{root}/etc/apt/apt.conf.d";
Dir::Etc::sourcelist "{root}/etc/apt/sources.list";
Dir::Etc::sourceparts "{root}/etc/apt/sources.list.d";
Dir::Etc::preferences "{root}/etc/apt/preferences";
Dir::Etc::preferencesparts "{root}/etc/apt/preferences.d";
Dir::Etc::trusted "{root}/etc/apt/trusted.gpg";
Dir::Etc::trustedparts "{root}/etc/apt/trusted.gpg.d";
Dir::State "{root}/var/lib/apt";
Dir::State::status "{root}/var/lib/dpkg/status";
Dir::Cache "{root}/var/cache/apt";
Dir::Log "{root}/var/log/apt";
APT::Architecture "amd64";
APT::Architectures {{ "amd64"; }};
APT::Sandbox::User "root";
APT::Get::List-Cleanup "true";
APT::Update::Error-Mode "any";
Acquire::Languages "none";
Acquire::Retries "0";
Acquire::http::Timeout "5";
Acquire::https::Timeout "5";
Acquire::IndexTargets::deb::DEP-11::MetaKey "$(COMPONENT)/dep11/Components-$(NATIVE_ARCHITECTURE).yml";
DPkg::Options {{ "--root={root}"; "--force-script-chrootless"; "--log={root}/var/log/dpkg.log"; }};
DPkg::Use-Pty "false";
#clear DPkg::Pre-Install-Pkgs;
#clear DPkg::Post-Invoke;
#clear APT::Update::Post-Invoke;
#clear APT::Update::Post-Invoke-Success;
''')
        self.env = {**os.environ, 'APT_CONFIG': str(self.config), 'HOME': str(root), 'LC_ALL': 'C',
                    'DEBIAN_FRONTEND': 'noninteractive'}

    def cmd(self, *args, ok=True, tool='apt-get'):
        result = execute([tool, *args], env=self.env)
        if ok and result.returncode:
            raise AssertionError(result.stdout.decode())
        if not ok and result.returncode == 0:
            raise AssertionError('Unexpected success: '+result.stdout.decode())
        return result.stdout.decode()

    def dpkg(self, *args):
        result = execute(['dpkg', '--root='+str(self.root), '--force-script-chrootless',
                          '--log='+str(self.root/'var/log/dpkg.log'), *args], env=self.env)
        if result.returncode:
            raise AssertionError(result.stdout.decode())
        return result.stdout.decode()


class Acceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Refuse real-root execution; scripts and dpkg can only act as namespace
        # root mapped to the invoking unprivileged uid on the host.
        mapping = Path('/proc/self/uid_map').read_text().split()
        if os.geteuid() != 0 or mapping[0] != '0' or mapping[1] == '0' or mapping[2] != '1':
            raise RuntimeError('isolated unprivileged user namespace required')
        cls.temp = tempfile.TemporaryDirectory(prefix='halcyon-apt-private-')
        cls.base = Path(cls.temp.name)
        cls.epoch = int(time.time())-120
        cls.subkey, cls.public = key(cls.base/'key-a')
        cls.subkey_b, cls.public_b = key(cls.base/'key-b')
        cls.bootstrap = enrollment.build(cls.public, '1.0', cls.base/'bootstrap', cls.epoch)
        cls.main = package(cls.base, archive.PACKAGES[0], '0.2.0')
        cls.islands = package(cls.base, archive.PACKAGES[1], '0.2.0')
        cls.unsigned = cls.base/'unsigned'
        cls.digest = archive.stage([cls.main, cls.islands, *cls.bootstrap], {}, cls.unsigned, cls.epoch)
        cls.signed = cls.base/'signed'
        archive.sign(cls.unsigned, cls.digest, cls.signed, cls.base/'key-a', cls.subkey, cls.public)

    @classmethod
    def tearDownClass(cls):
        for home in cls.base.glob('key-*'):
            execute(['gpgconf', '--homedir', home, '--kill', 'gpg-agent'])
        cls.temp.cleanup()

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir=self.base)
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.web = self.root/'web'
        self.web.mkdir()
        self.served = self.web/'apt'
        initial = self.root/'initial'
        shutil.copytree(self.signed, initial)
        self.served.symlink_to(initial)
        self.server = Server(self.web)
        self.addCleanup(self.server.close)
        self.apt = Apt(self.root/'client', self.server, self.public)

    def resign(self, root=None, subkey=None, home=None, stamp=None):
        root = root or self.served
        common = ['gpg', '--batch', '--yes', '--homedir', str(home or self.base/'key-a'),
                  '--local-user', (subkey or self.subkey)+'!', '--digest-algo', 'SHA256']
        if stamp:
            common += ['--faked-system-time', str(stamp)]
        for name, flag in [('InRelease', '--clearsign'), ('Release.gpg', '--detach-sign')]:
            result = execute([*common, '--output', root/archive.SUITE/name, flag, root/archive.SUITE/'Release'])
            if result.returncode:
                raise RuntimeError('fixture signing failed (private output suppressed)')

    def new_generation(self, version='0.2.1', boot=None):
        main = package(self.root, archive.PACKAGES[0], version)
        islands = package(self.root, archive.PACKAGES[1], version)
        unsigned = self.root/'next-unsigned'
        digest = archive.stage([main, islands, *(boot or [])], {}, unsigned,
                               self.epoch+60, self.signed, self.digest)
        signed = self.root/'next-signed'
        archive.sign(unsigned, digest, signed, self.base/'key-a', self.subkey, self.public)
        return signed, digest

    def serve(self, archive_root):
        generation = Path(tempfile.mkdtemp(dir=self.root))/'archive'
        shutil.copytree(archive_root, generation)
        pointer = self.web/'next'
        pointer.symlink_to(generation)
        os.replace(pointer, self.served)

    def hostile_index(self, extra, origin=None):
        """Simulate a signed but hostile publisher; production builder rejects it."""
        for path in extra:
            shutil.copyfile(path, self.served/'pool'/path.name)
        index = self.served/archive.SUITE/'main/binary-amd64/Packages'
        index.write_bytes(archive.run(['apt-ftparchive', 'packages', 'pool'], cwd=self.served))
        import gzip
        index.with_suffix('.gz').write_bytes(gzip.compress(index.read_bytes(), mtime=0))
        suite = self.served/archive.SUITE
        files = sorted(p for p in suite.rglob('*') if p.is_file() and '/by-hash/' not in str(p)
                       and p.name not in ('Release', 'InRelease', 'Release.gpg'))
        hashes = []
        for path in files:
            digest = archive.sha(path)
            archive.write(path.parent/'by-hash/SHA256'/digest, path.read_bytes())
            hashes.append(f' {digest} {path.stat().st_size:16} {path.relative_to(suite)}\n')
        release = suite/'Release'
        text = release.read_text().split('SHA256:\n')[0]+'SHA256:\n'+''.join(hashes)
        if origin:
            text = text.replace('Origin: Halcyon', 'Origin: '+origin)
        release.write_text(text)
        self.resign()

    def test_exact_package_allowlist_and_hostile_dependency(self):
        names = ['unrelated-hostile', 'cosmic-ext-halcyon-evil', 'halcyon-update-source-extra']
        extra = [package(self.root, name, '999:9.9') for name in names]
        malicious_main = package(self.root, archive.PACKAGES[0], '9.9', depends=names[0])
        with self.assertRaisesRegex(ValueError, 'package_not_allowlisted'):
            archive.control(extra[0])
        self.hostile_index([*extra, malicious_main])
        self.apt.cmd('update')
        for name in names:
            self.assertIn('Candidate: (none)', self.apt.cmd('policy', name, tool='apt-cache'))
            self.apt.cmd('-y', 'install', name, ok=False)
        self.assertIn('Candidate: 9.9', self.apt.cmd('policy', archive.PACKAGES[0], tool='apt-cache'))
        self.apt.cmd('-y', 'install', archive.PACKAGES[0], ok=False)

    def test_wrong_signed_origin_is_blocked_by_hostname_policy(self):
        self.hostile_index([], origin='DifferentPublisher')
        self.apt.cmd('update')
        for name in archive.PACKAGES:
            self.assertIn('Candidate: (none)', self.apt.cmd('policy', name, tool='apt-cache'))

    def test_bad_refresh_keeps_only_previously_authenticated_cache(self):
        self.apt.cmd('update')
        newer, _ = self.new_generation()
        self.serve(newer)
        self.resign(subkey=self.subkey_b, home=self.base/'key-b')
        self.apt.cmd('update', ok=False)
        self.assertIn('Candidate: 0.2.0', self.apt.cmd('policy', archive.PACKAGES[0], tool='apt-cache'))

    def test_offline_update_is_error_and_preserves_installed_package(self):
        self.apt.dpkg('--install', self.main)
        self.apt.cmd('update')
        self.server.close()
        self.apt.cmd('update', ok=False)
        self.assertIn('Version: 0.2.0', self.apt.dpkg('--status', archive.PACKAGES[0]))

    def test_expiry_monitor_warns_before_signing_key_deadline(self):
        from check_expiry import check
        warning = check(self.signed, self.digest, self.public)
        self.assertEqual(warning['state'], 'renew')
        self.assertGreater(warning['remaining_seconds'], 0)
        self.assertLess(warning['deadlines']['signing_key'], warning['deadlines']['release'])
        self.assertEqual(check(self.signed, self.digest, self.public, warning=10)['state'], 'current')

    def test_expired_signing_key_rejected(self):
        past = int(time.time())-3*86400
        home = self.root/'expired-key'
        subkey, public = key(home, past)
        self.addCleanup(lambda: execute(['gpgconf', '--homedir', home, '--kill', 'gpg-agent']))
        shutil.copyfile(public, self.apt.public)
        self.resign(subkey=subkey, home=home, stamp=past+60)
        output = self.apt.cmd('update', ok=False)
        self.assertIn('EXPKEYSIG', output)

    def test_primary_certification_key_cannot_be_selected_for_signing(self):
        primary = enrollment.public_key(self.public.read_bytes())[0]
        with self.assertRaisesRegex(ValueError, 'explicit_signing_subkey_required'):
            archive.sign(self.unsigned, self.digest, self.root/'rejected', self.base/'key-a', primary, self.public)

    def test_appstream_collection_parses(self):
        result = execute(['appstreamcli', 'validate', '--no-net',
                          self.served/archive.SUITE/'main/dep11/Components-amd64.xml'])
        self.assertEqual(result.returncode, 0, result.stdout.decode())

    def test_real_apt_by_hash_appstream_and_ordinary_upgrade(self):
        self.apt.dpkg('--install', self.main, self.islands)
        self.apt.cmd('update')
        self.assertTrue(any('/by-hash/SHA256/' in x for x in self.server.requests))
        self.assertTrue(any('dep11' in x for x in self.server.requests))
        newer, _ = self.new_generation()
        self.serve(newer)
        self.apt.cmd('update')
        self.apt.cmd('-y', 'upgrade')
        for name in archive.PACKAGES[:2]:
            self.assertIn('Version: 0.2.1', self.apt.dpkg('--status', name))

    def test_main_upgrade_does_not_install_absent_islands(self):
        self.apt.dpkg('--install', self.main)
        newer, _ = self.new_generation()
        self.serve(newer)
        self.apt.cmd('update')
        self.apt.cmd('-y', 'upgrade')
        status = (self.apt.root/'var/lib/dpkg/status').read_text()
        self.assertIn('Version: 0.2.1', status)
        self.assertNotIn('Package: '+archive.PACKAGES[1]+'\n', status)

    def test_deterministic_indexes_and_bootstrap(self):
        same = self.root/'same'
        digest = archive.stage([self.main, self.islands, *self.bootstrap], {}, same, self.epoch)
        self.assertEqual(digest, self.digest)
        rebuilt = enrollment.build(self.public, '1.0', self.root/'bootstrap', self.epoch)
        self.assertEqual([archive.sha(p) for p in rebuilt], [archive.sha(p) for p in self.bootstrap])

    def test_enrollment_installed_checksums_cover_payload_and_detect_changes(self):
        for path in (enrollment.KEY, enrollment.SOURCE, enrollment.PREFERENCES):
            (self.apt.root/path.lstrip('/')).unlink(missing_ok=True)
        self.apt.dpkg('--install', *self.bootstrap)
        info = self.apt.root/'var/lib/dpkg/info'
        recorded = {}
        for name in enrollment.PACKAGES[2:]:
            checksum_file = info/(name+'.md5sums')
            self.assertTrue(checksum_file.is_file(), name+' lacks installed checksum records')
            rows = checksum_file.read_text().splitlines()
            self.assertTrue(rows)
            for row in rows:
                digest, relative = row.split(maxsplit=1)
                self.assertFalse(relative.startswith('/'))
                payload = self.apt.root/relative
                self.assertEqual(hashlib.md5(payload.read_bytes()).hexdigest(), digest)
                recorded['/'+relative] = digest
            installed_files = (info/(name+'.list')).read_text().splitlines()
            regular = {path for path in installed_files if (self.apt.root/path.lstrip('/')).is_file()}
            self.assertTrue(regular.issubset(recorded))
        for path in (enrollment.KEY, enrollment.SOURCE, enrollment.PREFERENCES):
            self.assertIn(path, recorded)
        self.apt.public.write_bytes(self.apt.public.read_bytes()+b'changed')
        self.assertNotEqual(hashlib.md5(self.apt.public.read_bytes()).hexdigest(), recorded[enrollment.KEY])

    def test_immutable_versions_and_debian_equivalent_versions(self):
        for version in ('0.2.0', '0.2.0-0'):
            with self.subTest(version=version):
                changed = package(self.root, archive.PACKAGES[0], version)
                with changed.open('ab') as stream:
                    stream.write(b'changed container bytes')
                with self.assertRaisesRegex(ValueError, 'immutable_version_collision'):
                    archive.stage([changed], {}, self.root/('reject-'+version), self.epoch+60,
                                  self.signed, self.digest)

    def test_wrong_signature_missing_key_unsigned_and_corrupt_signature(self):
        for mode in ('wrong', 'missing', 'unsigned', 'corrupt'):
            with self.subTest(mode=mode):
                self.serve(self.signed)
                shutil.copyfile(self.public, self.apt.public)
                if mode == 'wrong':
                    self.resign(subkey=self.subkey_b, home=self.base/'key-b')
                elif mode == 'missing':
                    self.apt.public.unlink()
                elif mode == 'unsigned':
                    for name in ('InRelease', 'Release.gpg'):
                        (self.served/archive.SUITE/name).unlink()
                else:
                    path = self.served/archive.SUITE/'InRelease'
                    path.write_bytes(path.read_bytes().replace(b'Origin: Halcyon', b'Origin: Evilxxx'))
                self.apt.cmd('update', ok=False)

    def test_expired_release_and_future_release(self):
        for field, replacement in [('Valid-Until', 'Thu, 01 Jan 2015 00:00:00 GMT'),
                                    ('Date', 'Thu, 01 Jan 2037 00:00:00 GMT')]:
            self.serve(self.signed)
            path = self.served/archive.SUITE/'Release'
            lines = path.read_text().splitlines()
            path.write_text('\n'.join(field+': '+replacement if line.startswith(field+':') else line for line in lines)+'\n')
            self.resign()
            self.apt.cmd('update', ok=False)
            self.serve(self.signed)
            self.apt.cmd('update')  # Renewal recovers without a trust bypass.

    def test_corrupt_index_and_payload_hash_rejected(self):
        directory = self.served/archive.SUITE/'main/binary-amd64'
        for path in directory.rglob('*'):
            if path.is_file():
                path.write_bytes(b'corrupt index')
        failure = self.apt.cmd('update', ok=False)
        self.assertTrue('Hash Sum mismatch' in failure or 'unexpected size' in failure, failure)
        self.serve(self.signed)
        self.apt.cmd('update')
        record = next(r for r in json.loads((self.served/'archive.json').read_bytes())['packages']
                      if r['package'] == archive.PACKAGES[0])
        with (self.served/record['path']).open('ab') as stream:
            stream.write(b'corruption')
        output = self.apt.cmd('-y', 'install', archive.PACKAGES[0], ok=False)
        self.assertTrue('Hash Sum mismatch' in output or 'unexpected size' in output, output)

    def test_retained_version_ordering_and_no_implicit_downgrade(self):
        newer, _ = self.new_generation('0.2.1+islands.10')
        self.serve(newer)
        self.apt.cmd('update')
        policy = self.apt.cmd('policy', archive.PACKAGES[0], tool='apt-cache')
        self.assertIn('Candidate: 0.2.1+islands.10', policy)
        self.assertIn('0.2.0', policy)
        self.apt.cmd('-y', 'install', archive.PACKAGES[0]+'=0.2.0')
        self.apt.cmd('-y', 'upgrade')
        self.serve(self.signed)
        self.apt.cmd('update')
        self.assertIn('Candidate: 0.2.1+islands.10', self.apt.cmd('policy', archive.PACKAGES[0], tool='apt-cache'))
        self.assertTrue(archive.compare('0.2.1~rc1', 'lt', '0.2.1'))
        self.assertTrue(archive.compare('0.2.1+islands.9', 'lt', '0.2.1+islands.10'))

    def test_client_spanning_publication_uses_retained_hash(self):
        newer, _ = self.new_generation()
        self.server.after_release = lambda: self.serve(newer)
        self.apt.cmd('update')
        policy = self.apt.cmd('policy', archive.PACKAGES[0], tool='apt-cache')
        self.assertIn('Candidate: 0.2.0', policy)
        self.apt.cmd('-y', 'install', archive.PACKAGES[0])
        self.apt.cmd('update')
        self.assertIn('Candidate: 0.2.1', self.apt.cmd('policy', archive.PACKAGES[0], tool='apt-cache'))

    def test_failed_promotion_keeps_previous_complete_generation(self):
        deployment = self.root/'deployment'
        archive.promote(self.signed, self.digest, self.public, deployment, None)
        newer, digest = self.new_generation()
        (newer/archive.SUITE/'InRelease').write_bytes(b'interrupted')
        with self.assertRaises(ValueError):
            archive.promote(newer, digest, self.public, deployment, self.digest)
        self.assertEqual(archive.sha(deployment/'current/archive.json'), self.digest)
        archive.verify_signed(deployment/'current', self.digest, self.public)

    def test_enrollment_remove_purge_and_admin_disabled_source(self):
        self.apt.dpkg('--install', self.main, self.islands)
        settings = self.apt.root/'home/fixture/.config/halcyon/settings'
        settings.parent.mkdir(parents=True)
        settings.write_bytes(b'private user settings must survive enrollment')
        built = enrollment.build(self.public, '1.1', self.root/'enroll', self.epoch, self.server.uri)
        self.apt.source.unlink()
        self.apt.public.unlink()
        self.apt.dpkg('--install', *built)
        # Signed-By is an absolute OS path. Redirect only that path for this
        # isolated apt root; production package bytes retain /usr/share/keyrings.
        self.apt.source.write_text(self.apt.source.read_text().replace(enrollment.KEY, str(self.apt.public)))
        configured_bytes = self.apt.source.read_bytes()
        self.apt.cmd('update')
        admin = self.apt.source.with_name('administrator.disabled')
        admin.write_text('administrator owned')
        self.apt.dpkg('--remove', 'halcyon-update-source')
        self.assertFalse(self.apt.source.exists())
        saved = list(self.apt.source.parent.glob('halcyon.sources.removed.*'))
        self.assertEqual(len(saved), 1)
        self.assertEqual(saved[0].read_bytes(), configured_bytes)
        self.apt.cmd('update')
        self.assertNotIn(self.server.uri, self.apt.cmd('policy', archive.PACKAGES[0], tool='apt-cache'))
        self.apt.dpkg('--purge', 'halcyon-update-source', 'halcyon-archive-keyring')
        self.assertFalse(self.apt.public.exists())
        self.assertEqual(admin.read_text(), 'administrator owned')
        self.assertEqual(settings.read_bytes(), b'private user settings must survive enrollment')
        for name in archive.PACKAGES[:2]:
            self.assertIn('Version: 0.2.0', self.apt.dpkg('--status', name))
        self.apt.dpkg('--install', *built)
        self.apt.source.unlink()  # Administrator opted out.
        next_boot = enrollment.build(self.public, '1.2', self.root/'reenroll', self.epoch, self.server.uri)
        self.apt.dpkg('--install', *next_boot)
        self.assertFalse(self.apt.source.exists(), 'package upgrade must not re-enable deleted conffile')

    def test_rotation_overlap_keyring_then_single_new_signature(self):
        combined = self.root/'overlap.gpg'
        combined.write_bytes(self.public.read_bytes()+self.public_b.read_bytes())
        boot = enrollment.build(combined, '2.0', self.root/'rotation', self.epoch)
        self.apt.source.unlink()
        initial = enrollment.build(self.public, '1.0', self.root/'rotation-bootstrap', self.epoch, self.server.uri)
        self.apt.dpkg('--install', *initial)
        self.apt.source.write_text(enrollment.configuration(self.server.uri, str(self.apt.public))[0])
        overlap, _ = self.new_generation(boot=boot)
        self.serve(overlap)
        self.apt.cmd('update')
        self.apt.cmd('-y', 'install', 'halcyon-archive-keyring')
        self.assertEqual(self.apt.public.read_bytes(), combined.read_bytes())
        self.resign(subkey=self.subkey_b, home=self.base/'key-b')
        self.apt.cmd('update')
        shutil.copyfile(self.public, self.apt.public)
        self.apt.cmd('update', ok=False)  # Missed bridge requires explicit recovery.
        shutil.copyfile(combined, self.apt.public)
        self.apt.cmd('update')
        shutil.copyfile(self.public_b, self.apt.public)
        self.apt.cmd('update')
        self.resign()  # Retired key cannot sign for new-only clients.
        self.apt.cmd('update', ok=False)


if __name__ == '__main__':
    if os.geteuid() != 0:
        os.execvp('unshare', ['unshare', '--user', '--map-root-user', sys.executable, '-B', __file__, *sys.argv[1:]])
    unittest.main()
