#!/usr/bin/python3
"""Controller-only small evidence; owner token remains in controller env."""
import hashlib,json,pathlib,stat,subprocess,sys,zipfile
run,output=sys.argv[1:];dest=pathlib.Path(output);dest.mkdir(parents=True,exist_ok=True)
repo='repos/profilaz3/halcyon'
items=json.loads(subprocess.check_output(['gh','api',repo+'/actions/runs/'+run+'/artifacts?per_page=100']))['artifacts']
for item in items:
 if not item['name'].startswith('lockup-session-'):continue
 if item['size_in_bytes']>150*1024*1024:raise RuntimeError('Evidence exceeds small-only bound')
 archive=dest/(item['name']+'.zip')
 if not archive.exists():
  with archive.open('wb') as f:subprocess.run(['gh','api',repo+'/actions/artifacts/'+str(item['id'])+'/zip'],stdout=f,check=True,timeout=120)
 digest='sha256:'+hashlib.sha256(archive.read_bytes()).hexdigest()
 if digest!=item['digest']:raise RuntimeError('Artifact ZIP digest mismatch')
 target=dest/item['name']
 with zipfile.ZipFile(archive) as z:
  for i in z.infolist():
   p=pathlib.PurePosixPath(i.filename)
   if p.is_absolute() or '..' in p.parts or stat.S_ISLNK(i.external_attr>>16):raise RuntimeError('Unsafe ZIP member')
  z.extractall(target)
 print(json.dumps({k:item[k] for k in ['id','name','size_in_bytes','digest','expires_at']}))
(dest/'artifact-metadata.json').write_text(json.dumps(items,indent=2)+'\n')
