#!/usr/bin/env python3
"""One owned offline VM at a time; construction + standalone-image stock reboot.
Infrastructure evidence only. Does not install Halcyon or grant qualification.
"""
import base64,json,os,pathlib,shutil,socket,subprocess,time,traceback
from common import HERE,sha,pins
W=pathlib.Path(os.environ['WORK']);E=W/'evidence';R=W/'prepared';N=W/'new-artifact'

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
 args=[str(W/'tools/usr/bin/qemu-system-x86_64'),'-L',str(W/'tools/usr/share/qemu'),'-machine','pc-q35-8.2','-accel','kvm','-cpu','host','-smp','4','-m','6144',
 '-drive',f'if=pflash,format=raw,readonly=on,file={R}/OVMF_CODE_4M.fd','-drive',f'if=pflash,format=raw,file={vars_file}',
 '-drive',f'if=none,id=osdisk,format=qcow2,file={disk}','-device','virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0',
 '-device','virtio-vga,addr=0x2,xres=1280,yres=800','-device','qemu-xhci,addr=0x3','-device','usb-kbd','-device','usb-tablet',
 '-device','virtio-serial,addr=0x4','-chardev',f'socket,path={work}/agent.sock,server=on,wait=off,id=agent',
 '-device','virtserialport,chardev=agent,name=org.qemu.guest_agent.0','-nic','none','-display','none',
 '-qmp',f'unix:{work}/qmp.sock,server=on,wait=off','-serial',f'file:{work}/evidence/serial.log']
 (work/'evidence/qemu-args.json').write_text(json.dumps(args,indent=2)+'\n')
 log=(work/'evidence/qemu.log').open('wb');p=subprocess.Popen(args,stdout=log,stderr=log,env={'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','HOME':'/root','LANG':'C.UTF-8','LD_LIBRARY_PATH':str(W/'tools/usr/lib/x86_64-linux-gnu')})
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
