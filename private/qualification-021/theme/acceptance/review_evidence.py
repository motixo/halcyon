#!/usr/bin/python3
"""Controller-only read-only evidence review; no native fixture/palette writes."""
import hashlib,importlib.util,json,pathlib,re,subprocess,sys
from collections import Counter
THEME=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('pixels',THEME/'analyze-evidence.py');pixels=importlib.util.module_from_spec(spec);spec.loader.exec_module(pixels)
DAEMON='0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423'
PORTAL='73d5864b661f5e4d5995e788869b26a9b9091edf723277bae41f9cb38916d56b'
PRODUCT='d070ad5d8bf1362fcd063ee31a56700473744f2f03723948a8357961dd49736b'
PANELS={'stock':'e66271e6114ac21754f2969c910a36dfea3b958f949d74d9e7e22595caf0f937','islands':'f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31'}
def contrast(a,b):return (max(pixels.luminance(a),pixels.luminance(b))+.05)/(min(pixels.luminance(a),pixels.luminance(b))+.05)
def sha(path):return hashlib.file_digest(path.open('rb'),'sha256').hexdigest()
def review(root):
 source=(root/'infra-commit.txt').read_text().strip();manifest=json.loads((root/'harness-SHA256SUMS.json').read_text());harness={}
 for name,digest in manifest.items():
  actual=sha(root/'harness'/name);committed=hashlib.sha256(subprocess.check_output(['git','show',source+':private/qualification-021/theme/'+name])).hexdigest()
  assert digest==actual==committed,name;harness[name]=digest
 result={'artifact':json.loads((root/'controller-verified-artifact.json').read_text()),'executed_infra':source,'verified_harness_files':harness,'phases':{}}
 for phase in sorted(root.glob('*/theme-results.json')):
  folder=phase.parent;s=json.loads(phase.read_text());item={'errors':s['errors'],'scenarios':s['scenarios'],'automatic':s['automatic'],'media':json.loads((folder/'media-identity-assertion.json').read_text()) if (folder/'media-identity-assertion.json').exists() else None,'frames':[],'running_product':[],'payload_checks':{}}
  for payload in ['payload-candidate.txt','payload-islands.txt']:
   if (folder/payload).exists():
    lines=(folder/payload).read_text().splitlines();assert lines and all(l.endswith(': OK') for l in lines);item['payload_checks'][payload]=len(lines)
  for state_file in sorted(folder.glob('running-*-state.txt')):
   state=json.loads(state_file.read_text());actual=[p for p in state['processes'] if p['exe']=='/usr/bin/halcyon'];assert all(p['sha256']==PRODUCT for p in actual);item['running_product'].extend(actual)
  for f in s['frames']:
   state=json.loads((folder/(f['name']+'-state.txt')).read_text());files=state['files'];procs=state['processes'];assert f['stopped'],f['name']
   assert any(p['exe']=='/usr/bin/cosmic-settings-daemon' and p['sha256']==DAEMON for p in procs)
   assert any(p['exe']=='/usr/libexec/xdg-desktop-portal-cosmic' and p['sha256']==PORTAL for p in procs)
   assert any(p['exe'].endswith('/cosmic-panel-islands' if s['host']=='islands' else '/cosmic-panel') and p['sha256']==PANELS[s['host']] for p in procs)
   def raw(ending):return next((bytes.fromhex(v['hex']).decode(errors='replace') for k,v in files.items() if k.endswith(ending)),None)
   portal=(folder/(f['name']+'-portal.txt')).read_text().splitlines()[0];dark=bool(re.search(r'\bu 1$',portal));variant='Dark' if dark else 'Light';palette=raw('CosmicTheme.'+variant+'/v2/background') or raw('CosmicTheme.'+variant+'/v1/background');role=pixels.color(palette,'on') if palette else None
   box=(510,17,611,30) if s['host']=='islands' else (590,17,691,30);data=pixels.image_pixels(folder/(f['name']+'.png'),box);bg=Counter(data).most_common(1)[0][0]
   ink=[v for v in data if pixels.near(v,role,35)] if role else [v for v in data if contrast(bg,v)>=4.5]
   fg=Counter(ink).most_common(1)[0][0] if ink else None
   frame={'name':f['name'],'portal':portal,'native_mode':raw('CosmicTheme.Mode/v1/is_dark'),'auto':raw('CosmicTheme.Mode/v1/auto_switch'),'clock_label_bounds_inside_bar':box,'native_foreground_role':role,'actual_surface':bg,'actual_ink':fg,'glyph_pixels':len(ink),'actual_contrast':contrast(bg,fg) if fg else None,'stopped_ui':True,'product_identity':json.loads(raw('.config/halcyon/config.json')) if raw('.config/halcyon/config.json') else None}
   item['frames'].append(frame)
  if (folder/'natural-transition-proof.json').exists():
   n=json.loads((folder/'natural-transition-proof.json').read_text());samples=n['samples'];boundary=n['boundary']['sunrise_epoch'];light=[v for v in samples if not v['dark']];dark=[v for v in samples if v['dark']]
   stable=n['daemon_host_before']==n['daemon_host_after'];maxdrift=max(abs((v['realtime']-samples[0]['realtime'])-(v['monotonic']-samples[0]['monotonic'])) for v in samples)
   actual={'dark_samples_before_boundary':sum(v['realtime']<boundary for v in dark),'first_light_after_boundary_seconds':light[0]['realtime']-boundary if light else None,'max_realtime_monotonic_drift':maxdrift,'stable_native_processes':stable,'set_time_calls':n['set_time_calls_entire_phase'],'set_timezone_calls':n['set_timezone_calls_entire_phase']}
   actual['independent_natural_pass']=bool(dark and light and dark[0]['realtime']<boundary and boundary-2<=light[0]['realtime']<=boundary+60 and stable and maxdrift<2 and actual['set_time_calls']==1 and actual['set_timezone_calls']==1)
   item['independent_natural_observation']=actual
  result['phases'][folder.name]=item
 (root/'independent-acceptance-review.json').write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({'root':str(root),'artifact':result['artifact']['id'],'verified_harness_files':len(harness),'phases':{k:{'errors':v['errors'],'scenarios':v['scenarios'],'natural':v.get('independent_natural_observation'),'frames':[(f['name'],f['actual_contrast'],f['glyph_pixels']) for f in v['frames']]} for k,v in result['phases'].items()}},indent=2))
if __name__=='__main__':
 for path in sys.argv[1:]:review(pathlib.Path(path))
