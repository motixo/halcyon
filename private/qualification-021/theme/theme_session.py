#!/usr/bin/python3
"""Full installed session; QMP UI input and independent QGA/pixel evidence.
No planner fixtures, palette writers or product code modifications.
"""
import sys,os,pathlib,json,time,subprocess,traceback,hashlib,csv,io,re,shlex
from collections import Counter
# control.py contains only the reviewed QMP/QGA definitions, not exercise.py's
# top-level stock exercise. It deliberately creates no connection on import.
import control as c
work=c.work;ev=c.ev
lineage,host=sys.argv[2:4]
summary={'scope':'offline full-session DEVELOPMENT snapshot; not final 0.2.1 qualification',
         'lineage':lineage,'host':host,'scenarios':[],'frames':[],'errors':[],
         'automatic':{'native': 'not yet observed','injected_events':False}}
count=0
clock_box=None

def save():
 (ev/'theme-results.json').write_text(json.dumps(summary,indent=2)+'\n')

def command(label,cmd,as_user=False,required=True,timeout=60):
 rc,out=c.user(cmd) if as_user else c.guest(cmd,timeout)
 (ev/(label+'.txt')).write_text(out)
 c.record('command',label=label,exitcode=rc)
 if required and rc:raise RuntimeError(f'{label}: command exit {rc}: {out[-1500:]}')
 return out

def stop_ui():
 c.user('systemctl --user stop theme-ui.service cosmic-ui.service halcyon.service 2>/dev/null || true')
 c.guest("""python3 - <<'STOP'
import pathlib,os,signal
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  if p.stat().st_uid==1000 and os.readlink(p/'exe') in ['/usr/bin/halcyon','/usr/bin/cosmic-settings']:
   os.kill(int(p.name),signal.SIGTERM)
 except (OSError,ProcessLookupError):pass
STOP""")
 time.sleep(2)

def launch(binary,args,unit='theme-ui'):
 stop_ui()
 c.user('systemctl --user reset-failed '+unit+'.service 2>/dev/null || true')
 command('launch-'+str(time.time_ns()),'systemd-run --user --collect --unit='+unit+' '+shlex.join([binary,*args]),True)
 time.sleep(6)

def screen(label):
 global count
 count+=1
 name=f'{count:03d}-{label}'
 c.shot(name)
 path=ev/(name+'.png')
 p=subprocess.run(['tesseract',str(path),'stdout','--psm','11','tsv'],capture_output=True,text=True,check=True)
 (ev/(name+'.tsv')).write_text(p.stdout)
 rows=list(csv.DictReader(io.StringIO(p.stdout),delimiter='\t'))
 return name,path,[r for r in rows if r['text'].strip() and float(r['conf'])>30]

def target(label,phrase,optional=False,occurrence=0,dx=0,dy=0):
 name,path,words=screen(label)
 needle=phrase.casefold().split()
 hits=[]
 for i,w in enumerate(words):
  if int(w['top'])<140:continue
  part=words[i:i+len(needle)]
  if len(part)==len(needle) and [re.sub(r'[^\wé]','',r['text'].casefold()) for r in part]==[re.sub(r'[^\wé]','',a) for a in needle]:
   x=int(w['left']);y=int(w['top']);right=max(int(r['left'])+int(r['width']) for r in part)
   hits.append(((x+right)//2,y+int(w['height'])//2))
 if len(hits)<=occurrence:
  # Reviewed screenshot 001-wizard-start has a visible solid Start button
  # at these bounds; whole-screen sparse OCR misses it against the bird art.
  # This fallback is limited to the observed welcome copy and button pixels.
  if phrase=='Start' and {'Desktop','widgets','themes'}.issubset({w['text'] for w in words}):
   im=__import__('PIL.Image',fromlist=['Image']).open(path).convert('RGB')
   rgb=im.getpixel((345,438))
   if rgb[2]>200 and rgb[0]>110 and rgb[1]>80:
    c.record('observed-welcome-button-fallback',bounds=[279,431,411,471],pixel=rgb)
    c.click(345,451);time.sleep(3);return True
  if optional:return False
  raise RuntimeError(f'UI target {phrase!r} not visible in {name}; OCR: '+ ' '.join(w['text'] for w in words))
 x,y=hits[occurrence];c.click(x+dx,y+dy);time.sleep(3)
 return True

def scroll(down=True,n=5):
 # QMP wheel input, within content rather than the window sidebar.
 c.q.call('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(1000*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(650*32767/800)}}]})
 for _ in range(n):
  c.q.call('input-send-event',{'events':[{'type':'btn','data':{'button':'wheel-down' if down else 'wheel-up','down':True}},{'type':'btn','data':{'button':'wheel-down' if down else 'wheel-up','down':False}}]})
 time.sleep(2)

def snapshot(label):
 script=r'''python3 - <<'SNAP'
import os,pathlib,json,hashlib,subprocess
home=pathlib.Path('/home/probe'); rows={}
for sub in ['.config/cosmic','.config/halcyon','.config/cosmic-desktop-cards','.local/state/halcyon','.local/state/cosmic-desktop-cards']:
 root=home/sub
 if root.exists():
  for p in sorted(root.rglob('*')):
   if p.is_file() and not p.is_symlink() and p.stat().st_size<4000000:
    data=p.read_bytes();rows[str(p.relative_to(home))]={'sha256':hashlib.sha256(data).hexdigest(),'hex':data.hex()}
procs=[]
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  exe=os.readlink(p/'exe')
  if any(n in exe for n in ['cosmic','halcyon','dbus-daemon']):
   procs.append({'pid':int(p.name),'exe':exe,'sha256':hashlib.sha256((p/'exe').read_bytes()).hexdigest(),'cmdline':(p/'cmdline').read_bytes().decode(errors='replace').replace('\0',' ')})
 except (OSError,ProcessLookupError):pass
print(json.dumps({'files':rows,'processes':procs,'packages':subprocess.getoutput("dpkg-query -W 'cosmic*' 'xdg-desktop-portal*' 'dbus*' 'systemd*'")},indent=2))
SNAP'''
 raw=command(label+'-state',script)
 state=json.loads(raw)
 return state

def portal(label):
 command(label+'-portal',"busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop org.freedesktop.portal.Settings Read ss org.freedesktop.appearance color-scheme; busctl --user list",True,False)

def frame(label):
 global clock_box
 time.sleep(4)
 name,path,words=screen(label)
 state=snapshot(name)
 portal(name)
 procs=state['processes']
 services=[p['exe'] for p in procs]
 if '/usr/bin/cosmic-settings-daemon' not in services or '/usr/libexec/xdg-desktop-portal-cosmic' not in services:
  raise RuntimeError('Full native settings daemon/portal missing during observation')
 stopped=not any('/halcyon' in p['exe'] or p['exe'].endswith('/cosmic-settings') for p in procs)
 modes={k:bytes.fromhex(v['hex']).decode(errors='replace') for k,v in state['files'].items() if 'CosmicTheme.Mode/' in k}
 # Independent screen analysis: fixed safe ROI inside the clock label bounds
 # from native OCR. Surface bounds are inferred from flat horizontal runs,
 # never from wallpaper-wide matching or product fixture expectations.
 im=__import__('PIL.Image',fromlist=['Image']).open(path).convert('RGB')
 clock=[w for w in words if int(w['top'])<65 and ':' in w['text'] and any(x.isdigit() for x in w['text'])]
 metrics={'clock_ocr':clock,'bar_regions':{}}
 for bar,box in [('panel',(180,0,1100,55)),('dock',(260,725,1020,800))]:
  pix=list(im.crop(box).getdata());metrics['bar_regions'][bar]={'dominant':Counter(pix).most_common(8)}
 if clock:
  w=clock[0];x=int(w['left']);y=int(w['top']);ww=int(w['width']);hh=int(w['height'])
  clock_box=(x,y,x+ww,y+hh)
 if clock_box:
  box=clock_box;crop=im.crop(box);crop.save(ev/(name+'-clock-text.png'))
  colors=Counter(crop.getdata());bg,n=colors.most_common(1)[0]
  fg=[(rgb,n) for rgb,n in colors.items() if sum((a-b)**2 for a,b in zip(rgb,bg))>55**2]
  metrics['clock_text']={'bounds':box,'background':bg,'foreground_candidates':sorted(fg,key=lambda v:-v[1])[:12],'foreground_pixels':sum(n for rgb,n in fg), 'ocr_text':clock[0]['text'] if clock else None,'bounds_from_previous_frame':not bool(clock)}
 summary['frames'].append({'name':name,'stopped':stopped,'modes':modes,'pixels':metrics})
 save()
 if not stopped:raise RuntimeError('Halcyon or settings still running during stopped-widget observation')
 return state

def select_look(look):
 launch('/usr/bin/halcyon',['--settings','setup'])
 if not target('wizard-start','Start',True):target('wizard-continue','Continue',True)
 target('wizard-look-'+look,{'rose':'Rosé'}.get(look,look))
 screen('wizard-selected-'+look)
 snapshot('running-selected-'+look)
 stop_ui()
 state=frame('selected-'+look)
 # The real selected app config must name a family; absence means no success.
 configs=[bytes.fromhex(v['hex']).decode(errors='replace') for k,v in state['files'].items() if k.endswith('config.json')]
 if not any('family' in raw for raw in configs):raise RuntimeError('UI look selection did not persist an appearance family')
 return state

def mode(mode,label):
 launch('/usr/bin/cosmic-settings',['appearance'],'cosmic-ui')
 if mode=='Auto':
  name,path,words=screen('native-auto-control')
  word=next((w for w in words if w['text'].casefold()=='automatically'),None)
  if word is None:raise RuntimeError('Native automatic-mode toggler is not visible')
  c.click(978,int(word['top'])+int(word['height'])//2);time.sleep(3)
 else:target('native-mode-'+mode,mode,dy=-60)
 screen('native-selected-'+mode)
 stop_ui()
 state=frame(label)
 m=[bytes.fromhex(v['hex']).decode().strip() for k,v in state['files'].items() if k.endswith('CosmicTheme.Mode/v1/is_dark')]
 if mode=='Auto':
  auto=[bytes.fromhex(v['hex']).decode().strip() for k,v in state['files'].items() if k.endswith('CosmicTheme.Mode/v1/auto_switch')]
  if auto!=['true']:raise RuntimeError('Native automatic-mode switch did not enable: '+str(auto))
 if mode in ['Light','Dark'] and m!=[str(mode=='Dark').lower()]:
  # An unchanged native default need not create a per-user override. Require
  # independent live portal confirmation for this specific no-op Dark case.
  portal_files=sorted(ev.glob('*'+label+'-portal.txt'))
  current=portal_files[-1].read_text().splitlines()[0] if portal_files else ''
  if not (mode=='Dark' and m==[] and re.search(r'\bu 1$',current)):
   raise RuntimeError('Native mode effective state does not match request: '+str(m)+' portal='+current)
 return state

def cycle(look):
 before=snapshot(look+'-cycle-before')
 for i,m in enumerate(['Dark','Light','Dark']):mode(m,look+'-manual-'+m.lower()+'-'+str(i))
 c.click(640,16);time.sleep(3);screen(look+'-real-clock-applet');c.key('esc')
 after=snapshot(look+'-cycle-after')
 def hosts(s):return [(p['pid'],p['exe'],p['sha256']) for p in s['processes'] if p['exe'].endswith(('/cosmic-panel','/cosmic-panel-islands','/cosmic-applet-time'))]
 summary['scenarios'].append({'scenario':look+' Dark-Light-Dark native UI','execution':'complete; pixels require independent review','host_processes_preserved':hosts(before)==hosts(after),'before':hosts(before),'after':hosts(after)})
 save()

def install(which):
 package='cosmic-ext-halcyon_0.2.0_amd64.deb' if which=='public' else 'cosmic-ext-halcyon_0.2.1~dev.0b688c8_amd64.deb'
 command('install-'+which,'dpkg --install /mnt/candidate/'+package+' && dpkg --audit && apt-get check',timeout=120)
 # Match every installed payload to the finalized independent builder manifest.
 manifest=json.loads((ev/'package-build'/(package[:-4]+'-payload.json')).read_text())
 checks='\n'.join(v+'  /'+k for k,v in manifest.items())+'\n'
 encoded=__import__('base64').b64encode(checks.encode()).decode()
 command('payload-'+which,"echo "+shlex.quote(encoded)+" | base64 -d | sha256sum -c -",timeout=120)
 snapshot('installed-'+which)

try:
 c.wait_agent();c.q=c.Channel('qmp.sock',True);time.sleep(15)
 c.shot('000-greeter')
 if host=='islands':command('fixture-local-bin','install -d -o 1000 -g 1000 /home/probe/.local/bin')
 c.login('login');c.capture('baseline')
 command('actual-session-environment',"printf 'PATH=%s\\nXDG_SESSION_ID=%s\\nWAYLAND_DISPLAY=%s\\n' \"$PATH\" \"$XDG_SESSION_ID\" \"$WAYLAND_DISPLAY\"",True)
 command('offline','ip -brief addr; ip route; cat /proc/net/route; systemctl is-active dbus; pgrep -a cosmic-settings-daemon; pgrep -a xdg-desktop-portal',required=False)
 command('mount-inputs','mkdir -p /mnt/candidate && mount -o ro /dev/sr0 /mnt/candidate && cd /mnt/candidate && sha256sum -c SHA256SUMS')
 approved=(ev/'approved-inputs.sha256').read_text()
 checks='cd /mnt/candidate && sha256sum -c /dev/stdin <<\'INPUTS\'\n'+approved+'INPUTS\n'
 command('independent-input-check',checks)
 install(lineage)
 if host=='islands':
  command('install-islands','dpkg --install /mnt/candidate/cosmic-ext-halcyon-islands_0.2.0_amd64.deb && dpkg --audit && apt-get check',timeout=120)
  command('enable-islands','halcyon-islands enable',True)
  c.guest("loginctl list-sessions --no-legend | awk '$3==\"probe\" {print $1}' | xargs -r -n1 loginctl terminate-session")
  time.sleep(15);c.login('islands-login')
  command('islands-status','halcyon-islands status',True)
  st=snapshot('islands-process')
  if not any(p['exe'].endswith('/cosmic-panel-islands') for p in st['processes']):raise RuntimeError('Matching released Islands host did not actually start')
 looks=['halcyon','calm','evening','focus','paper','rose'] if lineage=='candidate' else ['halcyon']
 for look in looks:
  select_look(look);cycle(look)
 if lineage=='public':
  # Real native reset/default path in COSMIC Settings, before upgrade. Do not
  # delete configs as a surrogate for Reset. Failure is retained explicitly.
  try:
   launch('/usr/bin/cosmic-settings',['panel'],'cosmic-ui')
   scroll(True,18)
   if target('native-reset','Reset to default',True):
    screen('native-reset-confirmation');target('native-reset-confirm','Reset',True)
    stop_ui();frame('public-native-reset');cycle('public-native-reset')
    c.guest("loginctl list-sessions --no-legend | awk '$3==\"probe\" {print $1}' | xargs -r -n1 loginctl terminate-session")
    time.sleep(15);c.login('public-reset-relogin');frame('public-reset-after-relogin')
    summary['scenarios'].append({'scenario':'public native panel Reset','execution':'control activated; visual review required'})
   else:
    summary['scenarios'].append({'scenario':'public native panel Reset','execution':'BLOCKED: no visible Reset control; no config deletion substituted'})
   stop_ui()
  except Exception as e:summary['errors'].append({'reset':str(e)});stop_ui()
  install('candidate')
  before=snapshot('legacy-before-open')
  launch('/usr/bin/halcyon',['--settings','themes'])
  target('legacy-explicit-repair','Repair theme matching')
  screen('legacy-repair-preview');snapshot('legacy-repair-preview-running')
  target('legacy-repair-apply','Apply')
  stop_ui();frame('legacy-after-repair');cycle('legacy-repaired')
  summary['scenarios'].append({'scenario':'actual legacy explicit Repair','execution':'complete; raw baseline and provenance retained'})
 else:
  # Native automatic mode uses the daemon and system clock. Merely writing
  # is_dark with auto_switch=true would be synthetic, so do not do that.
  try:
   mode('Auto','native-auto-selected')
   command('native-auto-services','timedatectl; systemctl status systemd-timedated --no-pager; journalctl -b --no-pager -n 2000',required=False)
   # Clock advancement is fixture stimulus to the real timedated/daemon path;
   # not an injected theme event. Record both attempted times and mode results.
   command('native-auto-disable-ntp','timedatectl set-ntp false',required=False)
   for hour in ['12:00:00','23:00:00']:
    command('native-auto-time-'+hour.replace(':',''),'timedatectl set-time "2026-10-23 '+hour+'"',required=False)
    time.sleep(15);frame('native-auto-clock-'+hour.replace(':',''))
   summary['automatic']['native']='native UI Auto selected; noon/night system-clock stimuli observed; inspect state for actual switching'
  except Exception as e:summary['automatic']['native']='BLOCKED: '+str(e);stop_ui()
 c.capture('final')
 summary['execution_complete']=True
except Exception as e:
 summary['errors'].append({'fatal':str(e)});traceback.print_exc()
 try:screen('failure');snapshot('failure');c.capture('failure')
 except Exception:traceback.print_exc()
finally:
 save()
 try:c.guest('sync; systemctl poweroff',5)
 except Exception:pass
 time.sleep(8)
sys.exit(0 if summary.get('execution_complete') else 1)
