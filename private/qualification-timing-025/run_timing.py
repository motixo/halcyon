"""One offline VM, real stock input, bounded timing probes; never qualification."""
import base64,hashlib,json,os,pathlib,shutil,socket,subprocess,tarfile,time,traceback
from common import HERE,sha,pins
from vm import W,E,R,Channel,guest,put,boot,stop
from PIL import Image
T=W/'trial';Q=None

def record(event,**kw):
 with (E/'ui-timeline.jsonl').open('a') as f:f.write(json.dumps(dict(event=event,unix_time=time.time(),monotonic=time.monotonic(),**kw))+'\n')
def qmp(cmd,args=None):return Q.call(cmd,args)
def shot(name):
 path=E/(name+'.ppm');qmp('screendump',{'filename':str(path)});Image.open(path).save(E/(name+'.png'));path.unlink();record('screenshot',name=name)
def key(keys,delay=.12):
 record('key',keys=keys);qmp('human-monitor-command',{'command-line':'sendkey '+keys});time.sleep(delay)
def text(s):
 for ch in s:key({'-':'minus',' ':'spc'}.get(ch,ch))
def click(x,y):
 record('pointer',x=x,y=y);qmp('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(x*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(y*32767/800)}},{'type':'btn','data':{'down':True,'button':'left'}}]});time.sleep(.08);qmp('input-send-event',{'events':[{'type':'btn','data':{'down':False,'button':'left'}}]})
def exec_ok(path,args,timeout=60):
 rc,out,err=guest(T,path,args,timeout)
 if rc:raise RuntimeError('guest command failed: '+path+' '+err.decode(errors='replace')[-1000:])
 return out
def interaction(label):
 shot(label+'-before');click(640,16);time.sleep(.25);shot(label+'-clock-250ms');time.sleep(.75);shot(label+'-clock-1s');click(960,650);time.sleep(.3)
 key('meta_l');time.sleep(.3);text('terminal');time.sleep(.25);shot(label+'-typed-250ms');time.sleep(.75);shot(label+'-typed-1s');key('esc');time.sleep(.3)
def getfile(src,dst):
 c=Channel(T/'agent.sock');h=c.call('guest-file-open',{'path':src,'mode':'r'});n=0
 try:
  with dst.open('xb') as f:
   while True:
    r=c.call('guest-file-read',{'handle':h,'count':32768});b=base64.b64decode(r.get('buf-b64',''));n+=len(b)
    if n>150*1024**2:raise RuntimeError('guest evidence size bound')
    f.write(b)
    if r.get('eof'):break
 finally:c.call('guest-file-close',{'handle':h});c.close()
 return n
def main():
 global Q
 start=time.monotonic();original={name:sha(R/name) for name in pins()};assert original==pins()
 T.mkdir();shutil.copyfile(R/'cosmic-vars.fd',T/'vars.fd')
 disk=T/'overlay.qcow2';qimg=str(W/'tools/usr/bin/qemu-img')
 subprocess.run([qimg,'create','-f','qcow2','-F','qcow2','-b',str(R/'cosmic-1.11-1783-r1.qcow2'),str(disk)],check=True)
 p,log=boot(T,disk,T/'vars.fd')
 try:
  Q=Channel(T/'qmp.sock');Q.f.readline();qmp('qmp_capabilities')
  # Capture greeter then log in with the immutable disposable fixture account.
  time.sleep(20);shot('00-greeter');key('ret');time.sleep(2);text('cosmic-probe');key('ret');login_start=time.monotonic()
  for _ in range(180):
   rc,out,err=guest(T,'/usr/bin/pgrep',['-u','1000','-ax','cosmic-panel'],10)
   if rc==0:break
   time.sleep(.5)
  else:raise RuntimeError('real stock panel did not start')
  record('first-panel-process',seconds_since_password_enter=time.monotonic()-login_start);time.sleep(12)
  exec_ok('/bin/sh',['-c',"runuser -u probe -- sh -c 'mkdir -p $HOME/.config; touch $HOME/.config/cosmic-initial-setup-done'"])
  exec_ok('/usr/bin/python3',['-c',"import os,signal; [os.kill(int(p),signal.SIGTERM) for p in os.listdir('/proc') if p.isdigit() and os.path.exists('/proc/'+p+'/exe') and os.stat('/proc/'+p).st_uid==1000 and os.readlink('/proc/'+p+'/exe')=='/usr/bin/cosmic-initial-setup']"])
  time.sleep(2)
  shot('01-stock-desktop');interaction('02-idle')
  # Real stock COSMIC Settings, via launcher and keyboard, not Halcyon Settings.
  key('meta_l');time.sleep(.4);text('settings');time.sleep(.4);shot('03-settings-search');key('ret');time.sleep(.25);shot('03-settings-250ms');time.sleep(.75);shot('03-settings-1s');time.sleep(2);shot('03-settings-3s')
  # Close only the foreground settings window via actual input before probes.
  key('alt-f4');time.sleep(1)
  exec_ok('/usr/bin/mkdir',['-p','/var/lib/qtime'])
  payload=W/'guest-inputs.tar.gz'
  with tarfile.open(payload,'w:gz') as tar:
   for name in ['input-pins.json','cohort.json.gz','build_fixture.py','measure.py','guest_driver.py']:
    tar.add(HERE/name,arcname=name)
   tar.add(HERE/'frozen',arcname='frozen');tar.add(R/'recipes/locks.json',arcname='locks.json')
  put(T,payload,'/var/lib/qtime-inputs.tar.gz')
  verify="import hashlib; assert hashlib.file_digest(open('/var/lib/qtime-inputs.tar.gz','rb'),'sha256').hexdigest()=="+repr(sha(payload))
  exec_ok('/usr/bin/python3',['-c',verify]);exec_ok('/bin/tar',['-xzf','/var/lib/qtime-inputs.tar.gz','-C','/var/lib/qtime'])
  exec_ok('/bin/chmod',['-R','a+rX','/var/lib/qtime'])
  c=Channel(T/'agent.sock');pid=c.call('guest-exec',{'path':'/bin/sh','arg':['-c','exec /usr/bin/python3 -B /var/lib/qtime/guest_driver.py > /var/lib/qtime/driver.stdout 2> /var/lib/qtime/driver.stderr'],'capture-output':False})['pid'];c.close()
  deadline=time.monotonic()+3000;under_load=False;done=None
  while time.monotonic()<deadline:
   c=Channel(T/'agent.sock');s=c.call('guest-exec-status',{'pid':pid});c.close()
   if s.get('exited'):done=s;break
   if not under_load:
    rc,out,err=guest(T,'/usr/bin/tail',['-n','1','/var/lib/qtime/results/timeline.jsonl'],10)
    if rc==0:
     event=json.loads(out)
     if event.get('event')=='start' and event.get('name')=='audit-0':
      record('audit-running-start');interaction('04-during-full-audit');record('audit-running-ui-finished');under_load=True
   time.sleep(2)
  if done is None:raise RuntimeError('guest harness wall deadline; no additional allocation')
  record('guest-harness-exited',status=done,under_load_observed=under_load)
  # Keep native/fixture errors and output small and independently checksummed.
  exec_ok('/bin/tar',['-czf','/var/lib/qtime-results.tar.gz','-C','/var/lib/qtime','results','driver.stdout','driver.stderr'])
  getfile('/var/lib/qtime-results.tar.gz',E/'guest-results.tar.gz')
  remote=exec_ok('/usr/bin/sha256sum',['/var/lib/qtime-results.tar.gz']).decode().split()[0];assert sha(E/'guest-results.tar.gz')==remote
  with tarfile.open(E/'guest-results.tar.gz') as tar:
   members=tar.getmembers();assert sum(m.size for m in members)<150*1024**2
   for m in members:
    name=pathlib.PurePosixPath(m.name);assert not name.is_absolute() and '..' not in name.parts and (m.isfile() or m.isdir())
   tar.extractall(E/'guest-results',filter='data')
  shot('05-after-measurements');interaction('06-after')
  observations={}
  for label,args in {'session':['loginctl','list-sessions'],'processes':['ps','-eo','user,pid,ppid,comm'],'system':['uname','-a'],'audit':['dpkg','--audit']}.items():
   rc,out,err=guest(T,'/usr/bin/'+args[0],args[1:],30);(E/(label+'.txt')).write_bytes(out+err)
  if done.get('exitcode')!=0:raise RuntimeError('measurement failed; inspect preserved guest-results/failure.json')
 finally:
  stop(p,log,T)
  if Q:Q.close()
  shutil.copytree(T/'evidence',E/'vm',dirs_exist_ok=True)
 assert {name:sha(R/name) for name in pins()}==original
 (E/'timing-receipt.json').write_text(json.dumps(dict(prepared_artifact=12535269783,prepared_original_all_files_unchanged=True,input_payload_sha256=sha(payload),guest_results_sha256=sha(E/'guest-results.tar.gz'),seconds=time.monotonic()-start,qualified=False,publication='ON HOLD',development_fixture_only=True,stock_input_under_audit=under_load),indent=2)+'\n')
if __name__=='__main__':
 try:main()
 except Exception as e:
  (E/'failure.json').write_text(json.dumps(dict(error=str(e),traceback=traceback.format_exc(),qualified=False),indent=2)+'\n');traceback.print_exc();raise SystemExit(1)
