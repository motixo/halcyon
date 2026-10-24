"""Only bounded public loader/layout/native keys and ordinary-user graphics evidence."""
import pathlib,os,json,hashlib,stat,subprocess,glob,sys,time,shutil
D=pathlib.Path(__file__).resolve().parent;OUT=D/'results';OUT.mkdir(exist_ok=True);assert os.getuid()==1000
label=sys.argv[1];errors=[]
def command(args,limit=12000,timeout=12):
 try:
  p=subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  try:b=p.communicate(timeout=timeout)[0]
  except subprocess.TimeoutExpired:p.kill();b=p.communicate()[0];return {'error':'timeout','output':b[:limit].decode(errors='replace'),'truncated':len(b)>limit}
  return {'returncode':p.returncode,'output':b[:limit].decode(errors='replace'),'truncated':len(b)>limit}
 except OSError as e:return {'error':type(e).__name__}
def public_file(path,maxbytes=65536):
 p=pathlib.Path(path)
 try:
  s=p.lstat();r={'present':True,'mode':oct(stat.S_IMODE(s.st_mode)),'uid':s.st_uid,'gid':s.st_gid,'symlink':os.readlink(p) if p.is_symlink() else None,'resolved':str(p.resolve())}
  if not p.is_file() or p.stat().st_size>maxbytes or not (p.stat().st_mode & stat.S_IROTH) and not (p.is_relative_to(pathlib.Path.home()/'.config/cosmic') and p.stat().st_uid==1000):return dict(r,error='not bounded world-readable regular public file')
  raw=p.read_bytes();assert len(raw)<=maxbytes;r.update(bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest(),text=raw.decode('utf-8'));return r
 except FileNotFoundError:return {'present':False}
 except (OSError,UnicodeError) as e:return {'error':type(e).__name__}
def layers(rel):return {'user':public_file(pathlib.Path.home()/'.config/cosmic'/rel),'system':public_file(pathlib.Path('/usr/share/cosmic')/rel)}
keys=json.loads((D/'native-key-allowlist.json').read_text());native={k:layers(k) for k in keys};assert sum(v.get('bytes',0) for x in native.values() for v in x.values())<2*1024**2
layoutkeys=['com.system76.CosmicPanel/v1/entries']+[f'com.system76.CosmicPanel.{n}/v1/{k}' for n in ['Panel','Dock'] for k in ['name','output','plugins_center','plugins_wings']]+[f'io.github.desktopcards.DockIslands/v1/{n}' for n in ['Panel','Dock']]
layout={k:layers(k) for k in layoutkeys};assert len(layout)<=16 and sum(v.get('bytes',0) for x in layout.values() for v in x.values())<=1048576
for k,v in layout.items():v['effective']={'layer':'user' if v['user'].get('present') else 'system' if v['system'].get('present') else 'compiled-default-unresolved','value':v['user'].get('text') if v['user'].get('present') else v['system'].get('text')}
result={'qualified':False,'label':label,'time':time.time(),'native_keys':native,'layout':layout,'precedence':'Synthetic probe HOME/.config/cosmic then /usr/share/cosmic. Absent at both levels is not an inferred compiled value.'}
if label=='before':
 patterns=['/etc/ld.so.conf','/etc/ld.so.conf.d/*.conf','/etc/ld.so.preload','/etc/nsswitch.conf','/usr/share/glvnd/egl_vendor.d/*.json','/usr/share/pipewire/pipewire.conf','/usr/share/pipewire/client.conf','/usr/share/pipewire/client-rt.conf','/usr/share/pipewire/*/*.conf','/etc/pipewire/*.conf','/etc/pipewire/*/*.conf']
 paths=set();absent=[]
 for pat in patterns:
  hits=glob.glob(pat)
  if not hits:absent.append(pat)
  paths.update(hits)
 assert len(paths)<=128;loader={p:public_file(p) for p in sorted(paths)};assert sum(v.get('bytes',0) for v in loader.values())<=2097152
 aliases={}
 for name in json.loads((D/'loader-alias-allowlist.json').read_text()):
  p=pathlib.Path(name)
  try:
   s=p.stat();aliases[name]={'resolved':str(p.resolve(strict=True)),'size':s.st_size,'mode':oct(stat.S_IMODE(s.st_mode)),'links':[{'path':str(q),'target':os.readlink(q)} for q in [p,*p.parents] if q.is_symlink()]}
  except OSError as e:aliases[name]={'error':type(e).__name__}
 result['loader']={'files':loader,'absent_patterns':absent,'aliases':aliases,'ldconfig':command(['/sbin/ldconfig','-p'],60000)}
 # Record only configuration/loader/locale selectors from existing native peers, not an environment dump.
 selectors={};allowed={'XDG_CONFIG_HOME','XDG_CONFIG_DIRS','XDG_DATA_HOME','XDG_DATA_DIRS','LD_LIBRARY_PATH','LD_PRELOAD','LIBGL_DRIVERS_PATH','MESA_LOADER_DRIVER_OVERRIDE','GBM_BACKEND','__EGL_VENDOR_LIBRARY_FILENAMES','__EGL_VENDOR_LIBRARY_DIRS','LANG','LC_ALL','LC_CTYPE','LC_MESSAGES'}
 for proc in pathlib.Path('/proc').iterdir():
  if not proc.name.isdigit():continue
  try:
   if proc.stat().st_uid!=1000 or (proc/'exe').resolve().name not in ['cosmic-comp','cosmic-panel','cosmic-session']:continue
   vals={}
   for item in (proc/'environ').read_bytes().split(b'\0'):
    k,sep,v=item.partition(b'=');key=k.decode(errors='replace')
    if sep and key in allowed:vals[key]=v.decode(errors='replace')[:4096]
   selectors[proc.name]={'exe':str((proc/'exe').resolve()),'selected_variables':vals}
  except OSError as e:errors.append({'pid':proc.name,'error':type(e).__name__})
 result['selected_native_selectors']=selectors
 g={'context_identity_proven':False,'installed_diagnostics':{n:shutil.which(n) for n in ['eglinfo','glxinfo','vulkaninfo','cosmic-randr']},'nodes':{}}
 nodes=[p for p in pathlib.Path('/sys/class/drm').iterdir() if p.name.startswith('card') and '-' not in p.name or p.name.startswith('renderD')];assert len(nodes)<=8
 for p in nodes:
  dev=p/'device';r={'sysfs_resolved':str(p.resolve()),'device_resolved':str(dev.resolve()),'driver':str((dev/'driver').resolve()),'module':str((dev/'driver/module').resolve()),'attrs':{}}
  for n in ['vendor','device','subsystem_vendor','subsystem_device','uevent','features','revision','class']:
   q=dev/n
   try:r['attrs'][n]=q.read_text()[:4096]
   except OSError as e:r['attrs'][n]={'error':type(e).__name__}
  for n in ['version','srcversion']:
   q=dev/'driver/module'/n
   try:r['attrs']['module_'+n]=q.read_text()[:1024]
   except OSError as e:r['attrs']['module_'+n]={'error':type(e).__name__}
  # Virtio child provides feature bits; bounded one-level public sysfs names only.
  r['virtio_children']={}
  for child in sorted(dev.glob('virtio*'))[:8]:
   r['virtio_children'][child.name]={'resolved':str(child.resolve())}
   for n in ['features','device','vendor','status']:
    try:r['virtio_children'][child.name][n]=(child/n).read_text()[:4096]
    except OSError as e:r['virtio_children'][child.name][n]={'error':type(e).__name__}
  g['nodes'][p.name]=r
 g['compositor_graphics_log']=command(['journalctl','--user','-b','_COMM=cosmic-comp','--grep=EGL|GL renderer|GL vendor|GL version|renderer|render node|render_node|GBM|GLES|llvmpipe|softpipe','-n','70','--no-pager','-o','cat'],24000)
 g['compositor_system_journal_query']=command(['journalctl','-b','_UID=1000','_COMM=cosmic-comp','--grep=EGL|GL renderer|GL vendor|GL version|renderer|render node|render_node|GBM|GLES|llvmpipe|softpipe','-n','70','--no-pager','-o','cat'],24000)
 # No newly installed tools, no manufactured renderer/context, no companion launch.
 if g['installed_diagnostics']['eglinfo']:g['independent_eglinfo_context']=command(['eglinfo','-B'],12000,20)
 raw=json.dumps(g).encode();assert len(raw)<=65536;result['graphics']=g
result['errors']=errors
(OUT/(label+'-metadata.json')).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'label':label,'native_keys':len(native),'layout_keys':len(layout),'graphics_context_inference':False}))
