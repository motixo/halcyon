#!/usr/bin/python3
"""CI controller only. Bounded, explicitly addressed infra repair on same VM.
Never passes CI credentials to guest; no product/package/base changes allowed.
"""
import hashlib,json,os,pathlib,re,subprocess,sys,time
work=pathlib.Path(sys.argv[1]);cohort=sys.argv[2];ev=work/'evidence'
state=json.loads((work/'lockup-state.json').read_text())
if state.get('preflight_passed'):sys.exit(0)
if not state.get('fixture_ready') or any(x.get('category')=='core' for x in state.get('failures',[])):
 raise SystemExit('Not eligible for infra repair: fixture incomplete or actual core input failure')
base=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();run=os.environ['GITHUB_RUN_ID'];repo=os.environ['GITHUB_REPOSITORY']
records=[];deadline=time.monotonic()+420
allowed=re.compile(r'^private/qualification-021/lockup/[^/]+\.(py|json|sha256)$')
def api(path):return json.loads(subprocess.check_output(['gh','api','repos/'+repo+'/'+path],timeout=20))
print('Waiting up to seven minutes for an explicitly addressed, recorded infra-only repair',flush=True)
while time.monotonic()<deadline:
 head=api('commits/ci/qualification-probe-021');sha=head['sha'];message=head['commit']['message'].splitlines()[0]
 if sha!=base and message.startswith('infra: r3 live harness repair '+run):
  comparison=api('compare/'+base+'...'+sha);files=comparison.get('files',[])
  if comparison['status']!='ahead' or not files or len(files)>10 or any(not allowed.fullmatch(x['filename']) or x['status'] in ['removed','renamed'] for x in files):
   raise RuntimeError('Repair rejected: ancestry or strict infra-only changed-file boundary')
  subprocess.run(['git','fetch','--no-tags','--depth=100','origin',sha],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=45)
  revision=ev/('live-repair-'+sha);revision.mkdir()
  hashes={}
  for f in files:
   path=f['filename'];data=subprocess.check_output(['git','show',sha+':'+path],timeout=20)
   if len(data)>150000:raise RuntimeError('Repair helper exceeds bounded size')
   target=pathlib.Path(path);(revision/target.name).write_bytes(data);hashes[path]=hashlib.sha256(data).hexdigest();target.write_bytes(data)
  (revision/'identity.json').write_text(json.dumps({'base':base,'revision':sha,'files':hashes,'run':run},indent=2)+'\n')
  # Existing VM, installed packages, clock and fixture contents are retained.
  result=subprocess.run(['sudo','python3','private/qualification-021/lockup/session.py',str(work),'public',cohort,'recover'],timeout=180)
  raise SystemExit(result.returncode)
 records.append({'time':time.time(),'observed_head':sha,'eligible_message':message.startswith('infra: r3 live harness repair '+run)})
 (ev/'live-repair-poll.json').write_text(json.dumps(records,indent=2)+'\n');time.sleep(15)
raise SystemExit('No addressed infra repair received within seven-minute bound; no further allocation started')
