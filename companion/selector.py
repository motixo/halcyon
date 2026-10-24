#!/usr/bin/python3
"""Stable v1 selector: resolve one generation; preserve session descriptors."""
import hashlib
import fcntl
import signal
import stat
import subprocess
import json
import os
from pathlib import Path
import runpy
import sys
import time

sys.dont_write_bytecode = True


def fallback():
    """Fail closed without importing a damaged generation or duplicating a host."""
    runtime = os.environ.get('XDG_RUNTIME_DIR')
    if not runtime:
        return 1
    display = os.environ.get('WAYLAND_DISPLAY')
    for process in Path('/proc').iterdir():
        if not process.name.isdigit() or int(process.name) == os.getpid():
            continue
        try:
            if process.stat().st_uid != os.getuid():
                continue
            if (process/'exe').resolve().name.removesuffix(' (deleted)') not in ('cosmic-panel','cosmic-panel-islands'):
                continue
            env = dict(item.split(b'=',1) for item in (process/'environ').read_bytes().split(b'\0') if b'=' in item)
            if env.get(b'XDG_RUNTIME_DIR') == runtime.encode() and env.get(b'WAYLAND_DISPLAY') == (display or '').encode():
                return 0
        except OSError:
            continue
    fd = os.open(Path(runtime)/'desktop-cards-panel.lock', os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW, 0o600)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX|fcntl.LOCK_NB)
        except BlockingIOError:
            return 0
        inherited=[]
        for name in ('PANEL_NOTIFICATIONS_FD','WAYLAND_SOCKET'):
            try:
                value=int(os.environ[name]);os.fstat(value);inherited.append(value)
            except (KeyError,ValueError,OSError):
                pass
        child=subprocess.Popen(['/usr/bin/cosmic-panel'],pass_fds=inherited,start_new_session=True)
        def stop(signum, _):
            try:os.killpg(child.pid,signum)
            except ProcessLookupError:pass
        for signum in (signal.SIGTERM,signal.SIGINT,signal.SIGHUP):signal.signal(signum,stop)
        try:return child.wait()
        finally:stop(signal.SIGTERM,None)
    finally:
        os.close(fd)


def captured(path, limit):
    fd=os.open(path,os.O_RDONLY|os.O_NONBLOCK|os.O_NOFOLLOW)
    with os.fdopen(fd,'rb') as stream:
        before=os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size>limit: raise ValueError('unbounded_bootstrap_record')
        data=stream.read(before.st_size+1)
        after=os.fstat(stream.fileno())
        if len(data)!=before.st_size or (before.st_ino,before.st_size,before.st_mtime_ns,before.st_ctime_ns)!=(after.st_ino,after.st_size,after.st_mtime_ns,after.st_ctime_ns):
            raise ValueError('bootstrap_changed')
        return data


def main():
    home = Path.home()
    base = home / '.local/share/cosmic-desktop-cards/companion'
    deadline = time.monotonic()+5
    def expired(*_): raise TimeoutError('prelaunch_deadline')
    previous = signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, 5)
    try:
        # Verify the generation before importing any user-bundle helper. The
        # manifest's content address comes from the atomically selected record.
        pointer_path = base / 'generation.json'
        if any(p.is_symlink() for p in (pointer_path, *pointer_path.parents)):
            raise ValueError('symlink')
        pointer = json.loads(captured(pointer_path,16*1024*1024))
        generation = pointer['generation']
        if len(generation) != 64 or any(c not in '0123456789abcdef' for c in generation):
            raise ValueError('generation')
        bundle = base / 'generations' / generation
        manifest_path = bundle / 'manifest.json'
        if any(p.is_symlink() for p in (manifest_path, *manifest_path.parents)):
            raise ValueError('symlink')
        raw = captured(manifest_path,16*1024*1024)
        if hashlib.sha256(raw).hexdigest() != generation:
            raise ValueError('manifest')
        manifest = json.loads(raw)
        for name, entry in manifest['files'].items():
            if Path(name).name != name:
                raise ValueError('path')
            path = bundle / name
            if path.is_symlink() or path.stat().st_uid != os.getuid() or path.stat().st_mode & 0o777 != entry['mode']:
                raise ValueError('owner_or_mode')
            if name in ('catalog.json','cosmic-panel-islands'):
                # Data only: the verified helper below bounds, hashes and
                # validates the same opened bytes before refresh or launch.
                # Do not pre-hash an unbounded catalog before that reader.
                continue
            if hashlib.sha256(captured(path,16*1024*1024)).hexdigest() != entry['sha256']:
                raise ValueError('digest')
        sys.path.insert(0, str(bundle))
        # The read-only launch worker captures/validates the catalog once.
        # Staging is the independent package-owned bootstrap's responsibility;
        # never copy an update or wait for a transaction on the panel path.
        import panel_launcher
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)
        return panel_launcher.main(bundle=bundle, pointer=pointer, deadline=deadline)
    except (OSError, ValueError, KeyError, TypeError, AttributeError, ImportError):
        # No shell, no descriptor probe/consumption and no unsafe helper import.
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)
        return fallback()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)


if __name__ == '__main__':
    sys.exit(main())
