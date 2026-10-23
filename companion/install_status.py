#!/usr/bin/python3
"""Read-only Islands installation capability v1. Never starts a companion."""
import argparse
import hashlib
import json
from pathlib import Path
import stat
import subprocess
import panel_launcher

PACKAGE = 'cosmic-ext-halcyon-islands'
FORMAT = 'halcyon-islands-installation'


def regular(path):
    return not any(p.is_symlink() for p in (path, *path.parents)) and path.is_file()


def record(path):
    if not regular(path) or path.stat().st_size > 65536:
        raise ValueError('Not a bounded regular record')
    value = json.loads(path.read_bytes())
    if not isinstance(value, dict):
        raise ValueError('Not an object')
    return value


def digest(path):
    if not regular(path) or path.stat().st_size > 512 * 1024 * 1024:
        raise ValueError('Not a bounded regular binary')
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verified(bundle, manifest_name):
    try:
        manifest = record(bundle / manifest_name)
        binary = bundle / 'cosmic-panel-islands'
        if manifest.get('upstream') != 'ca20ce3185b3b4404f429d59d429c40b58a2ddef' or manifest.get('protocol') != 1 or digest(binary) != manifest.get('binary_sha256') or not binary.stat().st_mode & stat.S_IXUSR:
            return None
        return manifest
    except (OSError, ValueError, TypeError):
        return None


def detect(home, library, package_installed, versions):
    result = dict(format=FORMAT, version=1, installation='absent', provenance=None,
                  enabled=False, compatible=None, running=None, healthy=None)
    bundle = home / '.local/share/cosmic-desktop-cards/companion'
    manifest = verified(library, 'companion-manifest.json') if package_installed else None
    if manifest is not None:
        result.update(installation='installed', provenance='package')
    else:
        # A former packaged user bundle is not a supported manual install.
        # Only a completed, unrestored installer record with exact payloads is.
        for path in sorted((home / '.local/share/cosmic-desktop-cards/backups').glob('panel-*/install-record.json'), reverse=True)[:128]:
            try:
                saved = record(path)
                if saved.get('root') != str(home) or not saved.get('complete') or saved.get('restored') or saved.get('owner') is not None:
                    continue
                required = [str((bundle / n).relative_to(home)) for n in ('cosmic-panel-islands', 'manifest.json')]
                required.append('.local/bin/cosmic-panel')
                if not all(digest(home / rel) == saved['installed'][rel] and (home / rel).stat().st_mode & 0o777 == saved['modes'][rel] for rel in required):
                    continue
                manifest = verified(bundle, 'manifest.json')
                if manifest is not None:
                    result.update(installation='installed', provenance='manual')
                    break
            except (OSError, ValueError, TypeError, KeyError):
                continue
    if result['installation'] != 'installed':
        return result
    # Compatibility is the selector's single reviewed-source policy. It is
    # independent of installation and remains weaker than running/ABI health.
    result['compatible'] = panel_launcher.supported_system(versions) and (
        any(versions == row for row in panel_launcher.SUPPORTED_SYSTEMS)
        or manifest.get('runtime_probe') == 1)
    try:
        selection = record(bundle / 'selection.json')
        result['enabled'] = selection.get('enabled') is True and verified(bundle, 'manifest.json') is not None
    except (OSError, ValueError, TypeError):
        pass
    # Running and runtime ABI/health are separately verified through the socket.
    return result


def query(package, field):
    try:
        return subprocess.check_output(['/usr/bin/dpkg-query', '-W', '-f=' + field, package], stderr=subprocess.DEVNULL, text=True, timeout=1).strip()
    except (OSError, subprocess.SubprocessError):
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--library', type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    home = (args.root or Path.home()).resolve()
    if args.root:
        # Explicit fixtures never query host packages, XDG data or session IPC.
        library = args.library or home / 'package'
        versions = {}
        installed = library.is_dir()
    else:
        library = Path('/usr/lib') / PACKAGE
        versions = {name: query(name, '${Version}') for name in ('cosmic-panel', 'cosmic-comp', 'cosmic-session')}
        installed = query(PACKAGE, '${Status}') == 'install ok installed' and regular(Path('/usr/bin/halcyon-islands'))
    print(json.dumps(detect(home, library, installed, versions), separators=(',', ':')))


if __name__ == '__main__':
    main()
