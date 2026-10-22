#!/usr/bin/env python3
"""Concatenate dependency license files from locked Cargo metadata and sources.

Generate metadata with cargo metadata --locked --format-version 1
--filter-platform x86_64-unknown-linux-gnu. This helper never invokes Cargo,
downloads files, or changes the source trees. Git dependencies are included.
"""

import argparse
import json
import os
from pathlib import Path
import re
import sys


LICENSE_NAME = re.compile(r"^(licen[cs]e|copying|copyright|notice|unlicense)(?:$|[._-])", re.I)
SKIP_DIRS = {".git", "target", "node_modules", ".venv", "__pycache__"}


def source_directory(package, cargo_home):
    manifest = Path(package["manifest_path"])
    if manifest.is_file():
        return manifest.resolve().parent
    # Metadata can be generated in another checkout. Registry packages have
    # stable name/version paths; do not guess when multiple registries match.
    if (package.get("source") or "").startswith("registry+"):
        matches = sorted((cargo_home / "registry/src").glob(
            f"*/{package['name']}-{package['version']}/Cargo.toml"))
        if len(matches) == 1:
            return matches[0].resolve().parent
    raise ValueError(f"source missing or ambiguous: {package['name']} {package['version']}")


def license_files(package, source):
    found = set()
    if package.get("license_file"):
        explicit = (source / package["license_file"]).resolve()
        if not explicit.is_file():
            raise ValueError(f"missing declared license_file for {package['name']}")
        found.add(explicit)
    for directory, dirs, files in os.walk(source):
        dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
        parent = Path(directory)
        for name in files:
            path = parent / name
            if LICENSE_NAME.match(name) or any(
                part.lower() in {"license", "licenses", "licence", "licences"}
                for part in path.relative_to(source).parts[:-1]
            ):
                if path.is_file():
                    found.add(path.resolve())
    # Crates in git workspaces often share the repository's license. Walk
    # upward only as far as that repository, never into another Cargo source.
    boundary = next((parent for parent in (source, *source.parents)
                     if (parent / ".git").exists()), source)
    current = source
    while current != boundary:
        current = current.parent
        found.update(path.resolve() for path in current.iterdir()
                     if path.is_file() and LICENSE_NAME.match(path.name))
    return sorted(found)


def collect(metadata, cargo_home, allow_missing=False):
    root_id = (metadata.get("resolve") or {}).get("root")
    nodes = (metadata.get("resolve") or {}).get("nodes")
    active = {node["id"] for node in nodes} if nodes is not None else None
    workspace = set(metadata.get("workspace_members", []))
    chunks = ["Halcyon dependency license texts\n"
              "Generated from locked Cargo metadata and local dependency sources.\n"
              "Each dependency retains its own license and copyright notices.\n"]
    missing = []
    count = 0
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"], p["id"])):
        if active is not None and package["id"] not in active:
            continue
        if package["id"] == root_id or (
            not root_id and package["id"] in workspace
            and package["name"] == "cosmic-ext-halcyon"
        ):
            continue
        source = source_directory(package, cargo_home)
        files = license_files(package, source)
        if not files:
            missing.append(f"{package['name']} {package['version']}")
        count += 1
        chunks.append(
            "\n" + "=" * 72 + "\n"
            f"Package: {package['name']} {package['version']}\n"
            f"Source: {package.get('source') or 'local workspace'}\n"
            f"Declared license: {package.get('license') or 'see license files'}\n"
        )
        if not files:
            chunks.append("No standalone license text in the downloaded source.\n"
                          "See the cargo-about license report for this package's license text.\n")
        for path in files:
            label = os.path.relpath(path, source)
            chunks.append(f"\n--- {label} ---\n" + path.read_text(encoding="utf-8") + "\n")
    if missing:
        message = "no license texts found for: " + ", ".join(missing)
        if not allow_missing:
            raise ValueError(message + "; supply upstream license files, or combine "
                             "--allow-missing output with a cargo-about report")
        print("Warning: " + message, file=sys.stderr)
    if not count:
        raise ValueError("metadata contains no dependency licenses; do not use --no-deps")
    return "".join(chunks), count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata", required=True, type=Path)
    parser.add_argument("--cargo-home", type=Path,
                        default=Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")))
    parser.add_argument("--out", type=Path,
                        default=Path(__file__).resolve().parent / "licenses/THIRD_PARTY_LICENSES.txt")
    parser.add_argument("--allow-missing", action="store_true",
                        help="report missing texts; use only alongside a cargo-about license report")
    args = parser.parse_args()
    try:
        content, count = collect(json.loads(args.metadata.read_text()), args.cargo_home,
                                 args.allow_missing)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(content, encoding="utf-8")
        args.out.chmod(0o644)
        print(f"Collected license texts for {count} dependencies: {args.out}")
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"collect-licenses: {error}\n")


if __name__ == "__main__":
    main()
