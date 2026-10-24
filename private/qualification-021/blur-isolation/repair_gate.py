"""One addressed harness-only correction, before any trigger; controller token only."""
import base64,hashlib,json,os,pathlib,subprocess,sys,time
work=pathlib.Path(sys.argv[1]);context=sys.argv[2];active=work/'active-blur-case.json'
if not active.exists():sys.exit(0)
a=json.loads(active.read_text())
if a['context']!=context:sys.exit(0)
case=pathlib.Path(a['case']);ev=case/'evidence';state=json.loads((ev/'isolation-results.json').read_text())
if state.get('complete') or not state.get('repair_eligible') or (work/'live-repair-used.json').exists():sys.exit(0)
subprocess.run(['sudo','chmod','-R','a+rwX',str(ev)],check=True)
base=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();run=os.environ['GITHUB_RUN_ID'];repo=os.environ['GITHUB_REPOSITORY'];prefix='infra: blur r2 live harness repair '+run
allowed={'private/qualification-021/blur-isolation/'+n for n in ['session.py','configuration.py','focus_pixels.py','preflight.sha256','test_configuration.py','validation.txt']}
def api(path):return json.loads(subprocess.check_output(['gh','api','repos/'+repo+'/'+path],timeout=25))
print('Addressed harness repair window <=7min:',prefix,flush=True)
(ev/'original-failed-results.json').write_text(json.dumps(state,indent=2)+'\n')
deadline=time.monotonic()+420
while time.monotonic()<deadline:
 head=api('commits/ci/qualification-probe-021');sha=head['sha'];message=head['commit']['message'].splitlines()[0]
 if sha!=base and message.startswith(prefix):
  compare=api('compare/'+base+'...'+sha);files=compare.get('files',[])
  if compare['status']!='ahead' or not files or len(files)>6 or any(f['filename'] not in allowed or f['status'] in ['removed','renamed'] for f in files):raise RuntimeError('Repair outside strict harness/ancestry boundary')
  saved=ev/('live-repair-'+sha);saved.mkdir();hashes={}
  for f in files:
   name=f['filename'];target=pathlib.Path(name);data=base64.b64decode(api('contents/'+name+'?ref='+sha)['content']);assert len(data)<180000
   old=target.read_bytes();(saved/('old-'+target.name)).write_bytes(old);(saved/('new-'+target.name)).write_bytes(data)
   hashes[name]={'old':hashlib.sha256(old).hexdigest(),'new':hashlib.sha256(data).hexdigest()};target.write_bytes(data)
  receipt={'base':base,'revision':sha,'files':hashes,'original_error':state['setup_error'],'same_vm':True,'trigger_started':False,'configuration_reimported':False,'fixture_rewritten':False}
  (saved/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(work/'live-repair-used.json').write_text(json.dumps(receipt,indent=2)+'\n')
  subprocess.run(['python3','private/qualification-021/blur-isolation/test_configuration.py'],check=True,timeout=30)
  # sudo resets controller credentials; native user launch further allowlists guest environment.
  rc=subprocess.run(['sudo','python3','private/qualification-021/blur-isolation/session.py',str(case),'public','stock',context,a['variant'],'recover'],timeout=180).returncode
  raise SystemExit(rc)
 time.sleep(15)
raise SystemExit('No addressed repair within seven minutes; preserve error and continue independent comparisons')
