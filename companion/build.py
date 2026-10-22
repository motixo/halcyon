#!/usr/bin/env python3
"""Build the pinned companion independently of Desktop Cards' UI runtime."""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

REVISION = '3ce6ce1cec97669f91e809962d9446aaefe8a06e'
ARCHIVE_SHA256 = 'e589390a38ff01bd82d1b517129c3635484a9c06b0b677316085de5d8c279cfb'


def prepare(archive, destination):
    if hashlib.sha256(archive.read_bytes()).hexdigest() != ARCHIVE_SHA256:
        raise ValueError('Upstream archive hash does not match the reviewed pin')
    if destination.exists():
        raise ValueError('Choose a new empty build directory')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=destination.parent) as staging:
        with tarfile.open(archive) as source:
            source.extractall(staging, filter='data')
        extracted = Path(staging) / ('cosmic-panel-' + REVISION)
        shutil.move(extracted, destination)
    patch = Path(__file__).with_name('patches') / '0001-dock-islands.patch'
    subprocess.run(['patch', '--batch', '--forward', '-p1', '-i', str(patch.resolve())], cwd=destination, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--workdir', type=Path, required=True)
    parser.add_argument('--target-dir', type=Path, required=True)
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--jobs', type=int, default=6)
    args = parser.parse_args()
    archive = args.archive
    if archive is None:
        if args.offline:
            parser.error('--offline needs the verified --archive')
        archive = args.workdir.parent / ('cosmic-panel-' + REVISION + '.tar.gz')
        archive.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve('https://codeload.github.com/pop-os/cosmic-panel/tar.gz/' + REVISION, archive)
    prepare(archive.resolve(), args.workdir.resolve())
    if not args.prepare_only:
        env = {**os.environ, 'RUSTUP_TOOLCHAIN': '1.93.0', 'CARGO_TARGET_DIR': str(args.target_dir.resolve())}
        command = ['cargo', 'build', '--release', '--locked', '-j', str(args.jobs)]
        if args.offline:
            command.append('--offline')
        subprocess.run(command, cwd=args.workdir, env=env, check=True)


if __name__ == '__main__':
    main()
