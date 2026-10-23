#!/usr/bin/python3
"""Untouched clock comparison, clean natural boundaries, independent Islands UI."""
import base64,hashlib,json,pathlib,re,shlex,sys,time,traceback
from datetime import datetime
from zoneinfo import ZoneInfo
import auto_session as a
f=a.f;h=a.h;c=a.c;ev=a.ev
lineage,host,task=sys.argv[2:5]
SRC=pathlib.Path(__file__).resolve().parent
SOLAR=json.loads((SRC/'acceptance/solar-boundary.json').read_text())
h.summary.update(scope='DEVELOPMENT untouched comparison / real natural sunrise / independent released Islands manual',phase=lineage,task=task,tested_looks=['calm','evening','focus','paper','rose'] if task=='manual' else (['calm'] if lineage=='candidate' else []),deferred=['Repair','passed stock manual cases','release'],automatic={'native':'not observed in this phase','injected_events':False,'observations':[]})

from acceptance_wrapper import media_assertion

def absence():
 h.command('untouched-absence',"""python3 - <<'ABSENCE'
import json,os,pathlib,subprocess
checks={'halcyon_binary_absent':not pathlib.Path('/usr/bin/halcyon').exists(),'halcyon_config_absent':not pathlib.Path('/home/probe/.config/halcyon').exists(),'user_settings_daemon_absent':True}
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  if p.stat().st_uid==1000 and os.readlink(p/'exe')=='/usr/bin/cosmic-settings-daemon':checks['user_settings_daemon_absent']=False
 except OSError:pass
for package in ['cosmic-ext-halcyon','cosmic-ext-halcyon-islands']:
 r=subprocess.run(['dpkg-query','-W','-f=${db:Status-Status}',package],capture_output=True,text=True)
 checks[package+'_absent']=r.returncode!=0 or r.stdout.strip()!='installed'
print(json.dumps(checks,indent=2))
raise SystemExit(0 if all(checks.values()) else 1)
ABSENCE""")

def native_source(require_geodata):
 blob=(SRC/'acceptance/source/data-timezone-geodata.bitcode-v0-6').read_bytes()
 h.command('native-geodata-embedding',"python3 - <<'SOURCE'\nimport base64,hashlib,json,pathlib\nblob=base64.b64decode("+repr(base64.b64encode(blob).decode())+")\nelf=pathlib.Path('/usr/bin/cosmic-settings-daemon').read_bytes()\nproof={'native_elf_sha256':hashlib.sha256(elf).hexdigest(),'embedded_geodata_sha256':hashlib.sha256(blob).hexdigest(),'embedded_geodata_bytes':len(blob),'embedded_offset':elf.find(blob)}\nprint(json.dumps(proof,indent=2))\nassert proof['native_elf_sha256']=='0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423'\nif "+str(require_geodata)+":assert proof['embedded_offset']>=0\nSOURCE")

def install_candidate():
 h.command('mount-inputs','candidate_device=$(blkid -L HALCYON_INPUTS) && test -n "$candidate_device" && mkdir -p /mnt/candidate && mount -o ro "$candidate_device" /mnt/candidate && findmnt /mnt/candidate && cd /mnt/candidate && sha256sum -c SHA256SUMS')
 approved=(ev/'approved-inputs.sha256').read_text()
 h.command('independent-input-check',"cd /mnt/candidate && sha256sum -c /dev/stdin <<'INPUTS'\n"+approved+'INPUTS\n');h.install('candidate')
 if host=='islands':
  h.command('install-islands','dpkg --install /mnt/candidate/cosmic-ext-halcyon-islands_0.2.0_amd64.deb && dpkg --audit && apt-get check',timeout=120)
  manifest=json.loads((ev/'package-build/cosmic-ext-halcyon-islands_0.2.0_amd64-payload.json').read_text())
  checks='\n'.join(digest+'  /'+name for name,digest in manifest.items())+'\n'
  encoded=base64.b64encode(checks.encode()).decode()
  h.command('payload-islands','echo '+shlex.quote(encoded)+' | base64 -d | sha256sum -c -',timeout=120)
  h.command('enable-islands','halcyon-islands enable',True)
  c.guest("loginctl list-sessions --no-legend | awk '$3==\"probe\" {print $1}' | xargs -r -n1 loginctl terminate-session")
  time.sleep(15);c.login('released-islands-login')
  h.command('islands-status','halcyon-islands status',True)
  state=h.snapshot('released-islands-running')
  actual=[p for p in state['processes'] if p['exe'].endswith('/cosmic-panel-islands')]
  if not actual or any(p['sha256']!='f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31' for p in actual):raise RuntimeError('Released matching Islands running identity mismatch')

def native_toggle():
 f.dismiss_popup('before-auto')
 h.launch('/usr/bin/cosmic-settings',['appearance'],'cosmic-ui')
 _,path,_=h.screen('native-auto-before')
 evidence=f.point(path,'Auto',f.CAL)
 if evidence['enabled']:raise RuntimeError('Auto unexpectedly already enabled before the real UI action')
 f.calibrated_click('native-auto-enable','Auto');h.screen('native-auto-enabled');h.stop_ui()

def clock_sample(label):
 out=h.command(label,"python3 -c 'import json,time; print(json.dumps({\"realtime\":time.time(),\"monotonic\":time.monotonic()}))'; busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop org.freedesktop.portal.Settings Read ss org.freedesktop.appearance color-scheme",True)
 stamp=json.loads(out.splitlines()[0]);match=re.search(r'\bu ([12])\s*$',out)
 if not match:raise RuntimeError('Actual portal mode could not be parsed: '+out[-250:])
 return {**stamp,'portal':out.splitlines()[-1],'dark':match[1]=='1','controller_monotonic':time.monotonic()}

def natural():
 # This clean overlay has had exactly one pre-login clock setup. No manual
 # native mode selection, second SetTime, timezone change, or restart follows.
 if lineage=='candidate':f.select('calm')
 native_toggle()
 before=h.frame('natural-before-dark');f.verify_mode('natural-before',before,True,True)
 a.checkpoint('natural-window-before')
 first=clock_sample('natural-first-sample');boundary=SOLAR['sunrise_epoch']
 if boundary-first['realtime']<30:raise RuntimeError('Setup finished too near/past boundary; no clock reset or rerun attempted')
 baseline_ids=[(p['pid'],p['exe'],p['sha256']) for p in before['processes'] if p['exe'].endswith(('/cosmic-settings-daemon','/xdg-desktop-portal-cosmic','/cosmic-panel','/cosmic-panel-islands'))]
 samples=[first];deadline=time.monotonic()+600;transition=None;near_captured=False
 while time.monotonic()<deadline:
  time.sleep(5);s=clock_sample('natural-sample');samples.append(s)
  drift=(s['realtime']-first['realtime'])-(s['monotonic']-first['monotonic'])
  if abs(drift)>2:raise RuntimeError('Wall/monotonic trajectory indicates clock step or excessive drift during natural window')
  # Persist samples throughout the bounded window, including on later failure.
  (ev/'natural-samples.json').write_text(json.dumps(samples,indent=2)+'\n')
  if not near_captured and 0<boundary-s['realtime']<30:
   h.frame('natural-near-boundary-dark');near_captured=True
  if not s['dark']:
   transition=s;break
  if s['realtime']>boundary+60:break
 after=h.frame('natural-after-boundary');a.checkpoint('natural-window-after')
 final_ids=[(p['pid'],p['exe'],p['sha256']) for p in after['processes'] if p['exe'].endswith(('/cosmic-settings-daemon','/xdg-desktop-portal-cosmic','/cosmic-panel','/cosmic-panel-islands'))]
 raw=h.command('natural-window-dbus-traffic','cat /var/tmp/qualification-auto-dbus.log')
 # Exactly one SetTime/SetTimezone before login; no timedated stimulus follows.
 set_time_calls=len(re.findall(r'^method call .*member=SetTime\s*$',raw,re.M))
 timezone_calls=len(re.findall(r'^method call .*member=SetTimezone\s*$',raw,re.M))
 matched=transition is not None and boundary-2<=transition['realtime']<=boundary+60 and baseline_ids==final_ids and set_time_calls==1 and timezone_calls==1
 proof={'boundary':SOLAR,'stimulus':'one pre-login clock/timezone setup then unmodified time progression across actual calculated sunrise','first':first,'transition':transition,'last':samples[-1],'samples':samples,'daemon_host_before':baseline_ids,'daemon_host_after':final_ids,'stable_native_processes':baseline_ids==final_ids,'set_time_calls_entire_phase':set_time_calls,'set_timezone_calls_entire_phase':timezone_calls,'matched':matched,'manual_mode_writes':False,'synthetic_events':False,'daemon_restart_during_window':False}
 (ev/'natural-transition-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
 if matched:f.verify_mode('natural-after',after,False,True)
 h.summary['automatic'].update(native='PASS: real natural sunrise Dark-to-Light' if matched else 'FAIL/BLOCKED: natural sunrise did not meet assertions',natural=proof,injected_events=False)
 h.summary['scenarios'].append({'scenario':'real natural sunrise Dark-to-Light','matched':matched,'settings_widgets_stopped':True})
 h.save()
 if not matched:raise RuntimeError('Natural transition assertions failed; exact samples/screens retained')

def main():
 try:
  c.wait_agent();c.q=c.Channel('qmp.sock',True);time.sleep(15)
  devices=c.q.call('query-block');(ev/'qmp-block-devices.json').write_text(json.dumps(devices,indent=2)+'\n')
  proof=media_assertion(devices,str(c.work.parent/'candidate.iso') if lineage=='candidate' else None)
  (ev/'media-identity-assertion.json').write_text(json.dumps(proof,indent=2)+'\n')
  a.monitor_start();h.command('dbus-monitor-active','systemctl is-active qualification-auto-dbus.service')
  if lineage=='untouched':absence()
  native_source(task=='natural')
  h.command('offline-proof','ip -brief addr; ip route; systemctl is-active dbus')
  if task=='natural':
   start=SOLAR['sunrise_epoch']-480
   initial=datetime.fromtimestamp(start,ZoneInfo(SOLAR['timezone'])).strftime('%Y-%m-%d %H:%M:%S')
   (ev/'solar-setup.json').write_text(json.dumps({'boundary':SOLAR,'lead_seconds':480,'prelogin_epoch':start,'prelogin_local':initial},indent=2)+'\n')
   h.command('prelogin-natural-clock','timedatectl set-ntp false && timedatectl set-timezone '+shlex.quote(SOLAR['timezone'])+' && timedatectl set-time '+shlex.quote(initial)+' && date --iso-8601=ns && date +%s && cat /proc/uptime')
   h.command('prelogin-clock-assert',"python3 -c 'import time; assert abs(time.time()-"+str(start)+")<5'")
  elif task=='clock':h.command('prelogin-clock','timedatectl set-ntp false && timedatectl set-timezone Europe/London && timedatectl set-time "2026-10-23 23:00:00" && timedatectl && date --iso-8601=ns')
  if host=='islands':h.command('fixture-local-bin','install -d -o 1000 -g 1000 /home/probe/.local/bin')
  c.shot('000-prelogin');c.login('acceptance-login');c.capture('login')
  if lineage=='candidate':install_candidate()
  if task=='clock':a.automatic()
  elif task=='natural':natural()
  elif task=='manual':
   for look in ['calm','evening','focus','paper','rose']:
    try:f.select(look);f.cycle(look)
    except Exception as error:
     h.summary['errors'].append({'look':look,'blocked':str(error)});traceback.print_exc();h.screen('blocked-'+look);h.snapshot('blocked-'+look);h.stop_ui();f.dismiss_popup('blocked-'+look);h.save()
  else:raise RuntimeError('Unknown explicitly scoped phase')
  h.stop_ui();c.capture('final');h.summary['execution_complete']=True
 except Exception as error:
  h.summary['errors'].append({'fatal':str(error)});traceback.print_exc()
  try:h.screen('fatal');h.snapshot('fatal');c.capture('fatal')
  except Exception:traceback.print_exc()
 finally:
  try:
   h.command('final-dbus-traffic','cat /var/tmp/qualification-auto-dbus.log',required=False)
   h.command('final-monotonic-journal','journalctl -b -o short-monotonic --no-pager -n 5000',required=False)
  except Exception:traceback.print_exc()
  h.save()
  try:c.guest('sync; systemctl poweroff',5)
  except Exception:pass
  time.sleep(8)
 raise SystemExit(0 if h.summary.get('execution_complete') and not h.summary['errors'] else 1)

if __name__ == "__main__":main()
