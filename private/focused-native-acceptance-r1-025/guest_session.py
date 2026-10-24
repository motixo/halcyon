import pathlib,os
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
