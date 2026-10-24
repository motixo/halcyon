#!/usr/bin/python3
"""Bounded public inputs from the disposable session; never admission."""
import os,sys,pathlib,json,hashlib,stat,time,glob,subprocess,resource,tarfile,shutil
D=pathlib.Path('/var/lib/ps025');O=D/'results';B=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands');sys.path.insert(0,str(B));assert os.getuid()==1000
ALLOW='LD_PRELOAD LD_LIBRARY_PATH LD_AUDIT GLIBC_TUNABLES EGL_PLATFORM __EGL_VENDOR_LIBRARY_FILENAMES __EGL_VENDOR_LIBRARY_DIRS GBM_BACKEND GBM_BACKENDS_PATH MESA_LOADER_DRIVER_OVERRIDE LIBGL_ALWAYS_SOFTWARE GALLIUM_DRIVER DRI_PRIME PIPEWIRE_CONFIG_DIR PIPEWIRE_CONFIG_PREFIX PIPEWIRE_CONFIG_NAME PIPEWIRE_MODULE_DIR SPA_PLUGIN_DIR VK_ICD_FILENAMES VK_DRIVER_FILES VK_ADD_DRIVER_FILES VK_LAYER_PATH VK_ADD_LAYER_PATH VK_INSTANCE_LAYERS LANG LC_ALL LC_CTYPE GCONV_PATH LIBGL_DRIVERS_PATH XDG_CONFIG_HOME XDG_CONFIG_DIRS XDG_DATA_HOME XDG_DATA_DIRS PATH RUST_LOG'.split()
def save(n,obj):(O/n).write_text(json.dumps(obj,indent=2)+'\n')
def sha(p):
 with pathlib.Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def cmd(args,seconds=10,limit=65536):
 try:
  p=subprocess.run(args,capture_output=True,timeout=seconds);return dict(rc=p.returncode,stdout=p.stdout[:limit].decode(errors='replace'),stderr=p.stderr[:limit].decode(errors='replace'),truncated=len(p.stdout)>limit or len(p.stderr)>limit)
 except subprocess.TimeoutExpired:return {'error':'deadline'}
 except OSError as e:return {'error':type(e).__name__}
def procs():
 found={};files={};total=0
 wanted={'cosmic-session','cosmic-comp','cosmic-settings-daemon','cosmic-notifications','cosmic-panel','cosmic-panel-islands','cosmic-applets'}
 for p in pathlib.Path('/proc').iterdir():
  if not p.name.isdigit():continue
  try:
   if p.stat().st_uid!=1000:continue
   exe=(p/'exe').resolve(strict=True)
   if exe.name not in wanted:continue
   st=(p/'stat').read_text();tail=st.rsplit(')',1)[1].split();start=tail[19]
   env={}
   raw=(p/'environ').read_bytes();assert len(raw)<=65536
   for item in raw.split(b'\0'):
    k,sep,v=item.partition(b'=')
    if sep and k.decode(errors='replace') in ALLOW:env[k.decode()]=v.decode(errors='replace')[:4096]
   maps=(p/'maps').read_text();assert len(maps)<=1048576
   paths=set()
   for line in maps.splitlines():
    cols=line.split(None,5)
    if len(cols)==6 and cols[5].startswith('/'):paths.add(cols[5])
   rec={'pid':int(p.name),'ppid':int(tail[1]),'pgrp':int(tail[2]),'start_ticks':start,'exe':str(exe),'selectors':env,'cmdline':(p/'cmdline').read_bytes()[:8192].decode(errors='replace').split('\0'),'maps':maps,'descriptors':{}}
   for n in ('PANEL_NOTIFICATIONS_FD','WAYLAND_SOCKET'):
    v=next((i.partition(b'=')[2] for i in raw.split(b'\0') if i.startswith(n.encode()+b'=')),None)
    if v:
     try:rec['descriptors'][n]={'fd':int(v),'link':os.readlink(p/'fd'/v.decode())}
     except (OSError,ValueError) as e:rec['descriptors'][n]={'error':type(e).__name__}
   for x in sorted(paths|{str(exe)}):
    if x in files:continue
    q=pathlib.Path(x)
    if not x.startswith(('/usr/bin/','/usr/lib/','/lib/')):files[x]={'unsupported':'outside public allowlist'};continue
    s=q.stat();assert stat.S_ISREG(s.st_mode) and s.st_size<=128*1024**2 and len(files)<256 and total+s.st_size<=2*1024**3
    before=(s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns);v=sha(q);s=q.stat();assert before==(s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns);files[x]={'sha256':v,'size':s.st_size,'resolved':str(q.resolve()),'stamp':before};total+=s.st_size
   rec['binding_stable']=(p/'stat').read_text().rsplit(')',1)[1].split()[19]==start and (p/'exe').resolve()==exe;found[p.name]=rec
  except FileNotFoundError:continue
  except (OSError,ValueError) as e:found[p.name]={'error':type(e).__name__}
 assert len(found)<=64
 return {'processes':found,'mapped_public_files':files,'unique_bytes_hashed':total,'qualified':False}
def metadata(label):
 data={'label':label,'qualified':False,'time':time.time(),'files':{},'directories':{},'errors':[]};total=0;count=0
 def read(p):
  nonlocal total,count
  p=pathlib.Path(p);key=str(p)
  if key in data['files']:return
  try:
   st=p.lstat();rec={'exists':True,'resolved':str(p.resolve()),'symlink':os.readlink(p) if p.is_symlink() else None}
   assert p.is_file() and p.stat().st_size<=65536
   # Only explicitly selected system configs and the synthetic disposable user.
   assert str(p.resolve()).startswith(('/etc/','/usr/share/','/home/probe/'))
   raw=p.read_bytes();count+=1;total+=len(raw);assert count<=128 and total<=2097152
   rec.update(size=len(raw),sha256=hashlib.sha256(raw).hexdigest(),text=raw.decode());data['files'][key]=rec
  except FileNotFoundError:data['files'][key]={'exists':False}
  except (OSError,UnicodeError,AssertionError) as e:data['files'][key]={'error':type(e).__name__};data['errors'].append(key)
 def directory(p,pattern='*'):
  p=pathlib.Path(p)
  try:
   names=sorted(x.name for x in p.iterdir());assert len(names)<=256;data['directories'][str(p)]={'exists':True,'names':names,'resolved':str(p.resolve())}
   for x in sorted(p.glob(pattern)):
    if x.is_file():read(x)
  except FileNotFoundError:data['directories'][str(p)]={'exists':False}
 for base in ['/etc/glvnd/egl_vendor.d','/usr/share/glvnd/egl_vendor.d']:directory(base,'*.json')
 for base in ['/etc/pipewire','/usr/share/pipewire','/home/probe/.config/pipewire']:
  read(base+'/client.conf');directory(base+'/client.conf.d','*.conf')
 for x in ['/etc/drirc','/home/probe/.drirc','/home/probe/.config/drirc']:read(x)
 for root in ['/etc','/usr/local/share','/usr/share','/home/probe/.config','/home/probe/.local/share']:
  for sub in ['icd.d','explicit_layer.d','implicit_layer.d']:directory(root+'/vulkan/'+sub,'*.json')
 for base in ['/home/probe/.config','/home/probe/.local/share','/home/probe/.local/share/flatpak/exports/share','/var/lib/flatpak/exports/share','/usr/local/share','/usr/share']:
  for ns in ['com.system76.CosmicPanel','com.system76.CosmicPanel.Panel','com.system76.CosmicPanel.Dock','io.github.desktopcards.DockIslands']:
   q=pathlib.Path(base)/'cosmic'/ns/'v1'
   try:data['directories'][str(q)]={'exists':q.is_dir(),'names':sorted(x.name for x in q.iterdir()) if q.is_dir() else []}
   except OSError as e:data['directories'][str(q)]={'error':type(e).__name__}
   keys=['entries'] if ns=='com.system76.CosmicPanel' else ['Panel','Dock'] if ns=='io.github.desktopcards.DockIslands' else ['name','output','plugins_center','plugins_wings']
   for k in keys:read(q/k)
 draft=json.loads((D/'provider-draft.json').read_text())
 try:
  from panel_configuration import Snapshot,policy
  profile={'configuration':draft['proposed_configuration'],'processes':draft['processes_proposed']}
  snap=Snapshot(policy(profile));data['production_resolver']={'counts':snap.counts,'outputs':snap.outputs,'identity':snap.identity,'records':snap.records,'changed':snap.changed()};snap.close()
 except Exception as e:data['production_resolver']={'error':str(e),'type':type(e).__name__}
 data['bounds']={'files':count,'bytes':total};save(label+'-metadata.json',data);save(label+'-processes.json',procs());print(json.dumps({'metadata':label,'files':count,'bytes':total,'errors':len(data['errors'])}))
def export():
 candidates=json.loads((D/'vulkan-candidates.json').read_text())['files'];code={k:v for k,v in candidates.items() if k.startswith('/usr/lib/')};assert len(code)==10
 sizes={k:pathlib.Path(k).stat().st_size for k in code};assert max(sizes.values())<=128*1024**2 and sum(sizes.values())<=512*1024**2;assert shutil.disk_usage(D).free>=2*1024**3
 rec={'qualified':False,'files':{},'total_code_bytes':sum(sizes.values()),'metadata_count':9}
 for path,expected in candidates.items():
  p=pathlib.Path(path);s=p.stat();assert s.st_uid==0 and not s.st_mode&0o022;assert sha(p)==expected['sha256'];r={**expected,'size':s.st_size,'resolved':str(p.resolve()),'links':[{'path':str(x),'target':os.readlink(x)} for x in [p,*p.parents] if x.is_symlink()]}
  if path in code:r['readelf']=cmd(['/usr/bin/readelf','-l','-d',path],10,65536)
  rec['files'][path]=r
 save('vulkan-export-receipt.json',rec)
 archive=D/'vulkan-public.tar.gz'
 with tarfile.open(archive,'w:gz') as tar:
  for path in sorted(candidates):tar.add(path,arcname='objects/'+path.lstrip('/'),recursive=False)
 assert archive.stat().st_size<=512*1024**2
 pieces=[]
 with archive.open('rb') as f:
  for i in range(9):
   b=f.read(64*1024**2)
   if not b:break
   p=O/f'vulkan-public.part{i:02d}';p.write_bytes(b);pieces.append({'file':p.name,'size':len(b),'sha256':sha(p)})
 save('vulkan-archive.json',{'archive_sha256':sha(archive),'archive_size':archive.stat().st_size,'parts':pieces,'receipt':'vulkan-export-receipt.json'})
def benchmark():
 from runtime_identity import FileMeasurement
 d=json.loads((D/'provider-draft.json').read_text())['installed_files_lower_bound'];start=time.monotonic()
 with FileMeasurement(max_files=len(d),max_bytes=sum(x['size'] for x in d.values())) as m:
  for name,v in d.items():assert m.hash(m.resolve(name),v['size'])==v['sha256']
  m.check();print(json.dumps({'kind':'direct-source-owner-lower-bound-file-measurement-NOT-full-installed-identity-or-admission','files_hashed':len(m.files),'bytes_hashed':m.bytes,'seconds':time.monotonic()-start,'peak_rss_kib':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}))
if sys.argv[1]=='export':export()
elif sys.argv[1]=='benchmark':benchmark()
else:metadata(sys.argv[1])
