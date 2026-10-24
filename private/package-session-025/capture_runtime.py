"""Bounded public runtime evidence, never a complete dependency closure/admission."""
import os,pathlib,hashlib,json,subprocess,stat,re,time
assert os.getuid()==1000
P=pathlib.Path;files={};procs=[];errors=[];total=0;MAX=2_000_000_000

def command(args):
 try:
  r=subprocess.run(args,capture_output=True,text=True,timeout=8);assert len(r.stdout)+len(r.stderr)<1_000_000
  return dict(rc=r.returncode,stdout=r.stdout,stderr=r.stderr)
 except Exception as e:return {'error':type(e).__name__}
def add(path,reason):
 global total
 p=P(path)
 if not (path.startswith('/usr/') or path.startswith('/lib/') or path.startswith('/lib64/') or path.startswith('/etc/ld.so') or path.startswith('/etc/glvnd/') or path.startswith('/etc/xdg/cosmic/')):return
 if path in files:files[path]['reasons'].append(reason);return
 assert len(files)<800
 try:
  target=p.resolve(strict=True);s=p.stat();assert s.st_size<=512*1024**2
  row={'path':path,'resolved':str(target),'mode':oct(stat.S_IMODE(s.st_mode)),'uid':s.st_uid,'gid':s.st_gid,'size':s.st_size,'reasons':[reason],'symlinks':[]}
  for x in [p,*p.parents]:
   if x.is_symlink():row['symlinks'].append({'path':str(x),'target':os.readlink(x)})
  assert total+s.st_size<=MAX
  with p.open('rb') as f:head=f.read(4);f.seek(0);row['sha256']=hashlib.file_digest(f,'sha256').hexdigest()
  total+=s.st_size;row['elf']=head==b'\x7fELF';row['owner']=command(['dpkg-query','-S',path])
  if row['elf']:row['readelf']=command(['readelf','-W','-l','-d','-n',path])
  elif s.st_size<=65536 and (p.suffix in ('.desktop','.json') or '/cosmic/' in path):row['public_text']=p.read_text(errors='replace')
  files[path]=row
 except Exception as e:errors.append({'path':path,'error':type(e).__name__})
for p in P('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  if p.stat().st_uid!=1000:continue
  exe=os.readlink(p/'exe');name=P(exe).name
  if not (name.startswith('cosmic-') or name=='halcyon'):continue
  maps=[]
  for line in (p/'maps').read_text().splitlines():
   parts=line.split(None,5)
   if len(parts)==6 and parts[5].startswith(('/usr/','/lib/','/lib64/')):
    path=parts[5];maps.append({'path':path,'permissions':parts[1]});add(path,'observed mapping '+name)
  add(exe,'actual process '+name);procs.append({'pid':int(p.name),'exe':exe,'stat':(p/'stat').read_text(),'maps':maps})
 except OSError as e:errors.append({'process':p.name,'error':type(e).__name__})
for p in P('/usr/bin').glob('cosmic-*'):add(str(p),'installed potential supported peer/applet; support NOT inferred')
add('/usr/lib/cosmic-ext-halcyon-islands/cosmic-panel-islands','final packaged payload, never executed')
for pattern in ['/usr/share/applications/com.system76.Cosmic*.desktop','/usr/share/applications/cosmic*.desktop','/usr/share/glvnd/egl_vendor.d/*.json','/usr/share/vulkan/icd.d/*.json']:
 import glob
 for name in glob.glob(pattern):add(name,'public desktop/provider selection input')
for root in ['/usr/share/cosmic','/etc/xdg/cosmic']:
 if P(root).exists():
  for p in P(root).rglob('*'):
   if p.is_file() and p.stat().st_size<65536 and any(s in str(p).lower() for s in ('panel','applet','session','notification')):add(str(p),'public default/schema input')
for name in ['/etc/ld.so.cache','/etc/ld.so.conf']:add(name,'loader selection input')
graphics={}
for pat in ['/sys/class/drm/card*/device/uevent','/sys/class/drm/card*/device/vendor','/sys/class/drm/card*/device/device','/sys/class/drm/card*-*/status','/sys/class/drm/card*-*/modes','/sys/module/virtio_gpu/version','/sys/module/drm/version']:
 import glob
 for name in glob.glob(pat):
  try:graphics[name]=P(name).read_text()[:16384]
  except OSError as e:graphics[name]={'error':type(e).__name__}
print(json.dumps(dict(format='halcyon-public-runtime-capture',version=1,qualified=False,complete_dependency_closure=False,uid=os.getuid(),time=time.time(),files=files,processes=procs,errors=errors,hashed_bytes=total,hashed_files=len(files),kernel=command(['uname','-a']),ldconfig=command(['/sbin/ldconfig','-p']),graphics=graphics,limits={'max_files':800,'max_total_hash_bytes':MAX,'max_individual_bytes':512*1024**2},limitations=['Observed stock mappings only; no Islands host launched','Installed potential applets are not declared supported','Conditional/dlopen/provider resolution needs source review','Sysfs driver metadata is not actual EGL renderer identity']),indent=2))
