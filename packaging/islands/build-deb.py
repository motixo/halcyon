#!/usr/bin/env python3
"""Package an already built, reviewed Islands binary without activating it."""
import argparse
import datetime
import gzip
import hashlib
import json
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


def build(binary, manifest, version, out, licenses=None):
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
        # Shipped without debug symbols. Only symbols go, so the reviewed
        # build's behavior is unchanged; the manifest records the new hash.
        staged = library / 'cosmic-panel-islands'
        if staged.read_bytes()[:4] == b'\x7fELF':
            if licenses is None or not licenses.is_file() or not licenses.stat().st_size:
                raise ValueError('Native releases require --licenses with dependency notices')
            subprocess.run(['strip', '--strip-unneeded', str(staged)], check=True)
            record = json.loads((library / 'companion-manifest.json').read_text())
            record['binary_sha256'] = hashlib.sha256(staged.read_bytes()).hexdigest()
            (library / 'companion-manifest.json').write_text(json.dumps(record, indent=2) + '\n')
            if not panel_launcher.compatible(library, panel_launcher.PACKAGES, 'companion-manifest.json'):
                raise ValueError('The stripped binary fails the selector checks')
        record = json.loads((library / 'companion-manifest.json').read_text())
        record['packages'] = panel_launcher.PACKAGES
        record['supported_packages'] = panel_launcher.SUPPORTED_SYSTEMS
        if record.get('runtime_probe') == 1:
            record['source_policy'] = panel_launcher.SOURCE_POLICY
        (library / 'companion-manifest.json').write_text(json.dumps(record, indent=2) + '\n')
        for name in ('install.py', 'panel_launcher.py'):
            stage_file(COMPANION / name, library / name, 0o755)
        stage_file(HERE / 'halcyon-islands', staging / 'usr/bin/halcyon-islands', 0o755)
        docs = staging / 'usr/share/doc' / PACKAGE
        stage_file(HERE / 'copyright', docs / 'copyright')
        if licenses is not None:
            stage_file(licenses, docs / 'THIRD_PARTY_LICENSES.txt')
        stage_file(HERE / 'README.md', docs / 'README')
        changelog = (f'{PACKAGE} ({version}) unstable; urgency=medium\n\n'
                     '  * Package the optional, source-qualified Islands panel.\n\n'
                     f' -- {MAINTAINER}  {date.strftime("%a, %d %b %Y %H:%M:%S %z")}\n')
        (docs / 'changelog.gz').write_bytes(gzip.compress(changelog.encode(), mtime=0))
        man = staging / 'usr/share/man/man1/halcyon-islands.1.gz'
        man.parent.mkdir(parents=True)
        man.write_bytes(gzip.compress((HERE / 'halcyon-islands.1').read_bytes(), mtime=0))
        debian = Path(temporary) / 'debian'
        debian.mkdir()
        (debian / 'control').write_text(
            f'Source: {PACKAGE}\nSection: utils\nPriority: optional\n'
            f'Maintainer: {MAINTAINER}\n\nPackage: {PACKAGE}\n'
            'Architecture: amd64\nDescription: Optional Islands panel\n')
        shlibs = subprocess.run(
            ['dpkg-shlibdeps', '-O', f'-e{library / "cosmic-panel-islands"}'],
            cwd=Path(temporary), capture_output=True, text=True)
        native = next((line.removeprefix('shlibs:Depends=').strip()
                       for line in shlibs.stdout.splitlines()
                       if line.startswith('shlibs:Depends=')), '')
        if staged.read_bytes()[:4] == b'\x7fELF' and (shlibs.returncode != 0 or not native):
            raise ValueError('Cannot determine native library dependencies: ' + shlibs.stderr.strip())
        dependencies = ([native] if shlibs.returncode == 0 and native else ['libc6']) + [
            'python3', 'cosmic-ext-halcyon'] + [
            name for name in panel_launcher.PACKAGES]
        size = sum(path.stat().st_size for path in staging.rglob('*') if path.is_file())
        control = staging / 'DEBIAN/control'
        control.parent.mkdir()
        control.write_text(
            f'Package: {PACKAGE}\nVersion: {version}\nArchitecture: amd64\n'
            f'Maintainer: {MAINTAINER}\nSection: utils\nPriority: optional\n'
            f'Installed-Size: {(size + 1023) // 1024}\n'
            f'Depends: {", ".join(dependencies)}\n'
            'Homepage: https://github.com/profilaz3/halcyon\n'
            'Description: optional source-qualified floating panel islands for Halcyon\n'
            ' A patched COSMIC panel that splits the panel into floating islands.\n'
            ' The launcher uses the standard panel if source or runtime checks fail.\n'
            ' Installing the add-on never holds back COSMIC system updates.\n'
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
    parser.add_argument('--licenses', type=Path, help='dependency license report (required for native binaries)')
    parser.add_argument('--out', type=Path, required=True, help='output directory')
    args = parser.parse_args()
    try:
        print(build(args.binary, args.manifest, args.version, args.out, args.licenses))
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Cannot build Islands package: {error}\n')


if __name__ == '__main__':
    main()
