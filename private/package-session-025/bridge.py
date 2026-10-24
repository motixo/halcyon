"""Runner controller-only Actions evidence transport; no credentials enter guest."""
import os,pathlib,json,time,subprocess,urllib.request,base64
W=pathlib.Path(os.environ['WORK']);C=W/'control';C.mkdir(exist_ok=True)
repo=os.environ['GITHUB_REPOSITORY'];token=os.environ['GH_TOKEN'];branch='ci/qualification-inputs-025'
action=next(pathlib.Path(os.environ['RUNNER_WORKSPACE']).parent.glob('_actions/actions/upload-artifact/*/dist/upload/index.js'))
end=time.monotonic()+70*60;done=set()
while time.monotonic()<end:
 if (C/'bridge-stop').exists():break
 for path in sorted(C.glob('checkpoint-*.json')):
  if path.name in done:continue
  obj=json.loads(path.read_text());seq=obj['seq'];folder=W/'evidence'
  env=dict(os.environ,INPUT_NAME=f'package-session-checkpoint-{seq}-{os.environ["GITHUB_RUN_ID"]}',INPUT_PATH=str(folder),INPUT_RETENTION_DAYS='90',**{'INPUT_RETENTION-DAYS':'90','INPUT_IF-NO-FILES-FOUND':'error','INPUT_COMPRESSION-LEVEL':'6'})
  # Artifact service tokens stay in this controller process/action child only.
  r=subprocess.run(['node',str(action)],env=env,timeout=180)
  if r.returncode:raise SystemExit('checkpoint upload failed')
  done.add(path.name);(C/f'uploaded-{seq}').touch()
 try:
  req=urllib.request.Request(f'https://api.github.com/repos/{repo}/contents/private/package-session-control-025.json?ref={branch}',headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json'})
  with urllib.request.urlopen(req,timeout=15) as r:obj=json.load(r)
  data=base64.b64decode(obj['content']);assert len(data)<100000
  cmd=json.loads(data);seq=cmd['seq'];target=C/f'command-{seq}.json'
  if not target.exists():tmp=target.with_suffix('.tmp');tmp.write_bytes(data);tmp.replace(target)
 except Exception as e:print('control poll '+type(e).__name__,flush=True)
 time.sleep(5)
