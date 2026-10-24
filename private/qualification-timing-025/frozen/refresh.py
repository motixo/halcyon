"""Local package -> immutable user generation. Never starts or restarts a host."""
import argparse
import contextlib
import fcntl
import json
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

import install
from bundle_contract import canonical, digest, read_json, read_catalog_bytes, verify_bundle
from panel_launcher import session_identity

PACKAGE = 'cosmic-ext-halcyon-islands'
DATA = '.local/share/cosmic-desktop-cards/'
SELECTOR = '.local/bin/cosmic-panel'
SELECTION = DATA + 'companion/selection.json'
POINTER = DATA + 'companion/generation.json'
LIBRARY = Path('/usr/lib') / PACKAGE


def response(state, reason, generation=None):
    return dict(state=state, reason=reason, generation=generation,
                next_login=state == 'staged', runtime_readiness='not_measured')


@contextlib.contextmanager
def locked(home):
    path = install.safe_path(home, DATA + 'backups/.islands.lock')
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield
    finally:
        os.close(fd)


def records(home):
    backup = install.safe_path(home, DATA + 'backups')
    result = []
    for path in sorted(backup.glob('panel-*/install-record.json'), reverse=True):
        saved = read_json(install.safe_path(home, str(path.relative_to(home))))
        if saved.get('root') != str(home) or saved.get('restored'):
            continue
        if saved.get('owner') != PACKAGE:
            raise ValueError('manual_install_requires_explicit_management')
        result.append((path, saved))
    return result


def verify_owned(home, active):
    # Validate the current effective layer, not superseded values in old layers.
    seen = set()
    for _, saved in active:
        if not saved.get('complete'):
            raise ValueError('interrupted_install')
        for relative, sha in saved['installed'].items():
            if relative in seen:
                continue
            seen.add(relative)
            path = install.safe_path(home, relative)
            if digest(path) != sha or path.stat().st_mode & 0o777 != saved['modes'][relative] or path.stat().st_uid != os.getuid():
                raise ValueError('external_change')
    if SELECTOR not in seen or SELECTION not in seen:
        raise ValueError('unowned_selection')


def selected_generation(home):
    pointer = read_json(install.safe_path(home, POINTER))
    generation = pointer['generation']
    if len(generation) != 64 or any(c not in '0123456789abcdef' for c in generation):
        raise ValueError('invalid_generation')
    path = install.safe_path(home, DATA + 'companion/generations/' + generation)
    if digest(path / 'manifest.json') != generation:
        raise ValueError('generation_identity')
    manifest, catalog = verify_bundle(path, owner=os.getuid())
    return path, manifest, catalog, pointer


def recover_interrupted(home):
    for path, saved in records(home):
        if not saved.get('complete'):
            # Existing restore validates all before/current bytes and is itself
            # replayable. Never roll back an external edit to finish an upgrade.
            install.restore(path)
        else:
            break


def stage_installed_update(home, installed_bundle, *, session=None, fixture=False, boundary=lambda _: None):
    home = home.resolve()
    try:
        with locked(home):
            return stage_locked(home, installed_bundle, session=session, fixture=fixture, boundary=boundary)
    except BlockingIOError:
        return response('blocked', 'busy')
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        return response('blocked', 'unsafe_installation')


def require_selector_path(home):
    paths = os.environ.get('PATH', '').split(os.pathsep)
    local = str(home / '.local/bin')
    if local not in paths or '/usr/bin' not in paths or paths.index(local) > paths.index('/usr/bin'):
        raise ValueError('The session PATH must put ~/.local/bin before /usr/bin')


def stage_locked(home, library, *, session=None, fixture=False, boundary=lambda _: None, enable=False):
    if not fixture and Path(os.environ.get('XDG_DATA_HOME', home / '.local/share')) != home / '.local/share':
        return response('blocked', 'unsupported_data_location')
    if enable and not fixture:
        require_selector_path(home)
    recover_interrupted(home)
    active = records(home)
    if not active and not enable:
        return response('disabled', 'not_owned_enabled')
    if not active and (os.path.lexists(home / SELECTOR) or os.path.lexists(home / POINTER)):
        return response('external_change', 'unowned_selector')
    try:
        if active:
            verify_owned(home, active)
    except (OSError, ValueError):
        return response('external_change', 'owned_files_changed')
    selection = read_json(install.safe_path(home, SELECTION)) if active else {'enabled': enable}
    if selection.get('enabled') is not True:
        return response('disabled', 'disabled')
    if not fixture:
        if library != LIBRARY or library.is_symlink():
            return response('blocked', 'not_installed_package')
        result = subprocess.run(['/usr/bin/dpkg-query', '-W', '-f=${Status}\n${Version}', PACKAGE], capture_output=True, text=True, timeout=3)
        fields = result.stdout.splitlines()
        if result.returncode or len(fields) != 2 or fields[0] != 'install ok installed':
            return response('blocked', 'package_absent')
    manifest, catalog = verify_bundle(library, 'companion-manifest.json', owner=os.getuid() if fixture else 0)
    if not fixture and manifest['package_version'] != fields[1]:
        return response('blocked', 'package_version_mismatch')
    if not catalog['cohorts']:
        return response('blocked', 'no_qualified_cohort')
    from runtime_identity import installed_identity
    installed = installed_identity(catalog) if not fixture else read_json(home / 'installed-identity.json')
    if not any(r['id'] == installed.get('cohort') and installed.get('verified') is True
               and r['qualification'] == 'native-session' and r['payload_sha256'] == manifest['payload_sha256'] for r in catalog['cohorts']):
        return response('blocked', 'installed_cohort_unqualified')
    current = session if session is not None else session_identity()
    if not current:
        return response('blocked', 'session_unknown')
    manifest_bytes = (library / 'companion-manifest.json').read_bytes()
    if json.loads(manifest_bytes) != manifest:
        return response('blocked', 'package_changed_during_staging')
    generation = hashlib.sha256(manifest_bytes).hexdigest()
    old_version = '0.2.0' if active else '0'
    try:
        _, old, _, pointer = selected_generation(home)
        old_version = old['package_version']
        if pointer['generation'] == generation:
            return response('unchanged', 'already_staged', generation)
    except FileNotFoundError:
        if install.safe_path(home, POINTER).exists():
            return response('external_change', 'generation_missing')
    # A changed payload at the same version is also an unapproved replacement.
    if subprocess.run(['/usr/bin/dpkg', '--compare-versions', manifest['package_version'], 'gt', old_version], timeout=2).returncode != 0:
        return response('blocked', 'non_monotonic_version')
    generations = install.safe_path(home, DATA + 'companion/generations')
    generations.mkdir(parents=True, exist_ok=True, mode=0o700)
    target = generations / generation
    if not target.exists():
        with tempfile.TemporaryDirectory(prefix='.stage-', dir=generations) as temporary:
            staged = Path(temporary) / generation
            staged.mkdir(mode=0o700)
            for name, info in manifest['files'].items():
                data = (read_catalog_bytes(library / name, info['sha256']) if name == 'catalog.json'
                        else (library / name).read_bytes())
                install.place(staged / name, data, info['mode'])
            install.place(staged / 'manifest.json', manifest_bytes, 0o644)
            verify_bundle(staged, owner=os.getuid())
            boundary('copied')
            os.rename(staged, target)
            install.sync_directory(generations)
    verify_bundle(target, owner=os.getuid())
    if digest(target / 'manifest.json') != generation:
        raise ValueError('generation_changed')
    selector_bytes = (target / 'selector.py').read_bytes()
    selector_info = manifest['files']['selector.py']
    if hashlib.sha256(selector_bytes).hexdigest() != selector_info['sha256'] or selector_info['mode'] != 0o755:
        raise ValueError('generation_selector_mismatch')
    boundary('generation')
    pointer = dict(version=1, generation=generation, package_version=manifest['package_version'], defer_session=current)
    contents = {
        POINTER: (canonical(pointer) + b'\n', 0o600),
        SELECTOR: (selector_bytes, selector_info['mode']),
    }
    if not active:
        contents[SELECTION] = (canonical({'enabled': True, 'defer_session': current}) + b'\n', 0o600)
    # install_contents journals before the pointer or selector changes. Its
    # rollback restores the previous selector/pointer, preserving the 0.2.0
    # chain and the old flat payload. Immutable orphan generations are harmless.
    boundary('before_selection')
    install.install_contents(home, contents, PACKAGE)
    boundary('selected')
    return response('staged', 'next_login_after_staging', generation)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--library', type=Path, default=LIBRARY, help=argparse.SUPPRESS)
    args = parser.parse_args()
    home = args.root or Path.home()
    # Only default XDG layout is supported by the historical ownership chain.
    if not args.root and 'COSMIC' not in os.environ.get('XDG_CURRENT_DESKTOP', '').split(':'):
        result = response('blocked', 'not_cosmic_session')
    elif not args.root and Path(os.environ.get('XDG_DATA_HOME', home / '.local/share')) != home / '.local/share':
        result = response('blocked', 'unsupported_data_location')
    else:
        result = stage_installed_update(home, args.library, fixture=args.root is not None)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
