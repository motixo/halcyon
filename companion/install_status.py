#!/usr/bin/python3
"""Read-only Islands installation capability v1. Never starts a companion."""
import argparse
import hashlib
import json
import re
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


def detect(home, library, package_installed, versions, *, installed=None, running=None, host=None):
    result = dict(format=FORMAT, version=1, installation='absent', provenance=None,
                  enabled=False, compatible=None, running=None, healthy=None)
    result['package_versions'] = {name: value for name, value in versions.items()
        if name in ('cosmic-panel', 'cosmic-comp', 'cosmic-session', 'cosmic-settings-daemon')
        and isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9.+:~_-]{1,128}', value)}
    bundle = home / '.local/share/cosmic-desktop-cards/companion'
    manifest = None
    payload_qualified = False
    if package_installed:
        # dpkg + package CLI establish provenance. Payload qualification is a
        # separate health/compatibility result, never an installation detector.
        result.update(installation='installed', provenance='package')
        try:
            manifest = record(library / 'companion-manifest.json')
            binary = library / 'cosmic-panel-islands'
            intact = digest(binary) == manifest.get('binary_sha256') and bool(binary.stat().st_mode & stat.S_IXUSR)
            payload_qualified = manifest.get('upstream') == 'ca20ce3185b3b4404f429d59d429c40b58a2ddef' and manifest.get('protocol') == 1
            if not intact:
                result['healthy'] = False
        except (OSError, ValueError, TypeError):
            result['healthy'] = False
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
                    payload_qualified = True
                    break
            except (OSError, ValueError, TypeError, KeyError):
                continue
    if result['installation'] != 'installed':
        return result
    # V2 retains installation provenance even if the optional payload is broken.
    if (manifest or {}).get('version') == 2 or (library / 'catalog.json').exists() or (library / 'bundle_contract.py').exists():
        return generation_status(result, home, library, installed, running, host)
    # Compatibility is the selector's single reviewed-source policy. It is
    # independent of installation and remains weaker than running/ABI health.
    result['compatible'] = payload_qualified and panel_launcher.supported_system(versions) and (
        any(versions == row for row in panel_launcher.SUPPORTED_SYSTEMS)
        or (manifest or {}).get('runtime_probe') == 1)
    try:
        selection = record(bundle / 'selection.json')
        result['enabled'] = selection.get('enabled') is True and verified(bundle, 'manifest.json') is not None
    except (OSError, ValueError, TypeError):
        pass
    # Running and runtime ABI/health are separately verified through the socket.
    return result


def generation_status(result, home, library, installed=None, running=None, host=None):
    from bundle_contract import verify_bundle, classify_compatibility
    from refresh import selected_generation, records, verify_owned
    state = dict(state='invalid_bundle', reason='bundle_not_verified', cohort=None)
    generation = active_generation = None
    try:
        manifest, catalog = verify_bundle(library, 'companion-manifest.json')
        generation = digest(library / 'companion-manifest.json')
        state = classify_compatibility(installed or {}, running or {}, {**manifest, 'verified': True}, catalog)
        result['compatible'] = state['state'] in ('ready', 'logout_required')
    except (OSError, ValueError, KeyError, TypeError):
        result.update(compatible=False, healthy=False)
    try:
        selection = record(home / '.local/share/cosmic-desktop-cards/companion/selection.json')
        active = records(home)
        verify_owned(home, active)
        result['enabled'] = selection.get('enabled') is True
        _, _, _, pointer = selected_generation(home)
        active_generation = pointer['generation']
        current = panel_launcher.session_identity()
        deferred = current is None or current == pointer.get('defer_session')
    except (OSError, ValueError, KeyError, TypeError):
        deferred = True
        if result['enabled'] and (home / '.local/share/cosmic-desktop-cards/companion/generation.json').exists():
            result['healthy'] = False
            state = dict(state='invalid_bundle', reason='active_generation_invalid', cohort=None)
    failed = (home / '.local/state/desktop-cards-panel/failed').exists()
    if failed or not result['enabled']:
        label = 'Using standard panel'
        reason = 'previous_failure' if failed else 'disabled'
    elif state['state'] in ('unsupported', 'invalid_bundle'):
        label, reason = 'Compatibility update needed', state['reason']
    elif deferred or active_generation != generation or state['state'] == 'logout_required':
        label, reason = 'Log out to activate', state['reason']
    elif host is not True:
        label, reason = 'Using standard panel', 'host_not_ready'
    else:
        label, reason = 'Ready', state['reason']
    result['running'] = host
    if host is not None and result['healthy'] is not False:
        result['healthy'] = host
    result.update(status_version=1, status=label, reason=reason,
                  installed_generation=generation, active_generation=active_generation,
                  compatibility=state)
    # Deliberate allowlist: no filesystem paths, session IDs, environment or
    # exception strings. Diagnostic output cannot change the activation state.
    result['diagnostics'] = dict(version=1, installation=result['installation'],
        provenance=result['provenance'], enabled=result['enabled'], status=label,
        reason=reason, installed_generation=generation, active_generation=active_generation,
        installed_cohort=(installed or {}).get('cohort'), running_cohort=(running or {}).get('cohort'))
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
    measured = active = host = None
    if not args.root and installed:
        try:
            from bundle_contract import verify_bundle
            from runtime_identity import installed_identity, running_identity, host_health
            _, catalog = verify_bundle(library, 'companion-manifest.json')
            measured, active = installed_identity(catalog), running_identity(catalog)
            from refresh import selected_generation
            _, selected, _, _ = selected_generation(home)
            host = host_health(selected['binary_sha256'])
        except (OSError, ValueError, KeyError, TypeError):
            pass
    print(json.dumps(detect(home, library, installed, versions, installed=measured, running=active, host=host), separators=(',', ':')))


if __name__ == '__main__':
    main()
