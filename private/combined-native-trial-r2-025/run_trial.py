"""Single combined disposable trial; no production admission or shared host mounts."""
import os,pathlib,json,time,subprocess,shutil,tarfile,traceback,hashlib,base64
import ui
from trial_receipts import device_observation, FORMAT
from vm import W,E,R,Channel,guest,put,boot,stop
from common import HERE,sha,pins
T=ui.T;H='/var/lib/ps025';Q=None;failures=[];shot_count=0
FILTER='warn,smithay::backend::renderer::gles=info,smithay::backend::egl=info'
def shot(n):
 global shot_count
 if shot_count>=24:raise RuntimeError('screenshot limit')
 ui.shot(n);shot_count+=1

def checked(path,args,label,expected=0,timeout=120):
 print('CHECKPOINT '+label,flush=True);start=time.monotonic();rc,out,err=guest(T,path,args,timeout)
 (E/(label+'.stdout')).write_bytes(out);(E/(label+'.stderr')).write_bytes(err);(E/(label+'.receipt.json')).write_text(json.dumps({'returncode':rc,'expected':expected,'seconds':time.monotonic()-start,'path':path,'args':args},indent=2)+'\n')
 if expected is not None and rc!=expected:raise RuntimeError(f'{label}: rc {rc}, expected {expected}')
 return out

def attempt(label,fn):
 try:return fn()
 except Exception as ex:
  failures.append({'checkpoint':label,'error':str(ex),'traceback':traceback.format_exc()});(E/'checkpoint-failures.json').write_text(json.dumps(failures,indent=2)+'\n');print('FAILED CHECKPOINT '+label+': '+str(ex),flush=True);return None

def ops(args,label):return checked('/usr/bin/python3',['-B',H+'/guest_ops.py',*args],label)
def user(args,label):return ops(['user',label,*args],label)
def py(code,label):return checked('/usr/bin/python3',['-c',code],label)
def capture(label):return user(['/usr/bin/timeout','90','/usr/bin/python3','-B',H+'/capture_trial.py',label],'capture-'+label)
def drm(label):
 raw=user(['/usr/bin/timeout','3','/usr/bin/python3','-B',H+'/capture_drm.py','--helpers','/usr/lib/cosmic-ext-halcyon-islands'],'drm-'+label)
 pins=json.loads((HERE/'integration2.json').read_text())['helper_hashes']
 expected={k:pins[k] for k in ('runtime_identity.py','bundle_contract.py','verification.py')}
 observation=device_observation(raw,expected)
 path=E/('observation.drm-'+label+'.json')
 with path.open('x') as f:json.dump(observation,f,indent=2);f.write('\n')
 return observation
def probe_outputs(label):
 name='output-names-'+label
 raw=checked('/usr/bin/python3',['-B',H+'/guest_ops.py','user',name,'/usr/bin/timeout','3','/usr/bin/python3','-B',H+'/probe_output_names.py','--helpers','/usr/lib/cosmic-ext-halcyon-islands'],name,expected=None)
 envelope=json.loads(raw)
 if envelope.get('format')!=FORMAT or envelope.get('version')!=1:raise ValueError('output_probe_receipt_format')
 observation=json.loads(envelope['stdout'])
 pins=json.loads((HERE/'integration2.json').read_text())['helper_hashes']
 expected={k:pins[k] for k in ('panel_configuration.py','bundle_contract.py')}
 if observation.get('helper_sha256')!=expected:raise ValueError('output_probe_helper_identity')
 if observation.get('qualified') is not False:raise ValueError('output_probe_admission_invalid')
 path=E/('observation.output-names-'+label+'.json')
 with path.open('x') as f:json.dump(observation,f,indent=2);f.write('\n')
 if envelope.get('returncode')!=0 or observation.get('state')!='observed-not-admission':raise ValueError('output_probe_refused')
 if not isinstance(observation.get('output_names'),list) or not observation['output_names']:raise ValueError('output_names_missing')
 return observation
def getfile(src,dst,cap):
 c=Channel(T/'agent.sock');handle=c.call('guest-file-open',{'path':src,'mode':'r'});n=0
 try:
  with dst.open('xb') as f:
   while True:
    r=c.call('guest-file-read',{'handle':handle,'count':1048576});b=base64.b64decode(r.get('buf-b64',''));n+=len(b);assert n<=cap;f.write(b)
    if r.get('eof'):break
 finally:c.call('guest-file-close',{'handle':handle});c.close()
 assert sha(dst)==checked('/usr/bin/sha256sum',[src],dst.name+'-hash').decode().split()[0]
def collect():
 listing=json.loads(py("import pathlib,json;d=pathlib.Path('/var/lib/ps025/results');print(json.dumps([{'name':p.name,'size':p.stat().st_size} for p in d.iterdir() if p.is_file()]))",'result-list'))
 assert sum(x['size'] for x in listing)<=600*1024**2
 dest=E/'guest-results';dest.mkdir(exist_ok=True)
 for x in listing:
  assert '/' not in x['name'] and x['size']<=64*1024**2
  if not (dest/x['name']).exists():getfile(H+'/results/'+x['name'],dest/x['name'],64*1024**2)

def login(label,trial=False):
 shot(label+'-greeter');ui.key('ret');time.sleep(2);ui.text('cosmic-probe');ui.key('ret')
 for _ in range(100):
  rc,o,e=guest(T,'/usr/bin/pgrep',['-u','1000','-x','cosmic-session'],10)
  if rc==0:
   rc,o,e=guest(T,'/usr/bin/test',['-S','/run/user/1000/wayland-1'],5)
   # Actual display number is discovered by guest_session, no fallback assertion.
   try:
    out=guest(T,'/usr/bin/python3',['-B',H+'/guest_ops.py','user','session-check','/usr/bin/true'],10)
    if out[0]==0:break
   except Exception:pass
  time.sleep(.25)
 else:raise RuntimeError('ordinary login/session deadline')
 if trial:user(['/usr/bin/python3','-B',H+'/probe_ready.py'],'trial-readiness')
 else:
  for _ in range(60):
   rc,_,_=guest(T,'/usr/bin/pgrep',['-u','1000','-x','cosmic-panel'],5)
   if rc==0:break
   time.sleep(.5)
  else:raise RuntimeError('stock absent')
 time.sleep(3);shot(label+'-desktop')
def logout(label):
 user(['/usr/bin/gdbus','call','--session','--dest','com.system76.CosmicSession','--object-path','/com/system76/CosmicSession','--method','com.system76.CosmicSession.Exit'],label)
 for _ in range(80):
  rc,_,_=guest(T,'/usr/bin/pgrep',['-u','1000','-x','cosmic-session'],5)
  if rc!=0:break
  time.sleep(.25)
 else:raise RuntimeError('ordinary logout deadline')
 time.sleep(4)
def popup(label):
 shot(label+'-before');ui.click(640,16);time.sleep(.3);shot(label+'-popup-300ms');time.sleep(.7);shot(label+'-popup-1s');ui.key('esc');ui.click(980,680);time.sleep(.4)
def editor(label):
 path=H+'/results/'+label+'-typed.txt'
 user(['/bin/sh','-c','printf "seed\\n" > '+path],label+'-seed')
 ops(['launch',label+'-editor','/usr/bin/cosmic-edit',path],label+'-editor-launch');time.sleep(4)
 shot(label+'-editor-before');ui.click(500,280);ui.key('ctrl-a');ui.text('native-inputx');ui.key('backspace');ui.key('ctrl-s');time.sleep(.6);shot(label+'-editor-after')
 raw=checked('/usr/bin/cat',[path],label+'-typed');assert raw.strip()==b'native-input',f'{label}: editor input/save mismatch'
 ui.key('alt-f4');time.sleep(1)
def logs():
 for label,exe in [('compositor','/usr/bin/cosmic-comp'),('companion','/usr/lib/cosmic-ext-halcyon-islands/cosmic-panel-islands')]:
  checked('/bin/sh',['-c',"journalctl -b --no-pager -o json --output-fields=MESSAGE,_PID,_EXE,_BOOT_ID,__MONOTONIC_TIMESTAMP,_COMM -n 2500 _UID=1000 _EXE="+exe+" > "+H+'/results/'+label+'-own-context.journal.jsonl'],label+'-journal')
  py("import pathlib;p=pathlib.Path('"+H+'/results/'+label+"-own-context.journal.jsonl');assert p.stat().st_size<=8*1024**2",label+'-journal-bound')
def cleanup():
 # Remove only exact task-owned entry scripts, never unrelated files.
 py("import pathlib,json,hashlib;d=pathlib.Path('/var/lib/ps025');r=json.loads((d/'owned-session-files.json').read_text());\nfor name,h in r.items():\n p=pathlib.Path(name)\n if p.exists():\n  assert hashlib.sha256(p.read_bytes()).hexdigest()==h;p.unlink()",'remove-owned-shims')
 attempt('clear-session-logging',lambda:user(['/usr/bin/systemctl','--user','unset-environment','RUST_LOG'],'clear-session-logging'))

def recover_stock():
  # Exact captured host identity, not killall or a compositor/user termination.
  py("import pathlib,json,os,signal;d=pathlib.Path('/var/lib/ps025/results');r=json.loads((d/'readiness.json').read_text());p=pathlib.Path('/proc')/str(r['pid']);assert p.resolve().is_dir();assert (p/'stat').read_text().rsplit(')',1)[1].split()[19]==r['start_ticks'];assert (p/'exe').resolve()==pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands/cosmic-panel-islands');os.kill(r['pid'],signal.SIGTERM)",'controlled-host-term')
  for _ in range(80):
   rc,_,_=guest(T,'/usr/bin/pgrep',['-u','1000','-x','cosmic-panel'],5)
   if rc==0:break
   time.sleep(.25)
  else:raise RuntimeError('stock restart absent after controlled host exit')
  time.sleep(3);attempt('owned-children-after',lambda:user(['/usr/bin/python3','-B',H+'/observe_lifecycle.py','after'],'owned-children-after'));attempt('recovered-metadata',lambda:capture('recovered'));attempt('recovered-popup',lambda:popup('06-recovered'))
  attempt('stock-notification',lambda:user(['/usr/bin/gdbus','call','--session','--dest','org.freedesktop.Notifications','--object-path','/org/freedesktop/Notifications','--method','org.freedesktop.Notifications.Notify','HalcyonTrial','0','','Stock recovery','Offline native trial','[]','{}','5000'],'stock-notification'));time.sleep(.5);attempt('stock-notification-shot',lambda:shot('07-stock-notification'))
  attempt('recovered-editor-input',lambda:editor('08-recovered'))
  attempt('normal-cleanup',cleanup);logout('ordinary-final-logout');login('09-final-stock');attempt('final-metadata',lambda:capture('final'));attempt('final-logs',logs);attempt('final-collect',collect)

def exercise_trial():
 try:
   attempt('device-after',lambda:drm('after'));attempt('output-names-trial',lambda:probe_outputs('trial'));attempt('trial-metadata',lambda:capture('trial'));attempt('trial-logs',logs);attempt('owned-children-before',lambda:user(['/usr/bin/python3','-B',H+'/observe_lifecycle.py','before'],'owned-children-before'));attempt('trial-popup',lambda:popup('04-trial'));attempt('trial-editor-input',lambda:editor('05-trial'))
 finally:
  try:recover_stock()
  finally:attempt('trial-finally-cleanup',cleanup)

def main():
 global Q
 originals={n:sha(R/n) for n in pins()};assert originals==pins();T.mkdir();shutil.copyfile(R/'cosmic-vars.fd',T/'vars.fd');disk=T/'overlay.qcow2';subprocess.run([str(W/'tools/usr/bin/qemu-img'),'create','-f','qcow2','-F','qcow2','-b',str(R/'cosmic-1.11-1783-r1.qcow2'),str(disk)],check=True)
 p=log=None;installed=False;owned=False
 try:
  p,log=boot(T,disk,T/'vars.fd');Q=Channel(T/'qmp.sock');Q.f.readline();Q.call('qmp_capabilities');ui.Q=Q
  checked('/usr/bin/mkdir',['-p',H],'mkdir')
  payload=W/'trial-inputs.tar.gz'
  with tarfile.open(payload,'w:gz') as tar:
   for n in ['guest_ops.py','trial_receipts.py','guest_session.py','package-pins.json','test-input-packages','integration2.json','capture_drm.py','probe_output_names.py','observe_lifecycle.py','capture_layers.py','remaining-layer-candidates.json','capture_trial.py','trial_entry.py','probe_ready.py','provider-draft.json']:tar.add(HERE/n,arcname=n)
   tar.add(R/'recipes/locks.json',arcname='locks.json')
  put(T,payload,H+'-inputs.tar.gz');assert checked('/usr/bin/sha256sum',[H+'-inputs.tar.gz'],'input-transport-hash').decode().split()[0]==sha(payload)
  checked('/bin/tar',['-xzf',H+'-inputs.tar.gz','-C',H],'unpack');checked('/bin/chmod',['-R','a+rX',H],'readable');checked('/bin/mkdir',['-p',H+'/results'],'results');checked('/bin/chown',['probe:probe',H,H+'/results'],'disposable-state-owner');ops(['initial'],'initial')
  package_inputs=json.loads((HERE/'package-pins.json').read_text())
  main_packages=[n for n in package_inputs if n.startswith('cosmic-ext-halcyon_') and n.endswith('_amd64.deb')]
  islands_packages=[n for n in package_inputs if n.startswith('cosmic-ext-halcyon-islands_') and n.endswith('_amd64.deb')]
  if len(main_packages)!=1 or len(islands_packages)!=1 or len(package_inputs)!=2:raise ValueError('exact_two_package_inputs_required')
  ops(['install',main_packages[0]],'install-main');ops(['install',islands_packages[0]],'install-companion');installed=True
  # Set targeted log filter before the first actual desktop compositor starts.
  py("import pathlib; p=pathlib.Path('/etc/environment');raw=p.read_text(); assert 'RUST_LOG=' not in raw;pathlib.Path('/var/lib/ps025/environment-original').write_text(raw);p.write_text(raw+'\\nRUST_LOG=\""+FILTER+"\"\\n')",'disposable-log-selector')
  checked('/bin/sh',['-c',"runuser -u probe -- sh -c 'mkdir -p $HOME/.config; touch $HOME/.config/cosmic-initial-setup-done'"],'setup-marker')
  login('01-stock');attempt('close-welcome',lambda:py("import os,signal; [os.kill(int(p),signal.SIGTERM) for p in os.listdir('/proc') if p.isdigit() and os.path.exists('/proc/'+p+'/exe') and os.stat('/proc/'+p).st_uid==1000 and os.readlink('/proc/'+p+'/exe')=='/usr/bin/cosmic-initial-setup']",'close-welcome'))
  attempt('stock-popup',lambda:popup('02-stock'))
  for i in range(4):
   attempt('normal-status-'+str(i),lambda i=i:user(['/usr/bin/timeout','6','/usr/bin/halcyon-islands','--json','status'],'normal-status-'+str(i)))
  attempt('output-names-before',lambda:probe_outputs('before'));attempt('stock-metadata',lambda:capture('before'));attempt('device-before',lambda:drm('before'))
  for i in range(4):attempt('draft-measurement-'+str(i),lambda i=i:user(['/usr/bin/timeout','5','/usr/bin/python3','-B',H+'/capture_trial.py','benchmark'],'draft-measurement-'+str(i)))
  # Only the five remaining layer DSOs; no old 19-object/source export.
  attempt('layer-candidate-export',lambda:user(['/usr/bin/timeout','90','/usr/bin/python3','-B',H+'/capture_layers.py'],'layer-candidate-export'))
  # A user-owned one-shot entry, never the packaged launcher or accepted selection.
  code="import pathlib,hashlib,json,os;d=pathlib.Path('/var/lib/ps025');base=pathlib.Path('/home/probe/.local/bin');base.mkdir(parents=True,exist_ok=True);p=base/'cosmic-panel';assert not p.exists();raw=b'#!/bin/sh\\nexec /usr/bin/python3 -B /var/lib/ps025/trial_entry.py\\n';p.write_bytes(raw);p.chmod(0o755);os.chown(p,1000,1000);(d/'owned-session-files.json').write_text(json.dumps({str(p):hashlib.sha256(raw).hexdigest()}));(d/'trial-ticket').touch();os.chown(d/'trial-ticket',1000,1000)"
  py(code,'stage-one-shot');owned=True
  user(['/usr/bin/systemctl','--user','set-environment','PATH=/home/probe/.local/bin:/usr/local/bin:/usr/bin:/bin','RUST_LOG='+FILTER],'set-owned-session-path')
  logout('ordinary-logout-for-trial');login('03-trial',trial=True)
  exercise_trial();owned=False
 except BaseException as e:
  failures.append({'checkpoint':'main','error':str(e),'traceback':traceback.format_exc()});(E/'checkpoint-failures.json').write_text(json.dumps(failures,indent=2)+'\n');print('TRIAL STOPPED '+str(e),flush=True)
  if owned:attempt('cleanup',cleanup)
  attempt('failure-screenshot',lambda:shot('failure'));attempt('failure-capture',lambda:capture('failure'));attempt('failure-logs',logs);attempt('failure-collect',collect)
 finally:
  if owned:attempt('finally-remove-owned-shims',cleanup)
  if p is not None:
   attempt('restore-guest-environment',lambda:py("import pathlib;p=pathlib.Path('/var/lib/ps025/environment-original');\nif p.exists():pathlib.Path('/etc/environment').write_bytes(p.read_bytes())",'restore-guest-environment'))
   stop(p,log,T)
  if Q:Q.close()
  shutil.copytree(T/'evidence',E/'vm',dirs_exist_ok=True);unchanged={n:sha(R/n) for n in pins()}==originals
  total=sum(x.stat().st_size for x in E.rglob('*') if x.is_file());assert total<=640*1024**2
  (E/'session-receipt.json').write_text(json.dumps({'prepared_artifact':12535269783,'originals_unchanged':unchanged,'failures':failures,'qualified':False,'publication':'ON HOLD','no_host_mounts':True,'nic':'none','screenshots':shot_count,'evidence_bytes':total},indent=2)+'\n');assert unchanged
 if failures:raise SystemExit(1)
if __name__=='__main__':main()
