#!/usr/bin/env python3
"""Build explicit enrollment assets; never install or generate signing keys."""
import argparse
import gzip
import os
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import urlsplit

PACKAGES = ('cosmic-ext-halcyon', 'cosmic-ext-halcyon-islands',
            'halcyon-archive-keyring', 'halcyon-update-source')
URI = 'https://profilaz3.github.io/halcyon/apt'
KEY = '/usr/share/keyrings/halcyon-archive-keyring.gpg'
SOURCE = '/etc/apt/sources.list.d/halcyon.sources'
PREFERENCES = '/etc/apt/preferences.d/halcyon'


def configuration(uri=URI, key=KEY):
    parsed = urlsplit(uri)
    if (parsed.scheme not in ('https', 'http') or not parsed.hostname or
            parsed.username or parsed.password or parsed.query or parsed.fragment or
            any(c.isspace() for c in uri + key) or
            (parsed.scheme == 'http' and parsed.hostname != '127.0.0.1')):
        raise ValueError('invalid_archive_uri')
    source = (f'Types: deb\nURIs: {uri}\nSuites: noble\nComponents: main\n'
              f'Architectures: amd64\nSigned-By: {key}\n')
    # First specific match wins. Origin below means signed Release fields;
    # lowercase origin in the last record means the transport hostname.
    prefs = (f'Package: {" ".join(PACKAGES)}\n'
             'Pin: release o=Halcyon,l=Halcyon,a=noble,n=noble,c=main\nPin-Priority: 500\n\n'
             'Package: *\nPin: release o=Halcyon\nPin-Priority: -1\n\n'
             f'Package: *\nPin: origin "{parsed.hostname}"\nPin-Priority: -1\n')
    return source, prefs


def public_key(data):
    # Signed-By ends in .gpg: APT expects exported binary OpenPGP material,
    # not armored .asc or a GnuPG keybox renamed to that extension.
    if data.lstrip().startswith(b'-----BEGIN'):
        raise ValueError('binary_public_keyring_required')
    # Inspect in a private empty home: no implicit trust/key imports. Reject
    # secret packets even if GnuPG could otherwise export their public half.
    with tempfile.TemporaryDirectory(prefix='halcyon-public-key-') as work:
        key = Path(work) / 'public.gpg'
        key.write_bytes(data)
        result = subprocess.run(['gpg', '--batch', '--homedir', work, '--with-colons',
                                 '--show-keys', str(key)], capture_output=True, timeout=15)
        packets = subprocess.run(['gpg', '--batch', '--homedir', work, '--list-packets',
                                  str(key)], capture_output=True, timeout=15)
        if result.returncode or packets.returncode or b':secret' in packets.stdout:
            raise ValueError('public_keyring_required')
        lines = result.stdout.decode().splitlines()
        if not any(line.startswith('pub:') for line in lines):
            raise ValueError('public_keyring_empty')
        return [line.split(':')[9] for line in lines if line.startswith('fpr:')]


def build(keyring, version, out, epoch, uri=URI):
    if not re.fullmatch(r'[0-9][A-Za-z0-9.+~\-]*', version) or epoch < 0:
        raise ValueError('invalid_version_or_epoch')
    public_key(keyring.read_bytes())
    out.mkdir(parents=True, exist_ok=True)
    source, prefs = configuration(uri)
    results = []
    for name in PACKAGES[2:]:
        output = out / f'{name}_{version}_all.deb'
        if output.exists():
            raise ValueError('immutable_package_exists')
        with tempfile.TemporaryDirectory(prefix='halcyon-enrollment-') as work:
            root = Path(work)
            def put(path, data, mode=0o644):
                target = root / path.lstrip('/')
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data if isinstance(data, bytes) else data.encode())
                target.chmod(mode)
            depends = ''
            if name == PACKAGES[2]:
                put(KEY, keyring.read_bytes())
            else:
                depends = f'Depends: halcyon-archive-keyring (>= {version})\n'
                put(SOURCE, source)
                put(PREFERENCES, prefs)
                put('DEBIAN/conffiles', SOURCE + '\n' + PREFERENCES + '\n')
                put(f'usr/share/lintian/overrides/{name}',
                    '# This explicit third-party enrollment package exists to own these two files.\n'
                    '# The app never Depends/Recommends this package; initial installation is opt-in.\n'
                    f'{name}: package-installs-apt-sources [etc/apt/sources.list.d/halcyon.sources]\n'
                    f'{name}: package-installs-apt-preferences [etc/apt/preferences.d/halcyon]\n')
                # dpkg remove normally leaves conffiles active. Save the exact
                # source bytes under an ignored suffix instead, even if edited.
                # Never overwrite another admin file or resurrect a deleted source.
                put('DEBIAN/postrm', '''#!/bin/sh
set -eu
if [ "$1" = remove ]; then
    source="${DPKG_ROOT:-}/etc/apt/sources.list.d/halcyon.sources"
    if [ -e "$source" ] || [ -L "$source" ]; then
        saved=$(mktemp "${source}.removed.XXXXXX")
        mv -- "$source" "$saved"
    fi
fi
''', 0o755)
            put('DEBIAN/control', f'Package: {name}\nVersion: {version}\nArchitecture: all\n'
                'Section: admin\nPriority: optional\n'
                'Maintainer: profilaz3 <261673162+profilaz3@users.noreply.github.com>\n'
                'Homepage: https://github.com/profilaz3/halcyon\n' + depends +
                'Description: Explicit Halcyon system update enrollment\n'
                ' Scoped archive trust and source configuration; no desktop activation.\n')
            put(f'usr/share/doc/{name}/copyright',
                'Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n'
                'Upstream-Name: Halcyon\nSource: https://github.com/profilaz3/halcyon\n\n'
                'Files: *\nCopyright: 2026 profilaz3\nLicense: GPL-3+\n'
                ' See /usr/share/common-licenses/GPL-3.\n')
            put(f'usr/share/doc/{name}/changelog.gz', gzip.compress(
                (f'halcyon-updates ({version}) noble; urgency=medium\n\n'
                 '  * Explicit scoped system update enrollment.\n\n'
                 ' -- profilaz3 <261673162+profilaz3@users.noreply.github.com>  Thu, 01 Jan 2026 00:00:00 +0000\n').encode(), mtime=0))
            root.chmod(0o755)
            for path in root.rglob('*'):
                if path.is_dir():
                    path.chmod(0o755)
                os.utime(path, (epoch, epoch))
            env = {**os.environ, 'SOURCE_DATE_EPOCH': str(epoch)}
            subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(root), str(output.resolve())],
                           env=env, check=True, stdout=subprocess.DEVNULL, timeout=30)
        results.append(output)
    return results


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--public-keyring', type=Path, required=True)
    p.add_argument('--version', required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--epoch', type=int, required=True)
    a = p.parse_args()
    build(a.public_keyring, a.version, a.out, a.epoch)
