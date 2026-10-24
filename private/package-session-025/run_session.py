"""Ordinary disposable package/UI acceptance, no blur experiment or admission."""
import os,pathlib,json,time,subprocess,shutil,tarfile,traceback,hashlib
import ui
from vm import W,E,R,Channel,guest,put,boot,stop
from common import HERE,sha,pins
T=ui.T;C=W/'control';C.mkdir(exist_ok=True);H='/var/lib/ps025';Q=None

def checked(path,args,label,expected=0,timeout=120):
 start=time.monotonic();rc,out,err=guest(T,path,args,timeout);(E/(label+'.stdout')).write_bytes(out);(E/(label+'.stderr')).write_bytes(err)
 (E/(label+'.receipt.json')).write_text(json.dumps(dict(path=path,args=args,returncode=rc,expected=expected,seconds=time.monotonic()-start),indent=2))
 assert rc==expected,f'{label}: guest rc {rc}, expected {expected}; see exact stdout/stderr'
 return out

def ops(args,label,expected=0):return checked('/usr/bin/python3',['-B',H+'/guest_ops.py',*args],label,expected)
def collect():
 checked('/bin/tar',['-czf',H+'-results.tar.gz','-C',H,'results'],'collect-results')
 target=E/'guest-results.tar.gz'
 if target.exists():target.unlink() # only regenerate our own transient evidence bundle
 ui.getfile(H+'-results.tar.gz',target)
 assert sha(target)==checked('/usr/bin/sha256sum',[H+'-results.tar.gz'],'results-sha').decode().split()[0]

def login(label):
 time.sleep(10);ui.shot(label+'-greeter');ui.key('ret');time.sleep(2);ui.text('cosmic-probe');ui.key('ret')
 for _ in range(120):
  rc,o,e=guest(T,'/usr/bin/pgrep',['-u','1000','-ax','cosmic-panel'],10)
  if rc==0:break
  time.sleep(.5)
 else:raise RuntimeError('stock panel failed to start')
 time.sleep(10)
 checked('/bin/sh',['-c',"runuser -u probe -- sh -c 'mkdir -p $HOME/.config; touch $HOME/.config/cosmic-initial-setup-done'"],label+'-setup-marker')
 checked('/usr/bin/python3',['-c',"import os,signal; [os.kill(int(p),signal.SIGTERM) for p in os.listdir('/proc') if p.isdigit() and os.path.exists('/proc/'+p+'/exe') and os.stat('/proc/'+p).st_uid==1000 and os.readlink('/proc/'+p+'/exe')=='/usr/bin/cosmic-initial-setup']"],label+'-close-welcome')
 time.sleep(2);ui.shot(label+'-desktop')

def checkpoint(seq):
 collect();ui.shot(f'checkpoint-{seq}')
 (C/f'checkpoint-{seq}.json').write_text(json.dumps({'seq':seq}))
 end=time.monotonic()+1200
 while time.monotonic()<end:
  cmd=C/f'command-{seq+1}.json'
  if cmd.exists():return json.loads(cmd.read_text())
  time.sleep(2)
 raise RuntimeError('bounded operator checkpoint wait expired; no second allocation')

def main():
 global Q
 originals={name:sha(R/name) for name in pins()};assert originals==pins()
 T.mkdir();shutil.copyfile(R/'cosmic-vars.fd',T/'vars.fd');disk=T/'overlay.qcow2'
 subprocess.run([str(W/'tools/usr/bin/qemu-img'),'create','-f','qcow2','-F','qcow2','-b',str(R/'cosmic-1.11-1783-r1.qcow2'),str(disk)],check=True)
 p=log=None;failure=None
 try:
  p,log=boot(T,disk,T/'vars.fd');Q=Channel(T/'qmp.sock');Q.f.readline();Q.call('qmp_capabilities');ui.Q=Q
  login('00')
  checked('/usr/bin/mkdir',['-p',H],'mkdir-guest')
  payload=W/'package-session-inputs.tar.gz'
  with tarfile.open(payload,'w:gz') as tar:
   for name in ['package-pins.json','test-input-packages','guest_ops.py','guest_session.py','capture_runtime.py','readonly_identity_followup.py']:tar.add(HERE/name,arcname=name)
   tar.add(R/'recipes/locks.json',arcname='locks.json')
  put(T,payload,H+'-inputs.tar.gz');assert checked('/usr/bin/sha256sum',[H+'-inputs.tar.gz'],'transport-sha').decode().split()[0]==sha(payload)
  checked('/bin/tar',['-xzf',H+'-inputs.tar.gz','-C',H],'unpack-inputs');checked('/bin/chmod',['-R','a+rX',H],'guest-read-access')
  ops(['initial'],'initial')
  ops(['install','cosmic-ext-halcyon_0.2.5~dev.integration.1_amd64.deb'],'install-main')
  # Explicit synthetic pre-rename config: no user data. Validate migration retains original.
  ops(['user','synthetic-legacy','/usr/bin/python3','-c',"import pathlib,json; p=pathlib.Path.home()/'.config/cosmic-desktop-cards'; p.mkdir(parents=True,exist_ok=True); (p/'config.json').write_text(json.dumps({'theme':'teal','enabled_widgets':['clock'],'acceptance_synthetic_marker':'legacy-preserve-r1'}))"],'synthetic-legacy')
  ops(['launch','settings-general','/usr/bin/halcyon','--settings','general'],'launch-settings')
  time.sleep(8);ui.shot('01-main-settings-general')
  ops(['snapshot','after-main-launch'],'main-snapshot')
  # Independent ordinary-user public metadata. No all-OS file inventory.
  ops(['user','identity-followup','/usr/bin/python3','-B',H+'/readonly_identity_followup.py'],'identity-followup')
  seq=0
  while seq<20:
   cmd=checkpoint(seq);assert cmd['seq']==seq+1;seq+=1
   (E/f'operator-command-{seq}.json').write_text(json.dumps(cmd,indent=2))
   for i,a in enumerate(cmd['actions']):
    label=f'phase-{seq}-{i}';op=a['op']
    if op=='click':ui.click(a['x'],a['y'])
    elif op=='key':ui.key(a['keys'])
    elif op=='text':ui.text(a['text'])
    elif op=='wait':time.sleep(min(a['seconds'],65))
    elif op=='shot':ui.shot(a['name'])
    elif op=='guest':checked(a['path'],a.get('args',[]),label,a.get('expected',0),a.get('timeout',120))
    elif op=='ops':ops(a['args'],label,a.get('expected',0))
    elif op=='drag':
     ui.qmp('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(a['x1']*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(a['y1']*32767/800)}},{'type':'btn','data':{'down':True,'button':'left'}}]});time.sleep(.3)
     for j in range(1,11):
      x=a['x1']+(a['x2']-a['x1'])*j/10;y=a['y1']+(a['y2']-a['y1'])*j/10;ui.qmp('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(x*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(y*32767/800)}}]});time.sleep(.08)
     ui.qmp('input-send-event',{'events':[{'type':'btn','data':{'down':False,'button':'left'}}]})
    elif op=='relogin':
     checked('/usr/bin/loginctl',['terminate-user','probe'],label);time.sleep(10);login('relogin')
    elif op=='finish':
     collect();return
    else:raise ValueError('unknown action '+op)
  raise RuntimeError('phase budget exhausted')
 except BaseException as e:
  failure=str(e);(E/'failure.json').write_text(json.dumps({'error':str(e),'traceback':traceback.format_exc(),'qualified':False},indent=2))
  try:collect();ui.shot('failure-screen')
  except BaseException as extra:(E/'collection-error.txt').write_text(str(extra))
  raise
 finally:
  if p is not None:stop(p,log,T)
  if Q:Q.close()
  shutil.copytree(T/'evidence',E/'vm',dirs_exist_ok=True)
  unchanged={name:sha(R/name) for name in pins()}==originals
  (E/'session-receipt.json').write_text(json.dumps({'prepared_artifact':12535269783,'originals_unchanged':unchanged,'qualified':False,'publication':'ON HOLD','failure':failure},indent=2));assert unchanged
  (C/'bridge-stop').touch()
if __name__=='__main__':main()
