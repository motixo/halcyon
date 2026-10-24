#!/bin/bash
set -Eeuo pipefail
SRC=$(realpath private/qualification-021/theme)
if [[ ${SCENARIO_SET:-original} == acceptance ]]; then
  exec python3 "$SRC/acceptance_wrapper.py"
fi
if [[ ${SCENARIO_SET:-original} == auto-diagnosis ]]; then
  exec bash "$SRC/auto-session.sh"
fi
cd "$WORK"
cleanup() {
  if [[ -f qemu.pid ]]; then
    VM_PID=$(cat qemu.pid)
    kill "$VM_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT
cp base-vars.fd trial-vars.fd
qemu-img create -f qcow2 -F qcow2 -b "$WORK/base.qcow2" trial.qcow2
qemu-system-x86_64 --version > evidence/qemu-version.txt
qemu-system-x86_64 -machine pc-q35-8.2 -accel kvm -cpu host -smp 4 -m 6144 \
 -drive if=pflash,format=raw,readonly=on,file=retain/OVMF_CODE_4M.fd \
 -drive if=pflash,format=raw,file=trial-vars.fd \
 -drive if=none,id=osdisk,format=qcow2,file=trial.qcow2 \
 -device virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0 \
 -device virtio-vga,addr=0x2,xres=1280,yres=800 -device qemu-xhci,addr=0x3 -device usb-kbd -device usb-tablet \
 -device virtio-serial,addr=0x4 -chardev socket,path="$WORK/agent.sock",server=on,wait=off,id=agent \
 -device virtserialport,chardev=agent,name=org.qemu.guest_agent.0 \
 -drive if=none,id=media,format=raw,readonly=on,file=candidate.iso \
 -device virtio-scsi-pci,id=scsi,addr=0x6 -device scsi-cd,drive=media,bus=scsi.0 \
 -nic none -display none -vnc unix:display.sock \
 -qmp unix:qmp.sock,server=on,wait=off -serial file:evidence/serial.log \
 -pidfile qemu.pid -daemonize -D evidence/qemu.log
tr '\0' ' ' < "/proc/$(cat qemu.pid)/cmdline" > evidence/vm-command.txt
SCRIPT=theme_session.py
if [[ ${SCENARIO_SET:-original} == followup ]]; then
  SCRIPT=followup_session.py
  cp "$SRC/followup/control-calibration.json" evidence/control-calibration.json
  cp "$SRC/followup/calibration-local-verification.txt" evidence/
  cp "$SRC/calibrated_controls.py" "$SRC/followup_session.py" evidence/
fi
python3 "$SRC/$SCRIPT" "$WORK" "$LINEAGE" "$HOST_KIND" 2>&1 | tee evidence/theme-exercise.log
