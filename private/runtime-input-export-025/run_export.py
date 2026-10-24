"""One isolated offline session; exact bounded objects and normal theme/close only."""
import os,pathlib,json,time,subprocess,shutil,tarfile,traceback,hashlib,base64,gzip
import ui
from vm import W,E,R,Channel,guest,put,boot,stop
from common import HERE,sha,pins
T=ui.T;H='/var/lib/ps025';Q=None

def checked(path,args,label,expected=0,timeout=120):
 start=time.monotonic();rc,out,err=guest(T,path,args,timeout);(E/(label+'.stdout')).write_bytes(out);(E/(label+'.stderr')).write_bytes(err);(E/(label+'.receipt.json')).write_text(json.dumps({'returncode':rc,'expected':expected,'seconds':time.monotonic()-start,'path':path,'args':args},indent=2)+'\n');assert rc==expected,f'{label}: guest rc {rc}, expected {expected}';return out

def ops(args,label):return checked('/usr/bin/python3',['-B',H+'/guest_ops.py',*args],label)
def user(args,label):return ops(['user',label,*args],label)
def getfile(src,dst,cap):
 c=Channel(T/'agent.sock');h=c.call('guest-file-open',{'path':src,'mode':'r'});n=0
 try:
  with dst.open('xb') as f:
   while True:
    r=c.call('guest-file-read',{'handle':h,'count':1024*1024});b=base64.b64decode(r.get('buf-b64',''));n+=len(b);assert n<=cap;f.write(b)
    if r.get('eof'):break
 finally:c.call('guest-file-close',{'handle':h});c.close()
 assert sha(dst)==checked('/usr/bin/sha256sum',[src],dst.stem+'-hash').decode().split()[0]
def collect():
 checked('/bin/tar',['-czf',H+'-results.tar.gz','-C',H,'results'],'collect-results');dst=E/'guest-results.tar.gz'
 if dst.exists():dst.unlink()
 getfile(H+'-results.tar.gz',dst,25*1024**2)
def scroll(n):
 for _ in range(n):ui.qmp('input-send-event',{'events':[{'type':'btn','data':{'down':True,'button':'wheel-down'}},{'type':'btn','data':{'down':False,'button':'wheel-down'}}]});time.sleep(.1)
def capture(label):user(['/usr/bin/python3','-B',H+'/capture_metadata.py',label],'capture-'+label)
def wait_close(label):
 for _ in range(30):
  rc,o,e=guest(T,'/usr/bin/test',['-f',H+'/results/'+label+'.exit.json'],10)
  if rc==0:break
  time.sleep(1)
 else:raise RuntimeError('normal Settings close not observed: '+label)
 out=checked('/usr/bin/cat',[H+'/results/'+label+'.exit.json'],label+'-exit');assert json.loads(out)['returncode']==0

def main():
 global Q
 originals={n:sha(R/n) for n in pins()};assert originals==pins();T.mkdir();shutil.copyfile(R/'cosmic-vars.fd',T/'vars.fd');disk=T/'overlay.qcow2';subprocess.run([str(W/'tools/usr/bin/qemu-img'),'create','-f','qcow2','-F','qcow2','-b',str(R/'cosmic-1.11-1783-r1.qcow2'),str(disk)],check=True)
 p=log=None;failure=None
 try:
  p,log=boot(T,disk,T/'vars.fd');Q=Channel(T/'qmp.sock');Q.f.readline();Q.call('qmp_capabilities');ui.Q=Q
  time.sleep(10);ui.shot('00-greeter');ui.key('ret');time.sleep(2);ui.text('cosmic-probe');ui.key('ret')
  for _ in range(120):
   rc,o,e=guest(T,'/usr/bin/pgrep',['-u','1000','-ax','cosmic-panel'],10)
   if rc==0:break
   time.sleep(.5)
  else:raise RuntimeError('stock panel startup absent')
  time.sleep(12)
  checked('/bin/sh',['-c',"runuser -u probe -- sh -c 'mkdir -p $HOME/.config; touch $HOME/.config/cosmic-initial-setup-done'"],'setup-marker')
  checked('/usr/bin/python3',['-c',"import os,signal; [os.kill(int(p),signal.SIGTERM) for p in os.listdir('/proc') if p.isdigit() and os.path.exists('/proc/'+p+'/exe') and os.stat('/proc/'+p).st_uid==1000 and os.readlink('/proc/'+p+'/exe')=='/usr/bin/cosmic-initial-setup']"],'close-welcome');time.sleep(2);ui.shot('01-stock')
  checked('/usr/bin/mkdir',['-p',H],'mkdir')
  payload=W/'export-inputs.tar.gz'
  with tarfile.open(payload,'w:gz') as tar:
   for n in ['guest_ops.py','guest_session.py','package-pins.json','capture_metadata.py','watch_settings.py','capture_missing_public_objects.py','public-object-receipt.json','native-key-allowlist.json','loader-alias-allowlist.json','test-input-pins.json']:tar.add(HERE/n,arcname=n)
   tar.add(HERE.parent/'package-session-025/test-input-packages',arcname='test-input-packages');tar.add(HERE/'test-inputs',arcname='test-inputs');tar.add(R/'recipes/locks.json',arcname='locks.json')
  put(T,payload,H+'-inputs.tar.gz');assert checked('/usr/bin/sha256sum',[H+'-inputs.tar.gz'],'input-transport-hash').decode().split()[0]==sha(payload)
  checked('/bin/tar',['-xzf',H+'-inputs.tar.gz','-C',H],'unpack');checked('/bin/chmod',['-R','a+rX',H],'readable-inputs');checked('/bin/mkdir',['-p',H+'/results'],'results-dir');checked('/bin/chown',['probe:probe',H+'/results'],'results-owner')
  ops(['initial'],'initial')
  # Export only exact owner allowlist. Preserve the owner's script without changes.
  checked('/bin/mkdir',['-p','/mnt/public-export'],'mountpoint')
  checked('/bin/mount',['-t','9p','-o','trans=virtio,version=9p2000.L,msize=1048576','public_export','/mnt/public-export'],'mount-scoped-public-export')
  checked('/usr/bin/df',['-B1','/mnt/public-export'],'guest-export-storage')
  user(['/usr/bin/timeout','120','/usr/bin/python3','-B',H+'/capture_missing_public_objects.py','--root','/','--out','/mnt/public-export/public-objects'],'public-object-export')
  export=W/'public-export/public-objects';receipt=json.loads((export/'receipt.json').read_text());assert receipt['bytes']==186442704 and len(receipt['files'])==44
  expected=json.loads((HERE/'public-object-receipt.json').read_text())['missing'];assert set(receipt['files'])==set(expected)
  for name,f in expected.items():
   obj=export/'objects'/name.lstrip('/');assert obj.stat().st_size==f['size'] and sha(obj)==f['sha256']
  with tarfile.open(E/'public-objects.tar.gz','w:gz') as tar:
   tar.add(export/'receipt.json',arcname='receipt.json')
   for name in sorted(expected):tar.add(export/'objects'/name.lstrip('/'),arcname='objects/'+name.lstrip('/'),recursive=False)
  (E/'public-export-archive.json').write_text(json.dumps({'sha256':sha(E/'public-objects.tar.gz'),'size':(E/'public-objects.tar.gz').stat().st_size,'count':44,'object_bytes':186442704,'independent_host_rehash':True},indent=2)+'\n')
  # Same frozen packages; fresh main remains a separately named test executable.
  ops(['install','cosmic-ext-halcyon_0.2.5~dev.integration.1_amd64.deb'],'install-main');ops(['install','cosmic-ext-halcyon-islands_0.2.5~dev.integration.1_amd64.deb'],'install-companion')
  user(['/usr/bin/python3','-c',"import pathlib,json; p=pathlib.Path.home()/'.config/cosmic-desktop-cards';p.mkdir(parents=True,exist_ok=True);(p/'config.json').write_text(json.dumps({'theme':'teal','enabled_widgets':['clock'],'acceptance_synthetic_marker':'legacy-preserve-r1'}))"],'synthetic-prefs')
  capture('before')
  checked('/usr/bin/python3',['-B',H+'/watch_settings.py','start','original-settings','/usr/bin/halcyon'],'launch-original');time.sleep(8);ui.shot('02-original-settings')
  ui.click(317,207);time.sleep(2);ui.shot('03-theme-before');ui.click(967,439);time.sleep(1);ui.shot('04-theme-choices');ui.click(884,484);time.sleep(1);scroll(6);time.sleep(1);ui.shot('05-apply-controls');ui.click(994,651);time.sleep(3);ui.shot('06-applied');capture('applied');user(['/usr/bin/python3','-c',"import json,pathlib;assert json.loads((pathlib.Path.home()/'.config/halcyon/config.json').read_text())['appearance']['family']=='outline'"],'assert-outline')
  ui.click(945,653);time.sleep(3);ui.shot('07-undo');capture('after-undo');user(['/usr/bin/python3','-c',"import json,pathlib;v=json.loads((pathlib.Path.home()/'.config/halcyon/config.json').read_text());assert 'appearance' not in v and v['acceptance_synthetic_marker']=='legacy-preserve-r1'"],'assert-undo')
  ui.click(1042,102);wait_close('original-settings');ui.shot('08-original-normal-close')
  code="import pathlib,json,gzip,hashlib;d=pathlib.Path('/var/lib/ps025');p=json.loads((d/'test-input-pins.json').read_text())['main_test_input'];s=d/'test-inputs'/p['file'];assert s.stat().st_size==p['compressed_size'] and hashlib.sha256(s.read_bytes()).hexdigest()==p['compressed_sha256'];b=gzip.decompress(s.read_bytes());assert len(b)==p['elf_size'] and hashlib.sha256(b).hexdigest()==p['elf_sha256'];o=d/'halcyon-main-8a3d9b2.test-input';o.write_bytes(b);o.chmod(0o555);(d/'results/fresh-main-input.json').write_text(json.dumps(p,indent=2))"
  checked('/usr/bin/python3',['-c',code],'unpack-fresh-main')
  checked('/usr/bin/python3',['-B',H+'/watch_settings.py','start','fresh-settings',H+'/halcyon-main-8a3d9b2.test-input'],'launch-fresh');time.sleep(8);ui.shot('09-fresh-main');ui.click(1042,102);wait_close('fresh-settings');ui.shot('10-fresh-normal-close')
  checked('/usr/bin/python3',['-c',"import pathlib,os,json;xs=[]\nfor p in pathlib.Path('/proc').iterdir():\n if p.name.isdigit():\n  try:\n   x=os.readlink(p/'exe')\n   if pathlib.Path(x).name in ['cosmic-panel','cosmic-panel-islands']:xs.append({'pid':int(p.name),'exe':x})\n  except OSError:pass\nassert any(x['exe']=='/usr/bin/cosmic-panel' for x in xs);assert not any(x['exe'].endswith('/cosmic-panel-islands') for x in xs);pathlib.Path('/var/lib/ps025/results/final-stock.json').write_text(json.dumps({'processes':xs,'qualified':False}))"],'final-stock')
  collect()
 except BaseException as e:
  failure=str(e);(E/'failure.json').write_text(json.dumps({'error':str(e),'traceback':traceback.format_exc(),'qualified':False},indent=2))
  try:collect();ui.shot('failure')
  except BaseException as extra:(E/'collection-error.txt').write_text(type(extra).__name__)
  raise
 finally:
  if p is not None:stop(p,log,T)
  if Q:Q.close()
  shutil.copytree(T/'evidence',E/'vm',dirs_exist_ok=True);unchanged={n:sha(R/n) for n in pins()}==originals;(E/'session-receipt.json').write_text(json.dumps({'prepared_artifact':12535269783,'originals_unchanged':unchanged,'failure':failure,'qualified':False,'publication':'ON HOLD'},indent=2));assert unchanged;(W/'control/bridge-stop').touch()
if __name__=='__main__':main()
