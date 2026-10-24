"""Disposable observer-error/threaded-zombie regressions, never native evidence."""
import ctypes
import errno
import importlib.util
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import tempfile
import threading
import time

import panel_launcher as launcher
import verification  # Load before exhausting this fixture's FDs.
if os.environ.get('OWNED_GROUP_LAUNCHER'):
    spec=importlib.util.spec_from_file_location('frozen_launcher',os.environ['OWNED_GROUP_LAUNCHER'])
    launcher=importlib.util.module_from_spec(spec);spec.loader.exec_module(launcher)


def observer(mode):
    held=[];owner=[];returned=threading.Event();rescue_used=[];faults=[]
    original_init=launcher.OwnedHost.__init__;original_signal=launcher.OwnedHost.signal_group
    with tempfile.TemporaryDirectory() as tmp:
        host=Path(tmp)/'host'
        lines=''.join('echo "Protocol error fixture"\n' for _ in range(launcher.WEDGED_ERRORS)) if mode=='output-error' else ''
        host.write_text('#!/bin/sh\n'+lines+'exec /bin/sleep 30\n');host.chmod(0o755)
        os.environ.pop('XDG_RUNTIME_DIR',None)
        if mode=='emfile':
            soft,hard=resource.getrlimit(resource.RLIMIT_NOFILE)
            resource.setrlimit(resource.RLIMIT_NOFILE,(min(64,soft),hard))
        def init(self,*args,**kwargs):
            original_init(self,*args,**kwargs);owner.append((self,os.pidfd_open(self.pid)))
            if mode=='emfile':
                while True:
                    try:held.append(os.open('/dev/null',os.O_RDONLY))
                    except OSError as e:
                        assert e.errno==errno.EMFILE;faults.append('actual EMFILE');break
        def signal_group(self,signum):
            if mode!='emfile' and threading.current_thread() is not threading.main_thread():
                faults.append('observer signal error')
                raise launcher.GroupCleanupError('injected_observer_guard_failure')
            return original_signal(self,signum)
        launcher.OwnedHost.__init__=init;launcher.OwnedHost.signal_group=signal_group
        def rescue():
            # RED-only safety. GREEN must return before this runs and never
            # depend on freeing descriptors or an external signal to progress.
            if returned.wait(4):return
            rescue_used.append(True)
            while held:os.close(held.pop())
            if owner:signal.pidfd_send_signal(owner[0][1],signal.SIGKILL)
        helper=threading.Thread(target=rescue);helper.start();start=time.monotonic();result=None;error=None
        try:
            try:result=launcher.run_host(host,readiness=mode!='output-error')
            except launcher.GroupCleanupError as e:error=str(e)
            elapsed=time.monotonic()-start
        finally:
            returned.set();helper.join(1)
            while held:os.close(held.pop())
        child,fd=owner[0]
        # run_host must have terminated/reaped the exact direct child itself.
        try:os.waitpid(child.pid,os.WNOHANG);reaped=False
        except ChildProcessError:reaped=True
        if not reaped:
            signal.pidfd_send_signal(fd,signal.SIGKILL);child.process.wait(timeout=2)
        os.close(fd)
        report=dict(mode=mode,error=error,result=result,elapsed=elapsed,faults=faults,
                    external_rescue=bool(rescue_used),leader_reaped=reaped,identity_fd_closed=getattr(child,'stat_fd',None) is None)
        print(json.dumps(report))
        assert error and elapsed<3 and not rescue_used and reaped and faults,report
        if mode!='emfile':assert error=='host_observer_failed',report


SOURCE=r'''
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/syscall.h>
static void *work(void *unused) {
    FILE *f=fopen(getenv("ACTOR_FILE"),"w");
    fprintf(f,"%d %ld\n",getpid(),syscall(SYS_gettid)); fclose(f);
    for (;;) usleep(10000);
    return NULL;
}
int main(void) {
    if(fork()==0) {
        signal(SIGTERM,SIG_IGN);
        pthread_t thread; if(pthread_create(&thread,NULL,work,NULL)!=0) _exit(3);
        pthread_exit(NULL);
    }
    while(access(getenv("RELEASE_FILE"),F_OK)) usleep(1000);
    return 0;
}
'''

def threaded():
    assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0 # Test-only subreaper.
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp);source=root/'host.c';binary=root/'host';actors=root/'actors';release=root/'release'
        source.write_text(SOURCE)
        subprocess.run(['cc','-O2','-pthread',str(source),'-o',str(binary)],check=True,timeout=10)
        child=launcher.OwnedHost(binary,(),dict(os.environ,ACTOR_FILE=str(actors),RELEASE_FILE=str(release)))
        actorfd=None
        try:
            deadline=time.monotonic()+2
            while True:
                if actors.exists() and len(actors.read_text().split())==2:
                    pid,tid=map(int,actors.read_text().split())
                    if launcher.OwnedHost._stat(pid)[0]==b'Z':break
                assert time.monotonic()<deadline;time.sleep(.001)
            actorfd=os.pidfd_open(pid)
            fields=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
            threads=int(fields[17]);worker_before=launcher.OwnedHost._stat(tid)
            release.touch()
            while child.poll() is None:
                assert time.monotonic()<deadline;time.sleep(.001)
            live=child.live_members(deadline);start=time.monotonic();result=child.finish();elapsed=time.monotonic()-start
            try:worker_after=launcher.OwnedHost._stat(tid)[0].decode()
            except FileNotFoundError:worker_after=None
            report=dict(pid=pid,tid=tid,leader_state='Z',threads=threads,worker_before=worker_before[0].decode(),
                        live_members=live,result=result,elapsed=elapsed,worker_at_return=worker_after)
            print(json.dumps(report))
            assert threads==2 and worker_before[1]==child.pid and worker_before[2]==child.pid
            assert pid in [p for p,_ in live],report
            assert worker_after is None and result==0 and elapsed<2,report
        finally:
            if actorfd is not None:
                try:signal.pidfd_send_signal(actorfd,signal.SIGKILL)
                except ProcessLookupError:pass
                os.close(actorfd)
            if not child.closed:child.finish()
            child.stdout.close()
            deadline=time.monotonic()+2
            while True:
                try:
                    pid,_=os.waitpid(-1,os.WNOHANG)
                    if not pid:assert time.monotonic()<deadline;time.sleep(.001)
                except ChildProcessError:break

if __name__=='__main__':
    if sys.argv[1]=='threaded':threaded()
    else:observer(sys.argv[1])
