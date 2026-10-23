#!/bin/bash
# Runs ONLY on a disposable GitHub runner, as root. No host home is mounted.
set -Eeuo pipefail
umask 022
: "${WORK:?dedicated disposable build directory required}"
PHASE=${1:-all}
[[ "$PHASE" =~ ^(all|inputs|install|exercise)$ ]]
SRC=$(realpath private/qualification-021)
mkdir -p "$WORK"/{evidence,retain,iso,root}
exec 9>"$WORK/lock"; flock -n 9
cd "$WORK"
R=$WORK/root
LOOP=
cleanup() {
  set +e
  if [[ -f qemu.pid ]]; then kill "$(cat qemu.pid)" 2>/dev/null; fi
  for p in run sys proc dev; do mountpoint -q "$R/$p" && umount -R "$R/$p"; done
  mountpoint -q "$R/boot/efi" && umount "$R/boot/efi"
  mountpoint -q "$R" && umount "$R"
  mountpoint -q iso && umount iso
  [[ -n "$LOOP" ]] && losetup -d "$LOOP"
  if [[ -f guest.raw && ! -f retain/cosmic-1.11.qcow2 ]]; then
    qemu-img convert -O qcow2 -c -o compression_type=zstd guest.raw retain/partial-install.qcow2
    sha256sum retain/partial-install.qcow2 > evidence/partial-install.sha256
  fi
}
trap cleanup EXIT
date -u; uname -a; df -B1 .; free -b
[[ $(df --output=avail -B1 . | tail -1) -gt 45000000000 ]]
python3 - <<'PY'
import fcntl,os
f=os.open('/dev/kvm',os.O_RDWR); assert fcntl.ioctl(f,0xAE00,0)==12
os.close(fcntl.ioctl(f,0xAE01,0)); os.close(f)
PY
if [[ "$PHASE" == all || "$PHASE" == inputs ]]; then
curl -fL --retry 3 https://api.pop-os.org/builds/24.04/generic?arch=amd64 -o evidence/build-api.json
BASE=https://iso.pop-os.org/24.04/amd64/generic/28
for name in SHA256SUMS SHA256SUMS.gpg; do curl -fL --retry 3 "$BASE/$name" -o "retain/$name"; done
curl -fL --retry 3 https://raw.githubusercontent.com/pop-os/keyring/356a75e405d00a23889998d2e8f2508a9e67b535/etc/apt/trusted.gpg.d/pop-keyring-2017-archive.gpg -o retain/pop.gpg
gpg --show-keys --with-colons retain/pop.gpg | tee evidence/pop-key.txt
gpg --show-keys --with-colons retain/pop.gpg | grep -q ':63C46DF0140D738961429F4E204DD8AEC33A7AFF:'
gpgv --keyring "$WORK/retain/pop.gpg" retain/SHA256SUMS.gpg retain/SHA256SUMS 2>&1 | tee evidence/iso-signature.txt
aria2c --max-connection-per-server=8 --split=8 --min-split-size=16M \
 --file-allocation=none --max-tries=3 --timeout=30 --summary-interval=60 \
 --dir=retain --out=pop-os_24.04_amd64_generic_28.iso "$BASE/pop-os_24.04_amd64_generic_28.iso"
(cd retain; sha256sum -c SHA256SUMS) | tee evidence/iso-checksum.txt
fi
[[ "$PHASE" != inputs ]] || exit 0
if [[ "$PHASE" == all || "$PHASE" == install ]]; then
mount -o loop,ro retain/pop-os_24.04_amd64_generic_28.iso iso
cp iso/casper/filesystem.manifest evidence/iso-filesystem.manifest
truncate -s 40G guest.raw
sgdisk --clear --new=1:2048:+512M --typecode=1:ef00 --new=2:0:0 --typecode=2:8300 guest.raw
LOOP=$(losetup --find --show --partscan guest.raw)
mkfs.vfat -F32 -n COSMIC_EFI "${LOOP}p1"
mkfs.ext4 -F -L cosmic-root "${LOOP}p2"
mount "${LOOP}p2" "$R"
unsquashfs -f -d "$R" iso/casper/filesystem.squashfs > evidence/unsquash.log 2>&1
mkdir -p "$R/boot/efi"; mount "${LOOP}p1" "$R/boot/efi"
for p in dev proc sys; do mount --rbind "/$p" "$R/$p"; mount --make-rslave "$R/$p"; done
mount -t tmpfs tmpfs "$R/run"
cp --remove-destination /etc/resolv.conf "$R/etc/resolv.conf"
# Deny service starts in chroot. No host user/session environment is forwarded.
printf '#!/bin/sh\nexit 101\n' > "$R/usr/sbin/policy-rc.d"; chmod +x "$R/usr/sbin/policy-rc.d"
mkdir -p "$R/var/lib/qualification"
cp "$SRC/guest-prepare.sh" "$R/var/lib/qualification/"
cp "$SRC/package-map.py" "$R/var/lib/qualification/"
chroot "$R" /usr/bin/env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin HOME=/root LC_ALL=C DEBIAN_FRONTEND=noninteractive /bin/bash /var/lib/qualification/guest-prepare.sh 2>&1 | tee evidence/guest-prepare.log
cp -a "$R/var/lib/qualification/." evidence/
mkdir -p retain/apt
cp -a "$R/etc/apt" retain/apt/config
cp -a "$R/var/lib/apt/lists" retain/apt/lists
cp -a "$R/usr/share/keyrings" retain/apt/keyrings
cp -a "$R/etc/apt/trusted.gpg.d" retain/apt/trusted.gpg.d
cp -a "$R/var/cache/apt/archives" retain/apt/archives
cp "$R/var/lib/dpkg/status" retain/dpkg-status
cp -a "$R/var/log/apt" evidence/apt-log
rm "$R/usr/sbin/policy-rc.d"
KERNEL=$(find "$R/boot" -maxdepth 1 -name 'vmlinuz-*' | sort -V | tail -1)
INITRD=${KERNEL/vmlinuz-/initrd.img-}
mkdir -p "$R/boot/efi/EFI/BOOT" "$R/boot/efi/loader/entries"
cp "$R/usr/lib/systemd/boot/efi/systemd-bootx64.efi" "$R/boot/efi/EFI/BOOT/BOOTX64.EFI"
cp "$KERNEL" "$R/boot/efi/vmlinuz"
cp "$INITRD" "$R/boot/efi/initrd.img"
printf 'default cosmic.conf\ntimeout 0\n' > "$R/boot/efi/loader/loader.conf"
printf 'title COSMIC disposable probe\nlinux /vmlinuz\ninitrd /initrd.img\noptions root=LABEL=cosmic-root rw console=ttyS0,115200 console=tty0\n' > "$R/boot/efi/loader/entries/cosmic.conf"
sha256sum "$KERNEL" "$INITRD" "$R/boot/efi/EFI/BOOT/BOOTX64.EFI" > evidence/boot-hashes.txt
sync
for p in run sys proc dev; do umount -R "$R/$p"; done
umount "$R/boot/efi" "$R"; losetup -d "$LOOP"; LOOP=
umount iso
qemu-img convert -O qcow2 guest.raw base.qcow2
rm guest.raw
cp /usr/share/OVMF/OVMF_VARS_4M.fd base-vars.fd
cp /usr/share/OVMF/OVMF_CODE_4M.fd retain/OVMF_CODE_4M.fd
fi
[[ "$PHASE" != install ]] || exit 0
cp base-vars.fd trial-vars.fd
qemu-img create -f qcow2 -F qcow2 -b "$WORK/base.qcow2" trial.qcow2
qemu-system-x86_64 --version > evidence/qemu-version.txt
sha256sum /usr/share/OVMF/OVMF_{CODE,VARS}_4M.fd > evidence/firmware-hashes.txt
qemu-system-x86_64 -machine q35 -accel kvm -cpu host -smp 4 -m 6144 \
 -drive if=pflash,format=raw,readonly=on,file=retain/OVMF_CODE_4M.fd \
 -drive if=pflash,format=raw,file=trial-vars.fd \
 -drive if=virtio,format=qcow2,file=trial.qcow2 \
 -device virtio-vga,xres=1280,yres=800 -device qemu-xhci -device usb-kbd -device usb-tablet \
 -device virtio-serial -chardev socket,path="$WORK/agent.sock",server=on,wait=off,id=agent \
 -device virtserialport,chardev=agent,name=org.qemu.guest_agent.0 \
 -nic user,model=virtio-net-pci -display none -vnc unix:display.sock \
 -qmp unix:qmp.sock,server=on,wait=off -serial file:evidence/serial.log \
 -pidfile qemu.pid -daemonize -D evidence/qemu.log
set +e
python3 "$SRC/exercise.py" "$WORK" 2>&1 | tee evidence/exercise.log
RESULT=${PIPESTATUS[0]}
set -e
# Retain the exercised disk (including disposable user's baseline), not a preboot claim.
kill "$(cat qemu.pid)" 2>/dev/null || true
for i in {1..30}; do kill -0 "$(cat qemu.pid)" 2>/dev/null || break; sleep 1; done
qemu-img convert -O qcow2 -c -o compression_type=zstd trial.qcow2 retain/cosmic-1.11.qcow2
cp trial-vars.fd retain/cosmic-vars.fd
cp base-vars.fd retain/preboot-vars.fd
cp evidence/build-api.json retain/
cp "$SRC"/* retain/
find retain -type f ! -name SHA256SUMS.all -print0 | sort -z | xargs -0 sha256sum > retain/SHA256SUMS.all
# Compress signed apt indexes and package/source payloads; no release publication.
tar --zstd -cf retain/apt-closure.tar.zst -C retain apt dpkg-status
rm -rf retain/apt retain/dpkg-status
sha256sum retain/* > evidence/retained-hashes.txt
exit "$RESULT"
