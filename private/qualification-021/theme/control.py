#!/usr/bin/python3
"""Bounded real greeter/session exercise; records observations, never auto-passes pixels.
QMP controls screen/input. QGA collects separate process/session evidence.
"""
import base64, json, os, pathlib, socket, subprocess, sys, time, traceback
from PIL import Image

work = pathlib.Path(sys.argv[1]).resolve()
ev = work / 'evidence'
timeline = open(ev / 'timeline.jsonl', 'a', buffering=1)

def record(kind, **kw):
    print(kind, kw, flush=True)
    timeline.write(json.dumps(dict(time=time.time(), kind=kind, **kw))+'\n')

class Channel:
    def __init__(self, name, qmp=False):
        self.s = socket.socket(socket.AF_UNIX)
        self.s.settimeout(15)
        self.s.connect(str(work / name))
        self.f = self.s.makefile('rwb', buffering=0)
        if qmp:
            self.read()
            self.call('qmp_capabilities')

    def read(self):
        while True:
            line = self.f.readline()
            if not line:
                raise RuntimeError('channel closed')
            try:
                data = json.loads(line.lstrip(b'\xff'))
            except json.JSONDecodeError:
                continue
            if 'event' not in data:
                return data

    def call(self, cmd, args=None):
        self.f.write(json.dumps(dict(execute=cmd, arguments=args or {})).encode()+b'\n')
        result = self.read()
        if 'error' in result:
            raise RuntimeError(result)
        return result['return']

def guest(command, timeout=45):
    c = Channel('agent.sock')
    job = c.call('guest-exec', {'path':'/bin/bash','arg':['-lc',command],'capture-output':True})
    deadline = time.monotonic()+timeout
    while time.monotonic()<deadline:
        status = c.call('guest-exec-status', {'pid':job['pid']})
        if status.get('exited'):
            out = base64.b64decode(status.get('out-data','')).decode(errors='replace')
            err = base64.b64decode(status.get('err-data','')).decode(errors='replace')
            return status.get('exitcode',-1), out+err
        time.sleep(.3)
    raise TimeoutError(command)

q = None  # Explicitly initialized after QGA boot readiness by theme_session.py

def shot(name):
    ppm = ev / (name+'.ppm')
    q.call('screendump', {'filename':str(ppm)})
    Image.open(ppm).save(ev / (name+'.png'))
    ppm.unlink()
    record('screenshot', name=name)

def key(keys):
    record('input', keys=keys)
    q.call('human-monitor-command', {'command-line':'sendkey '+keys})
    time.sleep(.2)

def text(value):
    for ch in value:
        key({'-':'minus',' ':'spc','.':'dot','/':'slash'}.get(ch,ch))

def click(x,y):
    record('pointer',x=x,y=y,width=1280,height=800)
    q.call('input-send-event', {'events':[
        {'type':'abs','data':{'axis':'x','value':round(x*32767/1280)}},
        {'type':'abs','data':{'axis':'y','value':round(y*32767/800)}},
        {'type':'btn','data':{'down':True,'button':'left'}}]})
    time.sleep(.2)
    q.call('input-send-event', {'events':[{'type':'btn','data':{'down':False,'button':'left'}}]})

def user(command):
    # Use the actual logged-in COSMIC session's environment. A QGA root login
    # shell does not inherit the desktop PATH or logind session identity.
    script = """import os,pathlib,subprocess
found=None
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  if p.stat().st_uid==1000 and os.readlink(p/'exe')=='/usr/bin/cosmic-session':
   found=dict(v.split('=',1) for v in (p/'environ').read_bytes().decode().split('\\0') if '=' in v);break
 except (OSError,ProcessLookupError):pass
if found is None:raise RuntimeError('No actual COSMIC session environment')
allowed=['HOME','USER','LOGNAME','PATH','XDG_SESSION_ID','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS','WAYLAND_DISPLAY','DISPLAY','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_STATE_HOME','XDG_CURRENT_DESKTOP','XDG_SESSION_TYPE','LANG']
env={k:v for k,v in found.items() if k in allowed}
env.setdefault('HOME','/home/probe')
env.setdefault('DBUS_SESSION_BUS_ADDRESS','unix:path=/run/user/1000/bus')
env.setdefault('XDG_RUNTIME_DIR','/run/user/1000')
args=['/usr/sbin/runuser','-u','probe','--','/usr/bin/env','-i']+[k+'='+v for k,v in env.items()]+['/bin/bash','-c',COMMAND]
raise SystemExit(subprocess.run(args).returncode)
""".replace('COMMAND',repr(command))
    return guest("python3 - <<'DESKTOP_ENV'\n"+script+"\nDESKTOP_ENV")


def capture(name):
    commands = {
        'sessions':'loginctl list-sessions; loginctl user-status probe',
        'processes':'ps -eo user,pid,ppid,lstart,args --forest',
        'journal':'journalctl -b --no-pager -n 3000',
        'identity':'''python3 - <<'PY'
import os,hashlib,json,subprocess
rows=[]
for p in os.listdir('/proc'):
 if not p.isdigit():continue
 try:
  exe=os.readlink('/proc/'+p+'/exe')
  if not any(x in exe for x in ['cosmic','Xwayland','pipewire','dbus']):continue
  raw=open('/proc/'+p+'/exe','rb').read()
  rows.append(dict(pid=int(p),exe=exe,sha256=hashlib.sha256(raw).hexdigest(),
   status=open('/proc/'+p+'/status').read(),cmdline=open('/proc/'+p+'/cmdline','rb').read().decode().replace('\\0',' '),
   package=subprocess.getoutput('dpkg-query -S '+exe)))
 except (OSError,ProcessLookupError):pass
print(json.dumps(rows,indent=2))
PY''',
        'system':'uname -a; cat /proc/cmdline; dpkg --audit; systemctl --failed; lspci -nnk; dpkg-query -W cosmic-session cosmic-comp cosmic-panel cosmic-applets cosmic-notifications cosmic-greeter cosmic-settings-daemon',
        'graphics':"ls -l /dev/dri; lsmod | grep virtio; for p in $(pgrep -u 1000 -x cosmic-comp); do grep -E 'swrast|LLVM|libEGL|libgbm' /proc/$p/maps; done",
    }
    for label,command in commands.items():
        try:
            rc,out=guest(command)
            (ev/(name+'-'+label+'.txt')).write_text(out)
        except Exception as e:
            record('capture-error', label=label,error=str(e))

def wait_agent():
    deadline = time.monotonic()+240
    while time.monotonic()<deadline:
        try:
            rc,out=guest('systemctl is-active cosmic-greeter.service; cat /etc/os-release',10)
            if out.splitlines()[0] == 'active':
                record('booted', output=out)
                return
        except Exception:
            pass
        time.sleep(5)
    raise TimeoutError('guest OS/greeter did not become observable')

def wait_session():
    deadline=time.monotonic()+90
    while time.monotonic()<deadline:
        rc,out=guest('pgrep -u 1000 -ax cosmic-session; pgrep -u 1000 -ax cosmic-panel')
        if rc==0 and 'cosmic-session' in out and 'cosmic-panel' in out:
            record('session-running',output=out)
            time.sleep(12)
            return
        time.sleep(3)
    raise TimeoutError('login failed to start user session and panel')

def login(label):
    # Fresh single disposable user. Select focused user, then password prompt.
    key('ret');time.sleep(2);shot(label+'-selected-user')
    text('cosmic-probe');key('ret')
    wait_session()
    # A disposable fixture's first-login wizard is not a product scenario.
    # Preserve distribution defaults and mark onboarding complete; do not apply
    # themes, layouts or settings. Subsequent logins should show the stock shell.
    rc,out=guest('''runuser -u probe -- sh -c 'mkdir -p "$HOME/.config"; touch "$HOME/.config/cosmic-initial-setup-done"'
python3 - <<'PY'
import os,signal
for pid in os.listdir('/proc'):
 if not pid.isdigit():continue
 try:
  if os.stat('/proc/'+pid).st_uid==1000 and os.readlink('/proc/'+pid+'/exe')=='/usr/bin/cosmic-initial-setup':
   os.kill(int(pid),signal.SIGTERM)
 except (OSError,ProcessLookupError):pass
PY''')
    record('fixture-onboarding-complete',exitcode=rc,output=out)
    if rc:raise RuntimeError('fixture onboarding preparation failed')
    time.sleep(3)
    shot(label+'-desktop')
