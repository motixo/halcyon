#!/usr/bin/python3
"""Real native input/observable app save/render soak, never a fake health service."""
import base64,hashlib,json,os,pathlib,re,shlex,subprocess,sys,time,traceback
SRC=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(SRC.parent/'theme'))
import theme_session as h
from calibrated_controls import Pixels,validate_samples
import editor_pixels
c=h.c;ev=h.ev;work=c.work;cohort=sys.argv[3];stage=sys.argv[4]
state_path=work/'lockup-state.json'
state=json.loads(state_path.read_text()) if state_path.exists() else {'cohort':cohort,'checkpoints':[],'failures':[],'coverage':[],'count':0,'source_scope':'actual public v0.2.0 only; no development/fix2 package','hour_complete':False,'phase':'preflight','optional_failures':[],'optional_enabled':True}
h.count=state.get('count',0)
class CoreInputFailure(RuntimeError):pass
CAL=json.loads((SRC.parent/'theme/followup/control-calibration.json').read_text())

def save():
 state['count']=h.count;state_path.write_text(json.dumps(state,indent=2)+'\n');(ev/'lockup-results.json').write_text(json.dumps(state,indent=2)+'\n')
 progress=work/'progress';progress.mkdir(exist_ok=True);(progress/'lockup-results.json').write_text(json.dumps(state,indent=2)+'\n')
 h.count=state.get('count',0)
def command(label,cmd,user=False,required=True,timeout=60):return h.command(label,cmd,user,required,timeout)
def snap(label):
 name,path,words=h.screen(label);state['count']=h.count
 # Copy latest disposable guest screenshot for bounded progress uploads.
 import shutil;shutil.copy2(path,work/'progress/latest.png');save();return name,path,words

def install_guest_helpers():
 for rel in ['wayland_observer.py','resources.py','protocols/cosmic-toplevel-info-unstable-v1.xml','protocols/ext-foreign-toplevel-list-v1.xml']:
  data=(SRC/rel).read_bytes();target='/var/tmp/lockup-probe/'+rel
  command('helper-'+pathlib.Path(rel).stem,'mkdir -p '+shlex.quote(str(pathlib.PurePosixPath(target).parent))+' && echo '+shlex.quote(base64.b64encode(data).decode())+' | base64 -d > '+shlex.quote(target))
def observe(label):
 out=command(label,'systemd-run --user --quiet --collect --wait --pipe /usr/bin/python3 /var/tmp/lockup-probe/wayland_observer.py',True)
 result=json.loads(next(line for line in out.splitlines() if line.startswith('{')))
 return result

def editor_hits(r):return [w for w in r['windows'] if not w.get('closed') and ('cosmicedit' in w.get('app_id','').lower().replace('-','') or 'lockup-input' in w.get('title','').lower())]

def edit_window(label):
 try:
  r=observe(label);hits=editor_hits(r)
 except Exception as error:
  hits=[];state['coverage'].append({'observer_unqualified':{'label':label,'error':str(error)}})
 if len(hits)>1:raise RuntimeError('Multiple editor windows; preserve state rather than choose ambiguously')
 if len(hits)==1 and 'geometry' in hits[0]:
  hits[0]['geometry_source']='native protocol';return hits[0]
 name,path,words=snap(label+'-pixel-focus')
 g=editor_pixels.locate(path,words,identity_seen=len(hits)==1)
 w=hits[0] if hits else {'app_id':'com.system76.CosmicEdit','identity_source':'visible filename and actual process ELF'}
 w.update(geometry=g,geometry_source='screenshot calibration',calibration_frame=name)
 state['coverage'].append({'metadata_unqualified':{'label':label,'geometry':False,'state':w.get('states'),'frame':name}});save()
 return w

def center(w):
 g=w['geometry'];return (max(40,min(1240,g['x']+g['width']//2)),max(100,min(670,g['y']+g['height']//2)))
def launch(unit,binary,args):
 # Pinned cosmic-edit daemonizes before its UI exists. A Type=simple
 # transient service can kill the forked child when its parent exits.
 kind=' --property=Type=forking --property=GuessMainPID=yes' if binary=='/usr/bin/cosmic-edit' else ''
 command('launch-'+unit,'systemd-run --user --collect'+kind+' --unit='+unit+' '+shlex.join([binary,*args]),True);time.sleep(5)
def editor_open(seq):
 launch('lockup-edit-'+str(seq),'/usr/bin/cosmic-edit',['/home/probe/lockup-input.txt'])
 diagnostic=command('editor-unit-'+str(seq),'systemctl --user show lockup-edit-'+str(seq)+'.service -p Type -p MainPID -p Result -p ActiveState -p SubState; journalctl --user -b -u lockup-edit-'+str(seq)+'.service --no-pager',True,False)
 if seq==0 and not ('Type=forking' in diagnostic and 'ActiveState=active' in diagnostic and re.search(r'MainPID=[1-9][0-9]*',diagnostic)):raise RuntimeError('Native editor forking unit did not retain an active MainPID')
 return edit_window('editor-open-'+str(seq))

def pointer_move(x,y):
 c.q.call('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(x*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(y*32767/800)}}]})
def drag(start,end):
 pointer_move(*start);c.q.call('input-send-event',{'events':[{'type':'btn','data':{'button':'left','down':True}}]});time.sleep(.2)
 for i in range(1,11):pointer_move(round(start[0]+(end[0]-start[0])*i/10),round(start[1]+(end[1]-start[1])*i/10));time.sleep(.06)
 c.q.call('input-send-event',{'events':[{'type':'btn','data':{'button':'left','down':False}}]});time.sleep(1)

def input_probe(seq,tag='normal'):
 w=edit_window(f'{tag}-{seq}-before')
 focus_method='pointer'
 if 'states' in w and 2 not in w['states']:
  c.key('meta_l-tab');time.sleep(.6);w=edit_window(f'{tag}-{seq}-focus');focus_method='native switch then pointer'
 c.click(*center(w));time.sleep(.3)
 state['input_serial']=state.get('input_serial',0)+1
 token=f'p{cohort}{state["input_serial"]:06d}'
 before_name,before_path,_=snap(f'{tag}-{seq}-before-input')
 c.key('ctrl-a');c.text(token);c.key('ctrl-s');time.sleep(1)
 deadline=time.monotonic()+5;attempt=0
 while True:
  content=command(f'{tag}-{seq}-saved-file-{attempt}','cat /home/probe/lockup-input.txt')
  if content.strip()==token or time.monotonic()>=deadline:break
  attempt+=1;time.sleep(.5)
 name,path,words=snap(f'{tag}-{seq}-input')
 active=edit_window(f'{tag}-{seq}-after')
 observed=''.join(re.sub('[^a-z0-9]','',w['text'].lower()) for w in words)
 # Compare the real window interior, independently of file contents and OCR.
 from PIL import Image,ImageChops
 g=active['geometry'];box=(max(0,g['x']+8),max(0,g['y']+75),min(1280,g['x']+g['width']-8),min(800,g['y']+g['height']-8))
 diff=ImageChops.difference(Image.open(before_path).convert('RGB').crop(box),Image.open(path).convert('RGB').crop(box))
 changed=sum(1 for px in diff.getdata() if max(px)>15)
 proof={'token':token,'before_screenshot':before_name,'window_interior_changed_pixels':changed,'saved_exact':content.strip()==token,'focus_method':focus_method,'native_editor':active,'screenshot':name,'token_ocr_exact':token in observed,'phase':state['phase'],'elapsed':time.monotonic()-state.get('start_monotonic',state['setup_monotonic'])}
 state['coverage'].append({'input':proof});save()
 # Exact freshly changed saved token proves delivery even when optional
 # activation metadata is unavailable. Preserve metadata, never fake it.
 if not proof['saved_exact']:raise CoreInputFailure(f'Real editor input/save failed in {tag}, screenshot {name}; no controller file replacement')
 return active

def editor_identity(label):
 raw=command(label,"""python3 - <<'E'
import hashlib,json,os,pathlib,subprocess
rows=[]
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  if os.readlink(p/'exe')=='/usr/bin/cosmic-edit':rows.append({'pid':int(p.name),'sha256':hashlib.sha256((p/'exe').read_bytes()).hexdigest(),'args':(p/'cmdline').read_bytes().decode().replace('\\0',' ')})
 except OSError:pass
print(json.dumps({'processes':rows,'package':subprocess.getoutput('dpkg-query -W cosmic-edit')}))
E""")
 data=json.loads(raw)
 if not data['processes']:raise RuntimeError('Native editor executable no longer exists')
 hashes=sorted(set(p['sha256'] for p in data['processes']))
 if hashes!=['61f32f20df9c4da800b0df8054cb0b7ec6c65ccd4b9d11679f7f6c7303dc1149']:raise RuntimeError('Native editor differs from pinned guest ELF')
 if 'editor_elfs' not in state:state['editor_elfs']=hashes
 elif state['editor_elfs']!=hashes:raise RuntimeError('Native editor ELF changed during session')
 return data

def resource_sample(label):
 if state.get('preflight_passed'):editor_identity(label+'-editor-identity')
 raw=command(label,'python3 /var/tmp/lockup-probe/resources.py');obj=json.loads(raw)
 with (ev/'resource-samples.jsonl').open('a') as f:f.write(json.dumps(obj)+'\n')
 if cohort!='stock':
  expected='0abb0f393b14ba0a5ddf791a719c391dfa9a8a04e39196a1caddc8ff7028a01f'
  procs=command(label+'-public-identity',"python3 - <<'P'\nimport hashlib,json,os,pathlib\nr=[]\nfor p in pathlib.Path('/proc').iterdir():\n if not p.name.isdigit():continue\n try:\n  if os.readlink(p/'exe')=='/usr/bin/halcyon':r.append({'pid':int(p.name),'sha256':hashlib.sha256((p/'exe').read_bytes()).hexdigest(),'args':(p/'cmdline').read_bytes().decode().replace('\\0',' ')})\n except OSError:pass\nprint(json.dumps(r))\nP")
  values=json.loads(procs);desktop=[p for p in values if '--settings' not in p['args']]
  if not desktop or any(p['sha256']!=expected for p in desktop):raise RuntimeError('Released public main desktop missing or wrong ELF')
  if cohort=='islands':
   fork=command(label+'-islands-identity',"p=$(pgrep -u 1000 -x cosmic-panel-is 2>/dev/null || true); for n in /proc/[0-9]*/exe; do t=$(readlink \"$n\" 2>/dev/null || true); case \"$t\" in */cosmic-panel-islands) sha256sum \"$n\";; esac; done",required=False)
   if 'f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31' not in fork:raise RuntimeError('Actual matching released Islands ELF missing')
  current=[p['pid'] for p in desktop]
  if 'main_pids' not in state:state['main_pids']=current
  elif state['main_pids']!=current:raise RuntimeError('Public main process restarted during soak')
 return obj

def profile_public():
 launch('lockup-setup','/usr/bin/halcyon',['--settings','setup'])
 _,path,_=snap('public-look-before')
 try:detail=validate_samples(Pixels(path),CAL['wizard_samples'])
 except ValueError:
  detail=validate_samples(Pixels(path),CAL['welcome_samples']);c.click(345,451);time.sleep(2);_,path,_=snap('public-look-grid');detail=validate_samples(Pixels(path),CAL['wizard_samples'])
 dx,dy=detail['offset'];c.click(390+dx,336+dy);time.sleep(3);snap('public-frosted-selected')
 command('stop-setup','systemctl --user stop lockup-setup.service',True)
 conf=json.loads(command('public-selected-config','cat /home/probe/.config/halcyon/config.json'))
 if (conf.get('appearance',{}).get('family'),conf.get('theme'))!=('frosted','ocean'):raise RuntimeError('Actual public Frosted profile identity mismatch')
 state['profile']=conf

def init():
 c.wait_agent();c.q=c.Channel('qmp.sock',True);time.sleep(15)
 blocks=c.q.call('query-block');(ev/'qmp-blocks.json').write_text(json.dumps(blocks,indent=2)+'\n')
 if cohort=='stock' and any(x.get('removable') and x.get('inserted') for x in blocks):raise RuntimeError('Stock has inserted candidate media')
 command('prelogin-clock-readonly','timedatectl show --all')
 if cohort=='islands':command('fixture-local-bin','install -d -o 1000 -g 1000 /home/probe/.local/bin')
 c.login('lockup-login');c.capture('baseline');install_guest_helpers()
 command('shortcuts','cat /usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults; cat /home/probe/.config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom 2>/dev/null',required=False)
 command('native-environment','uname -a; cat /proc/cmdline; ip -brief addr; ip route; lspci -nnk; dpkg-query -W "cosmic*" "mesa*" "libegl*" "libgl*"; cat /sys/power/state; cat /sys/power/mem_sleep',required=False)
 command('driver-source','for d in /sys/class/drm/card*/device/driver; do readlink -f "$d"; done; journalctl -b -k --no-pager',required=False)
 if cohort!='stock':
  command('mount-public-media','candidate_device=$(blkid -L HALCYON_INPUTS) && mkdir -p /mnt/candidate && mount -o ro "$candidate_device" /mnt/candidate && cd /mnt/candidate && sha256sum -c SHA256SUMS')
  h.install('public')
  if cohort=='islands':
   command('install-public-islands','dpkg --install /mnt/candidate/cosmic-ext-halcyon-islands_0.2.0_amd64.deb && dpkg --audit && apt-get check',timeout=120)
   manifest=json.loads((ev/'package-build/cosmic-ext-halcyon-islands_0.2.0_amd64-payload.json').read_text());checks='\n'.join(v+'  /'+k for k,v in manifest.items())+'\n';command('islands-payload','echo '+shlex.quote(base64.b64encode(checks.encode()).decode())+' | base64 -d | sha256sum -c -')
   command('enable-public-islands','halcyon-islands enable',True)
   c.guest("loginctl list-sessions --no-legend | awk '$3==\"probe\" {print $1}' | xargs -r -n1 loginctl terminate-session");time.sleep(15);c.login('lockup-islands-login');command('public-islands-status','halcyon-islands status',True)
  profile_public();command('start-public','halcyon-ctl start',True);time.sleep(8)
 else:command('stock-public-absence','test ! -e /usr/bin/halcyon && test ! -e /home/probe/.config/halcyon')
 command('create-disposable-editor-fixture','install -o 1000 -g 1000 -m 600 /dev/null /home/probe/lockup-input.txt')
 state['fixture_ready']=True;state['setup_monotonic']=time.monotonic()
 preflight()

def preflight(recovery=False):
 if recovery:
  install_guest_helpers();edit_window('recovery-existing-editor')
 else:editor_open(0)
 input_probe(0,'preflight-first');core_close_reopen(0,'preflight')
 # Prove the actual native executable and the service child still exist after
 # real input, file save, close and reopen. Only now start the hour timer.
 editor_identity('native-editor-elf')
 h.snapshot('native-preflight-config')
 resource_sample('resource-preflight');c.capture('preflight');snap('preflight-complete')
 state['preflight_passed']=True;state['preflight_recovered']=recovery;state['phase']='soak';state['start_monotonic']=time.monotonic();state['start_realtime']=time.time();state['next_checkpoint']=0;state['next_resource']=0;state['initialized']=True;save()

def optional_gestures(seq,full=False):
 proof={'sequence':seq,'started_elapsed':time.monotonic()-state['start_monotonic'],'gestures':[]}
 w=edit_window(f'optional-before-{seq}')
 if w.get('geometry_source')!='native protocol':
  state['coverage'].append({'optional_unqualified':{'sequence':seq,'reason':'native geometry absent; screenshot focus is core-only'}});save();return
 # Maximize and restore: actual global shortcut and real compositor geometry.
 c.key('meta_l-m');time.sleep(1);maximized=edit_window(f'maximized-{seq}');c.key('meta_l-m');time.sleep(1);restored=edit_window(f'restored-{seq}')
 proof['gestures'].append({'action':'maximize/restore','before':w,'maximized':maximized,'restored':restored,'geometry_changed':w['geometry']!=maximized['geometry']})
 g=restored['geometry'];start=(g['x']+min(180,g['width']//3),g['y']+20);end=(max(25,min(1200,start[0]+(-45 if seq%2 else 45))),max(65,min(650,start[1]+20)))
 drag(start,end);moved=edit_window(f'moved-{seq}');proof['gestures'].append({'action':'titlebar move','before':g,'after':moved['geometry'],'changed':g!=moved['geometry']})
 g=moved['geometry'];corner=(min(1277,g['x']+g['width']-2),min(795,g['y']+g['height']-2));drag(corner,(max(g['x']+400,corner[0]-30),max(g['y']+250,corner[1]-20)));resized=edit_window(f'resized-{seq}');proof['gestures'].append({'action':'border resize','before':g,'after':resized['geometry'],'changed':g!=resized['geometry']})
 if not full:
  proof['completed_elapsed']=time.monotonic()-state['start_monotonic'];state['coverage'].append({'optional':proof});save();return
 # Native workspace overview and actual workspace navigation, then return.
 c.key('meta_l-w');time.sleep(1);snap(f'workspace-overview-{seq}');c.key('esc');c.key('meta_l-ctrl-down');time.sleep(1);snap(f'workspace-next-{seq}');away=observe(f'workspace-away-{seq}');c.key('meta_l-ctrl-up');time.sleep(1);returned=observe(f'workspace-return-{seq}')
 proof['gestures'].append({'action':'workspace next/return','away':away,'returned':returned})
 # Normal Files app, focus and real pointer close; editor stays alive.
 c.key('meta_l-f');files=[];deadline=time.monotonic()+12
 while time.monotonic()<deadline:
  time.sleep(1);windows=observe(f'files-open-{seq}');files=[v for v in windows['windows'] if not v.get('closed') and 'files' in v.get('app_id','').lower()]
  if files:break
 if files and 'geometry' in files[0]:
  g=files[0]['geometry'];c.click(g['x']+g['width']-24,g['y']+24);time.sleep(2);closed=observe(f'files-pointer-close-{seq}');proof['gestures'].append({'action':'normal Files pointer close','closed':not any(v.get('app_id')==files[0].get('app_id') and not v.get('closed') for v in closed['windows'])})
 else:proof['gestures'].append({'action':'normal Files open','observed':False,'state':windows})
 # Real native clock popup and outside dismissal; pointer moves over edge menu.
 _,_,words=snap(f'before-popup-{seq}');clock=next((v for v in words if ':' in v['text'] and int(v['top'])<50),None)
 if clock:c.click(int(clock['left'])+int(clock['width'])//2,int(clock['top'])+5);time.sleep(1);snap(f'native-popup-{seq}');c.click(100,650);c.key('esc');time.sleep(.5);snap(f'native-popup-dismissed-{seq}')
 if cohort!='stock':pointer_move(1279,400);time.sleep(1.4);snap(f'edge-menu-{seq}');pointer_move(600,650);time.sleep(.8)
 proof['completed_elapsed']=time.monotonic()-state['start_monotonic'];state['coverage'].append({'optional':proof});save()

def core_close_reopen(seq,tag):
 w=input_probe(seq,tag+'-before-close');c.key('meta_l-q');time.sleep(1)
 closed=False;attempts=[]
 for method in ['normal shortcut','calibrated native close button']:
  if method=='calibrated native close button':
   # Changed control path only after retaining the failed shortcut frame.
   g=edit_window(f'{tag}-pointer-close-calibration-{seq}')['geometry']
   c.click(g['x']+g['width']-26,g['y']+26);time.sleep(1)
  try:observed=observe(f'{tag}-close-{seq}')
  except Exception as error:observed={'windows':[],'observer_unqualified':str(error)}
  name,path,words=snap(f'{tag}-closed-{seq}-{len(attempts)}')
  try:editor_pixels.locate(path,words,identity_seen=True);visible=True
  except ValueError:visible=False
  closed=not editor_hits(observed) and not visible
  attempts.append({'method':method,'closed':closed,'frame':name,'native':observed})
  if closed:break
 state['coverage'].append({'core_close':{'sequence':seq,'closed':closed,'attempts':attempts,'phase':state['phase']}});save()
 if not closed:raise CoreInputFailure('Core real editor close failed via shortcut and calibrated pointer; exact states retained')
 editor_open(seq+1);input_probe(seq+10000,tag+'-reopen')

def cycle(seq):
 proof={'sequence':seq,'started_elapsed':time.monotonic()-state['start_monotonic']}
 input_probe(seq,'core')
 core_close_reopen(seq,'core')
 proof['core_passed']=True;proof['completed_elapsed']=time.monotonic()-state['start_monotonic']
 state['checkpoints'].append(proof);resource_sample('resource-checkpoint-'+str(seq));save()
 if state.get('optional_enabled',True):
  try:optional_gestures(seq)
  except Exception as error:
   # A speculative geometry/control failure does not discard proven core input.
   state['optional_failures'].append({'sequence':seq,'error':str(error)});state['optional_enabled']=False;snap('optional-coverage-failure');c.key('esc');save()


def run(until):
 if not state.get('initialized') or any(f.get('category')=='core' for f in state['failures']):raise RuntimeError('Soak not initialized or previous confirmed failure retained')
 c.q=c.Channel('qmp.sock',True);h.count=state.get('count',0)
 while time.monotonic()-state['start_monotonic']<until:
  elapsed=time.monotonic()-state['start_monotonic']
  if elapsed>=state['next_checkpoint']:
   cycle(len(state['checkpoints'])+1);state['next_checkpoint']+=300;save()
  if elapsed>=state['next_resource']:
   resource_sample('resource-minute-'+str(int(elapsed//60)));state['next_resource']=elapsed+60;save()
  time.sleep(min(10,max(0,until-(time.monotonic()-state['start_monotonic']))))
 state['observed_elapsed']=time.monotonic()-state['start_monotonic'];save()

def finish():
 c.q=c.Channel('qmp.sock',True);input_probe(9000,'hour-final');core_close_reopen(9001,'hour-final')
 state['observed_elapsed']=time.monotonic()-state['start_monotonic'];state['hour_complete']=state['observed_elapsed']>=3600
 state['core_finished']=True;resource_sample('resource-core-final');c.capture('core-final');save()
 if not state['hour_complete']:raise RuntimeError('Core hour duration was not reached')

def resume():
 c.q=c.Channel('qmp.sock',True);state['phase']='optional-resume'
 try:
  command('suspend-capability','cat /sys/power/state; cat /sys/power/mem_sleep; command -v rtcwake',required=False)
  command('resume-attempt','systemd-run --unit=lockup-resume --collect /usr/sbin/rtcwake -m mem -s 12',required=False)
  time.sleep(20);command('resume-service','journalctl -b -u lockup-resume --no-pager; journalctl -b -k --no-pager -n 120',required=False)
  input_probe(9500,'after-resume');state['resume_input_responded']=True
 except Exception as error:
  state['resume_input_responded']=False;state['resume_error']=str(error)
  try:snap('resume-failure')
  except Exception:pass
 save()

def collect():
 c.q=c.Channel('qmp.sock',True);c.capture('final');command('full-native-journal','journalctl -b -o short-monotonic --no-pager',required=False)
 state['finished']=True;save();c.guest('sync',5) # Owned-VM stop is handled by the controller; avoid QGA/poweroff race.

manifest={str(p.relative_to(SRC)):hashlib.sha256(p.read_bytes()).hexdigest() for p in SRC.rglob('*') if p.is_file() and p.suffix in ('.py','.json','.xml','.sh')}
(ev/('executed-source-'+stage+'-'+str(time.time_ns())+'.json')).write_text(json.dumps(manifest,indent=2)+'\n')
try:
 if stage=='init':init()
 elif stage=='run':run(int(sys.argv[5]))
 elif stage=='finish':finish()
 elif stage=='optional':
  c.q=c.Channel('qmp.sock',True)
  try:optional_gestures(9900,True)
  except Exception as error:state['optional_failures'].append({'stage':'after-hour','error':str(error)});save()
 elif stage=='resume':resume()
 elif stage=='collect':collect()
 elif stage=='recover':
  if not state.get('fixture_ready') or any(f.get('category')=='core' for f in state['failures']):raise RuntimeError('Recovery forbidden: core failure or fixture preparation incomplete')
  c.q=c.Channel('qmp.sock',True);preflight(True)
 else:raise RuntimeError('Unknown bounded stage')
except Exception as error:
 state['failures'].append({'stage':stage,'error':str(error),'host_time':time.time(),'category':'core' if isinstance(error,CoreInputFailure) else 'harness'});traceback.print_exc()
 try:
  if c.q is None:c.q=c.Channel('qmp.sock',True)
  snap('failure');c.capture('failure');resource_sample('resource-failure')
 except Exception:traceback.print_exc()
 save();raise
finally:save()
