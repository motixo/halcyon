import os,pathlib,signal,sys,time
work=pathlib.Path(sys.argv[1]);p=work/'lockup-qemu.pid'
if p.exists():
 pid=int(p.read_text());cmd=pathlib.Path(f'/proc/{pid}/cmdline')
 if cmd.exists():
  data=cmd.read_bytes()
  if str(work).encode() not in data or b'qmp.sock' not in data:raise RuntimeError('Refuse to signal unowned VM')
  os.kill(pid,signal.SIGTERM)
  for _ in range(30):
   if not pathlib.Path(f'/proc/{pid}/exe').exists():break
   time.sleep(.5)
  else:os.kill(pid,signal.SIGKILL)
