"""Ephemeral, cancellable read-only verifier processes. No persistent receipts.

The supervisor keeps session descriptors. Workers receive none, run in their own
process group, die with their parent, and are killed/reaped on every timeout.
"""
import ctypes
import array
import json
import os
import select
import signal
import socket
import time

MAX_REPLY = 65536


class Cancelled(BaseException):
    """Must not be swallowed by subprocess/select EINTR retry handling."""


def _isolate(keep, parent):
    os.setsid()
    def cancel(*_): raise Cancelled('verification_cancelled')
    signal.signal(signal.SIGTERM, cancel)
    libc = ctypes.CDLL(None, use_errno=True)
    # Owner death must unwind subprocess.run so it kills AND reaps its probe.
    # Install the handler before arming the signal; check the fork/prctl race.
    if libc.prctl(1, signal.SIGTERM, 0, 0, 0) or os.getppid() != parent:
        os._exit(1)
    for name in ('WAYLAND_SOCKET','PANEL_NOTIFICATIONS_FD','DAEMON_NOTIFICATIONS_FD','COSMIC_SESSION_SOCK'):
        os.environ.pop(name, None)
    for name in os.listdir('/proc/self/fd'):
        fd = int(name)
        if fd > 2 and fd != keep:
            try: os.close(fd)
            except OSError: pass


def probe_parent_death():
    """Second-hop lifetime binding, used only by the isolated verifier.

    The native probe does not spawn children. Even a forced worker death must
    kill it; ordinary owner cancellation additionally lets the worker reap it.
    Construct before fork so the expected parent cannot become the new reaper.
    """
    parent = os.getpid()
    libc = ctypes.CDLL(None, use_errno=True)
    def bind():
        if libc.prctl(1, signal.SIGKILL, 0, 0, 0) or os.getppid() != parent:
            os._exit(1)
    return bind


def _send(sock, value, fd=None):
    data = json.dumps(value, separators=(',', ':')).encode()+b'\n'
    if len(data) > MAX_REPLY: raise ValueError('verification_reply_limit')
    if fd is None: sock.sendall(data)
    else:
        sent=sock.sendmsg([data],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array("i",[fd]))])
        if sent<len(data):sock.sendall(data[sent:])


class Worker:
    def __init__(self, function, service=False):
        self.socket, child = socket.socketpair()
        self.buffer = b''; self.closed = False; self.pending = True; self.received_fds=[]
        parent = os.getpid()
        self.pid = os.fork()
        if self.pid == 0:
            try:
                self.socket.close(); _isolate(child.fileno(), parent)
                result = function()
                if not service:
                    _send(child, result)
                else:
                    config = getattr(result, 'configuration', None)
                    from panel_configuration import Watch
                    # Independent event queue, created inside the cancellable
                    # worker. Transfer only its FD; no parent filesystem waits.
                    parent_watch=Watch()
                    try:
                        if config:
                            for path,directory,entry in config.watch.registrations:
                                parent_watch.add(path,directory,entry=entry)
                        eligible=result is not None and not result.invalidated()
                        _send(child, {'eligible':eligible,
                            'watches':{str(k):list(v) for k,v in parent_watch.paths.items()},
                            'outputs':config.outputs if config else None},parent_watch.fd)
                    finally:parent_watch.close()
                    if result is not None:
                        with child.makefile('rb') as requests:
                            for line in requests:
                                if len(line) > MAX_REPLY: break
                                request = json.loads(line)
                                op = request['op']
                                if op == 'measure':
                                    # Process-like adapter; the actual PID/starttime,
                                    # executable and session are checked by admission.
                                    class Child:
                                        pid = request['pid']
                                        def poll(self):
                                            try: os.kill(self.pid, 0); return None
                                            except ProcessLookupError: return 1
                                    _send(child, bool(result(Child())))
                                elif op == 'changed': _send(child, bool(result.changed()))
                                elif op == 'invalidated': _send(child, bool(result.invalidated()))
                                elif op == 'accepted': result.accepted(); _send(child, True)
                                else: break
                        result.close()
            except BaseException:
                try: _send(child, {'error': 'verification_failed'})
                except BaseException: pass
            finally:
                child.close(); os._exit(0)
        child.close()
        self.socket.setblocking(False)

    def poll(self):
        if self.closed: raise ValueError('verification_closed')
        try:
            data,ancillary,flags,_ = self.socket.recvmsg(MAX_REPLY+1,socket.CMSG_SPACE(4))
            for level,kind,value in ancillary:
                if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
                    fds=array.array('i');fds.frombytes(value)
                    for fd in fds: os.set_inheritable(fd,False)
                    self.received_fds.extend(fds)
            if flags & socket.MSG_CTRUNC or len(self.received_fds)>1: raise ValueError('verification_fd_limit')
        except BlockingIOError: return None, False
        if not data: raise ValueError('verification_closed')
        self.buffer += data
        if len(self.buffer) > MAX_REPLY: raise ValueError('verification_reply_limit')
        if b'\n' not in self.buffer: return None, False
        line, self.buffer = self.buffer.split(b'\n', 1)
        value = json.loads(line)
        if isinstance(value, dict) and 'error' in value: raise ValueError(value['error'])
        self.pending = False
        return value, True

    def receive(self, deadline):
        while time.monotonic() < deadline:
            value, done = self.poll()
            if done: return value
            select.select([self.socket], [], [], max(0, deadline-time.monotonic()))
        raise TimeoutError('verification_timeout')

    def send(self, value):
        if self.pending: raise ValueError('verification_in_flight')
        _send(self.socket, value); self.pending = True

    def close(self):
        if self.closed: return
        self.closed = True; self.socket.close()
        for fd in self.received_fds: os.close(fd)
        self.received_fds.clear()
        # Let subprocess.run's exception path kill/reap its probe before the
        # hard group cutoff. No wait can keep verification running past budget.
        try: os.kill(self.pid, signal.SIGTERM)
        except ProcessLookupError: pass
        deadline = time.monotonic()+.15
        while time.monotonic()<deadline:
            if os.waitpid(self.pid, os.WNOHANG)[0]: return
            time.sleep(.005)
        # setsid may not have run yet. Kill the PID as well as its worker group.
        for kill in (lambda: os.killpg(self.pid, signal.SIGKILL), lambda: os.kill(self.pid, signal.SIGKILL)):
            try: kill()
            except ProcessLookupError: pass
        os.waitpid(self.pid, 0)

    def __enter__(self): return self
    def __exit__(self, *args): self.close()


def bounded(function, timeout=5):
    with Worker(function) as worker:
        return worker.receive(time.monotonic()+timeout)


class Admission:
    """One launch's frozen verifier; never an on-disk or cross-launch cache."""
    def __init__(self, factory, deadline):
        self.worker = Worker(factory, service=True)
        self.accepted_startup = False
        self.watch = None; self.outputs = None
        try:
            reply = self.worker.receive(deadline)
            if reply.get('eligible') is not True:
                raise ValueError('prelaunch_not_eligible')
            from panel_configuration import Watch
            if len(self.worker.received_fds)!=1: raise ValueError('configuration_watch_missing')
            self.watch=object.__new__(Watch);self.watch.fd=self.worker.received_fds.pop()
            self.watch.paths={int(k):set(v) for k,v in reply['watches'].items()};self.watch.dirty=False
            self.outputs=reply['outputs']
            # Independent parent queue survives a stalled/cancelled measurement.
            if self._ask('invalidated',deadline) or self.watch.changed() or time.monotonic()>=deadline:
                raise ValueError('prelaunch_changed')
        except BaseException:
            self.close(); raise

    def begin(self, child): self.worker.send({'op':'measure', 'pid':child.pid})
    def poll(self): return self.worker.poll()
    def __call__(self, child):
        self.begin(child)
        return self.worker.receive(time.monotonic()+15) is True
    def _ask(self, op, deadline=None):
        self.worker.send({'op':op})
        return self.worker.receive(min(time.monotonic()+.3,deadline) if deadline is not None else time.monotonic()+.3) is True
    def changed(self):
        if self.accepted_startup: return False
        if self.watch and self.watch.changed(): return True
        if self.outputs is not None:
            from panel_configuration import output_names
            try: return bounded(output_names,.3) != self.outputs
            except (OSError,ValueError,TimeoutError): return True
        return False
    def invalidated(self):
        if self.watch and self.watch.changed(): return True
        try: return self._ask('invalidated')
        except (OSError, ValueError, TimeoutError): return True
    def accepted(self):
        self._ask('accepted'); self.accepted_startup = True
        self.close()
    def close(self):
        if self.watch: self.watch.close()
        self.worker.close()
