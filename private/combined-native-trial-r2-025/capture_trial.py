#!/usr/bin/python3
"""Bounded public inputs from the disposable session; never admission."""
import os,sys,pathlib,json,hashlib,stat,time,glob,subprocess,resource,tarfile,shutil
D=pathlib.Path('/var/lib/ps025');O=D/'results';B=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands');sys.path.insert(0,str(B))
ALLOW='LD_PRELOAD LD_LIBRARY_PATH LD_AUDIT GLIBC_TUNABLES EGL_PLATFORM __EGL_VENDOR_LIBRARY_FILENAMES __EGL_VENDOR_LIBRARY_DIRS GBM_BACKEND GBM_BACKENDS_PATH MESA_LOADER_DRIVER_OVERRIDE LIBGL_ALWAYS_SOFTWARE GALLIUM_DRIVER DRI_PRIME PIPEWIRE_CONFIG_DIR PIPEWIRE_CONFIG_PREFIX PIPEWIRE_CONFIG_NAME PIPEWIRE_MODULE_DIR SPA_PLUGIN_DIR VK_ICD_FILENAMES VK_DRIVER_FILES VK_ADD_DRIVER_FILES VK_LAYER_PATH VK_ADD_LAYER_PATH VK_INSTANCE_LAYERS NODEVICE_SELECT ENABLE_LAYER_MESA_ANTI_LAG DISABLE_LAYER_MESA_ANTI_LAG LANG LC_ALL LC_CTYPE GCONV_PATH LIBGL_DRIVERS_PATH XDG_CONFIG_HOME XDG_CONFIG_DIRS XDG_DATA_HOME XDG_DATA_DIRS PATH RUST_LOG'.split()
def selected_environment(raw):
 if len(raw)>65536:raise CaptureFailure('environment_byte_bound',limit=65536)
 values={}
 for item in raw.split(b'\0'):
  key,sep,value=item.partition(b'=');name=key.decode(errors='replace')
  if sep and name in ALLOW:
   if len(value)>4096:raise CaptureFailure('selector_byte_bound',selector=name,limit=4096)
   values[name]=value.decode(errors='replace')
 return values,sorted(set(ALLOW)-set(values))
def save(n,obj):
 temporary=O/(n+'.partial');temporary.write_text(json.dumps(obj,indent=2)+'\n');os.replace(temporary,O/n)
def sha(p):
 with pathlib.Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def cmd(args,seconds=10,limit=65536):
 try:
  p=subprocess.run(args,capture_output=True,timeout=seconds);return dict(rc=p.returncode,stdout=p.stdout[:limit].decode(errors='replace'),stderr=p.stderr[:limit].decode(errors='replace'),truncated=len(p.stdout)>limit or len(p.stderr)>limit)
 except subprocess.TimeoutExpired:return {'error':'deadline'}
 except OSError as e:return {'error':type(e).__name__}
MAP_DEFAULT_BYTES=128*1024**2
MAP_KNOWN_MAX_BYTES=256*1024**2
MAP_TOTAL_BYTES=2*1024**3
MAP_MAX_FILES=256
class CaptureFailure(ValueError):
 def __init__(self,reason,**details):self.reason=reason;self.details=details;super().__init__(reason)
def mapped_limit(name,size,known):
 item=known.get(name)
 if item is None:
  if size>MAP_DEFAULT_BYTES:raise CaptureFailure('unknown_file_size_bound',observed_size=size,limit=MAP_DEFAULT_BYTES)
  return MAP_DEFAULT_BYTES
 expected=item.get('size')
 if type(expected) is not int or expected<0 or expected>MAP_KNOWN_MAX_BYTES:raise CaptureFailure('known_size_bound',declared_size=expected,limit=MAP_KNOWN_MAX_BYTES)
 if size!=expected:raise CaptureFailure('known_size_mismatch',observed_size=size,expected_size=expected)
 return max(MAP_DEFAULT_BYTES,expected)
def file_stamp(s):return (s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns,s.st_mode,s.st_uid)
def capture_map_files(paths,known,state,persist,root=pathlib.Path('/')):
 files=state['mapped_public_files'];root=root.resolve()
 for name in sorted(paths):
  if name in files:continue
  if len(files)>=MAP_MAX_FILES:
   state['errors'].append({'path':name,'reason':'mapped_file_count_bound','limit':MAP_MAX_FILES});persist();break
  entry={'path':name,'complete':False};files[name]=entry;persist()
  try:
   if not name.startswith(('/usr/bin/','/usr/lib/','/lib/')):raise CaptureFailure('outside_public_allowlist')
   q=root/name.lstrip('/');resolved=q.resolve(strict=True)
   if not resolved.is_relative_to(root):raise CaptureFailure('resolved_path_escape')
   canonical='/'+str(resolved.relative_to(root));entry['resolved']=canonical
   if not canonical.startswith(('/usr/bin/','/usr/lib/','/lib/')):raise CaptureFailure('resolved_outside_public_allowlist')
   expected_key=name if name in known else canonical
   # Nonblocking open prevents a replaced FIFO/device from hanging capture.
   fd=os.open(q,os.O_RDONLY|os.O_NONBLOCK|os.O_CLOEXEC)
   try:
    before=os.fstat(fd);entry['size']=before.st_size
    if not stat.S_ISREG(before.st_mode):raise CaptureFailure('not_regular_file',mode=oct(before.st_mode))
    limit=mapped_limit(expected_key,before.st_size,known);entry['limit']=limit
    if state['bytes_read']+before.st_size>MAP_TOTAL_BYTES:raise CaptureFailure('mapped_total_byte_bound',bytes_read=state['bytes_read'],requested=before.st_size,limit=MAP_TOTAL_BYTES)
    h=hashlib.sha256();n=0
    with os.fdopen(os.dup(fd),'rb') as stream:
     while n<before.st_size:
      data=stream.read(min(1024*1024,before.st_size-n))
      if not data:raise CaptureFailure('mapped_file_truncated',bytes_read=n,expected_size=before.st_size)
      h.update(data);n+=len(data);state['bytes_read']+=len(data)
     if stream.read(1):raise CaptureFailure('mapped_file_grew',expected_size=before.st_size)
    after=os.fstat(fd)
    if file_stamp(before)!=file_stamp(after):raise CaptureFailure('open_file_changed',before=file_stamp(before),after=file_stamp(after))
    current=q.stat()
    if file_stamp(before)!=file_stamp(current):raise CaptureFailure('path_replaced',before=file_stamp(before),after=file_stamp(current))
    if q.resolve(strict=True)!=resolved:raise CaptureFailure('resolution_changed')
    digest=h.hexdigest();entry.update(sha256=digest,stamp=file_stamp(before),bytes_hashed=n)
    if expected_key in known and digest!=known[expected_key]['sha256']:raise CaptureFailure('known_hash_mismatch',expected_sha256=known[expected_key]['sha256'],actual_sha256=digest)
    entry['complete']=True;state['unique_bytes_hashed']+=n
   finally:os.close(fd)
  except CaptureFailure as e:
   entry['error']={'reason':e.reason,**e.details};state['errors'].append({'path':name,**entry['error']})
  except (OSError,ValueError) as e:
   entry['error']={'reason':'file_io_error','type':type(e).__name__,'errno':getattr(e,'errno',None)};state['errors'].append({'path':name,**entry['error']})
  finally:persist()
def procs(label,procroot=pathlib.Path('/proc')):
 state={'label':label,'processes':{},'mapped_public_files':{},'unique_bytes_hashed':0,'bytes_read':0,'errors':[],'qualified':False,'complete':False}
 known=json.loads((D/'provider-draft.json').read_text())['installed_files_lower_bound']
 def persist():save(label+'-processes.json',state)
 persist();wanted={'cosmic-session','cosmic-comp','cosmic-settings-daemon','cosmic-notifications','cosmic-panel','cosmic-panel-islands','cosmic-applets'}
 for p in procroot.iterdir():
  if not p.name.isdigit():continue
  identified=False
  try:
   if p.stat().st_uid!=1000:continue
   exe=(p/'exe').resolve(strict=True)
   if exe.name not in wanted:continue
   if len(state['processes'])>=64:
    state['errors'].append({'pid':p.name,'reason':'process_count_bound','limit':64});persist();break
   rec={'pid':int(p.name),'exe':str(exe)};state['processes'][p.name]=rec;identified=True;persist()
   st=(p/'stat').read_text();tail=st.rsplit(')',1)[1].split();start=tail[19]
   rec.update(ppid=int(tail[1]),pgrp=int(tail[2]),start_ticks=start);env={}
   with (p/'environ').open('rb') as f:raw=f.read(65537)
   env,absent=selected_environment(raw)
   with (p/'maps').open() as f:maps=f.read(1048577)
   if len(maps)>1048576:raise CaptureFailure('maps_byte_bound',limit=1048576)
   paths=set()
   for line in maps.splitlines():
    cols=line.split(None,5)
    if len(cols)==6 and cols[5].startswith('/') and ('x' in cols[1] or cols[5].startswith(('/usr/bin/','/usr/lib/','/lib/'))):paths.add(cols[5])
   with (p/'cmdline').open('rb') as f:command=f.read(8193)
   if len(command)>8192:raise CaptureFailure('cmdline_byte_bound',limit=8192)
   rec.update(mapping_hash_scope='executable mappings and public binary/library paths; all maps retained',selectors=env,selectors_absent=absent,cmdline=command.decode(errors='replace').split('\0'),maps=maps,descriptors={});persist()
   for n in ('PANEL_NOTIFICATIONS_FD','WAYLAND_SOCKET'):
    v=next((i.partition(b'=')[2] for i in raw.split(b'\0') if i.startswith(n.encode()+b'=')),None)
    if v:
     try:rec['descriptors'][n]={'fd':int(v),'link':os.readlink(p/'fd'/v.decode())}
     except (OSError,ValueError) as e:rec['descriptors'][n]={'error':type(e).__name__}
   persist();capture_map_files(paths|{str(exe)},known,state,persist)
   rec['binding_stable']=(p/'stat').read_text().rsplit(')',1)[1].split()[19]==start and (p/'exe').resolve()==exe
   if not rec['binding_stable']:raise CaptureFailure('process_binding_changed')
  except (OSError,ValueError,IndexError) as e:
   if identified:
    err={'pid':p.name,'reason':getattr(e,'reason','process_io_error'),'type':type(e).__name__,**getattr(e,'details',{})};state['processes'][p.name]['capture_error']=err;state['errors'].append(err)
  finally:
   if identified:persist()
 state['complete']=not state['errors'];persist();return state

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
  if sha(B/'panel_configuration.py')!='6be28029f5d1e6725a21e91ded03d0c882ab3206469f7facc1f272695b49c97f':raise ValueError('reviewed_output_resolver_identity_required')
  from panel_configuration import Snapshot,policy
  profile={'configuration':draft['proposed_configuration'],'processes':draft['processes_proposed']}
  snap=Snapshot(policy(profile));data['production_resolver']={'counts':snap.counts,'outputs':snap.outputs,'identity':snap.identity,'records':snap.records,'changed':snap.changed()};snap.close()
 except Exception as e:data['production_resolver']={'error':str(e),'type':type(e).__name__}
 data['bounds']={'files':count,'bytes':total};save(label+'-metadata.json',data);process=procs(label)
 complete=not data['errors'] and 'error' not in data['production_resolver'] and process['complete']
 print(json.dumps({'metadata':label,'files':count,'bytes':total,'metadata_errors':data['errors'],'process_errors':process['errors'],'complete':complete,'qualified':False}))
 if not complete:raise SystemExit(1)
def benchmark():
 from runtime_identity import FileMeasurement
 d=json.loads((D/'provider-draft.json').read_text())['installed_files_lower_bound'];start=time.monotonic()
 with FileMeasurement(max_files=len(d),max_bytes=sum(x['size'] for x in d.values())) as m:
  for name,v in d.items():assert m.hash(m.resolve(name),v['size'])==v['sha256']
  m.check();print(json.dumps({'kind':'direct-source-owner-lower-bound-file-measurement-NOT-full-installed-identity-or-admission','files_hashed':len(m.files),'bytes_hashed':m.bytes,'seconds':time.monotonic()-start,'peak_rss_kib':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}))
def main():
 if os.getuid()!=1000:raise ValueError('disposable_user_required')
 if sys.argv[1]=='benchmark':benchmark()
 else:metadata(sys.argv[1])
 
if __name__=='__main__':main()
