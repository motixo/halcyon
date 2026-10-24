"""Runner-only tool preflight BEFORE image download. No OS or guest code runs."""
import hashlib,json,os,pathlib,socket,subprocess,time
W=pathlib.Path(os.environ['WORK']);P=W/'tools';E=W/'evidence';F=W/'device-preflight';F.mkdir(exist_ok=False)
env={'PATH':'/usr/bin:/bin','LANG':'C.UTF-8','LD_LIBRARY_PATH':str(P/'usr/lib/x86_64-linux-gnu'),'QEMU_MODULE_DIR':str(P/'usr/lib/x86_64-linux-gnu/qemu')}
q=str(P/'usr/bin/qemu-system-x86_64');files={}
for name in ['usr/lib/x86_64-linux-gnu/qemu/hw-display-virtio-vga.so','usr/lib/x86_64-linux-gnu/qemu/hw-display-virtio-gpu.so','usr/share/seabios/vgabios-virtio.bin','usr/share/seabios/bios-256k.bin']:
 path=P/name;assert path.is_file(),'missing extracted tool: '+name
 with path.open('rb') as stream:files[name]=hashlib.file_digest(stream,'sha256').hexdigest()
help=subprocess.run([q,'-device','virtio-vga,help'],capture_output=True,text=True,env=env,timeout=15)
(E/'device-preflight-help.txt').write_text(help.stdout+help.stderr)
assert help.returncode==0 and help.stdout.startswith('virtio-vga options:') and not help.stderr,'device help did not load local modules'
args=[q,'-L',str(P/'usr/share/qemu'),'-L',str(P/'usr/share/seabios'),'-L',str(P/'usr/lib/ipxe/qemu'),'-machine','pc-q35-8.2','-accel','kvm','-cpu','host','-m','128','-smp','1','-S','-nodefaults','-nic','none','-display','none','-device','virtio-vga,addr=0x2,xres=1280,yres=800','-device','qemu-xhci,addr=0x3','-device','usb-kbd','-device','usb-tablet','-device','virtio-serial,addr=0x4','-blockdev','driver=null-co,node-name=empty,size=1048576','-device','virtio-blk-pci,drive=empty,addr=0x5','-qmp',f'unix:{F}/qmp.sock,server=on,wait=off']
(E/'device-preflight-args.json').write_text(json.dumps(args,indent=2)+'\n')
with (E/'device-preflight-qemu.log').open('wb') as log:
 p=subprocess.Popen(args,env=env,stdout=log,stderr=log)
 try:
  for _ in range(100):
   if p.poll() is not None:raise RuntimeError('preflight QEMU exited; inspect device-preflight-qemu.log')
   if (F/'qmp.sock').exists():break
   time.sleep(.1)
  else:raise RuntimeError('preflight QMP deadline')
  s=socket.socket(socket.AF_UNIX);s.settimeout(5);s.connect(str(F/'qmp.sock'));f=s.makefile('rwb',buffering=0);greeting=json.loads(f.readline())
  def call(name):
   f.write((json.dumps({'execute':name})+'\n').encode())
   while True:
    obj=json.loads(f.readline())
    if 'event' not in obj:return obj
  assert 'return' in call('qmp_capabilities');status=call('query-status')['return'];assert status['running'] is False
  call('quit');f.close();s.close();p.wait(timeout=5)
  (E/'device-preflight.json').write_text(json.dumps(dict(passed=True,cpu_execution=False,guest_disk=False,guest_OS=False,kvm=True,query_status=status,tool_files=files),indent=2)+'\n')
 finally:
  if p.poll() is None:p.terminate();p.wait(timeout=5)
