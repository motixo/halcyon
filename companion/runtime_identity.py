"""Read-only measurements of installed payloads and session-bound mapped bytes.

Unknown processes, deleted mappings and inaccessible procfs are conservative.
There is no environment-variable compatibility override or generic version gate.
"""
import os
from pathlib import Path
import subprocess
from bundle_contract import digest


def versions():
    try:
        result = subprocess.run(['/usr/bin/dpkg-query', '-W', '-f=${binary:Package}\t${Version}\t${db:Status-Status}\n'],
                                capture_output=True, text=True, timeout=3, check=True)
        return {name.split(':')[0]: version for name, version, state in
                (line.split('\t') for line in result.stdout.splitlines()) if state == 'installed'}
    except (OSError, ValueError, subprocess.SubprocessError):
        return {}


def inventory(row):
    files = {}
    for package in row['packages'].values():
        for path, sha in package['files'].items():
            if path in files and files[path] != sha:
                raise ValueError('conflicting_package_files')
            files[path] = sha
    return files


def installed_identity(catalog, root=Path('/'), package_versions=None, system=None):
    package_versions = versions() if package_versions is None else package_versions
    if system is None:
        try:
            values = dict(line.split('=', 1) for line in (root / 'etc/os-release').read_text().splitlines() if '=' in line)
            architecture = subprocess.check_output(['/usr/bin/dpkg', '--print-architecture'], text=True, timeout=2).strip() if root == Path('/') else None
            system = dict(distribution=values['ID'].strip('"'), suite=values.get('VERSION_CODENAME', values.get('UBUNTU_CODENAME', '')).strip('"'), architecture=architecture)
        except (OSError, KeyError, subprocess.SubprocessError):
            return dict(verified=False, cohort=None)
    hashes = {}
    for row in catalog['cohorts']:
        if any(row[k] != system.get(k) for k in ('distribution','suite','architecture')):
            continue
        if not all(package_versions.get(name) == info['version'] for name, info in row['packages'].items()):
            continue
        try:
            for path, sha in inventory(row).items():
                # Distribution /lib -> /usr/lib symlinks are resolved inside the
                # explicit root; the package inventory still fixes the bytes.
                target = (root / path.lstrip('/')).resolve()
                if not target.is_relative_to(root.resolve()):
                    raise ValueError('root_escape')
                if path not in hashes:
                    hashes[path] = digest(target)
                if hashes[path] != sha:
                    raise ValueError('installed_bytes_changed')
            return dict(verified=True, cohort=row['id'])
        except (OSError, ValueError):
            continue
    return dict(verified=False, cohort=None)


def running_identity(catalog, proc=Path('/proc'), session=None, uid=None):
    session = os.environ.get('XDG_SESSION_ID') if session is None else session
    uid = os.getuid() if uid is None else uid
    if not session:
        return dict(verified=False, cohort=None)
    processes = {}
    try:
        for process in proc.iterdir():
            if not process.name.isdigit() or process.stat().st_uid != uid:
                continue
            try:
                name = (process / 'comm').read_text().strip()
                # Linux comm is truncated. Use executable basename instead.
                exe = (process / 'exe').resolve(strict=True)
                name = exe.name.removesuffix(' (deleted)')
                if name not in {'cosmic-comp', 'cosmic-session', 'cosmic-settings-daemon'}:
                    continue
                env = dict(item.split(b'=', 1) for item in (process / 'environ').read_bytes().split(b'\0') if b'=' in item)
                if env.get(b'XDG_SESSION_ID') != session.encode():
                    continue
                if name in processes or not exe.is_file() or ' (deleted)' in str(exe):
                    return dict(verified=False, cohort=None)
                mapped = {}
                for line in (process / 'maps').read_text().splitlines():
                    fields = line.split(None, 5)
                    # File-backed executable mappings identify loaded code.
                    # Graphics buffers, device mappings and mutable non-code
                    # resources are not package binaries; installed inventory
                    # separately verifies shipped data.
                    if len(fields) != 6 or 'x' not in fields[1] or not fields[5].startswith('/'):
                        continue
                    if ' (deleted)' in fields[5]:
                        raise ValueError('deleted_mapping')
                    path = Path(fields[5]).resolve(strict=True)
                    if path not in mapped:
                        mapped[str(path)] = digest(path)
                processes[name] = dict(exe=digest(exe), mapped=mapped)
            except (OSError, ValueError, UnicodeError):
                return dict(verified=False, cohort=None)
        matches = []
        for row in catalog['cohorts']:
            expected = {str(Path(path).resolve()): sha for path, sha in inventory(row).items()}
            if set(processes) != set(row['processes']):
                continue
            if all(processes[name]['exe'] == expected.get(str(Path(path).resolve())) and
                   all(expected.get(p) == sha for p, sha in processes[name]['mapped'].items())
                   for name, path in row['processes'].items()):
                matches.append(row['id'])
        if len(matches) == 1:
            return dict(verified=True, cohort=matches[0])
    except (OSError, KeyError, ValueError):
        pass
    return dict(verified=False, cohort=None)


def host_health(binary_sha256):
    """Read-only child-owner capability probe, independent of installation."""
    import hashlib
    import json
    import socket
    import struct
    runtime = os.environ.get('XDG_RUNTIME_DIR')
    session = os.environ.get('XDG_SESSION_ID')
    if not runtime or not session:
        return None
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(.3)
            client.connect(str(Path(runtime) / 'desktop-cards-islands-v1.sock'))
            pid, uid, _ = struct.unpack('3i', client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize('3i')))
            if uid != os.getuid():
                return False
            process = Path('/proc') / str(pid)
            env = dict(item.split(b'=', 1) for item in (process/'environ').read_bytes().split(b'\0') if b'=' in item)
            if env.get(b'XDG_SESSION_ID') != session.encode():
                return False
            # /proc/pid/exe pins the running inode, including an old deleted ELF;
            # never hash its replacement at the installed filename instead.
            with (process/'exe').open('rb') as stream:
                if hashlib.file_digest(stream, 'sha256').hexdigest() != binary_sha256:
                    return False
            client.sendall(b'"Capabilities"\n')
            with client.makefile('rb') as stream:
                line = stream.readline(65537)
            if len(line) > 65536 or not line.endswith(b'\n'):
                return False
            cap = json.loads(line).get('Capabilities', {})
            return cap.get('protocol') == 1 and cap.get('together') is True and cap.get('separate') is True
    except (OSError, ValueError, TypeError, AttributeError):
        return False
