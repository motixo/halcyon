#!/usr/bin/python3
"""Two-job follow-up: five remaining looks and native daemon automatic mode.
No legacy/public reproduction, Halcyon manual replay, Repair, or theme-key writes.
"""
import hashlib,json,pathlib,re,shlex,sys,time,traceback
import theme_session as h
from calibrated_controls import point,Pixels
c=h.c;ev=h.ev;host=h.host
CAL_PATH=pathlib.Path(__file__).parent/'followup/control-calibration.json'
CAL=json.loads(CAL_PATH.read_text())
h.summary.update(scope='Scoped DEVELOPMENT follow-up; remaining five looks and native automatic mode only',
 tested_looks=['calm','evening','focus','paper','rose'],deferred=['Repair','accepted Halcyon manual','public v0.2.0 reproduction'],
 calibration_sha256=hashlib.sha256(CAL_PATH.read_bytes()).hexdigest(),round_vm_limit=2)
FAMILIES={'calm':'soft','evening':'soft','focus':'bare','paper':'paper','rose':'glass'}
PALETTES={'calm':'teal','evening':'lavender','focus':'graphite','paper':'amber','rose':'rose'}

def calibrated_click(label,control):
 c.q.call('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(100*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(650*32767/800)}}]})
 name,path,_=h.screen(label)
 evidence=point(path,control,CAL)
 (ev/(name+'-calibration.json')).write_text(json.dumps(evidence,indent=2)+'\n')
 c.record('calibrated-ui-input',**evidence)
 c.click(*evidence['point']);time.sleep(2)
 return evidence

def cfg(state):
 item=state['files'].get('.config/halcyon/config.json')
 return json.loads(bytes.fromhex(item['hex'])) if item else {}

def raw_mode(state,key):
 suffix='CosmicTheme.Mode/v1/'+key
 values=[bytes.fromhex(v['hex']).decode().strip() for k,v in state['files'].items() if k.endswith(suffix)]
 if len(values)>1:raise RuntimeError('Ambiguous native mode keys')
 return values[0] if values else None

def verify_mode(label,state,dark,automatic=False):
 observed=raw_mode(state,'is_dark')
 if observed is not None and observed!=str(dark).lower():raise RuntimeError(f'{label}: native is_dark={observed}, expected {dark}')
 response=h.command(label+'-portal-assert','busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop org.freedesktop.portal.Settings Read ss org.freedesktop.appearance color-scheme',True)
 expected='1' if dark else '2'
 if not re.search(r'\bu '+expected+r'\s*$',response):raise RuntimeError(f'{label}: live portal mismatch {response!r}')
 auto=raw_mode(state,'auto_switch')
 if automatic and auto!='true':raise RuntimeError(f'{label}: automatic mode not enabled ({auto})')
 if not automatic and auto=='true':raise RuntimeError(f'{label}: native automatic mode unexpectedly enabled during manual cycle')
 proof={'native_is_dark_override':observed,'portal_response':response.strip(),'effective_dark':dark,'automatic':automatic,'native_auto_switch':auto}
 (ev/(label+'-assertion.json')).write_text(json.dumps(proof,indent=2)+'\n')
 return proof

def dismiss_popup(label):
 # Read-only full screenshot demonstrates the real outside-click dismissal.
 c.click(100,650);time.sleep(1)
 name,path,words=h.screen(label+'-outside-click')
 if any('calendar' in w['text'].lower() for w in words):raise RuntimeError('Calendar popup still visible after outside click')
 return name

def select(look):
 dismiss_popup('before-'+look)
 h.launch('/usr/bin/halcyon',['--settings','setup'])
 # On the first look only, the observed Welcome button must be used. Later
 # setup resumes the Look grid. Pixel pattern validation decides which state.
 name,path,_=h.screen('wizard-state-'+look)
 try:point(path,look,CAL)
 except ValueError:
  calibrated_click('wizard-welcome-'+look,'Start')
 calibrated_click('choose-'+look,look)
 h.screen('selected-ui-'+look);h.snapshot('running-'+look)
 h.stop_ui()
 state=h.frame('selected-'+look)
 conf=cfg(state);identity=(conf.get('appearance',{}).get('family'),conf.get('theme'))
 if identity!=(FAMILIES[look],PALETTES[look]):raise RuntimeError(f'{look}: persisted identity {identity} differs from selected starter')
 proof={'look':look,'family':identity[0],'palette':identity[1],'native_mode_palettes':conf.get('appearance',{}).get('native_mode_palettes',0)}
 h.summary.setdefault('identities',[]).append(proof)
 (ev/(look+'-identity-assertion.json')).write_text(json.dumps(proof,indent=2)+'\n')
 if proof['native_mode_palettes']!=1:raise RuntimeError(look+': missing accepted native palette provenance')
 return proof

def manual(mode,label):
 h.launch('/usr/bin/cosmic-settings',['appearance'],'cosmic-ui')
 calibrated_click('native-'+label,mode)
 h.screen('native-selected-'+label);h.stop_ui()
 state=h.frame(label)
 verify_mode(label,state,mode=='Dark')
 return state

def cycle(look):
 before=h.snapshot(look+'-cycle-before')
 for i,mode in enumerate(['Dark','Light','Dark']):manual(mode,look+'-manual-'+mode.lower()+'-'+str(i))
 # Actual public time applet, once per remaining look, with visible dismissal.
 # Clock center differs between stock and Islands. It is a native bar, not a
 # Halcyon window; coordinates derive from retained clock screenshots.
 c.click(585 if host=='islands' else 640,22);time.sleep(2)
 h.screen(look+'-actual-clock-popup');dismiss_popup(look+'-popup-dismissed')
 after=h.snapshot(look+'-cycle-after')
 def hosts(s):return [(p['pid'],p['exe'],p['sha256']) for p in s['processes'] if p['exe'].endswith(('/cosmic-panel','/cosmic-panel-islands','/cosmic-applet-time'))]
 stable=hosts(before)==hosts(after)
 h.summary['scenarios'].append({'scenario':look+' native manual Dark-Light-Dark','execution':'complete','look_identity_verified':True,'host_processes_preserved':stable,'before':hosts(before),'after':hosts(after)})
 h.save()
 if not stable:raise RuntimeError(look+': panel/applet executable or PID changed')

def wait_native(label,dark,timeout=40):
 deadline=time.monotonic()+timeout;last=''
 while time.monotonic()<deadline:
  response=h.command(label+'-poll','busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop org.freedesktop.portal.Settings Read ss org.freedesktop.appearance color-scheme',True)
  last=response.strip()
  if re.search(r'\bu '+('1' if dark else '2')+r'\s*$',response):break
  time.sleep(2)
 else:raise RuntimeError(f'{label}: native daemon did not reach scheduled mode, last portal {last!r}')
 state=h.frame(label);verify_mode(label,state,dark,automatic=True)
 return state

def automatic():
 # This is a real system clock/timezone stimulus, not writing is_dark or
 # emitting artificial theme notifications. The pinned daemon watches native
 # timedated changes and derives coordinates from its embedded timezone data.
 dismiss_popup('before-auto')
 manual('Dark','automatic-manual-dark-preparation')
 h.command('auto-disable-ntp','timedatectl set-ntp false')
 h.command('auto-timezone','timedatectl set-timezone Europe/London && timedatectl show && readlink -f /etc/localtime')
 h.command('auto-night-setup','timedatectl set-time "2026-10-23 23:00:00" && date --iso-8601=seconds')
 time.sleep(3)
 h.launch('/usr/bin/cosmic-settings',['appearance'],'cosmic-ui')
 name,path,_=h.screen('automatic-control-before')
 evidence=point(path,'Auto',CAL)
 if evidence['enabled']:raise RuntimeError('Automatic toggler already enabled before explicit native selection')
 calibrated_click('automatic-native-toggle','Auto')
 h.screen('automatic-control-after');h.stop_ui()
 first=wait_native('auto-native-night-initial',True)
 identity=cfg(first);family=identity.get('appearance',{}).get('family');palette=identity.get('theme')
 if (family,palette) not in [(FAMILIES[x],PALETTES[x]) for x in FAMILIES]:raise RuntimeError('Automatic test has no persisted requested starter identity')
 for label,time_value,dark in [('noon','12:00:00',False),('night','23:00:00',True)]:
  h.command('auto-timedated-'+label,'timedatectl set-time "2026-10-23 '+time_value+'" && date --iso-8601=seconds')
  state=wait_native('auto-native-'+label,dark)
  current=cfg(state)
  if (current.get('appearance',{}).get('family'),current.get('theme'))!=(family,palette):raise RuntimeError('Automatic stimulus changed product starter identity')
 h.command('auto-native-services','timedatectl; systemctl status systemd-timedated --no-pager; journalctl -b --no-pager -n 2500',required=False)
 h.summary['automatic'].update(native='PASS: native UI Auto; real timedated night-noon-night; native mode and portal assertions passed',injected_events=False,stimulus='real timedated wall clock / Europe/London timezone, offline',family=family,palette=palette)
 h.save()

try:
 c.wait_agent();c.q=c.Channel('qmp.sock',True);time.sleep(15)
 c.shot('000-followup-greeter')
 if host=='islands':h.command('fixture-local-bin','install -d -o 1000 -g 1000 /home/probe/.local/bin')
 c.login('followup-login');c.capture('baseline')
 h.command('offline-interfaces','ip -brief addr; ip route; systemctl is-active dbus',required=True)
 h.command('mount-inputs','mkdir -p /mnt/candidate && mount -o ro /dev/sr0 /mnt/candidate && cd /mnt/candidate && sha256sum -c SHA256SUMS')
 approved=(ev/'approved-inputs.sha256').read_text()
 h.command('independent-input-check',"cd /mnt/candidate && sha256sum -c /dev/stdin <<'INPUTS'\n"+approved+'INPUTS\n')
 h.install('candidate')
 if host=='islands':
  h.command('install-islands','dpkg --install /mnt/candidate/cosmic-ext-halcyon-islands_0.2.0_amd64.deb && dpkg --audit && apt-get check',timeout=120)
  h.command('enable-islands','halcyon-islands enable',True)
  c.guest("loginctl list-sessions --no-legend | awk '$3==\"probe\" {print $1}' | xargs -r -n1 loginctl terminate-session")
  time.sleep(15);c.login('followup-islands-login')
  h.command('islands-status','halcyon-islands status',True)
  state=h.snapshot('islands-running')
  if not any(p['exe'].endswith('/cosmic-panel-islands') and p['sha256']=='f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31' for p in state['processes']):raise RuntimeError('Matching released Islands executable not running')
 # Each look is independent evidence. Record a blocked case and continue with
 # the next actual selection; no retry of a failed action or broader matrix.
 for look in ['calm','evening','focus','paper','rose']:
  try:select(look);cycle(look)
  except Exception as error:
   h.summary['errors'].append({'look':look,'blocked':str(error)})
   traceback.print_exc();h.screen('blocked-'+look);h.snapshot('blocked-'+look)
   h.stop_ui();dismiss_popup('blocked-'+look);h.save()
 try:automatic()
 except Exception as error:
  h.summary['automatic'].update(native='BLOCKED: '+str(error),injected_events=False)
  traceback.print_exc();h.screen('blocked-automatic');h.snapshot('blocked-automatic')
  h.command('automatic-blocker-journal','timedatectl; readlink -f /etc/localtime; journalctl -b --no-pager -n 3500',required=False)
 h.stop_ui();c.capture('final')
 h.summary['execution_complete']=True
except Exception as error:
 h.summary['errors'].append({'fatal':str(error)});traceback.print_exc()
 try:h.screen('fatal');h.snapshot('fatal');c.capture('fatal')
 except Exception:traceback.print_exc()
finally:
 h.save()
 try:c.guest('sync; systemctl poweroff',5)
 except Exception:pass
 time.sleep(8)
passed=h.summary.get('execution_complete') and not h.summary['errors'] and h.summary['automatic']['native'].startswith('PASS')
sys.exit(0 if passed else 1)
