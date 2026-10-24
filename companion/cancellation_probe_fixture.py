"""Synthetic preflight cancellation, no desktop/Wayland/native process.

Run with SIGTERM, SIGKILL or WORKER_KILL. Uses current production Admission
and ABI subprocess dispatch; substitutes only successful eligibility and a sleeping ELF
stand-in. Subreaping and cleanup belong to this reviewer harness.
"""
import ctypes
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SOURCE = HERE
sys.path.insert(0,str(SOURCE))
import panel_launcher
ctypes.CDLL(None).prctl(36, 1, 0, 0, 0)

PARENT = '''
import sys
from pathlib import Path
sys.path.insert(0,sys.argv[1])
import panel_launcher
def eligible(bundle):
    def guard(child): return True
    guard.close=lambda:None
    guard.invalidated=lambda:False
    return guard
panel_launcher._prepare_launch=eligible
panel_launcher.prepare_launch(Path(sys.argv[2]))
'''

with tempfile.TemporaryDirectory() as tmp:
    root = Path(tmp)
    receipt = root / 'probe.json'
    (root / 'manifest.json').write_text('{"runtime_probe":1}')
    binary = root / 'cosmic-panel-islands'
    actor = root / 'actor/cosmic-panel-islands'
    actor.parent.mkdir()
    shutil.copy2('/usr/bin/sleep',actor)
    binary.write_text('#!/usr/bin/python3\nimport json,os,time\nfrom pathlib import Path\n'
        + 'Path('+repr(str(receipt))+').write_text(json.dumps(dict(pid=os.getpid(),worker=os.getppid())))\n'
        + 'os.execv('+repr(str(actor))+', ["cosmic-panel-islands", "30"])\n')
    binary.chmod(0o755)
    (root/'home').mkdir()
    env={**os.environ,'HOME':str(root/'home'),'XDG_DATA_HOME':str(root/'home/.local/share'),
         'XDG_STATE_HOME':str(root/'home/.local/state'),
         'WAYLAND_DISPLAY':'review-cancellation-display','XDG_RUNTIME_DIR':str(root),
         'XDG_SESSION_ID':'review-cancellation-synthetic'}
    parent = subprocess.Popen([sys.executable, '-B', '-c', PARENT, str(SOURCE), str(root)],env=env,
        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    ids = {}
    try:
        deadline = time.monotonic() + 3
        while not receipt.exists() and time.monotonic() < deadline:
            time.sleep(.01)
        ids = json.loads(receipt.read_text())
        while os.readlink('/proc/'+str(ids['pid'])+'/exe')!=str(actor) and time.monotonic()<deadline:
            time.sleep(.01)
        if sys.argv[1] == 'WORKER_KILL':
            os.kill(ids['worker'], signal.SIGKILL)
        else:
            os.kill(parent.pid, getattr(signal, sys.argv[1]))
        exit_code = parent.wait(timeout=1)
        # Cancellation must reap the probe promptly, without waiting for its deadline.
        time.sleep(.5)
        def state(pid):
            try:
                fields=(Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()
                return dict(state=fields[0], parent=int(fields[1]))
            except FileNotFoundError:
                return None
        result = dict(synthetic=True, native_qualification=False,
            production_path='prepare_launch -> Admission -> Worker -> runtime_compatible -> subprocess.run',
            parent_exit=exit_code, seconds_after_cancellation=.5,
            worker=state(ids['worker']), probe=state(ids['pid']))
        if sys.argv[1] == 'WORKER_KILL':
            # Dead owner's child is reaped by init (this isolated subreaper in
            # the test). Assert kernel termination BEFORE collecting the zombie.
            assert result['probe'] is None or result['probe']['state'] in ('Z','X'), result
            try: os.waitpid(ids['pid'], os.WNOHANG)
            except ChildProcessError: pass
            result['probe'] = state(ids['pid'])
        assert result['probe'] is None, result
        assert result['worker'] is None or result['worker']['state'] in ('Z','X'), result
        dispatch=[]
        with patch.dict(os.environ,env,clear=True), \
             patch.object(panel_launcher,'stock_host',side_effect=lambda *a,**k:dispatch.append('stock')):
            result['other_host_pid']=panel_launcher.other_host()
            result['replacement_launcher_return']=panel_launcher.main(bundle=root)
        result['replacement_dispatch']=dispatch
        result['probe_pid']=ids['pid']
        assert result['other_host_pid'] is None,result
        assert dispatch==['stock'],result
        print(json.dumps(result, indent=2))
    finally:
        if parent.poll() is None:
            parent.kill(); parent.wait()
        for pid in ids.values():
            try: os.kill(pid,signal.SIGKILL)
            except ProcessLookupError: pass
        deadline=time.monotonic()+2
        while time.monotonic()<deadline:
            try:
                pid,_=os.waitpid(-1,os.WNOHANG)
                if pid==0:time.sleep(.01)
            except ChildProcessError:break
