#!/usr/bin/python3
"""Explicit serial phase plan, independently executable and locally inspectable."""
import dataclasses,hashlib,json,os,pathlib,shutil,subprocess,sys,time,traceback
SRC=pathlib.Path(__file__).resolve().parent
@dataclasses.dataclass(frozen=True)
class Phase:
 name:str
 lineage:str
 host:str
 task:str
 @property
 def media(self):return self.lineage=='candidate'
def plan(host):
 if host=='stock':return [Phase('untouched-clock','untouched','stock','clock'),Phase('untouched-natural','untouched','stock','natural'),Phase('candidate-stock-natural','candidate','stock','natural')]
 if host=='islands':return [Phase('untouched-clock','untouched','stock','clock'),Phase('untouched-natural','untouched','stock','natural'),Phase('candidate-islands-manual','candidate','islands','manual'),Phase('candidate-islands-natural','candidate','islands','natural')]
 raise ValueError('One selected standard runner host is required')
def inputs():
 return ['acceptance_wrapper.py','acceptance_session.py','auto_session.py','followup_session.py','theme_session.py','control.py','calibrated_controls.py','prepare.sh','session.sh','followup/control-calibration.json','acceptance/solar-boundary.json','acceptance/solar-provenance.json','acceptance/source/data-timezone-geodata.bitcode-v0-6','acceptance/source/Cargo.toml','acceptance/source/Cargo.lock','acceptance/source/geonames-lib.rs','acceptance/solar-calculator/Cargo.toml','acceptance/solar-calculator/Cargo.lock','acceptance/solar-calculator/src/main.rs','upstream-daemon-time-655b14f.rs','upstream-daemon-theme-655b14f.rs','upstream-daemon-location-655b14f.rs']
def qemu_args(root,work,phase):
 args=['qemu-system-x86_64','-machine','pc-q35-8.2','-accel','kvm','-cpu','host','-smp','4','-m','6144','-drive',f'if=pflash,format=raw,readonly=on,file={root}/retain/OVMF_CODE_4M.fd','-drive','if=pflash,format=raw,file=trial-vars.fd','-drive','if=none,id=osdisk,format=qcow2,file=trial.qcow2','-device','virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0','-device','virtio-vga,addr=0x2,xres=1280,yres=800','-device','qemu-xhci,addr=0x3','-device','usb-kbd','-device','usb-tablet','-device','virtio-serial,addr=0x4','-chardev',f'socket,path={work}/agent.sock,server=on,wait=off,id=agent','-device','virtserialport,chardev=agent,name=org.qemu.guest_agent.0']
 if phase.media:args+=['-drive',f'if=none,id=media,format=raw,readonly=on,file={root}/candidate.iso','-device','virtio-scsi-pci,id=scsi,addr=0x6','-device','scsi-cd,drive=media,bus=scsi.0']
 return args+['-nic','none','-display','none','-vnc',f'unix:{work}/display.sock','-qmp',f'unix:{work}/qmp.sock,server=on,wait=off','-serial',f'file:{work}/evidence/serial.log']
def media_assertion(devices,expected):
 inserted=[d for d in devices if d.get('removable') and d.get('inserted')]
 if expected is None:
  if inserted or any(d.get('device')=='media' and d.get('inserted') for d in devices):raise RuntimeError('Untouched baseline has inserted removable media')
 else:
  matches=[d for d in devices if d.get('inserted',{}).get('file',d.get('inserted',{}).get('image',{}).get('filename'))==expected]
  if len(matches)!=1 or matches[0]['inserted'].get('ro') is not True:raise RuntimeError('Candidate media identity/read-only assertion failed')
 return {'expected_candidate_media':expected,'inserted_removable_media':inserted,'asserted':True}

def preflight(tracked=True):
 paths=[SRC/name for name in inputs()]+[SRC.parent/"restore.sh",SRC.parent/"artifact-fetch.py",SRC/"fetch-small-evidence.py",SRC.parents[2]/".github/workflows/qualification-capability-probe.yml"]
 for path in paths:assert path.is_file(),str(path)
 if tracked:subprocess.run(['git','ls-files','--error-unmatch',*map(str,paths)],check=True,stdout=subprocess.DEVNULL)
 # Exercise actual launch-argument generation, including phase names differing
 # from lineage. Neither manual nor natural Islands depends on baseline result.
 checks=[]
 for host in ['stock','islands']:
  for phase in plan(host):
   args=qemu_args(pathlib.Path('/disposable/root'),pathlib.Path('/disposable/phase'),phase)
   media=[x for x in args if 'id=media,' in x]
   assert bool(media)==phase.media
   if phase.media:assert len(media)==1 and 'readonly=on' in media[0] and 'candidate.iso' in media[0]
   assert args[args.index('-nic')+1]=='none'
   assert 'virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0' in args
   checks.append({**dataclasses.asdict(phase),'media':phase.media,'args':args})
 assert [p.task for p in plan('islands') if p.lineage=='candidate']==['manual','natural']
 return {'tracked_inputs_checked':len(paths),'launch_conditions_checked':checks,'independent_islands_manual':True}
def run(root,host):
 preflight();ev=root/'evidence';ev.mkdir(exist_ok=True)
 (ev/'infra-commit.txt').write_bytes(subprocess.check_output(['git','rev-parse','HEAD']))
 manifest={}
 for name in inputs():
  data=(SRC/name).read_bytes();manifest[name]=hashlib.sha256(data).hexdigest();dest=ev/'harness'/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
 (ev/'harness-SHA256SUMS.json').write_text(json.dumps(manifest,indent=2)+'\n')
 outcomes={};result=0
 for phase in plan(host):
  print(json.dumps({'event':'phase-start','phase':dataclasses.asdict(phase),'media':phase.media,'host_time':time.time()}),flush=True)
  work=root/('acceptance-'+phase.name);pe=work/'evidence';pe.mkdir(parents=True,exist_ok=True)
  shutil.copytree(ev/'package-build',pe/'package-build',dirs_exist_ok=True);shutil.copy2(ev/'approved-inputs.sha256',pe/'approved-inputs.sha256')
  shutil.copy2(root/'base-vars.fd',work/'trial-vars.fd')
  subprocess.run(['qemu-img','create','-f','qcow2','-F','qcow2','-b',str(root/'base.qcow2'),str(work/'trial.qcow2')],check=True)
  (pe/'qemu-version.txt').write_bytes(subprocess.check_output(['qemu-system-x86_64','--version']))
  args=qemu_args(root,work,phase);(pe/'launch-spec.json').write_text(json.dumps({'phase':dataclasses.asdict(phase),'media':phase.media,'args':args},indent=2)+'\n')
  vm=None
  try:
   with (pe/'qemu.log').open('wb') as output:
    vm=subprocess.Popen(args,cwd=work,stdout=output,stderr=subprocess.STDOUT)
    time.sleep(1)
    assert vm.poll() is None,('QEMU exited',vm.returncode)
    (pe/'vm-command.txt').write_bytes(pathlib.Path(f'/proc/{vm.pid}/cmdline').read_bytes().replace(b'\0',b' '))
    with (pe/'theme-exercise.log').open('wb') as log:
     # No source/package build; only the frozen finalized input is installed.
     command=['python3',str(SRC/'acceptance_session.py'),str(work),phase.lineage,phase.host,phase.task]
     code=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,timeout=1800).returncode
    outcomes[phase.name]={'exitcode':code};result|=bool(code)
  except Exception as error:
   traceback.print_exc();outcomes[phase.name]={'wrapper_error':str(error)};result=1
  finally:
   if vm is not None and vm.poll() is None:
    vm.terminate()
    try:vm.wait(timeout=20)
    except subprocess.TimeoutExpired:vm.kill();vm.wait(timeout=10)
   assert vm is None or vm.poll() is not None,'One VM must stop before another starts'
   shutil.copytree(pe,ev/phase.name,dirs_exist_ok=True)
  print(json.dumps({'event':'phase-end','phase':phase.name,'outcome':outcomes[phase.name],'host_time':time.time()}),flush=True)
 phases={p.parent.name:json.loads(p.read_text()) for p in ev.glob('*/theme-results.json')}
 summary={'scope':'DEVELOPMENT baseline/natural/independent Islands acceptance','host':host,'phases':phases,'outcomes':outcomes,'frames':[],'scenarios':[],'errors':[],'automatic':{'native':'see independently recorded phases','injected_events':False}}
 (ev/'theme-results.json').write_text(json.dumps(summary,indent=2)+'\n');return int(bool(result))
if __name__=='__main__':
 if '--preflight' in sys.argv:print(json.dumps(preflight(),indent=2))
 else:raise SystemExit(run(pathlib.Path(os.environ['WORK']).resolve(),os.environ['HOST_KIND']))
