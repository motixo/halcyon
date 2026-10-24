import hashlib,pathlib,stat,zipfile
ARTIFACT=12535269783
SIZE=9549451357
ZIP_SHA='64b6d3aae3fe479d40d58861b7f4e0d1f972f147efc205b8e71b9e136d311ac0'
HERE=pathlib.Path(__file__).resolve().parent
def sha(path):
 with pathlib.Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def pins():
 p=HERE/'prepared-SHA256SUMS'
 assert sha(p)=='d1b49e6acb0ca421b4828da89a6d056277b78933a9a58d37c17a036cb64e2337'
 result={}
 for line in p.read_text().splitlines():
  digest,name=line.split();q=pathlib.PurePosixPath(name)
  assert not q.is_absolute() and '..' not in q.parts and name not in result and len(digest)==64
  result[name]=digest
 result['SHA256SUMS']='d1b49e6acb0ca421b4828da89a6d056277b78933a9a58d37c17a036cb64e2337'
 return result
def extract_verified(archive,dest):
 assert archive.stat().st_size==SIZE and sha(archive)==ZIP_SHA,'archive identity'
 expected=pins()
 with zipfile.ZipFile(archive) as z:
  names=[]
  for i in z.infolist():
   p=pathlib.PurePosixPath(i.filename)
   assert not p.is_absolute() and '..' not in p.parts and not stat.S_ISLNK(i.external_attr>>16) and not i.is_dir(),'unsafe ZIP member'
   names.append(i.filename)
  assert len(set(names))==len(names) and set(names)==set(expected),'unexpected ZIP members'
  assert sum(i.file_size for i in z.infolist())<10_000_000_000,'extraction byte bound'
  dest.mkdir(exist_ok=False)
  for i in z.infolist():
   p=dest/i.filename;p.parent.mkdir(parents=True,exist_ok=True);n=0;h=hashlib.sha256()
   with z.open(i) as src,p.open('xb') as out:
    while b:=src.read(8*1024*1024):out.write(b);h.update(b);n+=len(b)
   assert n==i.file_size and h.hexdigest()==expected[i.filename],'member identity: '+i.filename
   p.chmod(0o444)
 return {n:{'sha256':h,'size':(dest/n).stat().st_size} for n,h in expected.items()}
