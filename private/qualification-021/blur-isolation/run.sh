#!/bin/bash
set -Eeuo pipefail
SRC=$(realpath private/qualification-021)
action=${1:?};context=${2:?}
[[ $context = stock-native-blur || $context = public-frosted || $context = public-native-off ]]
active_case='';keep_vm=0
stop_owned() { if [[ -n "$active_case" ]]; then python3 "$SRC/lockup/stop.py" "$active_case"; fi; }
trap 'if [[ $keep_vm = 0 ]]; then stop_owned; fi' EXIT
aggregate() {
 local name=$1
 mkdir -p "$WORK/evidence/native-blur-$name"
 cp -a "$active_case/evidence/." "$WORK/evidence/native-blur-$name/"
}
start_case() {
 local variant=$1
 sha256sum -c "$SRC/blur-isolation/preflight.sha256"
 local name="$context-$variant"
 active_case="$WORK/blur-cases/$name"
 test ! -e "$active_case"
 mkdir -p "$active_case/evidence/package-build" "$active_case/evidence/executed-source"
 cp -a "$WORK/evidence/package-build/." "$active_case/evidence/package-build/"
 cp "$WORK/base-vars.fd" "$active_case/vars.fd"
 qemu-img create -f qcow2 -F qcow2 -b "$WORK/base.qcow2" "$active_case/disk.qcow2"
 python3 "$SRC/blur-isolation/provenance.py" "$active_case" "$SRC"
 if [[ -e "$WORK/live-repair-used.json" ]]; then cp "$WORK/live-repair-used.json" "$active_case/evidence/"; fi
 local media=()
 if [[ $context = public-* ]]; then media=(-drive "if=none,id=media,format=raw,readonly=on,file=$WORK/candidate.iso" -device virtio-scsi-pci,id=scsi,addr=0x6 -device scsi-cd,drive=media,bus=scsi.0); fi
 qemu-system-x86_64 -machine pc-q35-8.2 -accel kvm -cpu host -smp 4 -m 6144 \
 -drive "if=pflash,format=raw,readonly=on,file=$WORK/retain/OVMF_CODE_4M.fd" -drive "if=pflash,format=raw,file=$active_case/vars.fd" \
 -drive "if=none,id=osdisk,format=qcow2,file=$active_case/disk.qcow2" -device virtio-blk-pci,drive=osdisk,addr=0x5,bootindex=0 \
 -device virtio-vga,addr=0x2,xres=1280,yres=800 -device qemu-xhci,addr=0x3 -device usb-kbd -device usb-tablet \
 -device virtio-serial,addr=0x4 -chardev "socket,path=$active_case/agent.sock,server=on,wait=off,id=agent" -device virtserialport,chardev=agent,name=org.qemu.guest_agent.0 \
 "${media[@]}" -nic none -display none -vnc "unix:$active_case/display.sock" -qmp "unix:$active_case/qmp.sock,server=on,wait=off" \
 -serial "file:$active_case/evidence/serial.log" -pidfile "$active_case/lockup-qemu.pid" -daemonize -D "$active_case/evidence/qemu.log"
 tr '\0' ' ' < "/proc/$(cat "$active_case/lockup-qemu.pid")/cmdline" > "$active_case/evidence/vm-command.txt"
 local rc=0
 python3 "$SRC/blur-isolation/session.py" "$active_case" public stock "$context" "$variant" || rc=$?
 if [[ $rc != 0 && ! -e "$WORK/live-repair-used.json" ]] && python3 - "$active_case" <<'PY'
import pathlib,json,sys
s=json.loads((pathlib.Path(sys.argv[1])/'evidence/isolation-results.json').read_text());raise SystemExit(0 if s.get('repair_eligible') else 1)
PY
 then
  python3 - "$WORK" "$active_case" "$context" "$variant" <<'PY'
import json,pathlib,sys
root,case,context,variant=sys.argv[1:];(pathlib.Path(root)/'active-blur-case.json').write_text(json.dumps({'case':case,'context':context,'variant':variant}))
PY
  aggregate "$name-initial-failure";keep_vm=1;return 3
 fi
 stop_owned;aggregate "$name";active_case=''
 return "$rc"
}
needs_preceding() {
 python3 - "$WORK" "$context" <<'PY'
import pathlib,json,sys
r=pathlib.Path(sys.argv[1])/'evidence'/('native-blur-'+sys.argv[2]+'-immediate')/'isolation-results.json'
s=json.loads(r.read_text());raise SystemExit(0 if s.get('complete') and not s.get('native_failure_observed') and not s.get('setup_error') else 1)
PY
}
mkdir -p "$WORK/blur-cases" "$WORK/evidence/isolation-infrastructure"
if [[ $action = context ]]; then
 rc=0;start_case immediate || rc=$?
 if [[ $rc = 0 ]] && needs_preceding; then start_case preceding; else exit "$rc"; fi
elif [[ $action = finish ]]; then
 if ! python3 - "$WORK/active-blur-case.json" "$context" <<'PY_CHECK'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1]);s=json.loads(p.read_text()) if p.exists() else {};raise SystemExit(0 if s.get('context')==sys.argv[2] and s.get('status')!='finalized' else 1)
PY_CHECK
 then exit 0;fi
 readarray -t info < <(python3 - "$WORK/active-blur-case.json" <<'PY'
import json,sys
s=json.load(open(sys.argv[1]));print(s['case']);print(s['context']);print(s['variant'])
PY
 )
 [[ ${info[1]} = "$context" ]];active_case=${info[0]};variant=${info[2]}
 stop_owned;aggregate "$context-$variant";active_case=''
 python3 - "$WORK/active-blur-case.json" <<'PY_DONE'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1]);s=json.loads(p.read_text());s['status']='finalized';p.write_text(json.dumps(s))
PY_DONE
 if [[ $variant = immediate ]] && needs_preceding; then start_case preceding; fi
else exit 2;fi
