"""Timing wrappers delegate unchanged production checks; no observation stubs."""
import contextlib,hashlib,io,json,os,pathlib,resource,sys,time,traceback
H=pathlib.Path(__file__).resolve().parent;sys.path.insert(0,str(H/'frozen'))
import bundle_contract as b, runtime_identity as r, install_status as status, panel_launcher as launcher, refresh
LIB=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands')
mode=sys.argv[1];profile='--profile' in sys.argv;stages={};errors=[];digest_paths=[];scope='other';counts={'digest_calls':0,'digest_success':0,'installed_bytes_hashed':0,'other_bytes_hashed':0}
def wrap(module,name):
 real=getattr(module,name)
 def call(*a,**kw):
  t=time.perf_counter()
  try:return real(*a,**kw)
  finally:
   row=stages.setdefault(module.__name__+'.'+name,{'calls':0,'seconds':0});row['calls']+=1;row['seconds']+=time.perf_counter()-t
 setattr(module,name,call)
if profile or mode=='audit':
 real_digest=r.digest;real_fd=hashlib.file_digest
 def fd(stream,*args,**kwargs):
  before=stream.tell()
  try:return real_fd(stream,*args,**kwargs)
  finally:counts[('installed' if scope=='installed' else 'other')+'_bytes_hashed']+=max(0,stream.tell()-before)
 hashlib.file_digest=fd
 def dg(path):
  global scope
  counts['digest_calls']+=1;t=time.perf_counter();old=scope;scope='installed'
  try:
   value=real_digest(path);counts['digest_success']+=1
   if len(digest_paths)<2:digest_paths.append(str(path))
   counts['last_digest_path']=str(path);counts['last_digest_sha256']=value
   return value
  except (OSError,ValueError) as e:
   if len(errors)<20:errors.append(dict(path=str(path),error=type(e).__name__,reason=str(e)))
   raise
  finally:
   scope=old;counts['digest_seconds']=counts.get('digest_seconds',0)+time.perf_counter()-t
 r.digest=dg
 for module,names in ((r,['versions','inventory','installed_identity','running_identity','host_identity','_snapshot','_process']), (b,['read_catalog_bytes','valid_catalog','verify_bundle','classify_compatibility']), (refresh,['selected_generation']), (status,['query','detect'])):
  for name in names:wrap(module,name)
result=None;error=None;output=io.StringIO();t=time.perf_counter()
try:
 if mode in ('identity','audit','runtime'):
  # Timed file load is reported separately from installed_identity itself.
  t0=time.perf_counter();catalog=b.read_catalog(LIB/'catalog.json');load=time.perf_counter()-t0
  if mode=='identity':result=r.installed_identity(catalog)
  elif mode=='runtime':result=r.running_identity(catalog,phase='prelaunch')
  else:
   inventory=r.inventory(catalog['cohorts'][0]);bad=[];ok=0;resolves=0;started=time.perf_counter()
   for path,want in inventory.items():
    try:
     rt=time.perf_counter();target=pathlib.Path(path).resolve();assert target.is_relative_to(pathlib.Path('/').resolve());resolves+=time.perf_counter()-rt
     got=r.digest(target)
     if got!=want:bad.append(dict(path=path,resolved=str(target),expected=want,actual=got,kind='hash_mismatch'))
     else:ok+=1
    except (OSError,ValueError) as e:bad.append(dict(path=path,kind=type(e).__name__,reason=str(e)))
   result=dict(scope='full inventory audit, NOT production installed_identity or admission',inventory_paths=len(inventory),matched=ok,mismatches=bad,audit_seconds=time.perf_counter()-started,resolve_seconds=resolves,qualified=False)
  result={'catalog_load_seconds':load,'measurement':result}
 elif mode=='status':
  sys.argv=['install_status.py']
  with contextlib.redirect_stdout(output):status.main()
  result=json.loads(output.getvalue())
 elif mode=='prelaunch':
  guard=launcher.prepare_launch(LIB);result={'eligible':guard is not None,'qualified':False}
  if guard is not None:guard.close();raise AssertionError('development bundle unexpectedly eligible')
 elif mode=='stage':
  with refresh.locked(pathlib.Path.home()):result=refresh.stage_locked(pathlib.Path.home(),LIB,enable=True)
  if result.get('state')!='blocked':raise AssertionError('development staging unexpectedly allowed')
 else:raise ValueError('unknown timing mode')
except BaseException as e:error=dict(type=type(e).__name__,reason=str(e),traceback=traceback.format_exc())
record=dict(mode=mode,profiled=profile or mode=='audit',seconds=time.perf_counter()-t,max_rss_kib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,cpu_user_seconds=resource.getrusage(resource.RUSAGE_SELF).ru_utime,cpu_system_seconds=resource.getrusage(resource.RUSAGE_SELF).ru_stime,stages=stages,counts=counts,first_digest_paths=digest_paths,digest_errors=errors,result=result,error=error,session_present=bool(os.environ.get('XDG_SESSION_ID')),uid=os.getuid(),qualified=False)
print(json.dumps(record,separators=(',',':')))
raise SystemExit(1 if error else 0)
