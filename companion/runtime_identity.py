"""Read-only measurements of installed payloads and session-bound mapped bytes.

Unrelated process churn is ignored; required candidates and inaccessible code fail closed.
There is no environment-variable compatibility override or generic version gate.
"""
import os
import hashlib
from pathlib import Path
import subprocess
from bundle_contract import digest, runtime_id, valid_runtime


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


def _stream_hash(path):
    # procfs links deliberately pin live inodes, including unlinked old code.
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def _stat(process):
    parts = (process / 'stat').read_bytes().rsplit(b') ', 1)
    if len(parts) != 2:
        raise ValueError('process_stat_incomplete')
    fields = parts[1].split()
    if len(fields) < 20:
        raise ValueError('process_stat_incomplete')
    if fields[0] in (b'Z', b'X'):
        raise ValueError('process_exited')
    return dict(start=fields[19].decode(), parent=int(fields[1]), group=int(fields[2]), sid=int(fields[3]))


def host_process(pid, binary_sha256, proc=Path('/proc'), session=None, uid=None):
    """Pin the actual supervised process, not a second discovery host."""
    session = os.environ.get('XDG_SESSION_ID') if session is None else session
    uid = os.getuid() if uid is None else uid
    process = proc / str(pid)
    before = _stat(process)
    env = dict(item.split(b'=', 1) for item in (process/'environ').read_bytes().split(b'\0') if b'=' in item)
    if (not session or process.stat().st_uid != uid or env.get(b'XDG_SESSION_ID') != session.encode()
            or before['group'] != pid or before['sid'] != pid
            or _stream_hash(process/'exe') != binary_sha256 or _stat(process) != before):
        raise ValueError('host_identity_unknown')
    return dict(pid=pid, sha256=binary_sha256, **before)


def _owned_by_host(process, host, proc):
    # Applets must descend from this host and remain in its supervised process
    # group/session. Matching old or unrelated applets cannot satisfy readiness.
    if process['group'] != host['pid'] or process['sid'] != host['pid'] or int(process['start']) < int(host['start']):
        return False
    parent = process['parent']
    seen = set()
    for _ in range(64):
        if parent == host['pid']:
            return True
        if parent <= 1 or parent in seen:
            return False
        seen.add(parent)
        ancestor = _stat(proc / str(parent))
        if ancestor['group'] != host['pid'] or ancestor['sid'] != host['pid'] or int(ancestor['start']) < int(host['start']):
            return False
        parent = ancestor['parent']
    return False


def _process(process):
    before = _stat(process)
    exe_link = os.readlink(process / 'exe')
    exe_path = exe_link.removesuffix(' (deleted)')
    with (process / 'exe').open('rb') as stream:
        st = os.fstat(stream.fileno())
        exe_hash = hashlib.file_digest(stream, 'sha256').hexdigest()
        maps_before = (process / 'maps').read_text()
        mapped = {}
        for line in maps_before.splitlines():
            fields = line.split(None, 5)
            if len(fields) != 6 or 'x' not in fields[1] or not fields[5].startswith('/'):
                continue
            name = fields[5].removesuffix(' (deleted)')
            major, minor = (int(v, 16) for v in fields[3].split(':'))
            inode = int(fields[4])
            dev = os.makedev(major, minor)
            if st.st_ino == inode and st.st_dev == dev:
                value = exe_hash
            else:
                # map_files may require ptrace privileges. The fallback is safe
                # only when the open file is the inode recorded in /proc/maps.
                try:
                    file = (process / 'map_files' / fields[0]).open('rb')
                except OSError:
                    file = Path(name).open('rb')
                with file:
                    mst = os.fstat(file.fileno())
                    if mst.st_ino != inode or mst.st_dev != dev:
                        raise ValueError('mapping_replaced')
                    value = hashlib.file_digest(file, 'sha256').hexdigest()
            if name in mapped and mapped[name] != value:
                raise ValueError('conflicting_mapping')
            mapped[name] = value
        if maps_before != (process / 'maps').read_text() or exe_link != os.readlink(process / 'exe'):
            raise ValueError('process_changed')
    if _stat(process) != before:
        raise ValueError('process_reused_or_reparented')
    return dict(executable=exe_path, sha256=exe_hash, mapped=mapped, **before)


def _snapshot(proc, names, session, uid):
    found = {name: [] for name in names}
    errors = set()
    for process in list(proc.iterdir()):
        if not process.name.isdigit():
            continue
        possible = set()
        try:
            if process.stat().st_uid != uid:
                continue
            comm = (process / 'comm').read_text().strip()
            possible = {n for n in names if comm == n[:15] or comm == n}
            try:
                name = Path(os.readlink(process / 'exe').removesuffix(' (deleted)')).name
            except OSError:
                # A zombie or vanished unrelated process says nothing about the
                # required COSMIC inventory. A potential required one is unsafe.
                errors.update(possible)
                continue
            if name not in names:
                continue
            possible = {name}
            env = dict(item.split(b'=', 1) for item in (process / 'environ').read_bytes().split(b'\0') if b'=' in item)
            if env.get(b'XDG_SESSION_ID') != session.encode():
                continue
            found[name].append((process.name, _process(process)))
        except (OSError, ValueError, UnicodeError, IndexError):
            errors.update(possible)
    return found, errors


def _constraints(expected, proc, sysroot):
    actual = {}
    if 'kernel_release' in expected:
        actual['kernel_release'] = (proc / 'sys/kernel/osrelease').read_text().strip()
    if 'kernel_version_sha256' in expected:
        actual['kernel_version_sha256'] = _stream_hash(proc / 'version')
    if 'module_versions' in expected:
        actual['module_versions'] = {name: {key: (sysroot / 'module' / name / key).read_text().strip()
            for key in attrs} for name, attrs in expected['module_versions'].items()}
    return actual == expected


def running_identity(catalog, proc=Path('/proc'), session=None, uid=None, sysroot=Path('/sys'), *, phase='running', host=None):
    session = os.environ.get('XDG_SESSION_ID') if session is None else session
    uid = os.getuid() if uid is None else uid
    result = dict(format='halcyon-runtime-measurement', version=2, phase=phase, verified=False, profiles=[])
    if not session or phase not in ('prelaunch', 'running'):
        return result
    try:
        profiles = {runtime_id(r['runtime']): valid_runtime(r['runtime']) for r in catalog['cohorts'] if r.get('runtime')}
        names = {name for p in profiles.values() for name in p['processes']}
        if phase == 'running' and host is not None:
            if host_process(host['pid'], host['sha256'], proc, session, uid) != host:
                return result
        # Two bounded attempts, each with two snapshots. Never wait for a process
        # to exit, and never substitute installed package versions for live code.
        for _ in range(2):
            found, errors = _snapshot(proc, names, session, uid)
            second, second_errors = _snapshot(proc, names, session, uid)
            matches = []
            for id, profile in profiles.items():
                required = profile['processes']
                if set(required) & (errors | second_errors):
                    continue
                if any(found[n] != second[n] or len(found[n]) != (0 if phase == 'prelaunch' and item['lifecycle'] == 'panel-child' else item['count']) for n, item in required.items()):
                    continue
                try:
                    if not _constraints(profile['constraints'], proc, sysroot):
                        continue
                except OSError:
                    continue
                try:
                    valid = True
                    for name, item in required.items():
                        if phase == 'prelaunch' and item['lifecycle'] == 'panel-child':
                            # Installed lock verification covers package resources;
                            # this also binds the exact executable about to spawn
                            # to the chosen profile (including equivalence rows).
                            valid &= digest(Path(item['executable']).resolve()) == item['sha256']
                            continue
                        for _, observed in found[name]:
                            valid &= observed['executable'] == item['executable'] and observed['sha256'] == item['sha256']
                            valid &= all(item['mapped'].get(path) == sha for path, sha in observed['mapped'].items())
                            if item['lifecycle'] == 'panel-child':
                                valid &= host is not None and _owned_by_host(observed, host, proc)
                    if valid:
                        matches.append(id)
                except (OSError, ValueError, IndexError):
                    continue
            if matches:
                if phase == 'running' and host is not None and host_process(host['pid'], host['sha256'], proc, session, uid) != host:
                    return result
                return {**result, 'verified': True, 'profiles': sorted(matches)}
    except (OSError, KeyError, TypeError, ValueError):
        pass
    return result


def host_identity(binary_sha256):
    """Read-only IPC-owner process identity, independent of installation."""
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
                return None
            host = host_process(pid, binary_sha256)
            client.sendall(b'"Capabilities"\n')
            with client.makefile('rb') as stream:
                line = stream.readline(65537)
            if len(line) > 65536 or not line.endswith(b'\n'):
                return None
            cap = json.loads(line).get('Capabilities', {})
            if cap.get('protocol') == 1 and cap.get('together') is True and cap.get('separate') is True and host_process(pid, binary_sha256) == host:
                return host
            return None
    except (OSError, ValueError, TypeError, AttributeError):
        return None


def host_health(binary_sha256):
    if not os.environ.get('XDG_RUNTIME_DIR') or not os.environ.get('XDG_SESSION_ID'):
        return None
    return host_identity(binary_sha256) is not None


def main():
    import argparse
    import json
    from bundle_contract import verify_bundle
    parser = argparse.ArgumentParser(description='Read-only installed lock and declared live-code measurements')
    parser.add_argument('--catalog', type=Path, required=True)
    parser.add_argument('--phase', choices=('prelaunch', 'running'), default='running')
    args = parser.parse_args()
    if args.catalog.name != 'catalog.json':
        parser.error('--catalog must identify a verified bundle catalog.json')
    bundle = args.catalog.parent
    manifest, catalog = verify_bundle(bundle, 'companion-manifest.json' if (bundle/'companion-manifest.json').exists() else 'manifest.json')
    host = host_identity(manifest['binary_sha256']) if args.phase == 'running' else None
    print(json.dumps(dict(installed=installed_identity(catalog), runtime=running_identity(catalog, phase=args.phase, host=host))))


if __name__ == '__main__':
    main()
