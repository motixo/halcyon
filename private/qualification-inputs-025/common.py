import hashlib,json,pathlib,stat,zipfile
ARTIFACT=12506756949
SIZE=15065458881
ZIP_SHA='e87c617f179da525d7d06e3e74b862fa87dcefabdbcbcb02f2e7258802a6ccd4'
HERE=pathlib.Path(__file__).resolve().parent

def sha(path):
 with pathlib.Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def pins(path=HERE/'original-retained-hashes.txt'):
 rows={}
 for line in path.read_text().splitlines():
  digest,name=line.split();p=pathlib.PurePosixPath(name)
  if len(p.parts)!=2 or p.parts[0]!='retain' or p.name in rows:raise ValueError('invalid original manifest')
  rows[p.name]=digest
 return rows

def extract_verified(archive,dest,expected_size=SIZE,expected_sha=ZIP_SHA,expected_files=None):
 if archive.stat().st_size!=expected_size or sha(archive)!=expected_sha:raise ValueError('ZIP identity mismatch')
 expected_files=pins() if expected_files is None else expected_files
 with zipfile.ZipFile(archive) as z:
  names=[]
  for i in z.infolist():
   p=pathlib.PurePosixPath(i.filename)
   if len(p.parts)!=1 or p.is_absolute() or '..' in p.parts or stat.S_ISLNK(i.external_attr>>16) or i.is_dir():raise ValueError('unsafe member')
   names.append(i.filename)
  if len(set(names))!=len(names) or set(names)!=set(expected_files):raise ValueError('member set mismatch')
  dest.mkdir(exist_ok=False)
  for i in z.infolist():
   with z.open(i) as source,(dest/i.filename).open('xb') as target:
    h=hashlib.sha256();n=0
    while b:=source.read(8*1024*1024):target.write(b);h.update(b);n+=len(b)
   if n!=i.file_size or h.hexdigest()!=expected_files[i.filename]:raise ValueError('top-level input mismatch: '+i.filename)
   (dest/i.filename).chmod(0o444)
 return {name:{'sha256':digest,'size':(dest/name).stat().st_size} for name,digest in expected_files.items()}

def purge_changes(plan,removed):
 rows=[line.split() for line in plan.splitlines() if line.startswith(('Remv ','Purg ','Inst ','Conf '))]
 if any(r[0] in ('Inst','Conf') for r in rows) or {r[1] for r in rows}!=set(removed):raise ValueError('purge plan changed extra inputs')
 return rows
