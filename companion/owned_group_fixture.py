"""Disposable real-process cleanup regression; synthetic actors, not native QA."""
import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import time


def stat(pid):
    try:
        fields=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
        return {'pid':pid,'state':fields[0],'ppid':int(fields[1]),'pgid':int(fields[2]),
                'sid':int(fields[3]),'start':int(fields[19])}
    except FileNotFoundError:return None


def actor(root,index,resistant):
    if resistant:signal.signal(signal.SIGTERM,signal.SIG_IGN)
    (root/f'actor-{index}').write_text(json.dumps(stat(os.getpid())))
    while True:time.sleep(1)


def host(root):
    ctypes.CDLL(None).prctl(15, b'fixture)host(', 0, 0, 0)
    if os.environ.get('RESIST_LEADER')=='1':signal.signal(signal.SIGTERM,signal.SIG_IGN)
    (root/'leader').write_text(json.dumps(stat(os.getpid())))
    if os.fork()==0:
        # A grandchild must receive escalation even after its parent/leader exit.
        if os.fork()==0:actor(root,2,True)
        actor(root,1,True)
    if os.fork()==0:actor(root,3,False)
    while not (root/'release').exists():time.sleep(.005)
    os._exit(7)


def driver(mode):
    import panel_launcher as launcher
    if os.environ.get('OWNED_GROUP_LAUNCHER'):
        import importlib.util
        spec=importlib.util.spec_from_file_location('frozen_launcher',os.environ['OWNED_GROUP_LAUNCHER'])
        launcher=importlib.util.module_from_spec(spec);spec.loader.exec_module(launcher)
    # Test harness alone adopts/reaps synthetic orphan actors. Production does
    # not change the subreaper role of the user's session.
    assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp);script=root/'host'
        script.write_text('#!/usr/bin/python3\nimport sys\nsys.path.insert(0,'+repr(str(Path(__file__).parent))+')\nfrom owned_group_fixture import host\nfrom pathlib import Path\nhost(Path('+repr(tmp)+'))\n');script.chmod(0o755)
        outsider=subprocess.Popen(['/bin/sleep','30'],start_new_session=True)
        reports={};errors=[];started=time.monotonic();finished=threading.Event()
        def control():
            try:
                deadline=time.monotonic()+3
                while len(list(root.glob('actor-*')))!=3:
                    assert time.monotonic()<deadline,'host did not start children'
                    time.sleep(.005)
                actors=[json.loads(p.read_text()) for p in sorted(root.glob('actor-*'))]
                leader=json.loads((root/'leader').read_text());reports.update(actors=actors,leader=leader)
                assert all(a['pgid']==leader['pid']==a['sid'] for a in actors)
                if mode.startswith('selector-'):
                    import fcntl
                    fd=os.open(root/'runtime/desktop-cards-panel.lock',os.O_RDWR)
                    try:
                        try:fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB);reports['lock_held_before_cleanup']=False
                        except BlockingIOError:reports['lock_held_before_cleanup']=True
                    finally:os.close(fd)
                if mode in ('outer-term','selector-term'):
                    os.kill(os.getpid(),signal.SIGTERM)
                    # RED-version escape hatch; never let a broken supervisor
                    # leave fixture actors behind on the test's outer timeout.
                    finished.wait(3)
                    now=stat(leader['pid'])
                    if now and now['start']==leader['start'] and now['state'] not in ('Z','X'):
                        fd=os.pidfd_open(leader['pid']);signal.pidfd_send_signal(fd,signal.SIGKILL);os.close(fd)
                else:(root/'release').touch()
            except BaseException as e:errors.append(repr(e))
        thread=threading.Thread(target=control);thread.start()
        original_env=os.environ.copy()
        if mode in ('outer-term','selector-term'):os.environ['RESIST_LEADER']='1'
        try:
            # Readiness is not under test; lifecycle fixture tests separately use
            # real IPC/ELF/admission. This fixture measures cleanup at handoff.
            if mode=='census-race':
                # Deterministically model getdents missing a child born while
                # the previous parent record changed to zombie. Actual child
                # actors remain live; only the first census is fault-injected.
                real_members=launcher.OwnedHost.live_members
                first=[True]
                def raced_members(owner,deadline):
                    if first:
                        first.pop();return []
                    return real_members(owner,deadline)
                launcher.OwnedHost.live_members=raced_members
            launcher.wait_ready=lambda *args:True
            launcher.compositor_gone=lambda:False
            def live():
                rows=[stat(a['pid']) for a in reports.get('actors',[])]
                return [a for a in rows if a and a['state'] not in ('Z','X')]
            def failed():
                reports['live_at_failed_marker']=live()
            if mode.startswith('selector-'):
                import selector, fcntl
                from unittest.mock import patch
                (root/'runtime').mkdir()
                real_popen=subprocess.Popen
                def spawn(args,**kwargs):
                    assert args==['/usr/bin/cosmic-panel']
                    return real_popen([str(script)],**kwargs)
                with patch.dict(os.environ,{'XDG_RUNTIME_DIR':str(root/'runtime'),'WAYLAND_DISPLAY':'isolated-selector'}),patch.object(selector.subprocess,'Popen',spawn):
                    reports['result']=selector.fallback()
                fd=os.open(root/'runtime/desktop-cards-panel.lock',os.O_RDWR)
                try:fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB);reports['lock_released_after_cleanup']=True
                finally:os.close(fd)
            elif mode=='handoff':
                result=launcher.supervise(script,'/unreachable/stock',failed=failed,session_socket=True)
                reports['result']=result
            elif mode=='fallback':
                stock=root/'stock'
                stock.write_text('#!/usr/bin/python3\nfrom pathlib import Path\nPath('+repr(str(root/'stock-start'))+').write_text("started")\n')
                stock.chmod(0o755)
                result=launcher.supervise(script,stock,failed=failed)
                reports.update(result=result,stock_started=(root/'stock-start').exists())
            else:reports['result']=launcher.run_host(script)
            reports['live_at_return']=live()
            reports['outsider_alive']=outsider.poll() is None
            reports['elapsed']=time.monotonic()-started
            try:os.waitpid(reports['leader']['pid'],os.WNOHANG);reports['leader_reaped']=False
            except ChildProcessError:reports['leader_reaped']=True
        finally:
            finished.set();thread.join(4);os.environ.clear();os.environ.update(original_env)
            # Never leave failed RED actors behind. PID identities are checked;
            # signal exact pidfds, not names or a possibly recycled group.
            for path in root.glob('actor-*'):
                a=json.loads(path.read_text());now=stat(a['pid'])
                if now and now['start']==a['start']:
                    try:
                        fd=os.pidfd_open(a['pid']);signal.pidfd_send_signal(fd,signal.SIGKILL);os.close(fd)
                    except ProcessLookupError:pass
            outsider.terminate();outsider.wait(timeout=2)
            deadline=time.monotonic()+2;reaped=[]
            while time.monotonic()<deadline:
                try:
                    pid,_=os.waitpid(-1,os.WNOHANG)
                    if pid:reaped.append(pid)
                    else:time.sleep(.005)
                except ChildProcessError:break
            reports['harness_reaped_descendants']=reaped
        assert not thread.is_alive() and not errors,errors
        print(json.dumps(reports,sort_keys=True))

if __name__=='__main__':driver(sys.argv[1])
