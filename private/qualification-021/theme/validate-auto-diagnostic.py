#!/usr/bin/python3
"""Require actual stock evidence before spending remaining serial Islands phase."""
import json,pathlib,sys
root=pathlib.Path(sys.argv[1]);proof={'gate':'stock native diagnostic method validated before serial Islands overlay','checks':{},'failures':[]}
try:
 results={k:json.loads((root/k/'theme-results.json').read_text()) for k in ['untouched','candidate']}
 for phase,r in results.items():
  obs=r['automatic']['observations'];by={o['label']:o for o in obs}
  assert r.get('execution_complete') and len(obs)==5,(phase,'incomplete diagnostic')
  assert by['startup-night']['matched'],(phase,'night startup unvalidated')
  assert by['live-timezone-paris']['matched'] and by['live-timezone-london']['matched'],(phase,'native live timezone recomputation unvalidated')
  daemon_ids={p['pid'] for o in obs for p in o['daemon_processes'] if p['exe']=='/usr/bin/cosmic-settings-daemon'}
  assert len(daemon_ids)==1,(phase,'settings daemon process changed')
  assert all(o['auto_switch']=='true' for o in obs),(phase,'Auto not persisted')
  traffic=(root/phase/'final-dbus-traffic.txt').read_text()
  assert 'member=SetTime' in traffic and 'member=SetTimezone' in traffic,(phase,'actual timedated method traffic missing')
  assert 'member=PropertiesChanged' in traffic and '"Timezone"' in traffic,(phase,'actual timezone signal missing')
  proof['checks'][phase]={'stable_daemon_pid':list(daemon_ids),'matches':{k:v['matched'] for k,v in by.items()},'actual_timedated_traffic':True}
 assert proof['checks']['untouched']['matches']==proof['checks']['candidate']['matches'],'baseline/candidate do not agree; inspect before Islands Auto'
 proof['validated']=True
except Exception as error:
 proof['validated']=False;proof['failures'].append(str(error))
(root/'stock-diagnostic-validation.json').write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps(proof,indent=2))
raise SystemExit(0 if proof['validated'] else 1)
