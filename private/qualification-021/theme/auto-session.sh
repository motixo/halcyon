#!/bin/bash
# Serial disposable overlays, one owned offline QEMU at a time. Never rebuild base.
set -Eeuo pipefail
SRC=$(realpath private/qualification-021/theme)
ROOT=$WORK
git rev-parse HEAD > "$ROOT/evidence/infra-commit.txt"
sha256sum "$SRC"/{auto_session.py,auto-session.sh,calibrated_controls.py,followup_session.py,prepare.sh} > "$ROOT/evidence/harness-SHA256SUMS.txt"
owned_pid=''
cleanup() { if [[ -n $owned_pid ]]; then kill "$owned_pid" 2>/dev/null || true; fi; }
trap cleanup EXIT
cp "$SRC"/{auto_session.py,auto-session.sh,calibrated_controls.py,followup_session.py,upstream-daemon-time-655b14f.rs,upstream-daemon-location-655b14f.rs,upstream-daemon-theme-655b14f.rs} "$ROOT/evidence/"
cp "$SRC/followup/control-calibration.json" "$ROOT/evidence/"
phases=(candidate)
[[ $HOST_KIND == stock ]] && phases=(untouched candidate islands)
result=0
for phase in "${phases[@]}"; do
  run_host=$HOST_KIND
  run_lineage=$phase
  if [[ $phase == islands ]]; then
    # The pre-VM wrapper failure consumed one CI job. The single remaining
    # job serializes Islands after actual stock diagnostic evidence validates.
    if ! python3 "$SRC/validate-auto-diagnostic.py" "$ROOT/evidence"; then
      echo 'Islands gated: stock diagnostic method was not validated.' | tee "$ROOT/evidence/islands-gated.txt"
      result=1
      break
    fi
    run_host=islands
    run_lineage=candidate
  fi
  PHASE_WORK="$ROOT/auto-$phase"
  mkdir -p "$PHASE_WORK/evidence"
  cp -a "$ROOT/evidence/package-build" "$PHASE_WORK/evidence/"
  cp "$ROOT/evidence/approved-inputs.sha256" "$PHASE_WORK/evidence/"
  cd "$PHASE_WORK"
  cp "$ROOT/base-vars.fd" trial-vars.fd
  qemu-img create -f qcow2 -F qcow2 -b "$ROOT/base.qcow2" trial.qcow2
  media=()
  if [[ $phase == candidate ]]; then
    media=(-drive "if=none,id=media,format=raw,readonly=on,file=$ROOT/candidate.iso" -device virtio-scsi-pci,id=scsi,addr=0x6 -device scsi-cd,drive=media,bus=scsi.0)
  fi
  qemu-system-x86_64 --version > evidence/qemu-version.txt
  qemu-system-x86_64 -machine pc-q35-8.2 -accel kvm -cpu host -smp 4 -m 6144 \
   -drive "if=pflash,format=raw,readonly=on,file=$ROOT/retain/OVMF_CODE_4M.fd" \
   -drive if=pflash,format=raw,file=trial-vars.fd \
   -drive if=none,id=osdisk,format=qcow2,file=trial.qcow2 \
   -device virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0 \
   -device virtio-vga,addr=0x2,xres=1280,yres=800 -device qemu-xhci,addr=0x3 -device usb-kbd -device usb-tablet \
   -device virtio-serial,addr=0x4 -chardev "socket,path=$PHASE_WORK/agent.sock,server=on,wait=off,id=agent" \
   -device virtserialport,chardev=agent,name=org.qemu.guest_agent.0 \
   "${media[@]}" -nic none -display none -vnc unix:display.sock \
   -qmp unix:qmp.sock,server=on,wait=off -serial file:evidence/serial.log \
   -pidfile qemu.pid -daemonize -D evidence/qemu.log
  owned_pid=$(cat qemu.pid)
  tr '\0' ' ' < "/proc/$owned_pid/cmdline" > evidence/vm-command.txt
  python3 "$SRC/auto_session.py" "$PHASE_WORK" "$run_lineage" "$run_host" 2>&1 | tee evidence/theme-exercise.log || result=1
  kill "$owned_pid" 2>/dev/null || true
  # Confirm termination before the next overlay. No second concurrent VM.
  for n in {1..20}; do [[ ! -e /proc/$owned_pid ]] && break; sleep 1; done
  if [[ -e /proc/$owned_pid/cmdline ]] && [[ -s /proc/$owned_pid/cmdline ]]; then echo 'owned QEMU did not terminate'; exit 1; fi
  owned_pid=''
  cp -a evidence "$ROOT/evidence/$phase"
done
python3 - "$ROOT/evidence" <<'PY'
import json,pathlib,sys
root=pathlib.Path(sys.argv[1]);phases={p.parent.name:json.loads(p.read_text()) for p in root.glob('*/theme-results.json')}
(root/'theme-results.json').write_text(json.dumps({'scope':'DEVELOPMENT Auto diagnosis; serial clean overlays','phases':phases,'frames':[],'scenarios':[],'errors':[],'automatic':{'native':'see per-phase observations','injected_events':False}},indent=2)+'\n')
PY
exit "$result"
