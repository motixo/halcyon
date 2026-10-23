#!/bin/bash
set -Eeuo pipefail
export DEBIAN_FRONTEND=noninteractive
Q=/var/lib/qualification
cp -a /etc/apt "$Q/original-apt"
dpkg-query -W -f='${binary:Package}\t${Architecture}\t${Version}\t${source:Package}\t${source:Version}\n' > "$Q/before.tsv"
cat > /etc/apt/apt.conf.d/99qualification <<'EOF'
APT::Keep-Downloaded-Packages "true";
Binary::apt::APT::Keep-Downloaded-Packages "true";
Acquire::Languages "none";
EOF
# Enable source indexes for the same official repositories; do not add PPAs.
python3 - <<'PY'
import pathlib,re
for f in pathlib.Path('/etc/apt/sources.list.d').glob('*.sources'):
 s=f.read_text();s=re.sub(r'^Types: deb$', 'Types: deb deb-src',s,flags=re.M)
 f.write_text(s)
PY
apt-get update
apt-get -y -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold dist-upgrade
apt-get install -y cosmic-session cosmic-store packagekit qemu-guest-agent libnotify-bin python3-apt systemd-boot-efi
# The ISO's live-boot stack/installer are not part of an installed system.
for pkg in casper pop-installer; do
 if dpkg-query -W -f='${db:Status-Status}' "$pkg" 2>/dev/null | grep -qx installed; then apt-get purge -y "$pkg"; fi
done
printf 'LABEL=cosmic-root / ext4 defaults 0 1\nLABEL=COSMIC_EFI /boot/efi vfat umask=0077 0 2\n' > /etc/fstab
echo cosmic-probe > /etc/hostname
echo '127.0.0.1 localhost\n127.0.1.1 cosmic-probe' > /etc/hosts
useradd -m -s /bin/bash -G adm,sudo probe
echo 'probe:cosmic-probe' | chpasswd
systemctl enable cosmic-greeter.service qemu-guest-agent.service
systemctl set-default graphical.target
rm -f /etc/machine-id /var/lib/dbus/machine-id
touch /etc/machine-id
ln -s /etc/machine-id /var/lib/dbus/machine-id
update-initramfs -u -k all
dpkg --audit > "$Q/dpkg-audit.txt"
test ! -s "$Q/dpkg-audit.txt"
apt-get check
dpkg-query -W -f='${binary:Package}\t${Architecture}\t${Version}\t${source:Package}\t${source:Version}\n' > "$Q/installed.tsv"
python3 "$Q/package-map.py" > "$Q/package-map.json"
dpkg-query -W -f='${binary:Package}=${Version}\n' | sed 's/^/Package: /' > "$Q/installed-versions.txt"
# Freeze discovery after installation. The exact filesystem, ISO and signed apt
# lists plus fetched delta packages are retained outside the guest.
apt-mark hold $(dpkg-query -W -f='${binary:Package} ') > "$Q/holds.txt"
systemctl disable apt-daily.timer apt-daily-upgrade.timer || true
