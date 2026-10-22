#!/usr/bin/env python3
"""Install the complete, bundled Desktop Cards wallpapers without network access."""
import argparse
import os
from pathlib import Path
import shutil
import struct
import sys
import tempfile
import zlib


THEMES = ('halcyon-night', 'halcyon-calm', 'halcyon-evening', 'halcyon-focus',
          'halcyon-workbench', 'halcyon-paper', 'halcyon-rose',
          'lavender', 'amber', 'teal', 'ocean', 'rose', 'graphite')
RESOLUTIONS = {'4k': (3840, 2160), '1080p': (1920, 1080)}
BUNDLE = Path(__file__).resolve().parent.parent / 'assets' / 'wallpapers'


def _themes(themes):
    selected = tuple(dict.fromkeys((themes,) if isinstance(themes, str) else themes))
    for theme in selected:
        if theme not in THEMES:
            raise RuntimeError(f'Unknown wallpaper theme: {theme}')
    return selected


def _destination(destination):
    return Path(destination) if destination is not None else Path.home() / 'Pictures' / 'Desktop Cards'


def validate_png(path, expected_size):
    """Check the real PNG dimensions and every chunk's CRC, including its end marker."""
    path = Path(path)

    def invalid(reason):
        return RuntimeError(f'Invalid wallpaper {path}: {reason}')

    try:
        with path.open('rb') as image:
            header = image.read(33)
            if (len(header) != 33 or header[:8] != b'\x89PNG\r\n\x1a\n'
                    or header[8:16] != b'\x00\x00\x00\rIHDR'
                    or zlib.crc32(header[12:29]) != struct.unpack('>I', header[29:33])[0]):
                raise invalid('missing or corrupt PNG header')
            size = struct.unpack('>II', header[16:24])
            if size != expected_size:
                raise invalid(f'expected {expected_size[0]}×{expected_size[1]}, found {size[0]}×{size[1]}')
            if header[26] != 0 or header[27] != 0 or header[28] not in (0, 1):
                raise invalid('unsupported PNG compression, filter or interlace method')
            has_pixels = False
            while True:
                chunk = image.read(8)
                if len(chunk) != 8:
                    raise invalid('incomplete PNG')
                length, kind = struct.unpack('>I4s', chunk)
                if kind == b'IHDR' or (kind == b'IEND' and length != 0):
                    raise invalid('unexpected PNG chunk')
                remaining = length
                checksum = zlib.crc32(kind)
                while remaining:
                    data = image.read(min(remaining, 65536))
                    if not data:
                        raise invalid('incomplete PNG chunk')
                    checksum = zlib.crc32(data, checksum)
                    remaining -= len(data)
                stored_checksum = image.read(4)
                if len(stored_checksum) != 4 or checksum != struct.unpack('>I', stored_checksum)[0]:
                    raise invalid('corrupt PNG chunk')
                if kind == b'IDAT' and length:
                    has_pixels = True
                if kind == b'IEND':
                    if not has_pixels or image.read(1):
                        raise invalid('missing image data or unexpected bytes after PNG end')
                    return size
    except OSError as error:
        raise RuntimeError(f'Cannot read wallpaper {path}: {error}') from error


def validate_bundle(source=None, *, themes=THEMES):
    """Validate all requested source files before an installation changes any files."""
    source = Path(source) if source is not None else BUNDLE
    paths = []
    for theme in _themes(themes):
        for resolution, size in RESOLUTIONS.items():
            path = source / f'{theme}-{resolution}.png'
            validate_png(path, size)
            paths.append(path)
    return paths


def _identical(source, destination):
    try:
        if source.stat().st_size != destination.stat().st_size:
            return False
        with source.open('rb') as original, destination.open('rb') as installed:
            while True:
                data = original.read(65536)
                if data != installed.read(65536):
                    return False
                if not data:
                    return True
    except FileNotFoundError:
        return False


def _atomic_copy(source, destination):
    temporary = None
    try:
        with source.open('rb') as original, tempfile.NamedTemporaryFile(
                mode='wb', prefix=f'.{destination.name}.', dir=destination.parent,
                delete=False) as installed:
            temporary = Path(installed.name)
            shutil.copyfileobj(original, installed, length=65536)
            os.fchmod(installed.fileno(), 0o644)
            installed.flush()
            os.fsync(installed.fileno())
        os.replace(temporary, destination)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def install_wallpapers(destination=None, *, source=None, themes=THEMES):
    """Copy bundled PNGs atomically, keeping identical copies and unrelated files."""
    sources = validate_bundle(source, themes=themes)
    destination = _destination(destination)
    destination.mkdir(parents=True, exist_ok=True)
    installed = []
    for original in sources:
        target = destination / original.name
        if not _identical(original, target):
            _atomic_copy(original, target)
        installed.append(target)
    return installed


def ensure_wallpaper(theme, destination=None, *, source=None):
    """Return a theme's 4K path, repairing missing or damaged local copies offline."""
    _themes((theme,))
    destination = _destination(destination)
    try:
        for resolution, size in RESOLUTIONS.items():
            validate_png(destination / f'{theme}-{resolution}.png', size)
    except RuntimeError:
        install_wallpapers(destination, source=source, themes=(theme,))
    return destination / f'{theme}-4k.png'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=BUNDLE, help='bundled PNG directory')
    parser.add_argument('--destination', type=Path, help='defaults to ~/Pictures/Desktop Cards')
    parser.add_argument('--themes', nargs='+', choices=THEMES, default=THEMES)
    parser.add_argument('--check', action='store_true', help='validate the bundle without installing it')
    args = parser.parse_args()
    if args.check:
        paths = validate_bundle(args.source, themes=args.themes)
        print(f'Validated {len(paths)} bundled wallpapers in {args.source}')
    else:
        paths = install_wallpapers(args.destination, source=args.source, themes=args.themes)
        print(f'Installed or verified {len(paths)} wallpapers in {_destination(args.destination)}')


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
