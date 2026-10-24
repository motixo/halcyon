#!/usr/bin/python3
"""Disposable qualification trial ONLY; never a production entry point/catalog."""
import os,sys,pathlib,json,hashlib,fcntl,time
B=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands');D=pathlib.Path('/var/lib/ps025');O=D/'results'
def sha(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def enter(launcher,bundle=B,root=D,record=None):
 def emit(event,**kw):
  data=dict(event=event,time=time.time(),monotonic=time.monotonic(),pid=os.getpid(),qualified=False,**kw)
  if record:record(data)
  else:
   with (root/'results/trial-entry.jsonl').open('a') as f:f.write(json.dumps(data)+'\n')
 if launcher.other_host() is not None:emit('duplicate-host-refused');return 73
 fd=os.open(pathlib.Path(os.environ['XDG_RUNTIME_DIR'])/'desktop-cards-panel.lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW|os.O_CLOEXEC,0o600)
 try:
  try:fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
  except BlockingIOError:emit('lock-refused');return 73
  inherited=[];descriptors={}
  for name in ('PANEL_NOTIFICATIONS_FD','WAYLAND_SOCKET'):
   try:
    n=int(os.environ[name]);s=os.fstat(n);inherited.append(n);os.set_inheritable(n,True);descriptors[name]={'fd':n,'device':s.st_dev,'inode':s.st_ino}
   except (KeyError,ValueError,OSError):pass
  ticket=root/'trial-ticket';claimed=root/'trial-ticket.consumed'
  try:
   # Atomic hard-link claim, no overwrite/rearm. A crash after claim is stock-only.
   os.link(ticket,claimed,follow_symlinks=False);ticket.unlink()
  except FileNotFoundError:emit('stock-no-ticket',descriptors=descriptors);return launcher.stock_host(launcher.STOCK,tuple(inherited))
  except FileExistsError:emit('stock-already-consumed',descriptors=descriptors);return launcher.stock_host(launcher.STOCK,tuple(inherited))
  expected=json.loads((root/'integration2.json').read_text());assert sha(bundle/'cosmic-panel-islands')==expected['packages']['islands']['elf_sha256']
  for name,h in expected['helper_hashes'].items():assert sha(bundle/name)==h
  assert sha(bundle/'catalog.json')==expected['catalog_sha256'] and not json.loads((bundle/'catalog.json').read_text())['cohorts']
  def failed():
   with (root/'trial-failed').open('a') as f:f.write('Trial exited; next session-managed host is stock.\n');f.flush();os.fsync(f.fileno())
   emit('trial-failed-stock-requested')
  auth=json.loads((root/'results/test-stage-authorization.json').read_text());stage=root/'test-stage'
  assert auth['qualified'] is False and auth['state']=='PENDING-OBSERVATION' and auth['run_id']==(root/'run-id.txt').read_text().strip()
  assert sha(stage/'manifest.json')==auth['test_manifest_sha256'] and sha(stage/'catalog.json')==auth['test_catalog_sha256']
  began=time.monotonic();admission=launcher.prepare_launch(stage)
  emit('production-prelaunch',seconds=time.monotonic()-began,eligible=admission is not None,test_manifest_sha256=auth['test_manifest_sha256'])
  if admission is None:
   failed();emit('production-prelaunch-refused')
   from diagnose_refusal import bounded_diagnostic
   bounded_diagnostic(stage,root/'results');return 75
  accepted=admission.accepted
  def record_accepted():
   accepted();emit('production-running-accepted',seconds_from_prelaunch_start=time.monotonic()-began)
  admission.accepted=record_accepted
  emit('trial-start',binary_sha256=sha(bundle/'cosmic-panel-islands'),descriptors=descriptors,admission='PRODUCTION-CHECKS-ISOLATED-PENDING-TEST')
  try:rc=launcher.supervise(stage/'cosmic-panel-islands',launcher.STOCK,tuple(inherited),failed,'PANEL_NOTIFICATIONS_FD' in descriptors,admission=admission)
  finally:admission.close()
  emit('trial-supervisor-exit',returncode=rc);return rc
 finally:os.close(fd)
if __name__=='__main__':
 assert os.getuid()==1000 and pathlib.Path.home()==pathlib.Path('/home/probe'),'disposable user only'
 sys.path.insert(0,str(B));import panel_launcher
 raise SystemExit(enter(panel_launcher))
