"""Only after a refused launch: bounded current scoped measurements, never override."""
import pathlib,json,time,os
def inspect(stage,out):
 from bundle_contract import verify_bundle,classify_compatibility
 from runtime_identity import installed_identity,running_identity,_snapshot,FileMeasurement,runtime_work,_constraint_snapshot
 from panel_configuration import Snapshot,policy
 start=time.monotonic();result={'qualified':False,'scope':'post-refusal diagnostic, not original prelaunch replay'}
 def save():(out/'prelaunch-refusal-diagnostic.json').write_text(json.dumps(result,indent=2)+'\n')
 try:
  m,c=verify_bundle(stage);result['bundle_verified']=True;save()
  installed=installed_identity(c);result['installed']=installed;result['elapsed_seconds']=time.monotonic()-start;save()
  running=running_identity(c,phase='prelaunch');result['running']=running
  result['classification']=classify_compatibility(installed,running,{**m,'verified':True},c,phase='prelaunch');save()
  if not running['verified']:
   profile=c['cohorts'][0]['runtime'];snap=Snapshot(policy(profile))
   try:
    with FileMeasurement(**runtime_work(c,host=True)) as measure:
     found,errors=_snapshot(pathlib.Path('/proc'),profile['processes'],os.environ['XDG_SESSION_ID'],os.getuid(),measure)
     result['required_counts']={n:0 if v['lifecycle']=='panel-child' else v['count'] for n,v in profile['processes'].items()}
     result['observed_counts']={n:len(v) for n,v in found.items()};result['process_error_roles']=sorted(errors)
     result['unrecognized_mapped_paths']={n:sorted({p for _,v in rows for p,h in v['mapped'].items() if profile['processes'][n]['mapped'].get(p)!=h}) for n,rows in found.items()}
     result['constraints_match']=_constraint_snapshot(profile['constraints'],pathlib.Path('/proc'),pathlib.Path('/sys'))[0]==profile['constraints'];save()
   finally:snap.close()
 except Exception as e:result['error']={'type':type(e).__name__,'detail':str(e)[:300]};save()
 result['elapsed_seconds']=time.monotonic()-start;save();return result
def bounded_diagnostic(stage,out):
 from verification import bounded
 try:return bounded(lambda:inspect(stage,out),5)
 except Exception as e:
  (out/'prelaunch-refusal-diagnostic-error.json').write_text(json.dumps({'type':type(e).__name__,'qualified':False})+'\n')
