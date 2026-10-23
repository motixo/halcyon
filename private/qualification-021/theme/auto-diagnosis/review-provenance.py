#!/usr/bin/python3
import hashlib,json,pathlib,subprocess,sys
root=pathlib.Path(sys.argv[1]);p=root/'candidate';sha=(root/'infra-commit.txt').read_text().strip()
proof={'executed_infra_commit':sha,'artifact':json.loads((root/'controller-verified-artifact.json').read_text()),'harness':{},'manual':{},'auto':{}}
for line in (root/'harness-SHA256SUMS.txt').read_text().splitlines():
 digest,filename=line.split(None,1);name=pathlib.Path(filename.strip()).name;src='private/qualification-021/theme/'+name;committed=hashlib.sha256(subprocess.check_output(['git','show',sha+':'+src])).hexdigest();copied=hashlib.sha256((root/name).read_bytes()).hexdigest() if (root/name).exists() else None
 assert digest==committed and (copied is None or copied==digest)
 proof['harness'][name]={'recorded_sha256':digest,'matches_executed_commit':True,'copied_bytes_match':copied==digest if copied else 'reconstructable from exact commit'}
expected_daemon='0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423';expected_portal='73d5864b661f5e4d5995e788869b26a9b9091edf723277bae41f9cb38916d56b'
r=json.loads((p/'theme-results.json').read_text());proof['manual'].update(scenarios=r['scenarios'],identities=r['identities']);proof['auto']['observations']=[{k:o[k] for k in ['label','expected_dark','observed_native_is_dark','observed_portal_dark','auto_switch','matched']} for o in r['automatic']['observations']]
for f in r['frames']:
 state=json.loads((p/(f['name']+'-state.txt')).read_text());assert f['stopped']
 assert any(v['exe']=='/usr/bin/cosmic-settings-daemon' and v['sha256']==expected_daemon for v in state['processes'])
 assert any(v['exe']=='/usr/libexec/xdg-desktop-portal-cosmic' and v['sha256']==expected_portal for v in state['processes'])
proof['native_cohort_verified_frames']=len(r['frames'])
payload=(p/'payload-candidate.txt').read_text().splitlines();assert payload and all(s.endswith(': OK') for s in payload)
proof['installed_payload_checked_files']=len(payload)
proof['actual_running_product']={}
for look in ['paper','rose']:
 state=json.loads((p/('running-'+look+'-state.txt')).read_text())
 actual=[v for v in state['processes'] if v['exe']=='/usr/bin/halcyon']
 assert actual and all(v['sha256']=='d070ad5d8bf1362fcd063ee31a56700473744f2f03723948a8357961dd49736b' for v in actual)
 proof['actual_running_product'][look]=actual
identities=[]
for o in r['automatic']['observations']:
 frame=next(f for f in r['frames'] if f['name'].endswith('-'+o['label']))
 state=json.loads((p/(frame['name']+'-state.txt')).read_text());conf=json.loads(bytes.fromhex(state['files']['.config/halcyon/config.json']['hex']))
 identities.append((conf['appearance']['family'],conf['theme'],conf['appearance']['native_mode_palettes']))
assert all(t==('glass','rose',1) for t in identities)
proof['auto']['unchanged_product_identity']=identities[0]
(root/'controller-provenance-review.json').write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps({'artifact':proof['artifact']['id'],'harness_files':len(proof['harness']),'frames_with_stopped_UI_and_verified_daemon_portal':proof['native_cohort_verified_frames'],'installed_payload_files':len(payload),'auto_identity_preserved':True}))
