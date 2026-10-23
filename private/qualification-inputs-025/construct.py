#!/usr/bin/env python3
"""One owned offline VM at a time; construction + standalone-image stock reboot.
Infrastructure evidence only. Does not install Halcyon or grant qualification.
"""
import base64,json,os,pathlib,shutil,socket,subprocess,time,traceback
from common import HERE,sha,pins
W=pathlib.Path(os.environ['WORK']);E=W/'evidence';R=W/'original';N=W/'new-artifact';L=json.loads((HERE/'locks.json').read_text())

class Channel:
 def __init__(self,path):
  self.s=socket.socket(socket.AF_UNIX);self.s.settimeout(30);self.s.connect(str(path));self.f=self.s.makefile('rwb',buffering=0)
 def close(self):self.f.close();self.s.close()
 def call(self,command,args=None):
  self.f.write((json.dumps({'execute':command,'arguments':args or {}})+'\n').encode())
  while True:
   line=self.f.readline()
   if not line:raise RuntimeError('QGA closed')
   try:r=json.loads(line.lstrip(b'\xff'))
   except json.JSONDecodeError:continue
   if 'event' in r:continue
   if 'error' in r:raise RuntimeError('QGA error: '+json.dumps(r['error']))
   return r['return']
def guest(work,path,args,timeout=900):
 c=Channel(work/'agent.sock')
 try:
  pid=c.call('guest-exec',{'path':path,'arg':args,'capture-output':True})['pid'];end=time.monotonic()+timeout
  while time.monotonic()<end:
   s=c.call('guest-exec-status',{'pid':pid})
   if s.get('exited'):
    if s.get('out-truncated') or s.get('err-truncated'):raise RuntimeError('QGA evidence truncated')
    return s.get('exitcode',-1),base64.b64decode(s.get('out-data','')),base64.b64decode(s.get('err-data',''))
   time.sleep(.5)
  raise RuntimeError('guest exec deadline')
 finally:c.close()
def put(work,source,target):
 c=Channel(work/'agent.sock');h=None
 try:
  h=c.call('guest-file-open',{'path':target,'mode':'w'})
  with source.open('rb') as f:
   while b:=f.read(32768):
    result=c.call('guest-file-write',{'handle':h,'buf-b64':base64.b64encode(b).decode()});assert result['count']==len(b)
  c.call('guest-file-flush',{'handle':h})
 finally:
  if h is not None:c.call('guest-file-close',{'handle':h})
  c.close()

def boot(work,disk,vars_file):
 work.mkdir(exist_ok=True);(work/'evidence').mkdir(exist_ok=True)
 args=['qemu-system-x86_64','-machine','pc-q35-8.2','-accel','kvm','-cpu','host','-smp','4','-m','6144',
 '-drive',f'if=pflash,format=raw,readonly=on,file={R}/OVMF_CODE_4M.fd','-drive',f'if=pflash,format=raw,file={vars_file}',
 '-drive',f'if=none,id=osdisk,format=qcow2,file={disk}','-device','virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0',
 '-device','virtio-vga,addr=0x2,xres=1280,yres=800','-device','qemu-xhci,addr=0x3','-device','usb-kbd','-device','usb-tablet',
 '-device','virtio-serial,addr=0x4','-chardev',f'socket,path={work}/agent.sock,server=on,wait=off,id=agent',
 '-device','virtserialport,chardev=agent,name=org.qemu.guest_agent.0','-nic','none','-display','none',
 '-qmp',f'unix:{work}/qmp.sock,server=on,wait=off','-serial',f'file:{work}/evidence/serial.log']
 (work/'evidence/qemu-args.json').write_text(json.dumps(args,indent=2)+'\n')
 log=(work/'evidence/qemu.log').open('wb');p=subprocess.Popen(args,stdout=log,stderr=log,env={'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','HOME':'/root','LANG':'C.UTF-8'})
 for _ in range(150):
  if p.poll() is not None:raise RuntimeError('QEMU exited before QGA')
  try:
   c=Channel(work/'agent.sock');c.call('guest-ping');c.close();return p,log
  except (OSError,RuntimeError):time.sleep(2)
 p.terminate();p.wait(timeout=30);log.close();raise RuntimeError('QGA boot timeout')
def stop(p,log,work,graceful=True):
 if p.poll() is None and graceful:
  try:
   c=Channel(work/'agent.sock');c.call('guest-shutdown',{'mode':'powerdown'});c.close()
  except (OSError,RuntimeError):pass
  try:p.wait(timeout=90)
  except subprocess.TimeoutExpired:pass
 if p.poll() is None:p.terminate();p.wait(timeout=30)
 log.close()
def action(work,mode):
 target='/var/lib/qualification-inputs-025'
 code,out,err=guest(work,'/usr/bin/mkdir',['-p',target],30);assert code==0
 for name in ['locks.json','guest_action.py','common.py']:put(work,HERE/name,target+'/'+name)
 expected={name:sha(HERE/name) for name in ['locks.json','guest_action.py','common.py']}
 verify="import pathlib,hashlib; expected="+repr(expected)+"; root=pathlib.Path("+repr(target)+"); assert all(hashlib.sha256((root/n).read_bytes()).hexdigest()==h for n,h in expected.items())"
 code,out,err=guest(work,'/usr/bin/python3',['-c',verify],30);assert code==0,'guest staged recipe hash mismatch'
 code,out,err=guest(work,'/usr/bin/python3',[target+'/guest_action.py',target+'/locks.json',mode],900)
 (work/'evidence'/f'{mode}.stdout').write_bytes(out);(work/'evidence'/f'{mode}.stderr').write_bytes(err)
 if code:raise RuntimeError('guest '+mode+' failed: inspect raw evidence')
 # Capture full JSON even if python-apt also writes progress lines to stdout.
 result=json.loads(next(line for line in reversed(out.decode().splitlines()) if line.startswith('{')))
 (work/'evidence'/f'{mode}.json').write_text(json.dumps(result,indent=2)+'\n');return result

def main():
 if shutil.disk_usage(W).free<30_000_000_000:raise RuntimeError('construction disk floor')
 initial={n:sha(R/n) for n in pins()};assert initial==pins()
 N.mkdir(exist_ok=False);(N/'recipes').mkdir();
 for p in HERE.iterdir():
  if p.is_file():shutil.copyfile(p,N/'recipes'/p.name)
 (E/'qemu-version.txt').write_bytes(subprocess.check_output(['qemu-system-x86_64','--version']))
 for name in ['OVMF_CODE_4M.fd','cosmic-vars.fd']:shutil.copyfile(R/name,N/name)
 mut=W/'mutation';mut.mkdir();shutil.copyfile(R/'cosmic-vars.fd',mut/'vars.fd')
 subprocess.run(['qemu-img','create','-f','qcow2','-F','qcow2','-b',str(R/'cosmic-1.11.qcow2'),str(mut/'overlay.qcow2')],check=True)
 p,log=boot(mut,mut/'overlay.qcow2',mut/'vars.fd')
 try:result=action(mut,'purge')
 finally:stop(p,log,mut)
 assert result['after']['count']==1783 and result['after']['broken_dependencies']==0
 disk=N/'cosmic-1.11-1783-r1.qcow2'
 subprocess.run(['qemu-img','convert','-O','qcow2','-c','-o','compression_type=zstd',str(mut/'overlay.qcow2'),str(disk)],check=True,timeout=1200)
 shutil.copyfile(mut/'vars.fd',N/'cosmic-vars.fd');disk.chmod(0o444)
 info=json.loads(subprocess.check_output(['qemu-img','info','--output=json',str(disk)]));assert not info.get('backing-filename')
 disk_hash=sha(disk);vars_hash=sha(N/'cosmic-vars.fd')
 receipt={'format':'halcyon-1783-guest-construction','version':1,'construction':'complete','qualified':False,'publication':'ON HOLD','upcoming_release':'0.2.5','original_artifact_id':12506756949,'baseline_hashes':initial,'removed':L['removed'],'new_count':1783,'kernel':L['kernel'],'new_disk_sha256':disk_hash,'new_vars_sha256':vars_hash,'disk_info':info,'reboot_check':'pending','stock_exercise':'pending','recipe_hashes':{p.name:sha(p) for p in (N/'recipes').iterdir()}}
 def save():
  (E/'construction-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(N/'construction-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 save();print('New standalone 1783-package disk constructed; starting isolated reboot check',flush=True)
 trial=W/'reboot';trial.mkdir();shutil.copyfile(N/'cosmic-vars.fd',trial/'vars.fd')
 subprocess.run(['qemu-img','create','-f','qcow2','-F','qcow2','-b',str(disk),str(trial/'overlay.qcow2')],check=True)
 p,log=boot(trial,trial/'overlay.qcow2',trial/'vars.fd')
 try:
  action(trial,'verify');receipt['reboot_check']='exact1783-kernel7.2.6-dpkg-audit-empty-dependency-gaps-zero';save()
  # This verbatim proven harness drives real QMP keys/pointer and captures pixels.
  with (trial/'evidence/stock-exercise.log').open('wb') as f:
   rc=subprocess.run(['/usr/bin/python3',str(HERE/'stock-exercise.py'),str(trial)],stdout=f,stderr=subprocess.STDOUT,timeout=600,env={'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','HOME':'/root','LANG':'C.UTF-8'}).returncode
  receipt['stock_exercise']='automated-observations-complete-visual-review-required' if rc==0 else 'failed-preserved-no-qualification';receipt['stock_exercise_returncode']=rc;save()
  if rc:raise RuntimeError('stock exercise failed; preserve constructed disk and stop')
 finally:stop(p,log,trial)
 assert sha(disk)==disk_hash and sha(N/'cosmic-vars.fd')==vars_hash
 final={n:sha(R/n) for n in pins()};assert final==initial
 receipt['original_all_inputs_unchanged']=True;receipt['new_image_unchanged_after_overlay_reboot']=True;receipt['free_bytes_final']=shutil.disk_usage(W).free;save()
 for folder in [mut,trial]:shutil.copytree(folder/'evidence',E/folder.name,dirs_exist_ok=True)
 (N/'SHA256SUMS').write_text(''.join(f'{sha(p)}  {p.relative_to(N)}\n' for p in sorted(N.rglob('*')) if p.is_file() and p.name!='SHA256SUMS'))
 print(json.dumps(receipt),flush=True)
if __name__=='__main__':
 try:main()
 except Exception as e:
  (E/'construction-failure.json').write_text(json.dumps({'error':str(e),'qualified':False,'construction_receipt_preserved':(E/'construction-receipt.json').exists()},indent=2)+'\n');traceback.print_exc();raise SystemExit(1)
 finally:
  for name in ['mutation','reboot']:
   src=W/name/'evidence'
   if src.exists():shutil.copytree(src,E/name,dirs_exist_ok=True)
  if N.exists():
   (N/'SHA256SUMS').write_text(''.join(f'{sha(p)}  {p.relative_to(N)}\n' for p in sorted(N.rglob('*')) if p.is_file() and p.name!='SHA256SUMS'))
