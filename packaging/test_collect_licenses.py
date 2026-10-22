"""Offline tests using tiny Cargo source trees; never invoke Cargo."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("collect-licenses.py")
spec = importlib.util.spec_from_file_location("collect_licenses", SCRIPT)
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)


class CollectorTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.cargo_home = self.root / "cargo"
        self.crate = self.cargo_home / "registry/src/example/demo-1.2.3"
        self.crate.mkdir(parents=True)
        (self.crate / "Cargo.toml").write_text('[package]\nname = "demo"\n')
        (self.crate / "LICENSE-MIT").write_text("MIT license text for test\n")
        (self.crate / "NOTICE").write_text("Copyright test contributor\n")
        self.package = {
            "id": "registry+example#demo@1.2.3", "name": "demo", "version": "1.2.3",
            "source": "registry+example", "license": "MIT", "license_file": None,
            "manifest_path": str(self.crate / "Cargo.toml"),
        }
        self.metadata = {"packages": [self.package], "resolve": {"root": "halcyon"}}

    def test_cli_collects_registry_text_and_notice(self):
        metadata = self.root / "metadata.json"
        metadata.write_text(json.dumps(self.metadata))
        output = self.root / "bundle.txt"
        subprocess.run([sys.executable, "-B", str(SCRIPT), "--metadata", str(metadata),
                        "--cargo-home", str(self.cargo_home), "--out", str(output)],
                       check=True, capture_output=True, text=True)
        text = output.read_text()
        self.assertIn("Package: demo 1.2.3", text)
        self.assertIn("MIT license text for test", text)
        self.assertIn("Copyright test contributor", text)
        self.assertEqual(output.stat().st_mode & 0o777, 0o644)

    def test_relocated_registry_source(self):
        self.package["manifest_path"] = "/nonexistent/demo/Cargo.toml"
        text, count = collector.collect(self.metadata, self.cargo_home)
        self.assertEqual(count, 1)
        self.assertIn("MIT license text for test", text)

    def test_git_workspace_uses_parent_license_and_declared_file(self):
        checkout = self.cargo_home / "git/checkouts/demo/123abcd"
        source = checkout / "subcrate"
        source.mkdir(parents=True)
        (checkout / ".git").mkdir()
        (checkout / "LICENSE").write_text("Workspace license text\n")
        (source / "Cargo.toml").write_text("[package]\n")
        (source / "terms.txt").write_text("Additional crate license text\n")
        self.package.update(manifest_path=str(source / "Cargo.toml"),
                            source="git+https://example.test/demo#123abcd",
                            license=None, license_file="terms.txt")
        text, count = collector.collect(self.metadata, self.cargo_home)
        self.assertEqual(count, 1)
        self.assertIn("Workspace license text", text)
        self.assertIn("Additional crate license text", text)

    def test_missing_text_is_an_error(self):
        (self.crate / "LICENSE-MIT").unlink()
        (self.crate / "NOTICE").unlink()
        with self.assertRaisesRegex(ValueError, "no license texts found for: demo 1.2.3"):
            collector.collect(self.metadata, self.cargo_home)

    def test_output_is_deterministic(self):
        first, _ = collector.collect(self.metadata, self.cargo_home)
        second, _ = collector.collect(self.metadata, self.cargo_home)
        self.assertEqual(first, second)

    def test_target_filter_excludes_packages_outside_resolve_graph(self):
        self.metadata["resolve"]["nodes"] = [{"id": self.package["id"]}]
        self.metadata["packages"].append({
            "id": "unused", "name": "unused", "version": "1.0.0",
            "manifest_path": "/nonexistent/unused/Cargo.toml",
        })
        text, count = collector.collect(self.metadata, self.cargo_home)
        self.assertEqual(count, 1)
        self.assertNotIn("Package: unused", text)

    def test_missing_text_is_explicit_when_combining_with_cargo_about(self):
        (self.crate / "LICENSE-MIT").unlink()
        (self.crate / "NOTICE").unlink()
        text, count = collector.collect(self.metadata, self.cargo_home, allow_missing=True)
        self.assertEqual(count, 1)
        self.assertIn("No standalone license text", text)
        self.assertIn("Declared license: MIT", text)


if __name__ == "__main__":
    unittest.main()
