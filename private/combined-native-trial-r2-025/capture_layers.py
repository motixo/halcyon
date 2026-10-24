"""Only five frozen public Vulkan layer candidates; availability, NOT activation."""
import pathlib,json,hashlib,os,stat,tarfile,subprocess,shutil,sys
D=pathlib.Path('/var/lib/ps025')
CANDIDATE_SHA='ac1ccc0ab67a617dd3d13b6d91d32d7cc6f4eba833208254887f01a757d11c9b'
NAMES={'/usr/lib/x86_64-linux-gnu/libVkLayer_'+n+'.so' for n in ('INTEL_nullhw','MESA_anti_lag','MESA_device_select','MESA_overlay','MESA_screenshot')}
FILE_CAP=32*1024**2;TOTAL_CAP=128*1024**2;CHUNK=64*1024**2
class LayerFailure(ValueError):pass
def sha(path):
 with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def stamp(s):return (s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns,s.st_mode,s.st_uid)
def plan(candidates,root):
 if set(candidates)!=NAMES:raise LayerFailure('exact_five_layer_paths_required')
 result={};total=0
 for name in sorted(candidates):
  q=root/name.lstrip('/');s=q.lstat()
  if not stat.S_ISREG(s.st_mode):raise LayerFailure('layer_not_regular: '+name)
  if not q.resolve().is_relative_to(root.resolve()):raise LayerFailure('layer_path_escape: '+name)
  if s.st_size>FILE_CAP:raise LayerFailure('layer_file_cap: '+name+' size='+str(s.st_size)+' limit='+str(FILE_CAP))
  total+=s.st_size
  if total>TOTAL_CAP:raise LayerFailure('layer_total_cap: bytes='+str(total)+' limit='+str(TOTAL_CAP))
  result[name]={'size':s.st_size,'stamp':stamp(s),'resolved':'/'+str(q.resolve().relative_to(root.resolve()))}
 return result,total
class HashReader:
 def __init__(self,stream):self.stream=stream;self.digest=hashlib.sha256();self.bytes=0
 def read(self,n):
  b=self.stream.read(n);self.digest.update(b);self.bytes+=len(b);return b
def capture(candidates,root,out,runner=subprocess.run):
 out.mkdir(exist_ok=True);record={'qualified':False,'activation':'NOT INFERRED','files':{},'complete':False,'errors':[]}
 def persist():
  p=out/'layer-capture-receipt.json.partial';p.write_text(json.dumps(record,indent=2)+'\n');os.replace(p,out/'layer-capture-receipt.json')
 persist()
 try:
  files,total=plan(candidates,root);record['planned_files']=files;record['total_bytes']=total;persist()
  if shutil.disk_usage(out).free<2*1024**3:raise LayerFailure('guest_export_headroom_below_2GiB')
  for name,info in files.items():
   q=root/name.lstrip('/');record['current_path']=name;persist();digest=sha(q)
   if digest!=candidates[name]['sha256']:raise LayerFailure('layer_hash_mismatch: '+name)
   if stamp(q.stat())!=tuple(info['stamp']):raise LayerFailure('layer_changed_during_hash: '+name)
   p=runner(['/usr/bin/readelf','-l','-d',str(q)],capture_output=True,timeout=10)
   if p.returncode!=0:raise LayerFailure('layer_readelf_failed: '+name+' rc='+str(p.returncode))
   if len(p.stdout)>65536 or len(p.stderr)>65536:raise LayerFailure('layer_readelf_output_cap: '+name)
   record['files'][name]={**candidates[name],**info,'sha256':digest,'readelf':{'returncode':p.returncode,'stdout':p.stdout.decode(errors='replace'),'stderr':p.stderr.decode(errors='replace')},'selected_at_runtime':'NOT PROVEN'};persist()
  partial=out/'layers-public.tar.gz.partial'
  with partial.open('xb') as dst,tarfile.open(fileobj=dst,mode='w:gz') as tar:
   for name,info in files.items():
    q=root/name.lstrip('/');record['current_path']=name;persist()
    with q.open('rb') as source:
     if stamp(os.fstat(source.fileno()))!=tuple(info['stamp']):raise LayerFailure('layer_replaced_before_archive: '+name)
     reader=HashReader(source);entry=tarfile.TarInfo('objects/'+name.lstrip('/'));entry.size=info['size'];entry.mode=0o444;tar.addfile(entry,reader)
     if reader.bytes!=info['size'] or reader.digest.hexdigest()!=candidates[name]['sha256']:raise LayerFailure('layer_archive_hash_mismatch: '+name)
     if stamp(os.fstat(source.fileno()))!=tuple(info['stamp']) or stamp(q.stat())!=tuple(info['stamp']):raise LayerFailure('layer_changed_during_archive: '+name)
  if partial.stat().st_size>TOTAL_CAP+1024**2:raise LayerFailure('layer_archive_byte_cap')
  final=out/'layers-public.tar.gz';partial.chmod(0o444);os.link(partial,final);partial.unlink()
  # The existing QGA collector accepts individual members no larger than 64 MiB.
  parts=[]
  with final.open('rb') as f:
   for i in range(3):
    b=f.read(CHUNK)
    if not b:break
    part=out/f'layers-public.part{i:02d}'
    with part.open('xb') as target:target.write(b)
    parts.append({'file':part.name,'size':len(b),'sha256':sha(part)})
   if f.read(1):raise LayerFailure('layer_part_count_bound')
  record['archive']={'sha256':sha(final),'size':final.stat().st_size,'parts':parts};record.pop('current_path',None);record['complete']=True;persist()
  # Remove only this verified duplicate archive after parts are independently hashed.
  # Archive bytes remain in bounded parts; failed partials are never removed.
  if sum(x['size'] for x in parts)!=final.stat().st_size:raise LayerFailure('layer_parts_size')
  final.unlink()
  return record
 except Exception as error:
  record['complete']=False;record['errors'].append({'path':record.get('current_path'),'type':type(error).__name__,'reason':str(error)});persist();raise

def main():
 if os.getuid()!=1000:raise LayerFailure('disposable_user_required')
 path=D/'remaining-layer-candidates.json'
 if sha(path)!=CANDIDATE_SHA:raise LayerFailure('layer_candidate_manifest_identity')
 result=capture(json.loads(path.read_text()),pathlib.Path('/'),D/'results')
 print(json.dumps({'qualified':False,'complete':result['complete'],'files':len(result['files']),'bytes':result['total_bytes']}))
if __name__=='__main__':main()
