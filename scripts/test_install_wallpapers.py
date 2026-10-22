import importlib.util
from functools import lru_cache
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zlib


spec = importlib.util.spec_from_file_location(
    'install_wallpapers', Path(__file__).with_name('install_wallpapers.py'))
wallpapers = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wallpapers)


def chunk(kind, data):
    return (struct.pack('>I', len(data)) + kind + data
            + struct.pack('>I', zlib.crc32(kind + data)))


@lru_cache(maxsize=4)
def png(width, height, shade=0):
    header = struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)
    compressor = zlib.compressobj()
    row = b'\0' + bytes([shade]) * (width * 3)
    image = b''.join(compressor.compress(row) for _ in range(height)) + compressor.flush()
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header)
            + chunk(b'IDAT', image) + chunk(b'IEND', b''))


class WallpaperInstallerTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.source = self.root / 'bundle'
        self.destination = self.root / 'Pictures' / 'Desktop Cards'
        self.source.mkdir()
        for theme in wallpapers.THEMES:
            for resolution, size in wallpapers.RESOLUTIONS.items():
                (self.source / f'{theme}-{resolution}.png').write_bytes(png(*size))

    def test_production_bundle_has_every_theme_at_both_real_resolutions(self):
        files = wallpapers.validate_bundle()
        self.assertEqual(len(files), 2 * len(wallpapers.THEMES))
        self.assertEqual({path.name for path in files}, {
            f'{theme}-{resolution}.png'
            for theme in ('halcyon-night', 'halcyon-calm', 'halcyon-evening', 'halcyon-focus',
                          'halcyon-workbench', 'halcyon-paper', 'halcyon-rose',
                          'lavender', 'amber', 'teal', 'ocean', 'rose', 'graphite')
            for resolution in ('4k', '1080p')})

    def test_install_is_idempotent_and_preserves_unrelated_files(self):
        self.destination.mkdir(parents=True)
        unrelated = self.destination / 'personal.png'
        unrelated.write_bytes(b'personal image')
        installed = wallpapers.install_wallpapers(self.destination, source=self.source)
        original_stats = {path: (path.stat().st_ino, path.stat().st_mtime_ns) for path in installed}
        self.assertEqual(len(installed), 2 * len(wallpapers.THEMES))
        for path in installed:
            self.assertEqual(path.read_bytes(), (self.source / path.name).read_bytes())
        wallpapers.install_wallpapers(self.destination, source=self.source)
        self.assertEqual(original_stats, {
            path: (path.stat().st_ino, path.stat().st_mtime_ns) for path in installed})
        self.assertEqual(unrelated.read_bytes(), b'personal image')

    def test_invalid_bundle_is_rejected_before_any_destination_change(self):
        invalid = self.source / 'graphite-1080p.png'
        for content in (png(1280, 720), png(1920, 1080)[:-12], b'not a PNG'):
            with self.subTest(content_length=len(content)):
                invalid.write_bytes(content)
                with self.assertRaisesRegex(RuntimeError, 'graphite-1080p.png'):
                    wallpapers.install_wallpapers(self.destination, source=self.source)
                self.assertFalse(self.destination.exists())

    def test_png_with_corrupt_image_chunk_is_rejected(self):
        invalid = self.source / 'teal-4k.png'
        content = bytearray(invalid.read_bytes())
        content[44] ^= 1
        invalid.write_bytes(content)
        with self.assertRaisesRegex(RuntimeError, 'teal-4k.png'):
            wallpapers.validate_bundle(self.source)

    def test_failed_atomic_replace_keeps_original_and_removes_temporary_file(self):
        self.destination.mkdir(parents=True)
        previous = self.destination / 'lavender-4k.png'
        previous.write_bytes(b'previous wallpaper')
        with patch.object(wallpapers.os, 'replace', side_effect=OSError('cannot replace')):
            with self.assertRaises(OSError):
                wallpapers.install_wallpapers(
                    self.destination, source=self.source, themes=('lavender',))
        self.assertEqual(previous.read_bytes(), b'previous wallpaper')
        self.assertEqual(list(self.destination.iterdir()), [previous])

    def test_ensure_repairs_missing_or_corrupt_copies_from_offline_bundle(self):
        for name in ('lavender-4k.png', 'lavender-1080p.png'):
            with self.subTest(name=name):
                result = wallpapers.ensure_wallpaper(
                    'lavender', self.destination, source=self.source)
                self.assertEqual(result, self.destination / 'lavender-4k.png')
                damaged = self.destination / name
                damaged.write_bytes(b'partial copy')
                wallpapers.ensure_wallpaper('lavender', self.destination, source=self.source)
                self.assertEqual(damaged.read_bytes(), (self.source / name).read_bytes())
        self.assertEqual(len(list(self.destination.iterdir())), 2)

    def test_valid_installed_wallpaper_does_not_need_source_bundle(self):
        wallpapers.install_wallpapers(self.destination, source=self.source, themes=('rose',))
        result = wallpapers.ensure_wallpaper(
            'rose', self.destination, source=self.root / 'unavailable')
        self.assertEqual(result, self.destination / 'rose-4k.png')

    def test_unknown_theme_cannot_escape_destination(self):
        with self.assertRaisesRegex(RuntimeError, 'Unknown wallpaper theme'):
            wallpapers.ensure_wallpaper('../outside', self.destination, source=self.source)
        self.assertFalse(self.destination.exists())


if __name__ == '__main__':
    unittest.main()
