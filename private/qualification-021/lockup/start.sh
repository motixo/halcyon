#!/bin/bash
set -Eeuo pipefail
SRC=$(realpath private/qualification-021)
cd "$WORK"
mkdir -p evidence progress
git -C "$SRC" rev-parse HEAD > evidence/infra-commit.txt
cp "$SRC/lockup/source-provenance.json" evidence/
cp "$SRC/lockup/preflight-manifest.sha256" evidence/
cp -a "$SRC/lockup" evidence/executed-harness
cp base-vars.fd lockup-vars.fd
qemu-img create -f qcow2 -F qcow2 -b "$WORK/base.qcow2" lockup.qcow2
media=()
if [[ $COHORT != stock ]]; then
 media=(-drive "if=none,id=media,format=raw,readonly=on,file=$WORK/candidate.iso" -device virtio-scsi-pci,id=scsi,addr=0x6 -device scsi-cd,drive=media,bus=scsi.0)
fi
qemu-system-x86_64 -machine pc-q35-8.2 -accel kvm -cpu host -smp 4 -m 6144 \
 -drive if=pflash,format=raw,readonly=on,file=retain/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=lockup-vars.fd \
 -drive if=none,id=osdisk,format=qcow2,file=lockup.qcow2 -device virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0 \
 -device virtio-vga,addr=0x2,xres=1280,yres=800 -device qemu-xhci,addr=0x3 -device usb-kbd -device usb-tablet \
 -device virtio-serial,addr=0x4 -chardev "socket,path=$WORK/agent.sock,server=on,wait=off,id=agent" -device virtserialport,chardev=agent,name=org.qemu.guest_agent.0 \
 "${media[@]}" -nic none -display none -vnc unix:display.sock -qmp unix:qmp.sock,server=on,wait=off \
 -serial file:evidence/serial.log -pidfile lockup-qemu.pid -daemonize -D evidence/qemu.log
tr '\0' ' ' < "/proc/$(cat lockup-qemu.pid)/cmdline" > evidence/vm-command.txt
qemu-system-x86_64 --version > evidence/qemu-version.txt
python3 "$SRC/lockup/session.py" "$WORK" public "$COHORT" init
