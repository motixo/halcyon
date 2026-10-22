#!/usr/bin/env python3
"""Package an already built, reviewed Islands binary without activating it."""
import argparse
import datetime
import gzip
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
COMPANION = HERE.parent.parent / 'companion'
sys.path.insert(0, str(COMPANION))
import panel_launcher

PACKAGE = 'cosmic-ext-halcyon-islands'
MAINTAINER = 'profilaz3 <261673162+profilaz3@users.noreply.github.com>'


def stage_file(source, destination, mode=0o644):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    destination.chmod(mode)


def build(binary, manifest, version, out):
    if not re.fullmatch(r'[0-9][A-Za-z0-9.+:~\-]*', version):
        raise ValueError('Invalid Debian version')
    subprocess.run(['dpkg', '--validate-version', version], check=True)
    epoch = int(os.environ.get('SOURCE_DATE_EPOCH', str(int(datetime.datetime.now().timestamp()))))
    date = datetime.datetime.fromtimestamp(epoch, datetime.timezone.utc)
    out.mkdir(parents=True, exist_ok=True)
    filename = f'{PACKAGE}_{version.split(":", 1)[-1]}_amd64.deb'
    with tempfile.TemporaryDirectory(prefix='islands-build-', dir=out) as temporary:
        staging = Path(temporary) / 'package'
        library = staging / 'usr/lib' / PACKAGE
        stage_file(binary, library / 'cosmic-panel-islands', 0o755)
        stage_file(manifest, library / 'companion-manifest.json')
        if not panel_launcher.compatible(library, panel_launcher.PACKAGES, 'companion-manifest.json'):
            raise ValueError('Binary or manifest fails the selector checksum, upstream or protocol checks')
        for name in ('install.py', 'panel_launcher.py'):
            stage_file(COMPANION / name, library / name, 0o755)
        stage_file(HERE / 'halcyon-islands', staging / 'usr/bin/halcyon-islands', 0o755)
        docs = staging / 'usr/share/doc' / PACKAGE
        stage_file(HERE / 'copyright', docs / 'copyright')
        stage_file(HERE / 'README.md', docs / 'README')
        changelog = (f'{PACKAGE} ({version}) unstable; urgency=medium\n\n'
                     '  * Package the optional, version-locked Islands panel.\n\n'
                     f' -- {MAINTAINER}  {date.strftime("%a, %d %b %Y %H:%M:%S %z")}\n')
        (docs / 'changelog.gz').write_bytes(gzip.compress(changelog.encode(), mtime=0))
        man = staging / 'usr/share/man/man1/halcyon-islands.1.gz'
        man.parent.mkdir(parents=True)
        man.write_bytes(gzip.compress((HERE / 'halcyon-islands.1').read_bytes(), mtime=0))
        dependencies = ['python3', 'cosmic-ext-halcyon'] + [
            f'{name} (= {pin})' for name, pin in panel_launcher.PACKAGES.items()]
        size = sum(path.stat().st_size for path in staging.rglob('*') if path.is_file())
        control = staging / 'DEBIAN/control'
        control.parent.mkdir()
        control.write_text(
            f'Package: {PACKAGE}\nVersion: {version}\nArchitecture: amd64\n'
            f'Maintainer: {MAINTAINER}\nSection: utils\nPriority: optional\n'
            f'Installed-Size: {(size + 1023) // 1024}\n'
            f'Depends: {", ".join(dependencies)}\n'
            'Homepage: https://github.com/profilaz3/halcyon\n'
            'Description: optional version-locked floating panel islands for Halcyon\n'
            ' A patched COSMIC panel that splits the panel into floating islands.\n'
            ' This separate add-on requires the exact tested COSMIC package versions.\n'
            ' Activation is per user with halcyon-islands enable, at the next login.\n'
            ' Installing this package does not activate or restart a panel.\n')
        staging.chmod(0o755)
        for path in staging.rglob('*'):
            if path.is_dir():
                path.chmod(0o755)
            elif path not in (library / 'cosmic-panel-islands', library / 'install.py',
                              library / 'panel_launcher.py', staging / 'usr/bin/halcyon-islands'):
                path.chmod(0o644)
            os.utime(path, (epoch, epoch))
        os.utime(staging, (epoch, epoch))
        artifact = Path(temporary) / filename
        subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(staging), str(artifact)],
                       env={**os.environ, 'SOURCE_DATE_EPOCH': str(epoch)}, check=True)
        checksum = hashlib.sha256(artifact.read_bytes()).hexdigest()
        destination = out / filename
        artifact.replace(destination)
        destination.with_suffix('.deb.sha256').write_text(f'{checksum}  {filename}\n')
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True, help='prebuilt cosmic-panel-islands')
    parser.add_argument('--manifest', type=Path, required=True, help='reviewed companion-manifest.json')
    parser.add_argument('--version', required=True)
    parser.add_argument('--out', type=Path, required=True, help='output directory')
    args = parser.parse_args()
    try:
        print(build(args.binary, args.manifest, args.version, args.out))
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Cannot build Islands package: {error}\n')


if __name__ == '__main__':
    main()
