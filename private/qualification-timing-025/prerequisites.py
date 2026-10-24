import fcntl,json,os,pathlib,shutil,subprocess
W=pathlib.Path(os.environ['WORK']);E=W/'evidence';E.mkdir(parents=True,exist_ok=True)
mem=int(next(l.split()[1] for l in pathlib.Path('/proc/meminfo').read_text().splitlines() if l.startswith('MemTotal:')))*1024
r={'free_bytes':shutil.disk_usage(W).free,'memory_total_bytes':mem,'cpus':os.cpu_count(),'kvm_create_vm':False,'runner_environment':os.environ.get('RUNNER_ENVIRONMENT','isolated-env'),'filesystem':subprocess.check_output(['df','-B1',str(W)],text=True)}
try:
 fd=os.open('/dev/kvm',os.O_RDWR);assert fcntl.ioctl(fd,0xAE00,0)==12;vm=fcntl.ioctl(fd,0xAE01,0);os.close(vm);os.close(fd);r['kvm_create_vm']=True
finally:(E/'runner-prerequisites.json').write_text(json.dumps(r,indent=2)+'\n')
assert r['free_bytes']>=70_000_000_000,'less than 70GB free; do not download'
assert mem>=14*1024**3,'less than 14GiB physical memory'
assert r['cpus']>=4,'fewer than four vCPU'
print(json.dumps(r))
