#!/usr/bin/env python3
"""Build immutable public bootstrap release assets and their corresponding source."""
import argparse
import gzip
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('enrollment', HERE/'build-deb.py')
enrollment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(enrollment)


def build(public, version, epoch, out):
    if out.exists():
        raise ValueError('fresh_bootstrap_output_required')
    debs = enrollment.build(public, version, out, epoch)
    license_path = HERE.parents[1]/'LICENSE'
    files = {'build-deb.py': (HERE/'build-deb.py').read_bytes(),
             'halcyon-archive-keyring.gpg': public.read_bytes(),
             'COPYING': license_path.read_bytes(),
             'BUILD.md': (f'# Halcyon update enrollment {version}\n\n'
                          'Copyright 2026 profilaz3. GPL-3.0-or-later; see COPYING.\n\n'
                          'Requirements: Python 3, dpkg-deb, GnuPG. No private key required.\n\n'
                          f'python3 build-deb.py --public-keyring halcyon-archive-keyring.gpg '
                          f'--version {version} --epoch {epoch} --out dist\n').encode()}
    source = out/f'halcyon-apt-bootstrap-{version}-source.tar.gz'
    with source.open('wb') as stream, gzip.GzipFile(filename='', fileobj=stream, mode='wb', mtime=0) as zipped:
        with tarfile.open(fileobj=zipped, mode='w', format=tarfile.USTAR_FORMAT) as archive:
            for name, data in sorted(files.items()):
                entry = tarfile.TarInfo(f'halcyon-apt-bootstrap-{version}/'+name)
                entry.mode, entry.mtime, entry.size = 0o644, epoch, len(data)
                archive.addfile(entry, io.BytesIO(data))
    assets = {p.name: dict(sha256=hashlib.sha256(p.read_bytes()).hexdigest(), bytes=p.stat().st_size,
                          kind='enrollment-deb' if p in debs else 'corresponding-source') for p in [*debs, source]}
    manifest = dict(format='halcyon-apt-bootstrap', version=1, package_version=version,
                    source_date_epoch=epoch, public_key_sha256=hashlib.sha256(public.read_bytes()).hexdigest(),
                    artifacts=assets)
    (out/'bootstrap-manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
    (out/'SHA256SUMS').write_text(''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n'
                                        for p in sorted(out.iterdir()) if p.is_file()))
    return manifest


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--public-keyring', type=Path, required=True)
    p.add_argument('--version', required=True)
    p.add_argument('--epoch', type=int, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    build(a.public_keyring, a.version, a.epoch, a.out)
