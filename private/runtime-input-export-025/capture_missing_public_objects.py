#!/usr/bin/env python3
"""Bounded future export from the exact guest/image root. NOT executed on a guest.
Only the explicit candidate-bound public objects are opened; no traversal, keys,
processes, preferences, activation or network. Output is not qualification.
"""
import argparse,hashlib,json,os,pathlib,shutil,stat,time
D=pathlib.Path(__file__).resolve().parent
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=pathlib.Path,required=True);p.add_argument('--out',type=pathlib.Path,required=True);a=p.parse_args()
root=a.root.resolve();a.out.mkdir(parents=True,exist_ok=False)
items=json.loads((D/'public-object-receipt.json').read_text())['missing'];budget=sum(x['size'] for x in items.values());assert budget==186442704 and len(items)==44
start=time.monotonic();total=0;rows={}
def stamp(s):return (s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns)
for path,f in items.items():
 assert path.startswith('/usr/lib/') and '..' not in pathlib.PurePosixPath(path).parts
 original=root/path.lstrip('/');actual=original.resolve(strict=True);assert actual.is_relative_to(root) and actual == original # canonical allowlist; refuse redirected paths
 assert time.monotonic()-start<90 and shutil.disk_usage(a.out).free-f['size']>30*1024**3
 fd=os.open(actual,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK|os.O_CLOEXEC)
 with os.fdopen(fd,'rb') as src:
  before=os.fstat(src.fileno());assert stat.S_ISREG(before.st_mode) and before.st_size==f['size'] and before.st_size<=512*1024**2
  dst=a.out/'objects'/path.lstrip('/');dst.parent.mkdir(parents=True,exist_ok=True);h=hashlib.sha256();n=0
  with dst.open('xb') as out:
   while n<before.st_size:
    assert time.monotonic()-start<90
    chunk=src.read(min(1024*1024,before.st_size-n));assert chunk
    out.write(chunk);h.update(chunk);n+=len(chunk)
  assert not src.read(1) and stamp(os.fstat(src.fileno()))==stamp(before)==stamp(actual.stat())
  assert h.hexdigest()==f['sha256'];total+=n;assert total<=budget
  rows[path]={'sha256':h.hexdigest(),'size':n,'resolved':str(actual.relative_to(root)),'uid':before.st_uid,'mode':oct(stat.S_IMODE(before.st_mode))}
assert total==budget
(a.out/'receipt.json').write_text(json.dumps({'state':'PUBLIC-OBJECT-EXPORT-NOT-QUALIFIED','files':rows,'bytes':total,'seconds':time.monotonic()-start},indent=2)+'\n')
print('Export complete:',len(rows),'files',total,'bytes; no admission')
