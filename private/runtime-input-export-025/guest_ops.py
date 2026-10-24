"""Disposable guest operations; ordinary session process credentials only."""
import sys,os,json,pathlib,subprocess,hashlib,time
from guest_session import session_env
H=pathlib.Path('/var/lib/ps025');O=H/'results';O.mkdir(exist_ok=True)
def save(name,obj): (O/name).write_text(json.dumps(obj,indent=2)+'\n')
def lock():
 s=subprocess.check_output(['dpkg-query','-W','-f=${Package}\t${Version}\t${db:Status-Status}\n'],text=True)
 return {a:b for a,b,c in (x.split('\t') for x in s.splitlines()) if c=='installed'}
def userargs(args):return ['/usr/sbin/runuser','-u','probe','--','/usr/bin/env','-i',*[k+'='+v for k,v in session_env().items()],*args]
def sha(p):
 with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
mode=sys.argv[1]
if mode=='initial':
 expected=json.loads((H/'locks.json').read_text());actual=lock();assert actual==expected['new'];assert subprocess.check_output(['uname','-r'],text=True).strip()==expected['kernel'];assert not subprocess.check_output(['dpkg','--audit'])
 save('initial.json',dict(packages=actual,kernel=expected['kernel'],count=len(actual),audit='empty'))
 for name,pin in json.loads((H/'package-pins.json').read_text()).items():assert (H/'test-input-packages'/name).stat().st_size==pin['size'] and sha(H/'test-input-packages'/name)==pin['sha256']
elif mode=='install':
 name=sys.argv[2];p=subprocess.run(['dpkg','-i',str(H/'test-input-packages'/name)],capture_output=True,text=True,timeout=120);save(name+'.install.json',dict(rc=p.returncode,stdout=p.stdout,stderr=p.stderr));assert p.returncode==0
 actual=lock();before=json.loads((O/'initial.json').read_text())['packages'];assert all(actual[k]==v for k,v in before.items());assert set(actual)-set(before)<= {'cosmic-ext-halcyon','cosmic-ext-halcyon-islands'};assert not subprocess.check_output(['dpkg','--audit'])
 files={}
 for path,expected in [('/usr/bin/halcyon','0944c24dcaa2e46f9fff8e1eb71aa114f79edd5bb78ec66d3761ba7c4b9b52b2'),('/usr/lib/cosmic-ext-halcyon-islands/cosmic-panel-islands','07d95f0eda4675bb5b01619d79192248e3a2a687b508cd752c0611b50d4cc4e8')]:
  if pathlib.Path(path).exists():files[path]=sha(path);assert files[path]==expected
 save(name+'.installed.json',dict(packages={k:v for k,v in actual.items() if k not in before},count=len(actual),base_unchanged=True,files=files,dpkg_audit='empty'))
elif mode=='launch':
 name=sys.argv[2];args=sys.argv[3:];out=(O/(name+'.stdout')).open('wb');err=(O/(name+'.stderr')).open('wb');p=subprocess.Popen(userargs(args),stdin=subprocess.DEVNULL,stdout=out,stderr=err,start_new_session=True);save(name+'.launch.json',dict(pid=p.pid,args=args,time=time.time()))
elif mode=='user':
 p=subprocess.run(userargs(sys.argv[3:]),capture_output=True,text=True,timeout=120);save(sys.argv[2]+'.json',dict(rc=p.returncode,stdout=p.stdout,stderr=p.stderr));print(json.dumps({'returncode':p.returncode,'stdout':p.stdout,'stderr':p.stderr}));sys.exit(p.returncode)
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
