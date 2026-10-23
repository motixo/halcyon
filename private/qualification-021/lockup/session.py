#!/usr/bin/python3
"""Real native input/observable app save/render soak, never a fake health service."""
import base64,hashlib,json,os,pathlib,re,shlex,subprocess,sys,time,traceback
SRC=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(SRC.parent/'theme'))
import theme_session as h
from calibrated_controls import Pixels,validate_samples
c=h.c;ev=h.ev;work=c.work;cohort=sys.argv[3];stage=sys.argv[4]
state_path=work/'lockup-state.json'
state=json.loads(state_path.read_text()) if state_path.exists() else {'cohort':cohort,'checkpoints':[],'failures':[],'coverage':[],'count':0,'source_scope':'actual public v0.2.0 only; no development/fix2 package','hour_complete':False}
h.count=state.get('count',0)
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

def edit_window(label):
 r=observe(label);hits=[w for w in r['windows'] if not w.get('closed') and ('cosmicedit' in w.get('app_id','').lower().replace('-','') or 'lockup-input' in w.get('title','').lower())]
 if len(hits)!=1:raise RuntimeError(f'{label}: expected one real editor, got {hits}')
 if 'geometry' not in hits[0]:raise RuntimeError('Native toplevel geometry unavailable; preserve exact observer state')
 return hits[0]
def center(w):
 g=w['geometry'];return (max(40,min(1240,g['x']+g['width']//2)),max(100,min(670,g['y']+g['height']//2)))
def launch(unit,binary,args):
 # Pinned cosmic-edit daemonizes before its UI exists. A Type=simple
 # transient service can kill the forked child when its parent exits.
 kind=' --property=Type=forking --property=GuessMainPID=yes' if binary=='/usr/bin/cosmic-edit' else ''
 command('launch-'+unit,'systemd-run --user --collect'+kind+' --unit='+unit+' '+shlex.join([binary,*args]),True);time.sleep(5)
def editor_open(seq):
 launch('lockup-edit-'+str(seq),'/usr/bin/cosmic-edit',['/home/probe/lockup-input.txt'])
 command('editor-unit-'+str(seq),'systemctl --user show lockup-edit-'+str(seq)+'.service -p Type -p MainPID -p Result -p ActiveState -p SubState; journalctl --user -b -u lockup-edit-'+str(seq)+'.service --no-pager',True,False)
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
 if 2 not in w.get('states',[]):
  c.key('meta_l-tab');time.sleep(.6);w=edit_window(f'{tag}-{seq}-focus');focus_method='native switch then pointer'
 c.click(*center(w));time.sleep(.3)
 token=f'p{cohort}{seq:04d}'
 c.key('ctrl-a');c.text(token);c.key('ctrl-s');time.sleep(1)
 content=command(f'{tag}-{seq}-saved-file','cat /home/probe/lockup-input.txt')
 name,path,words=snap(f'{tag}-{seq}-input')
 active=edit_window(f'{tag}-{seq}-after')
 observed=''.join(re.sub('[^a-z0-9]','',w['text'].lower()) for w in words)
 proof={'token':token,'saved_exact':content.strip()==token,'focus_method':focus_method,'native_editor':active,'screenshot':name,'token_ocr_exact':token in observed,'elapsed':time.monotonic()-state['start_monotonic']}
 state['coverage'].append({'input':proof});save()
 if not proof['saved_exact']:raise RuntimeError(f'Real editor input/save failed in {tag}, screenshot {name}; no controller file replacement')
 return active

def resource_sample(label):
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
 state['start_monotonic']=time.monotonic();state['start_realtime']=time.time();state['next_checkpoint']=0;state['next_resource']=0
 editor_open(0);input_probe(0,'warmup');resource_sample('resource-init');state['initialized']=True;save()

def cycle(seq):
 proof={'sequence':seq,'started_elapsed':time.monotonic()-state['start_monotonic'],'gestures':[]}
 w=input_probe(seq)
 # Maximize and restore: actual global shortcut and real compositor geometry.
 c.key('meta_l-m');time.sleep(1);maximized=edit_window(f'maximized-{seq}');c.key('meta_l-m');time.sleep(1);restored=edit_window(f'restored-{seq}')
 proof['gestures'].append({'action':'maximize/restore','before':w,'maximized':maximized,'restored':restored,'geometry_changed':w['geometry']!=maximized['geometry']})
 g=restored['geometry'];start=(g['x']+min(180,g['width']//3),g['y']+20);end=(max(25,min(1200,start[0]+(-45 if seq%2 else 45))),max(65,min(650,start[1]+20)))
 drag(start,end);moved=edit_window(f'moved-{seq}');proof['gestures'].append({'action':'titlebar move','before':g,'after':moved['geometry'],'changed':g!=moved['geometry']})
 g=moved['geometry'];corner=(min(1277,g['x']+g['width']-2),min(795,g['y']+g['height']-2));drag(corner,(max(g['x']+400,corner[0]-30),max(g['y']+250,corner[1]-20)));resized=edit_window(f'resized-{seq}');proof['gestures'].append({'action':'border resize','before':g,'after':resized['geometry'],'changed':g!=resized['geometry']})
 # Native workspace overview and actual workspace navigation, then return.
 c.key('meta_l-w');time.sleep(1);snap(f'workspace-overview-{seq}');c.key('esc');c.key('meta_l-ctrl-down');time.sleep(1);snap(f'workspace-next-{seq}');away=observe(f'workspace-away-{seq}');c.key('meta_l-ctrl-up');time.sleep(1);returned=observe(f'workspace-return-{seq}')
 proof['gestures'].append({'action':'workspace next/return','away':away,'returned':returned})
 input_probe(seq+100,'workspace-return')
 # Normal Files app, focus and real pointer close; editor stays alive.
 c.key('meta_l-f');files=[];deadline=time.monotonic()+12
 while time.monotonic()<deadline:
  time.sleep(1);windows=observe(f'files-open-{seq}');files=[v for v in windows['windows'] if not v.get('closed') and 'files' in v.get('app_id','').lower()]
  if files:break
 if files and 'geometry' in files[0]:
  g=files[0]['geometry'];c.click(g['x']+g['width']-24,g['y']+24);time.sleep(2);closed=observe(f'files-pointer-close-{seq}');proof['gestures'].append({'action':'normal Files pointer close','closed':not any(v.get('app_id')==files[0].get('app_id') and not v.get('closed') for v in closed['windows'])})
 else:proof['gestures'].append({'action':'normal Files open','observed':False,'state':windows})
 input_probe(seq+200,'files-return')
 # Real native clock popup and outside dismissal; pointer moves over edge menu.
 _,_,words=snap(f'before-popup-{seq}');clock=next((v for v in words if ':' in v['text'] and int(v['top'])<50),None)
 if clock:c.click(int(clock['left'])+int(clock['width'])//2,int(clock['top'])+5);time.sleep(1);snap(f'native-popup-{seq}');c.click(100,650);time.sleep(.5)
 if cohort!='stock':pointer_move(1279,400);time.sleep(1.4);snap(f'edge-menu-{seq}');pointer_move(600,650);time.sleep(.8)
 input_probe(seq+300,'popup-edge-return')
 # Close the real editor through the normal window shortcut, observe closure,
 # reopen the same file and prove the next input reaches that window.
 c.key('meta_l-q');time.sleep(2);closed=observe(f'editor-close-{seq}');proof['gestures'].append({'action':'editor shortcut close','closed':not any('cosmicedit' in v.get('app_id','').lower().replace('-','') and not v.get('closed') for v in closed['windows'])})
 editor_open(seq+1);input_probe(seq+400,'reopen')
 proof['completed_elapsed']=time.monotonic()-state['start_monotonic'];state['checkpoints'].append(proof);resource_sample('resource-checkpoint-'+str(seq));save()

def run(until):
 if not state.get('initialized') or state['failures']:raise RuntimeError('Soak not initialized or previous confirmed failure retained')
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
 c.q=c.Channel('qmp.sock',True);input_probe(9000,'duration-final');state['hour_complete']=time.monotonic()-state['start_monotonic']>=3600
 # Bounded real guest RTC/suspend attempt only after the hour is already proven.
 command('suspend-capability','cat /sys/power/state; cat /sys/power/mem_sleep; command -v rtcwake',required=False)
 command('resume-attempt','systemd-run --unit=lockup-resume --collect /usr/sbin/rtcwake -m mem -s 12',required=False)
 time.sleep(20)
 try:command('resume-service','journalctl -b -u lockup-resume --no-pager; journalctl -b -k --no-pager -n 120',required=False);input_probe(9001,'after-resume');state['resume_input_responded']=True
 except Exception as error:state['resume_input_responded']=False;state['resume_error']=str(error)
 resource_sample('resource-final');c.capture('final');command('full-native-journal','journalctl -b -o short-monotonic --no-pager',required=False)
 state['finished']=True;save();c.guest('sync; systemctl poweroff',5)

try:
 if stage=='init':init()
 elif stage=='run':run(int(sys.argv[5]))
 elif stage=='finish':finish()
 else:raise RuntimeError('Unknown bounded stage')
except Exception as error:
 state['failures'].append({'stage':stage,'error':str(error),'host_time':time.time()});traceback.print_exc()
 try:
  if c.q is None:c.q=c.Channel('qmp.sock',True)
  snap('failure');c.capture('failure');resource_sample('resource-failure')
 except Exception:traceback.print_exc()
 save();raise
finally:save()
