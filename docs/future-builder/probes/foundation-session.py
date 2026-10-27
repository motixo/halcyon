#!/usr/bin/env python3
"""Actual XTest -> native Wayland app -> nested compositor -> X framebuffer.
Private X server, user bus, preferences and compositor. No host desktop input.
Framebuffer timings include poll/readback overhead; they are not physical KMS
scanout measurements. Keep those qualifications in every derived report.
"""
import argparse, ctypes as C, hashlib, json, math, os
from pathlib import Path
import selectors, signal, subprocess, tempfile, time, shutil

p=argparse.ArgumentParser()
p.add_argument('binary',type=Path)
p.add_argument('tools',type=Path)
p.add_argument('output',type=Path)
p.add_argument('--asset-root',type=Path,help='Use wallpapers from an extracted package for exact-artifact acceptance')
p.add_argument('--style',choices=['paper','tidal'],default='paper')
p.add_argument('--inspect',action='store_true')
p.add_argument('--design',action='store_true',help='Migrate the private fixture and exercise canonical document writes')
p.add_argument('--moment-runtime',action='store_true',help='Actual desktop Day/Night config switches and idle')
p.add_argument('--recipe-runtime',action='store_true',help='Actual desktop personal recipes, layout switching and idle')
p.add_argument('--discovery',action='store_true',help='Offline discovery, personal remix and native Apply/Undo')
p.add_argument('--packages',action='store_true',help='Native portable-package import, review, Apply/Undo and export review')
p.add_argument('--recipes',action='store_true',help='Private native personal recipe Save/Apply/Undo and deletion')
p.add_argument('--moments',action='store_true',help='Day/Night local preview, activation and native Undo')
p.add_argument('--ridge-scale',type=float,choices=[1,1.5,2],default=1)
p.add_argument('--ridge-position',type=float,choices=[.2,.25],default=.25,help='Clock vertical placement for the bounded depth fixture')
p.add_argument('--ridge-fit',action='store_true')
p.add_argument('--ridge-studio',action='store_true',help='Ridge native candidate/current, Apply/Undo and cancellation')
p.add_argument('--ridge',action='store_true',help='prepared depth on actual Clock and cosmic-bg, private on/off comparison')
p.add_argument('--spine',action='store_true',help='private native Spine review, Apply/Undo and stock vertical panel from the actual applied fixture')
p.add_argument('--mix',action='store_true',help='private native Mix board Apply/Undo and cancel cycles')
p.add_argument('--canvas',action='store_true',help='private native canvas Apply/Undo and cancel cycles')
p.add_argument('--gallery',action='store_true',help='capture the four curated setup reviews through native editor commands')
p.add_argument('--size',default='820x600',choices=['820x600','1280x800'])
p.add_argument('--islands-fixture',action='store_true',help='illustrate enabled Islands capability in the private Settings fixture; no companion host')
p.add_argument('--baseline',action='store_true',help='same navigation/memory workload on the pre-foundation release')
p.add_argument('--desktop',action='store_true',help='capture and sample the actual widget process instead of Settings')
p.add_argument('--maximum',action='store_true',help='enable all standalone kinds and live tools; Glance is an alternative grouped layout')
p.add_argument('--idle-seconds',type=int,default=60,choices=range(1,61))
a=p.parse_args()
root=a.output.resolve();root.mkdir(parents=True,exist_ok=False)
(root/'probe-source.py').write_bytes(Path(__file__).read_bytes())
runtime=Path(tempfile.mkdtemp(prefix='halcyon-foundation-'));runtime.chmod(0o700)
wallpaper_assets=(a.asset_root or Path(__file__).resolve().parents[3]/'assets/wallpapers').resolve()
for name in ('config','state','cache','data','desktop'): (root/name).mkdir(mode=0o700)
config_path=root/('desktop/config.json' if a.design else 'config.json')
config={'onboarded':True,'setup_dismissed':True,'enabled_widgets':['clock','weather'],'project_roots':[],
 'settings_animations':False,'appearance':{'family':'paper' if a.style=='paper' else 'soft','wallpaper_light':a.style=='paper'},
 'clock':{'face':'analog' if a.style=='paper' else 'digital','surface':'panel' if a.style=='paper' else 'none','alignment':'center','center_vertically':True,'format_24h':True}}
if a.maximum:
 assert a.desktop, '--maximum needs --desktop'
 config.update(enabled_widgets=['clock','weather','calendar','devbox','projects'],music={'enabled':True},notes={'enabled':True},focus={'enabled':True},glance={'enabled':False},widget_positions={'clock':[.03,.03],'weather':[.40,.03],'calendar':[.03,.38],'devbox':[.40,.40],'projects':[.03,.85],'music':[.40,.85],'notes':[.94,.40],'focus':[.94,.03]},widget_sizes={'clock':[350,180],'weather':[300,150],'calendar':[350,200],'devbox':[300,226],'projects':[350,220],'music':[300,180],'notes':[300,200],'focus':[300,180]})
if a.moment_runtime:
 assert a.desktop
 config.update(enabled_widgets=['clock'],appearance={'family':'soft','wallpaper_light':True},clock={'face':'digital','surface':'panel','show_date':True,'format_24h':True},moments={'active':'day','day':{'family':'paper','clock':{'face':'analog'}},'night':{'family':'midnight','clock':{'face':'digital'}}},widget_positions={'clock':[.35,.2]},widget_sizes={'clock':[400,240]})
if a.ridge:
 assert a.desktop
 config.update(enabled_widgets=['clock'],appearance={'family':'soft','wallpaper_light':False},clock={'face':'digital','surface':'none','weight':'bold','depth':True,'scale':200,'show_date':False,'format_24h':True},widget_positions={'clock':[.5,a.ridge_position]},widget_sizes={'clock':[800/a.ridge_scale,330/a.ridge_scale]})
config_path.write_text(json.dumps(config));(root/'dconf-profile').write_text('')
if a.desktop:
 # Match the actual background polarity to the selected style. Otherwise the
 # normal wallpaper observer correctly updates Paper to the dark system default.
 color='(0.94,0.91,0.85)' if a.style=='paper' else '(0.055,0.105,0.14)'
 background='(output:"all",source:Color(Single('+color+')),filter_by_theme:true,rotation_frequency:3600,filter_method:Lanczos,scaling_mode:Zoom,sampling_method:Alphanumeric)'
 if a.ridge:
  picture=root/'prepared-ridge.png';shutil.copy2(wallpaper_assets/'halcyon-ridge.png',picture)
  background='(output:"all",source:Path('+json.dumps(str(picture))+'),filter_by_theme:true,rotation_frequency:3600,filter_method:Lanczos,scaling_mode:Zoom,sampling_method:Alphanumeric)'
 if a.ridge_fit: background=background.replace('scaling_mode:Zoom','scaling_mode:Fit((0.02,0.03,0.04))')
 for parent in (root/'desktop/config/cosmic',root/'config/cosmic'):
  target=parent/'com.system76.CosmicBackground/v1';target.mkdir(parents=True,exist_ok=True)
  for name,value in {'all':background,'same-on-all':'true','backgrounds':'[]'}.items():(target/name).write_text(value)
env=os.environ.copy()
for k in ('WAYLAND_DISPLAY','WAYLAND_SOCKET','COSMIC_SESSION_SOCK','PANEL_NOTIFICATIONS_FD','DAEMON_NOTIFICATIONS_FD','DBUS_SESSION_BUS_ADDRESS','SESSION_MANAGER','DISPLAY','DESKTOP_CARDS_SETTINGS_CAPTURE_DIR','DESKTOP_CARDS_CAPTURE_DIR'):env.pop(k,None)
for k in ('CONFIG','STATE','CACHE','DATA'):env['XDG_'+k+'_HOME']=str(root/k.lower())
env.update(XDG_RUNTIME_DIR=str(runtime),COSMIC_BACKEND='winit',WINIT_UNIX_BACKEND='x11',LIBGL_ALWAYS_SOFTWARE='1',RUST_LOG='warn',DCONF_PROFILE=str(root/'dconf-profile'),DESKTOP_CARDS_CONFIG=str(config_path),DESKTOP_CARDS_DESKTOP_ROOT=str(root/'desktop'),LD_LIBRARY_PATH=str(a.tools.resolve().parent/'lib/x86_64-linux-gnu'))
if a.ridge: env['HALCYON_DEPTH_TRACE']='1'
if a.gallery or a.canvas or a.mix or a.spine or a.ridge_studio or a.moments or a.recipes or a.packages or a.discovery:
 assert not a.desktop
 env['DESKTOP_CARDS_SETTINGS_CAPTURE_DIR']=str(root/'captures')
 env['DESKTOP_CARDS_SETTINGS_SIZE']=a.size
 (root/'captures').mkdir()
 bundle=root/'desktop/pictures/Desktop Cards';bundle.parent.mkdir(parents=True);bundle.symlink_to(wallpaper_assets)
 apps=root/'desktop/applications';apps.mkdir()
 for name in ['com.system76.CosmicAppletTime','com.system76.CosmicAppletWorkspaces','com.system76.CosmicAppletAudio','com.system76.CosmicAppList']:
  source=Path('/usr/share/applications')/(name+'.desktop')
  if source.is_file():shutil.copy2(source,apps/source.name)
 def seed(ns,key,value):
  target=root/'desktop/system/cosmic'/ns/'v1'/key;target.parent.mkdir(parents=True,exist_ok=True);target.write_text(value)
 if a.canvas or a.mix or a.spine or a.ridge_studio or a.moments or a.recipes or a.packages or a.discovery:
  wallpaper=bundle/('halcyon-paper.jpg' if a.style=='paper' else 'halcyon-calm.jpg')
  seed('com.system76.CosmicBackground','all','(output:"all",source:Path('+json.dumps(str(wallpaper))+'),filter_by_theme:true,rotation_frequency:3600,filter_method:Lanczos,scaling_mode:Zoom,sampling_method:Alphanumeric)')
  seed('com.system76.CosmicBackground','same-on-all','true')
  seed('com.system76.CosmicBackground','backgrounds','[]')
 seed('com.system76.CosmicPanel','entries','["Panel","Dock"]')
 for name,edge in [('Panel','Top'),('Dock','Bottom')]:
  for key,value in {'anchor':edge,'name':json.dumps(name),'size':'M','output':'All','expand_to_edges':'true' if name=='Panel' else 'false','plugins_center':'Some(["com.system76.CosmicAppletTime"])' if name=='Panel' else 'Some(["com.system76.CosmicAppList"])'}.items():seed('com.system76.CosmicPanel.'+name,key,value)
 seed('com.system76.CosmicPanel.Panel','plugins_wings','Some((["com.system76.CosmicAppletWorkspaces"],["com.system76.CosmicAppletAudio"]))')
 if a.islands_fixture:
  (root/'desktop/islands-installation.json').write_text(json.dumps(dict(format='halcyon-islands-installation',version=1,installation='installed',provenance='manual',enabled=True,compatible=True,status_version=2,status='Ready')))
  seed('io.github.desktopcards.DockIslands','Panel','(version:1,enabled:true,placement:Separate,gap:18,default_layout:(together:(order:(Start,Center,End),position:5000),separate:(order:(Start,Center,End),positions:(500,5000,9500))),outputs:{})')
owned=[];logs=[];display=None
result={'asset_root':str(wallpaper_assets),'asset_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(wallpaper_assets.iterdir()) if p.suffix in ['.jpg','.png']},'discovery':a.discovery,'packages':a.packages,'recipe_runtime':a.recipe_runtime,'recipes':a.recipes,'design':a.design,'moment_runtime':a.moment_runtime,'moments':a.moments,'ridge':a.ridge,'ridge_studio':a.ridge_studio,'ridge_scale':a.ridge_scale,'ridge_position':a.ridge_position,'ridge_fit':a.ridge_fit,'binary_sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),'binary':str(a.binary.resolve()),'style':a.style,'baseline':a.baseline,'desktop':a.desktop,'maximum':a.maximum,'gallery':a.gallery,'canvas':a.canvas,'mix':a.mix,'spine':a.spine,'islands_capability_fixture':a.islands_fixture,'window_size':a.size,'scope':'Private nested COSMIC on Xvfb; actual native input and composite framebuffer visibility, including polling/readback; not physical display presentation','timings':{},'memory_cycles':[],'completed':False}

def start(name,args,extra=None,pass_fds=()):
 log=(root/(name+'.log')).open('w');logs.append(log)
 proc=subprocess.Popen(args,env=env|(extra or {}),stdout=log,stderr=log,start_new_session=True,pass_fds=pass_fds);owned.append(proc);return proc

def stop(proc):
 if proc.poll() is not None:return
 try:os.killpg(proc.pid,signal.SIGTERM)
 except ProcessLookupError:return
 try:proc.wait(timeout=3)
 except subprocess.TimeoutExpired:
  os.killpg(proc.pid,signal.SIGKILL);proc.wait(timeout=3)

def wait_for(fn,seconds=10):
 until=time.monotonic()+seconds
 while not fn():
  if time.monotonic()>until:raise TimeoutError('Private fixture did not reach expected state')
  time.sleep(.01)

def stats(proc):
 assert proc.poll() is None
 fields=Path(f'/proc/{proc.pid}/stat').read_text().rsplit(')',1)[1].split()
 values=dict(s.split(':',1) for s in Path(f'/proc/{proc.pid}/status').read_text().splitlines() if ':' in s)
 drm=[]
 for fd in Path(f'/proc/{proc.pid}/fdinfo').iterdir():
  try:
   item=dict(s.split(':',1) for s in fd.read_text().splitlines() if s.startswith('drm-'))
   if item:drm.append({k:v.strip() for k,v in item.items()})
  except FileNotFoundError:pass
 return {'rss_kib':int(values['VmRSS'].split()[0]),'cpu_seconds':(int(fields[11])+int(fields[12]))/os.sysconf('SC_CLK_TCK'),'drm':drm}

def capture(name):
 subprocess.run(['import','-display',env['DISPLAY'],'-window','root',str(root/(name+'.png'))],env=env,check=True,timeout=10)

class XImage(C.Structure):
 _fields_=[('width',C.c_int),('height',C.c_int),('xoffset',C.c_int),('format',C.c_int),('data',C.c_void_p),('byte_order',C.c_int),('bitmap_unit',C.c_int),('bitmap_bit_order',C.c_int),('bitmap_pad',C.c_int),('depth',C.c_int),('bytes_per_line',C.c_int),('bits_per_pixel',C.c_int)]

def api(lib,name,restype,args):
 fn=getattr(lib,name);fn.restype=restype;fn.argtypes=args;return fn

class Screen:
 def __init__(self):
  x=C.CDLL('libX11.so.6');t=C.CDLL('libXtst.so.6');D=C.c_void_p;U=C.c_ulong;I=C.c_int
  self.open=api(x,'XOpenDisplay',D,[C.c_char_p]);self.close=api(x,'XCloseDisplay',I,[D]);self.d=self.open(env['DISPLAY'].encode());assert self.d
  self.root=api(x,'XDefaultRootWindow',U,[D])(self.d)
  self.sync=api(x,'XSync',I,[D,I]);self.motion=api(t,'XTestFakeMotionEvent',I,[D,I,I,I,U]);self.button=api(t,'XTestFakeButtonEvent',I,[D,C.c_uint,I,U]);self.key=api(t,'XTestFakeKeyEvent',I,[D,C.c_uint,I,U])
  self.sym=api(x,'XStringToKeysym',U,[C.c_char_p]);self.code=api(x,'XKeysymToKeycode',C.c_uint,[D,U])
  self.get=api(x,'XGetImage',C.POINTER(XImage),[D,U,I,I,C.c_uint,C.c_uint,U,I]);self.destroy=api(x,'XDestroyImage',I,[C.POINTER(XImage)])
 def move(self,x,y):self.motion(self.d,0,x,y,0);self.sync(self.d,0)
 def click(self,button=1):self.button(self.d,button,1,0);self.button(self.d,button,0,0);self.sync(self.d,0)
 def undo(self):
  ctrl=self.code(self.d,self.sym(b'Control_L'));z=self.code(self.d,self.sym(b'z'))
  for code,pressed in [(ctrl,1),(z,1),(z,0),(ctrl,0)]:self.key(self.d,code,pressed,0)
  self.sync(self.d,0)
 def pixels(self):
  im=self.get(self.d,self.root,435,475,200,120,C.c_ulong(-1).value,2);assert im
  try:return hashlib.blake2b(C.string_at(im.contents.data,im.contents.bytes_per_line*im.contents.height),digest_size=16).digest()
  finally:self.destroy(im)
 def settle(self):
  previous=self.pixels();stable=time.monotonic();until=stable+2
  while time.monotonic()<until:
   time.sleep(.01);current=self.pixels()
   if current!=previous:stable=time.monotonic();previous=current
   elif time.monotonic()-stable>.06:return
  raise TimeoutError('Framebuffer did not settle')

def summary(values):
 ordered=sorted(values)
 return {'samples':len(values),'p50_ms':ordered[(len(ordered)-1)//2],'p95_ms':ordered[math.ceil(.95*len(ordered))-1],'max_ms':max(values),'raw_ms':values}

def idle_settle(proc):
 # Exclude the final edit's queued file-watch/Undo deliveries from static idle.
 # Require one full second without CPU ticks, commits or an active image child.
 began=time.monotonic();quiet=began;previous=stats(proc)['cpu_seconds'];offset=(root/'settings.log').stat().st_size
 while time.monotonic()-began<10:
  time.sleep(.05);now=stats(proc)['cpu_seconds']
  with (root/'settings.log').open() as log:
   log.seek(offset);new=log.read();offset=log.tell()
  children=[]
  for thread in Path(f'/proc/{proc.pid}/task').iterdir():
   try:children.extend(thread.joinpath('children').read_text().split())
   except FileNotFoundError:pass
  if now!=previous or children or any('wl_surface' in s and '.commit(' in s for s in new.splitlines()):quiet=time.monotonic()
  previous=now
  if time.monotonic()-quiet>=1:return time.monotonic()-began
 raise TimeoutError('Native app did not reach a stable idle state')

try:
 read,write=os.pipe();start('xvfb',[str(a.tools/'Xvfb'),'-displayfd',str(write),'-screen','0','1600x1000x24','-nolisten','tcp'],pass_fds=(write,));os.close(write)
 with selectors.DefaultSelector() as sel:
  sel.register(read,selectors.EVENT_READ);assert sel.select(10)
  env['DISPLAY']=':'+os.read(read,64).decode().strip()
 os.close(read)
 start('bus',['dbus-daemon','--session','--nofork','--print-address=1']);wait_for(lambda:(root/'bus.log').stat().st_size>0);env['DBUS_SESSION_BUS_ADDRESS']=(root/'bus.log').read_text().splitlines()[0]
 if a.maximum:
  player=start('sample-player',['python3',str(Path(__file__).with_name('paused-player.py'))],{'HALCYON_PRIVATE_MPRIS':'1'})
  wait_for(lambda:'ready' in (root/'sample-player.log').read_text());assert player.poll() is None
  (root/'player-source.py').write_bytes(Path(__file__).with_name('paused-player.py').read_bytes())
 comp=start('compositor',['/usr/bin/cosmic-comp','--no-xwayland']);wait_for(lambda:any(p.is_socket() for p in runtime.glob('wayland-*')));assert comp.poll() is None
 env['WAYLAND_DISPLAY']=str(next(p for p in runtime.glob('wayland-*') if p.is_socket()))
 if a.ridge and a.ridge_scale!=1:
  subprocess.run(['cosmic-randr','mode','WINIT-0','1280','800','--scale',str(a.ridge_scale)],env=env,check=True,timeout=10)
  time.sleep(.5)
 result['outputs']=subprocess.check_output(['cosmic-randr','list','--kdl'],env=env,text=True,timeout=10)
 if a.design:
  assert a.moments or a.moment_runtime or a.recipes or a.packages or a.discovery or a.recipe_runtime, '--design needs a migrated inspector or desktop probe'
  subprocess.run([str(a.binary.resolve()),'--migrate-design'],env=env,check=True,capture_output=True,timeout=20)
  config=json.loads(config_path.read_text())
  result['migrated_instance_ids']=list(json.loads(config_path.with_name('design-v1.json').read_text())['instances'])
 def read_effective():
  if a.design:return json.loads(subprocess.check_output([str(a.binary.resolve()),'--read-widget-config'],env=env,text=True,timeout=10))
  return json.loads(config_path.read_text())
 def write_effective(current):
  if a.design:
   subprocess.run([str(a.binary.resolve()),'--edit-widget-config'],env=env,input=json.dumps({'before':read_effective(),'after':current}),text=True,capture_output=True,check=True,timeout=20)
  else:
   staged=root/'moment-config.tmp';staged.write_text(json.dumps(current));staged.replace(config_path)
 def seal_recipe(recipe):
  recipe['revision']=''
  recipe['revision']=hashlib.sha256(json.dumps(recipe,ensure_ascii=False,separators=(',',':')).encode()).hexdigest()
  return recipe
 if a.recipe_runtime:
  assert a.desktop and a.design
  applied=read_effective()
  def personal(kind,parts):
   return seal_recipe({'version':1,'id':'r-'+('1' if kind=='clock' else '2')*32,'revision':'','name':'Native '+kind,'kind':kind,'parts':parts,'caption':'Make room for today','compact':'stack','wide':'split'})
  applied['recipes']={'clock':personal('clock',['face','date','caption','settings']),'weather':personal('weather',['city','reading','conditions','forecast','settings'])}
  applied['widget_positions']={'clock':[.05,.14],'weather':[.82,.16]};applied['widget_sizes']={'clock':[520,240],'weather':[320,220]}
  write_effective(applied);recipe_initial=read_effective()
 if a.desktop:background_process=start('background',['/usr/bin/cosmic-bg'])
 if a.packages:
  import base64
  png=(wallpaper_assets/'halcyon-ridge.png').read_bytes();picture_hash=hashlib.sha256(png).hexdigest()
  origin={'identity':{'id':'s-'+'1'*32,'revision':''},'credits':{'author':'Halcyon fixture','license':'Private acceptance fixture','source':''},'parent':None,'picture':picture_hash}
  profile={'format':'desktop-cards-profile','version':3,'name':'Portable ridge','wallpaper':{'background':{'source':{'color':'#16252b'},'fit':'fill'}}}
  package={'format':'halcyon-setup-package','version':1,'origin':origin,'profile':profile,'wallpaper':{'sha256':picture_hash,'png':base64.b64encode(png).decode()}}
  origin['identity']['revision']=hashlib.sha256(json.dumps(package,separators=(',',':')).encode()).hexdigest()
  (root/'desktop/portable.halcyon').write_text(json.dumps(package,separators=(',',':')))
 app=start('settings',[str(a.binary.resolve())]+([] if a.desktop else ['--settings','builder' if a.recipes else 'moments' if a.moments else 'desktop-profiles' if (a.gallery or a.mix or a.spine or a.ridge_studio or a.moments or a.packages or a.discovery) else 'overview' if a.canvas else 'clock']),{'LIBGL_ALWAYS_SOFTWARE':'0','WAYLAND_DEBUG':'client'})
 time.sleep(3);assert app.poll() is None
 if a.desktop:assert background_process.poll() is None
 # Xvfb has no window manager. Pointer events reach the nested window without
 # focus, but keyboard events need an explicit focus on our sole X window.
 # No other X clients are launched in this private server.
 windows=subprocess.check_output([str(a.tools/'xdotool'),'search','--onlyvisible','--name','.'],env=env,text=True,timeout=5).splitlines()
 assert len(windows)==1, windows
 subprocess.run([str(a.tools/'xdotool'),'windowfocus',windows[0]],env=env,check=True,timeout=5)
 display=Screen();display.move(1100,750);display.settle();capture('clock-initial');result['initial']=stats(app)
 if a.recipe_runtime:
  before=config_path.read_bytes();capture('recipes-split')
  for cycle in range(6):
   current=read_effective();current['recipes']['clock']['wide']='stack' if cycle%2==0 else 'split';seal_recipe(current['recipes']['clock']);write_effective(current);time.sleep(.6);display.settle()
   if cycle==0:
    capture('recipes-stack');aa=subprocess.check_output(['convert',str(root/'recipes-split.png'),'rgba:-']);bb=subprocess.check_output(['convert',str(root/'recipes-stack.png'),'rgba:-']);result['recipe_changed_pixels']=sum(aa[i:i+4]!=bb[i:i+4] for i in range(0,len(aa),4));assert result['recipe_changed_pixels']>1000
   result['memory_cycles'].append({'cycle':cycle,**stats(app)})
  assert config_path.read_bytes()==before
  result['applied_recipe_intent_unchanged']=read_effective()==recipe_initial;assert result['applied_recipe_intent_unchanged']
  result['warm_cycle_growth_mib']=(result['memory_cycles'][-1]['rss_kib']-result['memory_cycles'][1]['rss_kib'])/1024
 if a.moment_runtime:
  before=config_path.read_bytes()
  capture('moment-day')
  def switch_moment(phase):
   current=read_effective();current['moments']['active']=phase
   write_effective(current)
   time.sleep(.6);display.settle()
  switch_moment('night');capture('moment-night')
  aa=subprocess.check_output(['convert',str(root/'moment-day.png'),'rgba:-']);bb=subprocess.check_output(['convert',str(root/'moment-night.png'),'rgba:-'])
  changed=sum(aa[i:i+4]!=bb[i:i+4] for i in range(0,len(aa),4))
  assert changed>1000,changed
  result['moment_changed_pixels']=changed
  for cycle in range(6):
   switch_moment('day' if cycle%2==0 else 'night')
   result['memory_cycles'].append({'cycle':cycle,**stats(app)})
  if a.design:
   switch_moment('day');assert config_path.read_bytes()==before
  else:
   staged=root/'moment-config.tmp';staged.write_bytes(before);staged.replace(config_path)
  time.sleep(.6);display.settle();capture('moment-restored')
  result['warm_cycle_growth_mib']=(result['memory_cycles'][-1]['rss_kib']-result['memory_cycles'][1]['rss_kib'])/1024
 if a.ridge:
  wait_for(lambda:'pixels=true' in (root/'settings.log').read_text())
  capture('ridge-depth')
  def change_depth(value):
   current=json.loads(config_path.read_text());current['clock']['depth']=value
   staged=root/'config.tmp';staged.write_text(json.dumps(current));staged.replace(config_path)
   time.sleep(.8);display.settle()
  change_depth(False);capture('ridge-flat')
  aimage=subprocess.check_output(['convert',str(root/'ridge-depth.png'),'rgba:-']);bimage=subprocess.check_output(['convert',str(root/'ridge-flat.png'),'rgba:-'])
  assert len(aimage)==len(bimage)
  changed=[i//4 for i in range(0,len(aimage),4) if max(abs(aimage[i+c]-bimage[i+c]) for c in range(3))>8]
  result['depth_changed_bbox']=None if not changed else [min(i%1600 for i in changed),min(i//1600 for i in changed),max(i%1600 for i in changed),max(i//1600 for i in changed)]
  result['depth_changed_pixels']=len(changed)
  assert result['depth_changed_pixels']>1000, result['depth_changed_pixels']
  change_depth(True);capture('ridge-restored')
  result['depth_cycles']=[stats(app)]
  for _ in range(5):
   change_depth(False);change_depth(True);result['depth_cycles'].append(stats(app))
  # Leave the enabled intent in place but change the pair identity. Both
  # backgrounds receive an atomic replacement and must return to a flat clock.
  offset=(root/'settings.log').stat().st_size
  picture=root/'prepared-ridge.png';shutil.copy2(wallpaper_assets/'halcyon-calm.jpg',root/'changed.tmp');(root/'changed.tmp').replace(picture)
  wait_for(lambda:'pixels=false' in (root/'settings.log').read_text()[offset:])
  time.sleep(.3);display.settle();capture('ridge-source-replaced')
  result['replaced_source_observed_plain_fallback']=True
  result['source_replacement_retained_intent']=json.loads(config_path.read_text())['clock']['depth'] is True
  assert result['source_replacement_retained_intent']
  offset=(root/'settings.log').stat().st_size
  shutil.copy2(wallpaper_assets/'halcyon-ridge.png',root/'restored.tmp');(root/'restored.tmp').replace(picture)
  wait_for(lambda:'pixels=true' in (root/'settings.log').read_text()[offset:])
  time.sleep(.3);display.settle();capture('ridge-source-restored')
  result['restored_source_observed_depth']=True
 if a.gallery or a.canvas or a.mix or a.spine or a.ridge_studio or a.moments or a.recipes or a.packages or a.discovery:
  def alive():
   assert app.poll() is None
   return True
  def command(data):
   target=config_path.with_name('settings-command.json');staged=root/'command.tmp';staged.write_text(json.dumps(data));staged.replace(target)
   wait_for(lambda:alive() and not target.exists())
   # Native jobs and image-child completion run after command admission.
   time.sleep(.5)
  def keep(name):
   meta=root/('captures/settings-overview.json' if a.canvas else 'captures/settings-builder.json' if a.recipes else 'captures/settings-moments.json' if a.moments else 'captures/settings-desktop-profiles.json');rgba=meta.with_suffix('.rgba')
   if a.moments or a.recipes or a.packages or a.discovery:
    # Capture requests wait 500 + 120 ms in the app. Await completion after
    # the command, not an old frame that happened to exist already.
    admitted=time.time_ns();command({})
    wait_for(lambda:meta.exists() and meta.stat().st_mtime_ns>admitted+600_000_000)
   wait_for(lambda:meta.exists() and rgba.exists())
   for attempt in range(30):
    dimensions=json.loads(meta.read_text());pixels=rgba.read_bytes()
    if len(pixels)==dimensions[0]*dimensions[1]*4:break
    time.sleep(.04)
   else:raise AssertionError('Capture never settled')
   subprocess.run(['convert','-size',f'{dimensions[0]}x{dimensions[1]}','-depth','8','rgba:-',str(root/'captures'/f'{name}.png')],input=pixels,check=True,timeout=10)
  if a.discovery:
   before=config_path.read_bytes();effective_before=read_effective()
   keep('discover')
   command({'profiles':[{'discover':'paper'}]});keep('discover-search')
   command({'profiles':[{'discover':''},{'select':'tidal'}]});keep('curated-review')
   command({'profiles':['remix']});keep('remix-start')
   command({'profiles':[{'mix_pick':['style','paper-studio']}]});keep('remix-changed')
   assert config_path.read_bytes()==before and read_effective()==effective_before
   command({'profiles':[{'save_mix':'My offline remix'}]})
   library=root/'desktop/state/profiles';wait_for(lambda:library.exists() and any(json.loads(p.read_text()).get('name')=='My offline remix' for p in library.glob('*.json')))
   saved=next(p for p in library.glob('*.json') if json.loads(p.read_text()).get('name')=='My offline remix');saved_before=saved.read_bytes()
   profile=json.loads(saved_before);assert len(profile['package']['ancestors'])==2
   keep('remix-saved');command({'scroll':.48});keep('remix-credits');command({'scroll':0})
   for cycle in range(4):
    command({'profiles':[{'select':'My offline remix'}]})
    # Selection starts palette work. Wait for a fresh capture of the enabled
    # native suggested button, not general app idle (the scene has a clock).
    deadline=time.monotonic()+10
    while True:
     keep(f'remix-ready-{cycle}')
     dimensions=json.loads((root/'captures/settings-desktop-profiles.json').read_text());pixels=(root/'captures/settings-desktop-profiles.rgba').read_bytes()
     x,y=dimensions[0]-38,dimensions[1]-24;offset=(y*dimensions[0]+x)*4;r,g,b,alpha=pixels[offset:offset+4]
     if g>r+30 and b>r+30 and alpha>200:break
     assert time.monotonic()<deadline, 'Apply did not become enabled after palette preparation'
    command({'profiles':['apply']})
    wait_for(lambda:read_effective()!=effective_before)
    if cycle==0:keep('remix-applied')
    display.undo();wait_for(lambda:read_effective()==effective_before)
    assert config_path.read_bytes()==before and saved.read_bytes()==saved_before
    result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   command({'profiles':[{'select':'paper-studio'}]});assert saved.read_bytes()==saved_before
   command({'profiles':['discard']});keep('discover-restored')
   result['capture_scope']='Offline bundled discovery/search, native remix preview/save, retained source credits, four Apply/keyboard Undo cycles and independent saved copy. No network catalog or host writes.'
  elif a.packages:
   before=config_path.read_bytes();effective_before=read_effective()
   command({'profiles':[{'import':'portable.halcyon'}]})
   library=root/'desktop/state/profiles';wait_for(lambda:library.exists() and len(list(library.glob('*.json')))==1)
   saved=json.loads(next(library.glob('*.json')).read_text());asset=Path(saved['wallpaper']['background']['source']['image']);assert asset.is_file() and hashlib.sha256(asset.read_bytes()).hexdigest()==picture_hash
   assert saved['package']['identity']==origin['identity'];keep('package-import-review')
   native_root=root/'desktop/config/cosmic'
   def native_values():
    # Undo may remove a key between directory enumeration and read. A partial
    # observation is not an empty/restored snapshot; retry the complete sample.
    for _ in range(20):
     try:return {str(p.relative_to(native_root)):p.read_bytes() for p in native_root.rglob('*') if p.is_file()} if native_root.exists() else {}
     except FileNotFoundError:time.sleep(.01)
    raise AssertionError('Native files did not settle for observation')
   native_before=native_values()
   for cycle in range(4):
    command({'profiles':[{'select':'Portable ridge'},'apply']})
    wait_for(lambda:any(str(asset).encode() in b for b in native_values().values()))
    if cycle==0:keep('package-applied')
    display.undo();wait_for(lambda:native_values()==native_before)
    assert config_path.read_bytes()==before and read_effective()==effective_before and asset.exists()
    result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   command({'profiles':[{'select':'Portable ridge'},{'package':{'action':'open'}}]});keep('package-export-review')
   command({'profiles':[{'package':{'action':'picture','value':True}}]});keep('package-picture-review')
   command({'profiles':[{'package':{'action':'close'}}]});keep('package-credits-retained')
   assert native_values()==native_before and config_path.read_bytes()==before
   result['capture_scope']='Native independent-format package import, stored image/credits verification, four Apply/keyboard Undo cycles and compact/wide export review. File chooser save itself is covered by native-tree and file-IO tests.'
  elif a.recipes:
   assert a.design
   before=config_path.read_bytes();effective_before=read_effective()
   keep('builder-start')
   command({'recipe':{'action':'name','value':'Morning paper'}})
   command({'recipe':{'action':'add','part':'caption'}})
   command({'recipe':{'action':'save'}})
   library=root/'desktop/state/widget-recipes.json'
   wait_for(lambda:library.exists())
   saved=json.loads(library.read_text());assert len(saved['entries'])==1
   for cycle in range(4):
    command({'recipe':{'action':'apply'}})
    wait_for(lambda:read_effective().get('recipes',{}).get('clock',{}).get('name')=='Morning paper')
    if cycle==0:keep('builder-applied')
    display.undo();wait_for(lambda:read_effective()==effective_before)
    assert config_path.read_bytes()==before
    result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   command({'recipe':{'action':'wide','value':True}});keep('builder-wide')
   command({'recipe':{'action':'delete'}});command({'recipe':{'action':'confirm_delete'}})
   wait_for(lambda:len(json.loads(library.read_text())['entries'])==0)
   command({'recipe':{'action':'new','kind':'weather'}});keep('builder-weather')
   command({'recipe':{'action':'missing','value':True}});keep('builder-weather-missing')
   assert read_effective()==effective_before and config_path.read_bytes()==before
   result['capture_scope']='Native personal recipe Save/Apply, four keyboard Undo cycles, independent library delete, compact/wide and missing-location samples. No duplicate instance or new provider.'
  elif a.moments:
   before=config_path.read_bytes();effective_before=read_effective()
   keep('moments-day')
   command({'moment':{'action':'preview','phase':'night'}});keep('moments-night')
   assert (root/'captures/moments-day.png').read_bytes()!=(root/'captures/moments-night.png').read_bytes()
   assert config_path.read_bytes()==before
   for cycle in range(6):
    phase='day' if cycle%2==0 else 'night'
    command({'moment':{'action':'activate','phase':phase}})
    wait_for(lambda:read_effective().get('moments',{}).get('active')==phase)
    if cycle==0:keep('moments-active')
    display.undo();wait_for(lambda:read_effective()==effective_before)
    assert config_path.read_bytes()==before
    result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   command({'moment':{'action':'face','phase':'day','value':'analog'}})
   wait_for(lambda:read_effective().get('moments',{}).get('day',{}).get('clock',{}).get('face')=='analog')
   assert read_effective().get('moments',{}).get('active') is None
   command({'moment':{'action':'preview','phase':'day'}});keep('moments-analog-preview')
   display.undo();wait_for(lambda:read_effective()==effective_before)
   assert config_path.read_bytes()==before
   keep('moments-restored')
   result['capture_scope']='Native Day/Night previews, six activation/keyboard Undo cycles, inactive Day clock edit and exact config restoration; sample content and private desktop only.'
  elif a.ridge_studio:
   before=config_path.read_bytes()
   keep('studio')
   command({'profiles':[{'select':'ridge'}]});time.sleep(1);keep('ridge-review')
   command({'profiles':[{'compare_current':True}]});time.sleep(.6);keep('ridge-current')
   command({'profiles':[{'compare_current':False}]});time.sleep(1);keep('ridge-candidate')
   assert config_path.read_bytes()==before
   for cycle in range(6):
    if cycle:command({'profiles':[{'select':'ridge'}]})
    command({'profiles':['apply']})
    wait_for(lambda:json.loads(config_path.read_text()).get('clock',{}).get('depth') is True)
    if cycle==0:time.sleep(1);keep('ridge-applied')
    command({'undo_preferences':True})
    wait_for(lambda:config_path.read_bytes()==before)
    command({'profiles':['discard']})
    result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   keep('ridge-restored')
   result['capture_scope']='Native Ridge candidate/current comparison, six actual worker Apply/Undo cycles, cancel and exact config restoration. Public fixture content; no host changes.'
  elif a.spine:
   before=config_path.read_bytes()
   command({'profiles':[{'select':'spine'}]});time.sleep(.6);keep('spine-review')
   command({'profiles':['apply']});time.sleep(.6);keep('spine-applied')
   source=root/'desktop/config/cosmic/com.system76.CosmicPanel.Panel/v1'
   assert (source/'anchor').read_text().strip()=='Left'
   # Use committed values and existing system defaults, in a second private
   # XDG config tree visible only to this owned stock panel process.
   panel_home=root/'stock-config';target=panel_home/'cosmic/com.system76.CosmicPanel.Panel/v1';target.mkdir(parents=True)
   defaults=root/'desktop/system/cosmic/com.system76.CosmicPanel.Panel/v1'
   for directory in [defaults,source]:
    for entry in directory.iterdir():
     if entry.is_file():shutil.copy2(entry,target/entry.name)
   container=panel_home/'cosmic/com.system76.CosmicPanel/v1';container.mkdir(parents=True)
   (container/'entries').write_text('["Panel"]');(target/'name').write_text('"Panel"');(target/'output').write_text('All')
   # A single real native Clock applet; no personal account/provider data.
   (target/'plugins_wings').write_text('None');(target/'plugins_center').write_text('Some(["com.system76.CosmicAppletTime"])')
   panel=start('spine-stock-panel',['/usr/bin/cosmic-panel'],{'XDG_CONFIG_HOME':str(panel_home),'WAYLAND_DEBUG':'client'})
   time.sleep(2);assert panel.poll() is None
   raw=(root/'spine-stock-panel.log').read_text()
   events=[line.strip() for line in raw.splitlines() if 'zwlr_layer_surface' in line and any(word in line for word in ('set_anchor','set_size','.configure('))]
   assert any('set_anchor(7)' in line for line in events),events[-12:]
   result['stock_panel_protocol']=events
   capture('spine-stock-panel')
   offset=(root/'settings.log').stat().st_size
   stop(panel)
   # Panel removal returns focus asynchronously. The first probe emitted keys
   # before wl_keyboard.enter, so no key event reached Settings.
   wait_for(lambda:any('wl_keyboard' in line and '.enter(' in line for line in (root/'settings.log').read_text()[offset:].splitlines()))
   display.undo()
   wait_for(lambda:json.loads(config_path.read_text())==json.loads(before))
   assert json.loads(config_path.read_text())==json.loads(before)
   command({'profiles':['discard']});keep('spine-restored')
   result['capture_scope']='Actual native Settings review and one fixture Apply/Undo; actual stock vertical panel consumes committed profile settings with one real native Clock applet. Keyboard Undo waits for compositor focus. No Islands orientation claim.'
   result['memory_cycles']=[{'cycle':0,**stats(app)},{'cycle':1,**stats(app)}]
  elif a.mix:
   keep('setups')
   before=config_path.read_bytes()
   def choose_mix():
    command({'profiles':[{'mix_start':True}]})
    for part,source in [('picture','current'),('style','paper-studio'),('layout','tidal')]:
     command({'profiles':[{'mix_pick':[part,source]}]})
    time.sleep(.8)
   choose_mix();keep('mix-preview')
   assert config_path.read_bytes()==before
   command({'profiles':['discard']});assert config_path.read_bytes()==before
   for cycle in range(26):
    choose_mix()
    assert config_path.read_bytes()==before
    command({'profiles':['apply']})
    wait_for(lambda:config_path.read_bytes()!=before)
    if cycle==0:keep('mix-applied')
    command({'undo_preferences':True})
    wait_for(lambda:config_path.read_bytes()==before)
    if cycle in (0,5,10,15,20,25):result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   result['capture_scope']='Actual native release window; Mix board candidate, cancel and 26 combined Apply/Undo cycles. Fixture writes; production worker admission has a separate forced-worker regression. No provider or installed desktop changes.'
  elif a.canvas:
   keep('layout')
   before=config_path.read_bytes()
   command({'canvas':'start','placement':[{'select':'clock'}]});keep('canvas-current')
   command({'canvas_style':{'field':'/clock/face','value':'digital' if a.style=='paper' else 'analog'},'placement':[{'nudge':[1,0]},{'resize':[0,1]}]})
   keep('canvas-preview')
   assert config_path.read_bytes()==before
   command({'scroll':.65});keep('canvas-inspector');command({'scroll':0})
   command({'canvas':'cancel'});assert config_path.read_bytes()==before
   for cycle in range(26):
    command({'canvas':'start','placement':[{'select':'clock'}]})
    command({'canvas_style':{'field':'/clock/face','value':'digital' if a.style=='paper' else 'analog'},'placement':[{'nudge':[1,0]}]})
    assert config_path.read_bytes()==before
    command({'canvas':'apply'})
    wait_for(lambda:config_path.read_bytes()!=before)
    if cycle==0:keep('canvas-applied')
    command({'undo_preferences':True})
    wait_for(lambda:config_path.read_bytes()==before)
    if cycle in (0,5,10,15,20,25):result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   result['capture_scope']='Actual native Settings release windows; local canvas/style drafts, cancel and 26 worker Apply/Undo cycles. No providers or installed desktop changes.'
  else:
   keep('gallery')
   for name in ['tidal','paper-studio','moss','carbon']:
    command({'profiles':[{'select':name}]});time.sleep(.6);keep(name)
    command({'profiles':['discard']})
   for cycle in range(26):
    command({'profiles':[{'select':['tidal','paper-studio','moss','carbon'][cycle%4]}]})
    command({'profiles':['discard']})
    if cycle in (0,5,10,15,20,25):result['memory_cycles'].append({'cycle':cycle,**stats(app)})
   result['capture_scope']='Actual native Settings release windows; four local preset candidates; public sample data and fixture applet/capability observations. No companion runtime claim.'
  result['warm_cycle_growth_mib']=None if a.spine else (result['memory_cycles'][-1]['rss_kib']-result['memory_cycles'][1]['rss_kib'])/1024
  result['configuration_unchanged']=json.loads(config_path.read_text())==config
  assert result['configuration_unchanged']
 elif not (a.inspect or a.desktop):
  display.move(1010,550);display.settle()
  latencies=[]
  for n in range(34):
   before=display.pixels();began=time.monotonic();display.click(5 if n%2==0 else 4)
   while display.pixels()==before:
    if time.monotonic()-began>1:raise TimeoutError('Scroll did not change the observed native content')
    time.sleep(.0003)
   elapsed=(time.monotonic()-began)*1000
   if n>=4:latencies.append(elapsed)
   display.settle()
  result['timings']['input_to_visible_frame_upper_bound']=summary(latencies)
  display.move(1010,550)
  frames=[];before=display.pixels();began=time.monotonic();next_move=began;steps=0
  while time.monotonic()-began<3:
   now=time.monotonic()
   if now>=next_move:
    display.click(5 if steps%8<4 else 4);steps+=1;next_move=began+steps/60
   current=display.pixels()
   if current!=before:frames.append(time.monotonic());before=current
   time.sleep(.0003)
  display.settle()
  intervals=[(b-a)*1000 for a,b in zip(frames,frames[1:])];assert len(intervals)>=30, f'Not enough native scroll frames: {len(intervals)}'
  result['timings']['changed_composite_frame_intervals']=summary(intervals)
  result['timings']['frame_workload']='Three seconds of native wheel input at 60 Hz, reversing every four steps within the content; composite framebuffer changes'
  capture('clock-scrolled')
  def page(name):
   subprocess.run([str(a.binary.resolve()),'--settings',name],env=env|{'LIBGL_ALWAYS_SOFTWARE':'0'},check=True,timeout=5,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
   display.settle();assert app.poll() is None
  for cycle in range(26):
   # Same page-open/close workload for the retained published release and current binary.
   for name in ('weather','themes','clock'):page(name)
   if cycle in (0,1,5,10,15,20,25):result['memory_cycles'].append({'cycle':cycle,**stats(app)})
  result['warm_cycle_growth_mib']=(result['memory_cycles'][-1]['rss_kib']-next(r['rss_kib'] for r in result['memory_cycles'] if r['cycle']==5))/1024
  capture('clock-after-cycles')
  if not a.baseline:
   result['edit_cancel_cycles']=[]
   for cycle in range(26):
    page('weather');page('clock')
    display.move(1010,550)
    for _ in range(30):display.click(4)
    display.settle()
    display.move(995,637 if a.style=='paper' else 629)
    display.click()
    wait_for(lambda:json.loads(config_path.read_text()).get('clock',{}).get('format_24h') is False)
    display.undo()
    wait_for(lambda:json.loads(config_path.read_text()).get('clock',{}).get('format_24h') is True)
    display.settle()
    if cycle in (0,1,5,10,15,20,25):result['edit_cancel_cycles'].append({'cycle':cycle,**stats(app)})
   result['edit_cancel_growth_mib']=(result['edit_cancel_cycles'][-1]['rss_kib']-next(r['rss_kib'] for r in result['edit_cancel_cycles'] if r['cycle']==5))/1024
 if not (a.inspect or a.gallery or a.canvas or a.mix or a.spine or a.ridge_studio or a.moments or a.recipes or a.packages or a.discovery):
  result['idle_settle_seconds']=idle_settle(app)
  print(json.dumps({'phase':'static_idle','style':a.style,'desktop':a.desktop,'settle_seconds':result['idle_settle_seconds'],'timings_p95_ms':{k:v['p95_ms'] for k,v in result['timings'].items() if isinstance(v,dict)},'warm_cycle_growth_mib':result.get('warm_cycle_growth_mib'),'edit_cancel_growth_mib':result.get('edit_cancel_growth_mib')}),flush=True)
  first=stats(app);log_offset=(root/'settings.log').stat().st_size;start_idle=time.monotonic();time.sleep(a.idle_seconds);last=stats(app);elapsed=time.monotonic()-start_idle
  with (root/'settings.log').open() as log:
   log.seek(log_offset);idle_protocol=log.read()
  result['idle']={'seconds':elapsed,'cpu_percent_one_core':(last['cpu_seconds']-first['cpu_seconds'])*100/elapsed,'surface_commits':sum('wl_surface' in s and '.commit(' in s for s in idle_protocol.splitlines()),'frame_requests':sum('wl_surface' in s and '.frame(' in s for s in idle_protocol.splitlines()),'first':first,'last':last}
  result['configuration_unchanged']=json.loads(config_path.read_text())==config
  if a.desktop:capture('desktop-after-idle')
  assert result['configuration_unchanged']
 result['completed']=True
except BaseException as error:
 result['error']=repr(error)
 if display is not None:
  try:capture('failure')
  except Exception:pass
 raise
finally:
 if display is not None:display.close(display.d)
 for proc in reversed(owned):stop(proc)
 for log in logs:log.close()
 result['owned_children_reaped']=all(p.poll() is not None for p in owned)
 (root/'result.json').write_text(json.dumps(result,indent=2)+'\n')
 shutil.rmtree(runtime)
 print(json.dumps({k:result.get(k) for k in ('completed','error','warm_cycle_growth_mib','owned_children_reaped')}),flush=True)
