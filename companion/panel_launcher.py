#!/usr/bin/env python3
"""COSMIC session panel selector. One host at a time; stock on incompatibility."""
import collections
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time

STOCK = Path('/usr/bin/cosmic-panel')
PACKAGES = {
    'cosmic-panel': '1.10.0~1791315703~24.04~3ce6ce1',
    'cosmic-comp': '0.1~1791979042~24.04~742b800',
    'cosmic-session': '1.10.0~1791314577~24.04~8093b59',
}

def digest(path):
    h=hashlib.sha256()
    with open(path,'rb') as f:
        for part in iter(lambda:f.read(1024*1024),b''):h.update(part)
    return h.hexdigest()

def compatible(bundle, versions=None, manifest_name='manifest.json'):
    try:
        manifest=json.loads((bundle/manifest_name).read_text())
        if not isinstance(manifest, dict):return False
        if manifest['upstream']!='3ce6ce1cec97669f91e809962d9446aaefe8a06e' or manifest['protocol']!=1:return False
        if digest(bundle/'cosmic-panel-islands')!=manifest['binary_sha256']:return False
        if versions is None:
            versions={name:subprocess.check_output(['dpkg-query','-W','-f=${Version}',name],text=True,stderr=subprocess.DEVNULL).strip() for name in PACKAGES}
        return versions==PACKAGES
    except (OSError,ValueError,KeyError,subprocess.SubprocessError):return False

def session_identity(env=None):
    env = os.environ if env is None else env
    session = env.get('XDG_SESSION_ID')
    if not session:
        return None
    try:
        boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    except OSError:
        return None
    return boot + ':' + session

def display_identity(env):
    display=env.get('WAYLAND_DISPLAY','wayland-0')
    return display if display.startswith('/') else str(Path(env.get('XDG_RUNTIME_DIR',''))/display)

def other_host():
    wanted=display_identity(os.environ)
    for process in Path('/proc').iterdir():
        if not process.name.isdigit() or int(process.name)==os.getpid():continue
        try:
            if process.stat().st_uid!=os.getuid():continue
            if (process/'exe').resolve().name.removesuffix(' (deleted)') not in ('cosmic-panel','cosmic-panel-islands'):continue
            env=dict(item.split('=',1) for item in (process/'environ').read_bytes().decode().split('\0') if '=' in item)
            if display_identity(env)==wanted:return int(process.name)
        except (OSError,UnicodeError):continue
    return None

# A panel whose compositor connection failed can keep running, reporting the
# same protocol error in a loop with no panel or dock on screen. This many
# reports within WEDGED_SECONDS means the host is gone.
WEDGED_ERRORS = 200
WEDGED_SECONDS = 10
# Repeats of one output line passed to the journal before they are counted.
REPEATS_SHOWN = 20
# Waits before restarting a failed host, by consecutive failure. A host that
# ran HEALTHY_SECONDS starts the sequence again.
RESTART_DELAYS = (1, 2, 5, 10, 30, 60)
HEALTHY_SECONDS = 300


def watch_output(stream, child, out, wedged):
    """Pass the host's output on, collapsing floods, and stop a wedged host."""
    last, repeats, errors = None, 0, collections.deque(maxlen=WEDGED_ERRORS)
    for line in stream:
        if b'Protocol error ' in line:
            errors.append(time.monotonic())
            if (len(errors) == WEDGED_ERRORS and errors[-1] - errors[0] < WEDGED_SECONDS
                    and not wedged.is_set()):
                wedged.set()
                out.write(b'cosmic-panel stopped responding to the compositor; restarting it\n')
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        if line == last:
            repeats += 1
            if repeats > REPEATS_SHOWN:
                continue
        else:
            if repeats > REPEATS_SHOWN:
                out.write(b'(previous line repeated %d more times)\n' % (repeats - REPEATS_SHOWN))
            last, repeats = line, 0
        out.write(line)
        out.flush()
    if repeats > REPEATS_SHOWN:
        out.write(b'(previous line repeated %d more times)\n' % (repeats - REPEATS_SHOWN))
        out.flush()


def run_host(binary, pass_fds=(), env=None):
    """Keep supervision descriptors in this process, outside native applets.

    Returns the exit code, whether a signal asked us to stop, and whether the
    host was stopped because it wedged."""
    child = subprocess.Popen([str(binary)], pass_fds=pass_fds, start_new_session=True, env=env,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    stopping = False
    wedged = threading.Event()
    watcher = threading.Thread(target=watch_output, args=(child.stdout, child, sys.stderr.buffer, wedged), daemon=True)
    watcher.start()

    def terminate(signum, frame):
        nonlocal stopping
        stopping = True
        try:
            os.killpg(child.pid, signum)
        except ProcessLookupError:
            pass

    previous = {s: signal.signal(s, terminate) for s in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)}
    try:
        code = child.wait()
    finally:
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        for sig, handler in previous.items():
            signal.signal(sig, handler)
    # Applets share the output pipe; they are stopped with the host's group.
    watcher.join(2)
    if not watcher.is_alive():
        child.stdout.close()
    return code, stopping, wedged.is_set()


def pause(seconds):
    """Sleep between restarts; True when a signal asked us to stop instead."""
    stop = threading.Event()
    previous = {s: signal.signal(s, lambda signum, frame: stop.set()) for s in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)}
    try:
        stop.wait(seconds)
    finally:
        for sig, handler in previous.items():
            signal.signal(sig, handler)
    return stop.is_set()


def stock_host(binary, pass_fds=(), limit=None, env=None, keep_running=False):
    """Run the stock host, restarting it here when it fails.

    cosmic-session's own restart delay doubles with every failure and never
    resets, reaching hours, so a failure is not handed back to it. A restarted
    host cannot reuse the session's notifications stream, so it runs without
    it until the next login."""
    failures, runs = 0, 0
    while True:
        started = time.monotonic()
        code, stopping, wedged = run_host(binary, pass_fds, env)
        runs += 1
        if stopping:
            return 0
        if code == 0 and not wedged and not keep_running:
            return 0
        if limit is not None and runs >= limit:
            return code if code >= 0 else 128 - code
        if time.monotonic() - started >= HEALTHY_SECONDS:
            failures = 0
        delay = RESTART_DELAYS[min(failures, len(RESTART_DELAYS) - 1)]
        failures += 1
        sys.stderr.write('cosmic-panel exited (%s); restarting in %ds\n' % ('wedged' if wedged else code, delay))
        sys.stderr.flush()
        if 'PANEL_NOTIFICATIONS_FD' in os.environ:
            notifications = int(os.environ['PANEL_NOTIFICATIONS_FD'])
            pass_fds = tuple(fd for fd in pass_fds if fd != notifications)
            env = {k: v for k, v in os.environ.items() if k != 'PANEL_NOTIFICATIONS_FD'}
        if pause(delay):
            return 0


def supervise(binary, fallback, pass_fds=(), failed=None, session_socket=False):
    """Reap the selected host before starting stock or returning to the session."""
    started = True
    try:
        _, stopping, _ = run_host(binary, pass_fds)
    except OSError:
        started, stopping = False, False
    if stopping:
        return 0
    persisted = False
    if failed:
        try:
            failed()
            persisted = True
        except Exception:
            pass
    if session_socket and started and persisted:
        # cosmic-session recreates the panel/notifications socket pair when
        # its supervised process exits. Reusing an authenticated stream in
        # a new panel would break notifications. The marker selects stock
        # on that restart, which receives the fresh socket from the session.
        return 75
    if not persisted:
        # Without a marker, keep the stock decision and lock in this process
        # until logout. A consumed notification stream cannot be reused.
        env = None
        if session_socket and started:
            notifications = os.environ.get('PANEL_NOTIFICATIONS_FD')
            pass_fds = tuple(fd for fd in pass_fds if str(fd) != notifications)
            env = {k: v for k, v in os.environ.items() if k != 'PANEL_NOTIFICATIONS_FD'}
        return stock_host(fallback, pass_fds, env=env, keep_running=True)
    return stock_host(fallback, pass_fds)

def main():
    data=Path(os.environ.get('XDG_DATA_HOME',Path.home()/'.local/share'))
    state=Path(os.environ.get('XDG_STATE_HOME',Path.home()/'.local/state'))/'desktop-cards-panel'
    bundle=data/'cosmic-desktop-cards/companion'
    selection=bundle/'selection.json'
    try:
        selected=json.loads(selection.read_text())
        enabled=isinstance(selected, dict) and selected.get('enabled') is True
        if enabled and selected.get('defer_session') is not None:
            current=session_identity()
            enabled=current is not None and current != selected['defer_session']
    except (OSError,ValueError):enabled=False
    # A manual launch must never duplicate the session's existing host.
    if other_host() is not None:return 0
    runtime=Path(os.environ['XDG_RUNTIME_DIR']);lockpath=runtime/'desktop-cards-panel.lock'
    fd=os.open(lockpath,os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
    try:fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:return 0
    # The supervisor owns the lock. Passing it into a host also lets native
    # applets inherit it, which can block the next host after a shutdown.
    inherited=[]
    for name in ('PANEL_NOTIFICATIONS_FD','WAYLAND_SOCKET'):
        try:
            value=int(os.environ[name]);os.fstat(value);inherited.append(value);os.set_inheritable(value,True)
        except (KeyError,ValueError,OSError):pass
    if not enabled or not compatible(bundle) or (state/'failed').exists():
        return stock_host(STOCK, tuple(inherited))
    def failed():
        state.mkdir(parents=True,exist_ok=True,mode=0o700)
        (state/'failed').write_text('The companion exited unexpectedly. Stock panel selected.\n')
    return supervise(bundle/'cosmic-panel-islands',STOCK,tuple(inherited),failed,'PANEL_NOTIFICATIONS_FD' in os.environ)

if __name__=='__main__':sys.exit(main())
