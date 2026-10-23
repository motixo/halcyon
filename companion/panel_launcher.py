#!/usr/bin/env python3
"""COSMIC session panel selector. One host at a time; stock on incompatibility."""
import collections
import errno
import fcntl
import hashlib
import json
import os
import re
from pathlib import Path
import signal
import socket
import subprocess
import struct
import sys
import threading
import time

STOCK = Path('/usr/bin/cosmic-panel')
PACKAGES = {
    'cosmic-panel': '1.11.0~1792526322~24.04~ca20ce3',
    'cosmic-comp': '0.1~1792508925~24.04~8cb1802',
    'cosmic-session': '1.11.0~1792525615~24.04~4b0a0a2',
}
# Whole combinations, never a cross-product of individually supported packages.
# Add a combination only after native validation with this companion revision.
SUPPORTED_SYSTEMS = (
    PACKAGES,
    {
        'cosmic-panel': '1.10.0~1791315703~24.04~3ce6ce1',
        'cosmic-comp': '0.1~1791979042~24.04~742b800',
        'cosmic-session': '1.10.0~1791314577~24.04~8093b59',
    },
)


# A package rebuild may change timestamps/distribution versions without changing
# native source. Preserve whole reviewed triples, not a per-package wildcard.
SOURCE_POLICY = 'reviewed-source-v1'

def source_revision(name, version):
    if not isinstance(version, str):
        return None
    match = re.fullmatch(r'(\d+(?:\.\d+)*)~\d+~[0-9.]+~([0-9a-f]{7,40})', version)
    if not match:
        return None
    release = tuple(int(n) for n in match[1].split('.'))
    if name != 'cosmic-comp' and release < (1, 10):
        return None
    return match[2]


def supported_system(versions):
    if not isinstance(versions, dict) or versions.keys() != PACKAGES.keys():
        return False
    if any(versions == system for system in SUPPORTED_SYSTEMS):
        return True
    sources = {name: source_revision(name, version) for name, version in versions.items()}
    return all(sources.values()) and any(
        sources == {name: source_revision(name, version) for name, version in system.items()}
        for system in SUPPORTED_SYSTEMS)


def digest(path):
    h=hashlib.sha256()
    with open(path,'rb') as f:
        for part in iter(lambda:f.read(1024*1024),b''):h.update(part)
    return h.hexdigest()

def installed_versions():
    """Keep partial diagnostics when one package is missing or cannot be read."""
    versions = {}
    for name in PACKAGES:
        try:
            value = subprocess.check_output(
                ['dpkg-query', '-W', '-f=${Version}', name], text=True,
                stderr=subprocess.DEVNULL, timeout=5).strip()
            versions[name] = value or None
        except (OSError, ValueError, subprocess.SubprocessError):
            versions[name] = None
    return versions

def compatible(bundle, versions=None, manifest_name='manifest.json'):
    try:
        manifest=json.loads((bundle/manifest_name).read_text())
        if not isinstance(manifest, dict):return False
        if manifest['upstream']!='ca20ce3185b3b4404f429d59d429c40b58a2ddef' or manifest['protocol']!=1:return False
        if digest(bundle/'cosmic-panel-islands')!=manifest['binary_sha256']:return False
        if versions is None:
            versions=installed_versions()
        return supported_system(versions) and (
            any(versions == row for row in SUPPORTED_SYSTEMS)
            or manifest.get('runtime_probe') == 1)
    except (OSError,ValueError,KeyError,subprocess.SubprocessError):return False

def runtime_compatible(bundle):
    """Execute the checksummed binary with no inherited session descriptors.

    This loads its native ABI and checks advertised globals on a fresh named
    Wayland connection. No surface, applet, preference or notification stream.
    A timeout, loader failure, missing capability or malformed reply picks stock.
    Legacy exact-version bundles retain their existing supervised startup.
    """
    try:
        manifest = json.loads((bundle / 'manifest.json').read_text())
        if not isinstance(manifest, dict):
            return False
        if manifest.get('runtime_probe') != 1:
            versions = installed_versions()
            return any(versions == row for row in SUPPORTED_SYSTEMS)
        env = {k: v for k, v in os.environ.items()
               if k not in ('WAYLAND_SOCKET', 'PANEL_NOTIFICATIONS_FD',
                            'DAEMON_NOTIFICATIONS_FD', 'COSMIC_SESSION_SOCK')}
        result = subprocess.run([str(bundle / 'cosmic-panel-islands'), '--islands-probe'],
                                env=env, capture_output=True, text=True, timeout=5,
                                close_fds=True)
        report = json.loads(result.stdout)
        return (result.returncode == 0 and isinstance(report, dict)
                and report.get('probe') == 1 and report.get('ready') is True)
    except (OSError, ValueError, TypeError, subprocess.SubprocessError):
        return False


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

def compositor_gone():
    """A vanished compositor is session teardown, not a panel crash.

    Probe a separate named socket, never the inherited WAYLAND_SOCKET or
    notifications stream. Unknown/access-denied states retain crash handling.
    """
    display = os.environ.get('WAYLAND_DISPLAY')
    if not display:
        return False
    if not display.startswith('/'):
        runtime = os.environ.get('XDG_RUNTIME_DIR')
        if not runtime:
            return False
        display = str(Path(runtime) / display)
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as probe:
            probe.settimeout(0.2)
            probe.connect(display)
        return False
    except OSError as error:
        return error.errno in (errno.ENOENT, errno.ECONNREFUSED)


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
READY_SECONDS = 15


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


def wait_ready(child, done, wedged):
    """Confirm the real child dispatches IPC; a live but hung process is unsafe."""
    deadline = time.monotonic() + READY_SECONDS
    runtime = os.environ.get('XDG_RUNTIME_DIR')
    while runtime and time.monotonic() < deadline and not done.is_set():
        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
                client.settimeout(min(.5, max(.01, deadline - time.monotonic())))
                client.connect(str(Path(runtime) / 'desktop-cards-islands-v1.sock'))
                pid, uid, _ = struct.unpack('3i', client.getsockopt(
                    socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize('3i')))
                if pid != child.pid or uid != os.getuid():
                    raise ValueError('wrong IPC owner')
                client.sendall(b'"Capabilities"\n')
                with client.makefile('rb') as stream:
                    line = stream.readline(65537)
                if len(line) > 65536 or not line.endswith(b'\n'):
                    raise ValueError('invalid IPC reply')
                cap = json.loads(line).get('Capabilities', {})
                if (cap.get('protocol') == 1 and cap.get('together') is True
                        and cap.get('separate') is True):
                    return
        except (OSError, ValueError, AttributeError, TypeError):
            pass
        if done.wait(.1):
            return
    if not done.is_set() and child.poll() is None:
        wedged.set()
        sys.stderr.write('Islands startup IPC health check failed; selecting stock\n')
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def run_host(binary, pass_fds=(), env=None, readiness=False):
    """Keep supervision descriptors in this process, outside native applets.

    Returns the exit code, whether a signal asked us to stop, and whether the
    host was stopped because it wedged."""
    child = subprocess.Popen([str(binary)], pass_fds=pass_fds, start_new_session=True, env=env,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    stopping = False
    wedged = threading.Event()
    watcher = threading.Thread(target=watch_output, args=(child.stdout, child, sys.stderr.buffer, wedged), daemon=True)
    watcher.start()
    done = threading.Event()
    if readiness:
        threading.Thread(target=wait_ready, args=(child, done, wedged), daemon=True).start()

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
        done.set()
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
        if stopping or compositor_gone():
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
        _, stopping, _ = run_host(binary, pass_fds, readiness=True)
    except OSError:
        started, stopping = False, False
    if stopping or (started and compositor_gone()):
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
    if not runtime_compatible(bundle):
        sys.stderr.write('Islands ABI/compositor preflight failed; selecting stock\n')
        return stock_host(STOCK, tuple(inherited))
    def failed():
        state.mkdir(parents=True,exist_ok=True,mode=0o700)
        (state/'failed').write_text('The companion exited unexpectedly. Stock panel selected.\n')
    return supervise(bundle/'cosmic-panel-islands',STOCK,tuple(inherited),failed,'PANEL_NOTIFICATIONS_FD' in os.environ)

if __name__=='__main__':sys.exit(main())
