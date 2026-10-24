#!/bin/bash
set -Eeuo pipefail
SRC=$(realpath private/qualification-021)
sha256sum -c "$SRC/blur-isolation/preflight.sha256"
test -r "$WORK/base.qcow2"
test -r "$WORK/candidate.iso"
mkdir -p "$WORK/blur-cases" "$WORK/evidence/isolation-infrastructure"
git -C "$SRC" rev-parse HEAD > "$WORK/evidence/isolation-infrastructure/infra-commit.txt"
cp "$SRC/blur-isolation/source-provenance.json" "$WORK/evidence/isolation-infrastructure/"
cp "$SRC/blur-isolation/validation.txt" "$WORK/evidence/isolation-infrastructure/"
active_case=''
stop_owned() { if [[ -n "$active_case" ]]; then python3 "$SRC/lockup/stop.py" "$active_case"; active_case=''; fi; }
trap stop_owned EXIT
for context in stock-baseline stock-native-blur public-frosted public-native-off; do
 case_dir="$WORK/blur-cases/$context"
 test ! -e "$case_dir"
 mkdir -p "$case_dir/evidence/package-build" "$case_dir/evidence/executed-source"
 cp -a "$WORK/evidence/package-build/." "$case_dir/evidence/package-build/"
 cp "$WORK/base-vars.fd" "$case_dir/vars.fd"
 qemu-img create -f qcow2 -F qcow2 -b "$WORK/base.qcow2" "$case_dir/disk.qcow2"
 python3 "$SRC/blur-isolation/provenance.py" "$case_dir" "$SRC"
 media=()
 if [[ $context = public-* ]]; then
  media=(-drive "if=none,id=media,format=raw,readonly=on,file=$WORK/candidate.iso" -device virtio-scsi-pci,id=scsi,addr=0x6 -device scsi-cd,drive=media,bus=scsi.0)
 fi
 active_case=$case_dir
 qemu-system-x86_64 -machine pc-q35-8.2 -accel kvm -cpu host -smp 4 -m 6144 \
  -drive "if=pflash,format=raw,readonly=on,file=$WORK/retain/OVMF_CODE_4M.fd" -drive "if=pflash,format=raw,file=$case_dir/vars.fd" \
  -drive "if=none,id=osdisk,format=qcow2,file=$case_dir/disk.qcow2" -device virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0 \
  -device virtio-vga,addr=0x2,xres=1280,yres=800 -device qemu-xhci,addr=0x3 -device usb-kbd -device usb-tablet \
  -device virtio-serial,addr=0x4 -chardev "socket,path=$case_dir/agent.sock,server=on,wait=off,id=agent" -device virtserialport,chardev=agent,name=org.qemu.guest_agent.0 \
  "${media[@]}" -nic none -display none -vnc "unix:$case_dir/display.sock" -qmp "unix:$case_dir/qmp.sock,server=on,wait=off" \
  -serial "file:$case_dir/evidence/serial.log" -pidfile "$case_dir/lockup-qemu.pid" -daemonize -D "$case_dir/evidence/qemu.log"
 tr '\0' ' ' < "/proc/$(cat "$case_dir/lockup-qemu.pid")/cmdline" > "$case_dir/evidence/vm-command.txt"
 rc=0
 python3 "$SRC/blur-isolation/session.py" "$case_dir" public stock "$context" || rc=$?
 stop_owned
 cp -a "$case_dir/evidence" "$WORK/evidence/native-blur-$context"
 # A native crash is a retained case result. Setup failure cannot qualify comparisons.
 if [[ $rc != 0 ]]; then printf 'Stop after setup/input infrastructure failure in %s (exit %s)\n' "$context" "$rc"; exit "$rc"; fi
 done
sha256sum "$WORK/base.qcow2" "$WORK/base-vars.fd" > "$WORK/evidence/isolation-infrastructure/final-base-hashes.txt"
python3 "$SRC/blur-isolation/summary.py" "$WORK/evidence"
