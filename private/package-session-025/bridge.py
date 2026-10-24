"""Runner controller-only Actions evidence transport; no credentials enter guest."""
import os,pathlib,json,time,subprocess,urllib.request,urllib.error,base64,sys,hashlib,zipfile
W=pathlib.Path(os.environ['WORK']);C=W/'control';C.mkdir(exist_ok=True)
repo=os.environ['GITHUB_REPOSITORY'];token=os.environ['GH_TOKEN'];branch='ci/qualification-inputs-025'
action=next(pathlib.Path(os.environ['RUNNER_WORKSPACE']).parent.glob('_actions/actions/upload-artifact/*/dist/upload/index.js'))

required=('ACTIONS_RUNTIME_TOKEN','ACTIONS_RESULTS_URL','ACTIONS_RUNTIME_URL','GITHUB_RUN_ID','GITHUB_RUN_ATTEMPT','GH_TOKEN')
missing=[k for k in required if not os.environ.get(k)]
if missing:raise RuntimeError('controller action context missing names: '+','.join(missing))
if sys.argv[1:]==['--preflight']:
 folder=W/'checkpoint-preflight';folder.mkdir(exist_ok=False)
 (folder/'receipt.json').write_text(json.dumps({'purpose':'small pre-download checkpoint transport test','run':os.environ['GITHUB_RUN_ID'],'credentials_exported':False}))
 output=W/'checkpoint-action-outputs.txt'
 output.touch(exist_ok=False) # @actions/core requires an existing GITHUB_OUTPUT file
 env=dict(os.environ,INPUT_NAME='package-session-transport-preflight-'+os.environ['GITHUB_RUN_ID'],INPUT_PATH=str(folder),INPUT_OVERWRITE='false',**{'INPUT_INCLUDE-HIDDEN-FILES':'false','INPUT_RETENTION-DAYS':'90','INPUT_IF-NO-FILES-FOUND':'error','INPUT_COMPRESSION-LEVEL':'6','GITHUB_OUTPUT':str(output)})
 subprocess.run(['node',str(action)],env=env,timeout=45,check=True)
 values={};lines=iter(output.read_text().splitlines())
 for line in lines:
  if '<<' in line:
   name,delimiter=line.split('<<',1);parts=[]
   for value in lines:
    if value==delimiter:break
    parts.append(value)
   else:raise RuntimeError('unterminated action output')
   values[name]='\n'.join(parts)
  elif '=' in line:
   name,value=line.split('=',1);values[name]=value
 artifact=int(values['artifact-id']);digest=values['artifact-digest']
 req=urllib.request.Request(f'https://api.github.com/repos/{repo}/actions/artifacts/{artifact}',headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json'})
 with urllib.request.urlopen(req,timeout=12) as r:metadata=json.load(r)
 assert metadata['workflow_run']['id']==int(os.environ['GITHUB_RUN_ID']) and metadata['digest']=='sha256:'+digest
 # Actual tiny ZIP retrieval and independent disk rehash, not metadata alone.
 assert metadata['id']==artifact and 0<metadata['size_in_bytes']<1024*1024 and not metadata['expired']
 class NoRedirect(urllib.request.HTTPRedirectHandler):
  def redirect_request(self,*a,**kw):return None
 request=urllib.request.Request(f'https://api.github.com/repos/{repo}/actions/artifacts/{artifact}/zip',headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json'})
 try:
  response=urllib.request.build_opener(NoRedirect).open(request,timeout=12);response.close();raise RuntimeError('expected artifact redirect missing')
 except urllib.error.HTTPError as error:
  if error.code!=302:raise RuntimeError('artifact retrieval API failed, HTTP '+str(error.code)) from None
  location=error.headers['Location'];error.close()
 assert location.startswith('https://')
 archive=W/'checkpoint-preflight.zip';h=hashlib.sha256();count=0
 try:
  with urllib.request.urlopen(location,timeout=20) as src,archive.open('xb') as dst:
   while chunk:=src.read(65536):
    count+=len(chunk)
    if count>metadata['size_in_bytes']:raise RuntimeError('artifact size bound')
    dst.write(chunk);h.update(chunk)
   dst.flush();os.fsync(dst.fileno())
 except Exception:raise RuntimeError('bounded tiny artifact download failed; redirect suppressed') from None
 assert count==metadata['size_in_bytes'] and h.hexdigest()==digest
 with archive.open('rb') as stream:assert hashlib.file_digest(stream,'sha256').hexdigest()==digest
 with zipfile.ZipFile(archive) as z:
  assert z.namelist()==['receipt.json'] and z.getinfo('receipt.json').file_size<4096
  assert z.read('receipt.json')==(folder/'receipt.json').read_bytes()
 archive.chmod(0o444)
 # Also verify exact controller read route, never require a guest credential.
 req=urllib.request.Request(f'https://api.github.com/repos/{repo}/git/ref/heads/{branch}',headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json'})
 with urllib.request.urlopen(req,timeout=12) as r:ref=json.load(r)
 (W/'evidence/checkpoint-transport-preflight.json').write_text(json.dumps({'passed':True,'artifact':artifact,'digest':digest,'branch_head':ref['object']['sha'],'action_context_present':True,'no_credentials_recorded':True,'api_zip_retrieved':True,'downloaded_bytes':count,'independent_zip_rehash':True,'exact_member_bytes_match':True},indent=2))
 raise SystemExit(0)

end=time.monotonic()+70*60;done=set()
while time.monotonic()<end:
 if (C/'bridge-stop').exists():break
 for path in sorted(C.glob('checkpoint-*.json')):
  if path.name in done:continue
  obj=json.loads(path.read_text());seq=obj['seq'];folder=W/'evidence'
  env=dict(os.environ,INPUT_NAME=f'package-session-checkpoint-{seq}-{os.environ["GITHUB_RUN_ID"]}',INPUT_PATH=str(folder),INPUT_OVERWRITE='false',**{'INPUT_INCLUDE-HIDDEN-FILES':'false','INPUT_RETENTION-DAYS':'90','INPUT_IF-NO-FILES-FOUND':'error','INPUT_COMPRESSION-LEVEL':'6'})
  # Artifact service tokens stay in this controller process/action child only.
  r=subprocess.run(['node',str(action)],env=env,timeout=180)
  if r.returncode:raise SystemExit('checkpoint upload failed')
  done.add(path.name);(C/f'uploaded-{seq}').touch()
 try:
  req=urllib.request.Request(f'https://api.github.com/repos/{repo}/contents/private/package-session-control-025.json?ref={branch}',headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json'})
  with urllib.request.urlopen(req,timeout=15) as r:obj=json.load(r)
  data=base64.b64decode(obj['content']);assert len(data)<100000
  cmd=json.loads(data)
  if cmd.get('run')!=int(os.environ['GITHUB_RUN_ID']):raise ValueError('command is not bound to this run')
  seq=cmd['seq'];target=C/f'command-{seq}.json'
  if not target.exists():tmp=target.with_suffix('.tmp');tmp.write_bytes(data);tmp.replace(target)
 except Exception as e:print('control poll '+type(e).__name__,flush=True)
 time.sleep(5)
