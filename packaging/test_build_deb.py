"""Exercise the real Debian archiver with disposable stub inputs, never Cargo."""

import argparse
import contextlib
import gzip
import hashlib
import importlib.util
import io
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("build_deb", Path(__file__).with_name("build-deb.py"))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


@unittest.skipUnless(shutil.which("dpkg-deb"), "dpkg-deb is required")
class PackageTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        repo = self.root / "repo"
        # Actual enrollment inputs are public and independently pinned.
        shutil.copytree(builder.REPO / "packaging/updates", repo / "packaging/updates")
        files = [
            "scripts/desktop-cards", "scripts/calendar-sync.py", "scripts/project-status.py",
            "companion/panel_launcher.py", "companion/install_status.py",
            "companion/runtime_compatibility.py",
            "companion/bundle_contract.py", "companion/runtime_identity.py", "companion/refresh.py", "companion/install.py", "companion/panel_configuration.py", "companion/runtime_target.py", "companion/verification.py",
            "docs/google-calendar-setup.html", "LICENSE", "THIRD_PARTY.md", "ASSETS.md",
            "packaging/debian/copyright", "packaging/debian/changelog",
            "packaging/systemd/halcyon.service", "packaging/man/halcyon.1",
            "packaging/man/halcyon-ctl.1",
            f"packaging/{builder.APP_ID}.desktop", f"packaging/{builder.APP_ID}.metainfo.xml",
            "packaging/licenses/THIRD_PARTY_LICENSES.txt", "assets/wallpapers-jpg/scene.jpg",
        ]
        files.extend(f"packaging/icons/{size}/{builder.APP_ID}.png" for size in builder.ICON_SIZES)
        for name in files:
            path = repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("stub " + name + "\n")
            path.chmod(0o600)
        self.binary = self.root / "halcyon"
        self.binary.write_text("#!/bin/sh\nexit 0\n")
        self.client = self.root / "client.json"
        self.client.write_text('{"installed":{"client_id":"test"}}\n')
        self.args = argparse.Namespace(binary=self.binary, version="0.1.0~ci7",
                                       out=self.root / "out", wallpapers=None,
                                       google_client=self.client)
        replacement = patch.object(builder, "REPO", repo)
        replacement.start()
        self.addCleanup(replacement.stop)

    def build(self):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            builder.build(self.args)
        return next(self.args.out.glob("*.deb"))

    def test_layout_modes_license_bundle_and_checksums(self):
        package = self.build()
        data = subprocess.check_output(["dpkg-deb", "--fsys-tarfile", str(package)])
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:
            members = {m.name.removeprefix("./"): m for m in archive.getmembers()}
            executables = {"usr/bin/halcyon", "usr/bin/halcyon-ctl",
                           "usr/lib/cosmic-ext-halcyon/calendar-sync.py",
                           "usr/lib/cosmic-ext-halcyon/project-status.py",
                           "usr/lib/cosmic-ext-halcyon/islands-status.py"}
            for name, member in members.items():
                self.assertEqual((member.uid, member.gid), (0, 0), name)
                self.assertEqual(member.mode, 0o755 if member.isdir() or name in executables else 0o644, name)
            client = members["usr/share/cosmic-ext-halcyon/google-calendar-client.json"]
            self.assertEqual(archive.extractfile(client).read(), self.client.read_bytes())
            self.assertIn("usr/lib/cosmic-ext-halcyon/islands-status.py", members)
            self.assertIn("usr/lib/cosmic-ext-halcyon/panel_launcher.py", members)
            self.assertIn("usr/lib/cosmic-ext-halcyon/runtime_compatibility.py", members)
            self.assertIn("usr/share/backgrounds/halcyon/scene.jpg", members)
            self.assertIn("usr/lib/systemd/user/halcyon.service", members)
            self.assertIn("usr/share/doc/cosmic-ext-halcyon/THIRD_PARTY_LICENSES.txt", members)
            changelog = archive.extractfile(members["usr/share/doc/cosmic-ext-halcyon/changelog.gz"]).read()
            self.assertEqual(changelog[4:8], b"\x00" * 4)
            self.assertEqual(gzip.decompress(changelog),
                             (builder.REPO / "packaging/debian/changelog").read_bytes())
        checksum = package.with_suffix(".deb.sha256").read_text()
        self.assertEqual(checksum, hashlib.sha256(package.read_bytes()).hexdigest() + "  " + package.name + "\n")
        depends = subprocess.check_output(["dpkg-deb", "-f", str(package), "Depends"], text=True)
        self.assertEqual(depends.strip(), "libc6, libxkbcommon0, python3, git, systemd")

    def test_client_is_optional_and_bundle_build_is_reproducible(self):
        self.args.google_client = None
        with patch.dict("os.environ", {"SOURCE_DATE_EPOCH": "1792627200"}):
            first = self.build().read_bytes()
            second = self.build()
            self.assertEqual(first, second.read_bytes())
        listing = subprocess.check_output(["dpkg-deb", "-c", str(second)], text=True)
        self.assertNotIn("google-calendar-client.json", listing)

    def test_default_update_payload_and_hooks_are_self_contained(self):
        package = self.build()
        root = self.root / "unpacked"
        subprocess.run(["dpkg-deb", "-R", str(package), str(root)], check=True)
        source = root / "usr/share/cosmic-ext-halcyon/updates/source.sources"
        self.assertIn("Signed-By: /usr/share/keyrings/halcyon-archive-keyring.gpg", source.read_text())
        self.assertFalse((root / "etc/apt").exists(), "Do not overwrite bootstrap conffiles on unpack")
        for name in ("postinst", "postrm"):
            script = root / "DEBIAN" / name
            self.assertTrue(script.stat().st_mode & 0o111)
            subprocess.run(["sh", "-n", str(script)], check=True)
            self.assertNotIn("apt-get", script.read_text())
            self.assertNotIn("systemctl", script.read_text())

    def test_invalid_version_and_empty_wallpapers_fail_before_archiving(self):
        self.args.version = "../escape"
        with self.assertRaisesRegex(ValueError, "version must"):
            builder.build(self.args)
        self.args.version = "0.1.0"
        self.args.wallpapers = self.root / "empty-wallpapers"
        self.args.wallpapers.mkdir()
        with self.assertRaisesRegex(ValueError, "no <name>.jpg"):
            builder.build(self.args)


if __name__ == "__main__":
    unittest.main()
