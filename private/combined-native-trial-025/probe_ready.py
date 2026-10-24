import os,pathlib,socket,struct,json,time,sys,hashlib
D=pathlib.Path('/var/lib/ps025');end=time.monotonic()+15;error='not started'
while time.monotonic()<end:
 try:
  events=[json.loads(l) for l in (D/'results/trial-entry.jsonl').read_text().splitlines()];event=next(x for x in reversed(events) if x['event']=='trial-start');deadline=event['monotonic']+15
  if time.monotonic()>deadline:raise RuntimeError('15-second readiness deadline')
  with socket.socket(socket.AF_UNIX) as s:
   s.settimeout(.5);s.connect(os.environ['XDG_RUNTIME_DIR']+'/desktop-cards-islands-v1.sock');pid,uid,gid=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert uid==1000
   exe=pathlib.Path('/proc')/str(pid)/'exe';assert exe.resolve()==pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands/cosmic-panel-islands')
   with exe.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()=='07d95f0eda4675bb5b01619d79192248e3a2a687b508cd752c0611b50d4cc4e8'
   s.sendall(b'"Capabilities"\n');b=s.makefile('rb').readline(65537);assert len(b)<=65536 and b.endswith(b'\n');cap=json.loads(b)['Capabilities'];assert cap['protocol']==1 and cap['together'] and cap['separate']
   out={'pid':pid,'uid':uid,'capabilities':cap,'seconds_from_trial_start':time.monotonic()-event['monotonic'],'qualified':False,'admission':'NONE-IPC-ONLY','start_ticks':(pathlib.Path('/proc')/str(pid)/'stat').read_text().rsplit(')',1)[1].split()[19]};assert out['seconds_from_trial_start']<15
   (D/'results/readiness.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out));sys.exit(0)
 except (OSError,AssertionError,ValueError,KeyError,StopIteration) as e:error=type(e).__name__;time.sleep(.1)
raise RuntimeError('readiness failed: '+error)
