"""Root controller in disposable offline guest. Measurements run as real probe user."""
import hashlib,json,os,pathlib,signal,subprocess,time,traceback
H=pathlib.Path(__file__).resolve().parent;OUT=H/'results';OUT.mkdir(exist_ok=True);timeline=OUT/'timeline.jsonl'
def emit(**kw):
 with timeline.open('a') as f:f.write(json.dumps(dict(unix_time=time.time(),**kw))+'\n')
def command(args,timeout=90):
 p=subprocess.run(args,capture_output=True,text=True,timeout=timeout)
 if p.returncode:raise RuntimeError('command failed '+args[0]+': '+p.stderr[-2000:])
 return p.stdout
def lock():
 return {a:b for a,b,c in (line.split('\t') for line in command(['dpkg-query','-W','-f=${Package}\t${Version}\t${db:Status-Status}\n']).splitlines()) if c=='installed'}
def session_env():
 for p in pathlib.Path('/proc').iterdir():
  if not p.name.isdigit():continue
  try:
   if p.stat().st_uid!=1000 or (p/'exe').resolve().name!='cosmic-session':continue
   e=dict(x.split(b'=',1) for x in (p/'environ').read_bytes().split(b'\0') if b'=' in x)
   sid=e.get(b'XDG_SESSION_ID',b'').decode()
   if not sid:continue
   return dict(HOME='/home/probe',USER='probe',LOGNAME='probe',LANG='C.UTF-8',PATH='/home/probe/.local/bin:/usr/bin:/bin',XDG_RUNTIME_DIR='/run/user/1000',XDG_SESSION_ID=sid,DBUS_SESSION_BUS_ADDRESS='unix:path=/run/user/1000/bus',WAYLAND_DISPLAY=e.get(b'WAYLAND_DISPLAY',b'wayland-1').decode(),PYTHONDONTWRITEBYTECODE='1')
  except OSError:continue
 raise RuntimeError('no real probe COSMIC session')
def run(name,mode,*,profile=False,cold=False,deadline=600):
 if cold:
  os.sync();pathlib.Path('/proc/sys/vm/drop_caches').write_text('3\n');emit(event='guest_caches_dropped',name=name,host_storage_cache='NOT dropped; backing image hash warmed controller cache')
 args=['/usr/sbin/runuser','-u','probe','--','/usr/bin/env','-i',*[k+'='+v for k,v in session_env().items()],'/usr/bin/python3','-B',str(H/'measure.py'),mode]+(['--profile'] if profile else [])
 emit(event='start',name=name,mode=mode,cold=cold,deadline=deadline);started=time.perf_counter()
 with (OUT/(name+'.stdout')).open('wb') as out,(OUT/(name+'.stderr')).open('wb') as err:
  p=subprocess.Popen(args,stdout=out,stderr=err,start_new_session=True)
  try:code=p.wait(timeout=deadline);killed=False
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait(timeout=10);code=p.returncode;killed=True
 summary=dict(name=name,mode=mode,wall_seconds=time.perf_counter()-started,returncode=code,timed_out=killed,deadline=deadline,cold_guest_cache=cold)
 (OUT/(name+'.controller.json')).write_text(json.dumps(summary,indent=2)+'\n');emit(event='end',**summary)
 if (code or killed) and deadline!=6:raise RuntimeError('bounded measurement failed '+name)
 return summary
def main():
 expected=json.loads((H/'locks.json').read_text());before=lock();assert before==expected['new'];assert command(['uname','-r']).strip()==expected['kernel'];assert not command(['dpkg','--audit']).strip()
 (OUT/'initial-system.json').write_text(json.dumps(dict(packages=before,count=len(before),kernel=expected['kernel'],dpkg_audit='empty'),indent=2)+'\n')
 records=json.loads(command(['/usr/bin/python3','-B',str(H/'build_fixture.py'),str(H/'packages')],180));(OUT/'fixture-identities.json').write_text(json.dumps(records,indent=2)+'\n')
 for rows in (1,2):
  install=command(['dpkg','-i',str(H/'packages'/('fixture-'+str(rows)+'.deb'))]);(OUT/f'fixture-{rows}-install.txt').write_text(install)
  actual=lock();assert {k:v for k,v in actual.items() if k!='cosmic-ext-halcyon-islands'}==before;assert len(actual)==1784
  assert not command(['dpkg','--audit']).strip()
  # No selected generation, no enabled panel, no admission. Native status is real.
  if rows==1:
   for index in range(3):run(f'identity-{index}','identity',profile=True,cold=index==0)
   for index in range(3):run(f'audit-{index}','audit',cold=index==0)
   run('runtime-profile','runtime',profile=True)
  for index in range(3):run(f'status-{rows}-{index}','status',cold=index==0)
  run(f'status-{rows}-profile','status',profile=True)
  run(f'status-{rows}-six-second','status',deadline=6)
  if rows==1:
   for index in range(3):run(f'prelaunch-{index}','prelaunch',profile=True,cold=index==0)
   run('stage-profile','stage',profile=True)
 # Never change or enable actual user panel configuration or selector.
 assert not pathlib.Path('/home/probe/.local/share/cosmic-desktop-cards/companion/generation.json').exists()
 assert not pathlib.Path('/home/probe/.local/bin/cosmic-panel').exists()
 (OUT/'final-system.json').write_text(json.dumps(dict(count=len(lock()),base_package_versions_unchanged=True,only_added_package='cosmic-ext-halcyon-islands development timing fixture',qualified=False),indent=2)+'\n')
 emit(event='completed',qualified=False)
if __name__=='__main__':
 try:main()
 except BaseException as e:
  (OUT/'failure.json').write_text(json.dumps(dict(error=str(e),traceback=traceback.format_exc(),qualified=False),indent=2)+'\n');raise
