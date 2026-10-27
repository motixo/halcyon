#!/usr/bin/env python3
"""Kill an owned parent; verify its real image worker exits and is reaped."""
import argparse
import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import time

p = argparse.ArgumentParser()
p.add_argument('binary', type=Path)
p.add_argument('output', type=Path)
a = p.parse_args()
libc = ctypes.CDLL(None, use_errno=True)
assert libc.prctl(36, 1, 0, 0, 0) == 0  # This probe alone adopts its descendants.
read, write = os.pipe()
parent = None
worker = None
reaped = False
try:
    code = ('import subprocess,time; '
            'p=subprocess.Popen([' + repr(str(a.binary.resolve())) +
            ',"--backdrop-worker"],stdin=' + str(read) +
            ',stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); '
            'print(p.pid,flush=True); time.sleep(30)')
    parent = subprocess.Popen(['python3', '-c', code], pass_fds=(read,),
                              stdout=subprocess.PIPE, text=True)
    os.close(read)
    read = None
    worker = int(parent.stdout.readline())
    until = time.monotonic() + 3
    while 'pipe_read' not in Path(f'/proc/{worker}/wchan').read_text():
        assert time.monotonic() < until, 'Worker did not start its bounded input read'
        time.sleep(.001)
    began = time.monotonic()
    parent.kill()
    parent.wait(timeout=1)
    while True:
        done, status = os.waitpid(worker, os.WNOHANG)
        if done:
            reaped = True
            break
        assert time.monotonic() - began < 1, 'Worker survived its parent'
        time.sleep(.001)
    result = {'parent_exit': parent.returncode,
              'worker_signal': os.WTERMSIG(status),
              'elapsed_ms': (time.monotonic() - began) * 1000,
              'worker_reaped': not Path(f'/proc/{worker}').exists(),
              'scope': 'Actual release worker blocked reading stdin; controller '
                       'retains the pipe writer so EOF cannot substitute for '
                       'parent-death termination'}
    assert os.WIFSIGNALED(status) and result['worker_signal'] == signal.SIGKILL
    assert result['worker_reaped']
    a.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
finally:
    if parent is not None and parent.poll() is None:
        parent.kill()
        parent.wait(timeout=1)
    if worker is not None and not reaped:
        try:
            os.kill(worker, signal.SIGKILL)
            os.waitpid(worker, 0)
        except ProcessLookupError:
            pass
    if read is not None:
        os.close(read)
    os.close(write)
