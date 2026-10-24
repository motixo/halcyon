"""Disposable guest operations; ordinary session process credentials only."""
import sys,os,json,pathlib,subprocess,hashlib,time
from guest_session import session_env
H=pathlib.Path('/var/lib/ps025');O=H/'results'
from trial_receipts import command_receipt
def save(name,obj): (O/name).write_text(json.dumps(obj,indent=2)+'\n')
def lock():
 s=subprocess.check_output(['dpkg-query','-W','-f=${Package}\t${Version}\t${db:Status-Status}\n'],text=True)
 return {a:b for a,b,c in (x.split('\t') for x in s.splitlines()) if c=='installed'}
def userargs(args):return ['/usr/sbin/runuser','-u','probe','--','/usr/bin/env','-i',*[k+'='+v for k,v in session_env().items()],*args]
def sha(p):
 with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def run_user(args,label,out=O,runner=subprocess.run,argv_builder=userargs):
 try:
  p=runner(argv_builder(args),capture_output=True,text=True,timeout=120)
  return command_receipt(label,p.returncode,p.stdout,p.stderr,out)
 except subprocess.TimeoutExpired as e:
  text=lambda x:x.decode(errors='replace') if isinstance(x,bytes) else (x or '')
  return command_receipt(label,124,text(e.stdout),text(e.stderr)+'\ncommand deadline exceeded',out)
def main():
 O.mkdir(exist_ok=True)
 mode=sys.argv[1]
 if mode=='initial':
  expected=json.loads((H/'locks.json').read_text());actual=lock();assert actual==expected['new'];assert subprocess.check_output(['uname','-r'],text=True).strip()==expected['kernel'];assert not subprocess.check_output(['dpkg','--audit'])
  save('initial.json',dict(packages=actual,kernel=expected['kernel'],count=len(actual),audit='empty'))
  for name,pin in json.loads((H/'package-pins.json').read_text()).items():assert (H/'test-input-packages'/name).stat().st_size==pin['size'] and sha(H/'test-input-packages'/name)==pin['sha256']
 elif mode=='install':
  name=sys.argv[2];p=subprocess.run(['dpkg','-i',str(H/'test-input-packages'/name)],capture_output=True,text=True,timeout=120);save(name+'.install.json',dict(rc=p.returncode,stdout=p.stdout,stderr=p.stderr));assert p.returncode==0
  actual=lock();before=json.loads((O/'initial.json').read_text())['packages'];assert all(actual[k]==v for k,v in before.items());assert set(actual)-set(before)<= {'cosmic-ext-halcyon','cosmic-ext-halcyon-islands'};assert not subprocess.check_output(['dpkg','--audit'])
  files={}
  for path,expected in [('/usr/bin/halcyon','8a3d9b2db6518d57e697b1ec562969781b07b496bce9c253b7f4b6ffca359dab'),('/usr/lib/cosmic-ext-halcyon-islands/cosmic-panel-islands','07d95f0eda4675bb5b01619d79192248e3a2a687b508cd752c0611b50d4cc4e8')]:
   if pathlib.Path(path).exists():files[path]=sha(path);assert files[path]==expected
  identity=json.loads((H/'integration2.json').read_text());helper_hashes={}
  library=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands')
  if library.exists():
   for helper,expected in identity['helper_hashes'].items():
    helper_hashes[helper]=sha(library/helper);assert helper_hashes[helper]==expected,helper
   assert sha(library/'companion-manifest.json')==identity['manifest_sha256']
   assert sha(library/'catalog.json')==identity['catalog_sha256'] and not json.loads((library/'catalog.json').read_text())['cohorts']
  for item in identity['packages'].values():
   if item['package'] in actual:assert actual[item['package']]==item['version']
  save(name+'.installed.json',dict(packages={k:v for k,v in actual.items() if k not in before},count=len(actual),base_unchanged=True,files=files,helper_hashes=helper_hashes,manifest_sha256=identity['manifest_sha256'] if helper_hashes else None,empty_catalog=True,dpkg_audit='empty'))
 elif mode=='launch':
  name=sys.argv[2];args=sys.argv[3:];out=(O/(name+'.stdout')).open('wb');err=(O/(name+'.stderr')).open('wb');p=subprocess.Popen(userargs(args),stdin=subprocess.DEVNULL,stdout=out,stderr=err,start_new_session=True);save(name+'.launch.json',dict(pid=p.pid,args=args,time=time.time()))
 elif mode=='user':
  value=run_user(sys.argv[3:],sys.argv[2]);print(json.dumps(value));return value['returncode']
 elif mode=='snapshot':
  name=sys.argv[2];data={}
  for rel in ['.config/halcyon/config.json','.config/cosmic-desktop-cards/config.json','.local/state/halcyon/usage.json']:
   path=pathlib.Path('/home/probe')/rel
   if path.exists():
    raw=path.read_bytes();obj=json.loads(raw)
    if rel.endswith('usage.json'):obj.pop('installation',None) # never export pseudonymous identifier
    data[rel]={'sha256':hashlib.sha256(raw).hexdigest(),'value':obj}
  save(name+'.json',data)
 else:raise ValueError('unknown operation')

if __name__=='__main__':raise SystemExit(main())
