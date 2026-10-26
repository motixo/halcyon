#!/usr/bin/python3
"""One-time package enrollment; no network, child installer, user files or service actions."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import tempfile

BASE = '/usr/share/cosmic-ext-halcyon/updates'
STATE = '/var/lib/halcyon/updates/default-source.json'
SOURCE = '/etc/apt/sources.list.d/halcyon.sources'
PREFS = '/etc/apt/preferences.d/halcyon'
KEY = '/usr/share/keyrings/halcyon-archive-keyring.gpg'
TARGETS = {'archive-keyring.gpg': KEY, 'preferences': PREFS, 'source.sources': SOURCE}
FORMAT = 'halcyon-default-updates'


def path(root, name):
    return root / name.lstrip('/')


def read(file, limit=65536):
    try:
        fd = os.open(file, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except FileNotFoundError:
        return None
    with os.fdopen(fd, 'rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError('Not a regular enrollment file')
        data = stream.read(limit + 1)
        if len(data) > limit:
            raise ValueError('Enrollment file exceeds its limit')
        return data


def write_new(file, data, mode=0o644):
    file.parent.mkdir(parents=True, exist_ok=True)
    # A crash must leave either no destination or the complete file. link()
    # publishes without replacing an administrator's file created meanwhile.
    fd, staging = tempfile.mkstemp(prefix='.halcyon-enroll-', dir=file.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            os.fchmod(stream.fileno(), mode)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(staging, file, follow_symlinks=False)
    finally:
        Path(staging).unlink(missing_ok=True)
    fd = os.open(file.parent, os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def receipt(root, value):
    target = path(root, STATE)
    target.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
    fd, name = tempfile.mkstemp(prefix='.default-source-', dir=target.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            os.fchmod(stream.fileno(), 0o644)  # Public status; never contains user data.
            json.dump(value, stream, sort_keys=True)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, target)
        fd = os.open(target.parent, os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    finally:
        Path(name).unlink(missing_ok=True)


def legacy(root):
    # Residual dpkg conffile records also mean a prior enrollment decision.
    for package in ('halcyon-update-source', 'halcyon-archive-keyring'):
        info = path(root, '/var/lib/dpkg/info')
        if info.exists() and any(info.glob(package + '*')):
            return True
    status = read(path(root, '/var/lib/dpkg/status'), 32 * 1024 * 1024) or b''
    return any(b'Package: ' + package in status for package in
               (b'halcyon-update-source', b'halcyon-archive-keyring'))


def existing(root, allowed=()):
    if legacy(root):
        return True
    for target in TARGETS.values():
        if target not in allowed and os.path.lexists(path(root, target)):
            return True
    # Do not shadow an administrator's renamed, disabled or duplicate source.
    for name in ('/etc/apt/sources.list', '/etc/apt/preferences',
                 '/etc/apt/sources.list.d', '/etc/apt/preferences.d'):
        p = path(root, name)
        candidates = list(p.iterdir()) if p.is_dir() else [p]
        for candidate in candidates:
            if candidate in {path(root, name) for name in allowed}:
                continue
            # Unpublished staging files from a killed write are not sources.
            if candidate.name.startswith('.halcyon-enroll-'):
                continue
            if 'halcyon' in candidate.name.lower():
                return True
            if candidate.is_file() or candidate.is_symlink():
                try:
                    data = read(candidate, 1024 * 1024) or b''
                except (OSError, ValueError):
                    return True  # Unknown configuration is never a clean install.
                if b'profilaz3.github.io' in data.lower() or b'halcyon' in data.lower():
                    return True
    return False


def active_source_needs_trust(root):
    """Preserve dependencies of administrator-owned active APT entries."""
    candidates = [path(root, '/etc/apt/sources.list')]
    folder = path(root, '/etc/apt/sources.list.d')
    if folder.is_dir():
        candidates += [p for p in folder.iterdir() if p.suffix in ('.list', '.sources')]
    for candidate in candidates:
        try:
            data = read(candidate, 1024 * 1024)
            if data is None:
                continue
            text = data.decode('utf-8')
        except (OSError, ValueError, UnicodeError):
            return True  # An uninspectable active entry may depend on our key.
        text = '\n'.join(line for line in text.splitlines() if not line.lstrip().startswith('#'))
        entries = re.split(r'\n\s*\n', text) if candidate.suffix == '.sources' else text.splitlines()
        for entry in entries:
            if candidate.suffix == '.sources':
                if re.search(r'^Enabled:\s*no\s*$', entry, re.I | re.M):
                    continue
            elif not re.match(r'^\s*deb(?:-src)?\s', entry):
                continue
            if KEY in entry or 'profilaz3.github.io/halcyon' in entry.lower():
                return True
    return False


def configure(root):
    try:
        raw = read(path(root, STATE))
    except (OSError, ValueError):
        print('Halcyon update state is unreadable; existing settings preserved.')
        return
    try:
        state = json.loads(raw) if raw is not None else None
    except (ValueError, TypeError):
        state = {}
    if state is not None and (not isinstance(state, dict) or
            state.get('format') != FORMAT or state.get('version') != 1):
        print('Halcyon update state is unknown; existing settings preserved.')
        return
    if state is not None and state.get('phase') != 'pending':
        return  # Never restore a source removed or disabled after installation.
    payload = {name: read(path(root, BASE + '/' + name)) for name in TARGETS}
    if any(not data for data in payload.values()):
        raise ValueError('Missing packaged enrollment files')
    hashes = {TARGETS[name]: hashlib.sha256(data).hexdigest() for name, data in payload.items()}
    if not state:
        state = dict(format=FORMAT, version=1, phase='preserved' if existing(root) else 'pending', files=hashes)
        receipt(root, state)
        if state['phase'] == 'preserved':
            return
    if state.get('files') != hashes or legacy(root):
        state['phase'] = 'conflict'
        receipt(root, state)
        print('Halcyon enrollment changed during installation; existing settings preserved.')
        return
    missing = []
    # Validate every existing file before writing any missing member.
    for name, target in TARGETS.items():
        file = path(root, target)
        try:
            current = read(file)
        except (OSError, ValueError):
            current = b''  # An unreadable/nonregular file must not be replaced.
        if current is None:
            missing.append((file, payload[name]))
        elif current != payload[name]:
            state['phase'] = 'conflict'
            receipt(root, state)
            print('Halcyon enrollment file changed; existing settings preserved.')
            return
    # A pending receipt does not authorize recreating a renamed/disabled source.
    # Canonical files were verified above; inspect everything else again.
    if existing(root, allowed=TARGETS.values()):
        state['phase'] = 'preserved'
        receipt(root, state)
        print('Halcyon enrollment settings changed; existing settings preserved.')
        return
    # Publish the source last, after its key and restrictive pins are durable.
    for file, data in missing:
        write_new(file, data)
    state['phase'] = 'complete'
    receipt(root, state)
    print('Halcyon system updates enabled. APT/COSMIC Store manages future updates.')


def remove(root, purge=False):
    try:
        raw = read(path(root, STATE))
        state = json.loads(raw) if raw is not None else None
    except (OSError, ValueError):
        return
    if not isinstance(state, dict) or state.get('format') != FORMAT or state.get('version') != 1:
        return
    def unchanged(file, expected):
        try:
            data = read(file)
            return data is not None and hashlib.sha256(data).hexdigest() == expected
        except (OSError, ValueError):
            return False
    if not legacy(root) and state.get('phase') in ('complete', 'pending', 'disabled'):
        hashes = state.get('files', {})
        source = path(root, SOURCE)
        if unchanged(source, hashes.get(SOURCE)):
            # Keep the exact source as an ignored recovery file on removal.
            fd, saved = tempfile.mkstemp(prefix='halcyon.sources.removed.', dir=source.parent)
            os.close(fd)
            os.replace(source, saved)
        if purge and not os.path.lexists(source) and not active_source_needs_trust(root):
            for target in (PREFS, KEY):
                file = path(root, target)
                if unchanged(file, hashes.get(target)):
                    file.unlink()
        state['phase'] = 'disabled'
        receipt(root, state)
    if purge:
        path(root, STATE).unlink(missing_ok=True)


def main():
    import sys
    root = Path(os.environ.get('DPKG_ROOT') or '/')
    if not root.is_absolute():
        raise ValueError('DPKG_ROOT must be absolute')
    action = sys.argv[1] if len(sys.argv) > 1 else ''
    if action == 'configure':
        configure(root)
    elif action in ('remove', 'purge'):
        remove(root, action == 'purge')


if __name__ == '__main__':
    main()
