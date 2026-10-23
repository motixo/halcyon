#!/usr/bin/python3
"""Native baseline comparison. No daemon restart, mode-key writes or fake events."""
import json,pathlib,re,shlex,time,traceback
import followup_session as f
h=f.h;c=f.c;ev=f.ev;phase=h.lineage;host=h.host
looks=['paper','rose'] if host=='stock' else ['calm','evening','focus','paper','rose']
h.summary.update(scope='DEVELOPMENT native Auto diagnosis; untouched baseline or identical frozen candidate',phase=phase,tested_looks=[] if phase=='untouched' else looks,round_vm_job_limit=2,deferred=['Repair','already accepted manual/public scenarios'],automatic={'injected_events':False,'native':'not yet observed','observations':[]})

def monitor_start():
 matches=["path='/org/freedesktop/timedate1'", "type='signal',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.timedate1'", "type='signal',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'"]
 cmd=['systemd-run','--unit=qualification-auto-dbus','--property=StandardOutput=append:/var/tmp/qualification-auto-dbus.log','--property=StandardError=append:/var/tmp/qualification-auto-dbus.log','/usr/bin/dbus-monitor','--system',*matches]
 h.command('dbus-monitor-start',shlex.join(cmd));time.sleep(1)

def checkpoint(label):
 # Raw native state plus real clocks and service identity at every boundary.
 h.command(label+'-clock-services',"date --iso-8601=ns; cat /proc/uptime; readlink -f /etc/localtime; timedatectl show; systemctl show systemd-timedated -p ActiveState -p MainPID -p InvocationID; busctl --system status org.freedesktop.timedate1; ps -eo pid,etimes,args | grep -E 'cosmic-settings-daemon|xdg-desktop-portal-cosmic'",required=False)
 state=h.snapshot(label)
 h.command(label+'-dbus-traffic','cat /var/tmp/qualification-auto-dbus.log',required=False)
 return state

def observe(label,expected,stimulus,timeout=40):
 start=time.monotonic();samples=[]
 while True:
  response=h.command(label+'-portal-sample','busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop org.freedesktop.portal.Settings Read ss org.freedesktop.appearance color-scheme',True).strip()
  samples.append({'elapsed_monotonic':time.monotonic()-start,'response':response})
  if re.search(r'\bu '+('1' if expected else '2')+r'\s*$',response) or time.monotonic()-start>=timeout:break
  time.sleep(2)
 state=h.frame(label);checkpoint(label)
 actual=f.raw_mode(state,'is_dark');auto=f.raw_mode(state,'auto_switch')
 portal_dark=bool(re.search(r'\bu 1\s*$',samples[-1]['response']))
 matched=portal_dark==expected and actual in (None,str(expected).lower()) and auto=='true'
 observation={'label':label,'stimulus':stimulus,'expected_dark':expected,'observed_native_is_dark':actual,'observed_portal_dark':portal_dark,'auto_switch':auto,'matched':matched,'samples':samples,'daemon_processes':[p for p in state['processes'] if p['exe'].endswith(('/cosmic-settings-daemon','/xdg-desktop-portal-cosmic'))]}
 h.summary['automatic']['observations'].append(observation)
 (ev/(label+'-observation.json')).write_text(json.dumps(observation,indent=2)+'\n');h.save()
 return observation

def automatic():
 f.dismiss_popup('before-auto')
 checkpoint('before-auto-enable')
 h.launch('/usr/bin/cosmic-settings',['appearance'],'cosmic-ui')
 _,path,_=h.screen('automatic-control-before')
 evidence=f.point(path,'Auto',f.CAL)
 if evidence['enabled']:raise RuntimeError('Auto already enabled; baseline not the expected native default')
 f.calibrated_click('automatic-native-toggle','Auto');h.screen('automatic-control-after');h.stop_ui()
 observe('startup-night',True,'Europe/London 23:00 configured BEFORE native login; startup schedule, UI Auto enabled after login')
 for label,cmd,dark,description in [
  ('live-noon','timedatectl set-time "2026-10-23 12:00:00"',False,'live real timedated SetTime; same timezone; no daemon restart'),
  ('live-timezone-paris','timedatectl set-timezone Europe/Paris',False,'live real timedated SetTimezone at daytime; native timezone/location recomputation'),
  ('live-night','timedatectl set-time "2026-10-23 23:00:00"',True,'live real timedated SetTime; same timezone; no daemon restart'),
  ('live-timezone-london','timedatectl set-timezone Europe/London',True,'live real timedated SetTimezone at night; native timezone/location recomputation')]:
  checkpoint(label+'-before')
  h.command(label+'-real-stimulus',cmd+' && date --iso-8601=ns')
  observe(label,dark,description)
 h.command('auto-complete-journal','journalctl -b --no-pager -n 5000',required=False)
 h.command('auto-monotonic-journal','journalctl -b -o short-monotonic --no-pager -n 5000',required=False)
 obs=h.summary['automatic']['observations']
 h.summary['automatic'].update(native='PASS' if all(x['matched'] for x in obs) else 'OBSERVED MISMATCH; see raw native observations',natural_solar_boundary='not exercised; all live stimuli are timedated operations',daemon_restart=False)
 h.save()

try:
 c.wait_agent();c.q=c.Channel('qmp.sock',True)
 monitor_start()
 # Initial state must precede any user settings daemon. No Halcyon in baseline.
 h.command('prelogin-user-processes',"ps -u probe -o pid,args; dpkg-query -W cosmic-ext-halcyon cosmic-ext-halcyon-islands 2>&1; test ! -e /usr/bin/halcyon; test ! -e /home/probe/.config/halcyon; ip -brief addr; ip route",required=False)
 if phase=='untouched':
  devices=c.q.call('query-block')
  (ev/'untouched-block-devices.json').write_text(json.dumps(devices,indent=2)+'\n')
  # QEMU may create an implicit EMPTY DVD drive. Reject inserted removable
  # media, not the harmless existence of its /dev/sr0 device node.
  if any(d.get('removable') and d.get('inserted') for d in devices):raise RuntimeError('Untouched overlay has inserted removable media')
  h.command('untouched-absence-assertion',"""python3 - <<'ABSENCE'
import json,os,pathlib
checks={'halcyon_binary_absent':not pathlib.Path('/usr/bin/halcyon').exists(),'halcyon_config_absent':not pathlib.Path('/home/probe/.config/halcyon').exists(),'user_settings_daemon_absent':True}
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  if p.stat().st_uid==1000 and os.readlink(p/'exe')=='/usr/bin/cosmic-settings-daemon':checks['user_settings_daemon_absent']=False
 except OSError:pass
print(json.dumps(checks,indent=2))
raise SystemExit(0 if all(checks.values()) else 1)
ABSENCE""")
 h.command('prelogin-clock','timedatectl set-ntp false && timedatectl set-timezone Europe/London && timedatectl set-time "2026-10-23 23:00:00" && timedatectl && date --iso-8601=ns')
 c.shot('000-prelogin-clock-configured')
 if host=='islands':h.command('fixture-local-bin','install -d -o 1000 -g 1000 /home/probe/.local/bin')
 c.login('auto-diagnosis-login');c.capture('login')
 checkpoint('fresh-login')
 if phase=='candidate':
  h.command('mount-inputs','mkdir -p /mnt/candidate && mount -o ro /dev/sr0 /mnt/candidate && cd /mnt/candidate && sha256sum -c SHA256SUMS')
  approved=(ev/'approved-inputs.sha256').read_text()
  h.command('independent-input-check',"cd /mnt/candidate && sha256sum -c /dev/stdin <<'INPUTS'\n"+approved+'INPUTS\n')
  h.install('candidate')
  if host=='islands':
   h.command('install-islands','dpkg --install /mnt/candidate/cosmic-ext-halcyon-islands_0.2.0_amd64.deb && dpkg --audit && apt-get check',timeout=120)
   h.command('enable-islands','halcyon-islands enable',True)
   c.guest("loginctl list-sessions --no-legend | awk '$3==\"probe\" {print $1}' | xargs -r -n1 loginctl terminate-session")
   time.sleep(15);c.login('auto-diagnosis-islands-login')
   h.command('islands-status','halcyon-islands status',True)
   state=h.snapshot('islands-running')
   if not any(p['exe'].endswith('/cosmic-panel-islands') and p['sha256']=='f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31' for p in state['processes']):raise RuntimeError('Matching Islands executable not running')
  for look in looks:
   try:f.select(look);f.cycle(look)
   except Exception as error:
    h.summary['errors'].append({'look':look,'blocked':str(error)});traceback.print_exc();h.screen('blocked-'+look);h.snapshot('blocked-'+look);h.stop_ui();f.dismiss_popup('blocked-'+look);h.save()
 automatic()
 h.stop_ui();c.capture('final');h.summary['execution_complete']=True
except Exception as error:
 h.summary['errors'].append({'fatal':str(error)});traceback.print_exc()
 try:h.screen('fatal');h.snapshot('fatal');c.capture('fatal')
 except Exception:traceback.print_exc()
finally:
 try:h.command('final-dbus-traffic','cat /var/tmp/qualification-auto-dbus.log',required=False)
 except Exception:traceback.print_exc()
 h.save()
 try:c.guest('sync; systemctl poweroff',5)
 except Exception:pass
 time.sleep(8)
# A captured upstream mismatch is a completed diagnostic, not a passing Auto gate.
raise SystemExit(0 if h.summary.get('execution_complete') and not h.summary['errors'] else 1)
