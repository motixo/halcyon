#!/usr/bin/python3
"""Controller-only download of bounded evidence; never download retained guests.
GH_TOKEN must be set privately by the controller. No credential is printed.
"""
import hashlib,json,pathlib,stat,subprocess,sys,zipfile
run,destination=sys.argv[1:]
assert run.isdigit()
root=pathlib.Path(destination);root.mkdir(parents=True,exist_ok=True)
repo='repos/profilaz3/halcyon'
response=json.loads(subprocess.check_output(['gh','api',repo+'/actions/runs/'+run+'/artifacts']))
(root/'artifacts.json').write_text(json.dumps(response,indent=2)+'\n')
for item in response['artifacts']:
 if not item['name'].startswith(('theme-session-','theme-followup-','theme-auto-','theme-acceptance-')):continue
 assert not item['expired'] and item['size_in_bytes']<150000000
 dest=root/item['name'];marker=dest/'controller-verified-artifact.json'
 if marker.exists() and json.loads(marker.read_text())['digest']==item['digest']:continue
 archive=root/(item['name']+'.zip')
 with archive.open('wb') as f:subprocess.run(['gh','api',repo+'/actions/artifacts/'+str(item['id'])+'/zip'],stdout=f,check=True)
 assert 'sha256:'+hashlib.file_digest(archive.open('rb'),'sha256').hexdigest()==item['digest']
 with zipfile.ZipFile(archive) as z:
  for f in z.infolist():
   p=pathlib.PurePosixPath(f.filename)
   assert not p.is_absolute() and '..' not in p.parts and not stat.S_ISLNK(f.external_attr>>16)
  z.extractall(dest)
 marker.write_text(json.dumps({k:item[k] for k in ['id','name','digest','size_in_bytes','expires_at']},indent=2)+'\n')
 # Preserve the authenticated original ZIP alongside its extracted evidence.
 if not (dest/'theme-results.json').exists():
  print(json.dumps({'artifact':item['id'],'name':item['name'],'execution':'pre-VM infrastructure failure; original verified artifact retained'}),flush=True);continue
 r=json.loads((dest/'theme-results.json').read_text())
 print(json.dumps({'artifact':item['id'],'name':item['name'],'frames':len(r['frames']),'scenarios':len(r['scenarios']),'errors':r['errors'],'automatic':r['automatic']}),flush=True)
