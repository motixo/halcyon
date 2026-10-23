#!/usr/bin/python3
"""Stable v1 selector: resolve one generation; preserve session descriptors."""
import hashlib
import fcntl
import signal
import subprocess
import json
import os
from pathlib import Path
import runpy
import sys

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


def main():
    home = Path.home()
    base = home / '.local/share/cosmic-desktop-cards/companion'
    try:
        # Verify the generation before importing any user-bundle helper. The
        # manifest's content address comes from the atomically selected record.
        pointer_path = base / 'generation.json'
        if any(p.is_symlink() for p in (pointer_path, *pointer_path.parents)):
            raise ValueError('symlink')
        pointer = json.loads(pointer_path.read_bytes())
        generation = pointer['generation']
        if len(generation) != 64 or any(c not in '0123456789abcdef' for c in generation):
            raise ValueError('generation')
        bundle = base / 'generations' / generation
        manifest_path = bundle / 'manifest.json'
        if any(p.is_symlink() for p in (manifest_path, *manifest_path.parents)):
            raise ValueError('symlink')
        raw = manifest_path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != generation:
            raise ValueError('manifest')
        manifest = json.loads(raw)
        for name, entry in manifest['files'].items():
            if Path(name).name != name:
                raise ValueError('path')
            path = bundle / name
            if path.is_symlink() or path.stat().st_uid != os.getuid() or path.stat().st_mode & 0o777 != entry['mode']:
                raise ValueError('owner_or_mode')
            with path.open('rb') as stream:
                if hashlib.file_digest(stream, 'sha256').hexdigest() != entry['sha256']:
                    raise ValueError('digest')
        sys.path.insert(0, str(bundle))
        import bundle_contract
        bundle_contract.verify_bundle(bundle, owner=os.getuid())
        # Package bootstrap also runs independently via XDG autostart, covering
        # the very first upgrade from the old copied 0.2.0 launcher. Staging here
        # never changes this launch's already resolved generation.
        import refresh
        refresh.stage_installed_update(home, refresh.LIBRARY)
        import panel_launcher
        return panel_launcher.main(bundle=bundle, pointer=pointer)
    except (OSError, ValueError, KeyError, TypeError, AttributeError, ImportError):
        # No shell, no descriptor probe/consumption and no unsafe helper import.
        return fallback()


if __name__ == '__main__':
    sys.exit(main())
