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
KEYS=()
for key in /etc/apt/trusted.gpg.d/*.gpg; do KEYS+=(--keyring "$key"); done
for release in /var/lib/apt/lists/*_InRelease; do
  printf '\nSIGNED METADATA: %s\n' "$release" >> "$Q/apt-signatures.txt"
  gpgv --status-fd=1 "${KEYS[@]}" "$release" >> "$Q/apt-signatures.txt" 2>&1
done
apt-get -y -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold dist-upgrade
apt-get install -y cosmic-session cosmic-store packagekit qemu-guest-agent libnotify-bin python3-apt systemd-boot-efi
# The ISO's live-boot stack/installer are not part of an installed system.
for pkg in casper pop-installer; do
 if dpkg-query -W -f='${db:Status-Status}' "$pkg" 2>/dev/null | grep -qx installed; then apt-get purge -y "$pkg"; fi
done
printf 'LABEL=cosmic-root / ext4 defaults 0 1\nLABEL=COSMIC_EFI /boot/efi vfat umask=0077 0 2\n' > /etc/fstab
echo cosmic-probe > /etc/hostname
printf '127.0.0.1 localhost\n127.0.1.1 cosmic-probe\n' > /etc/hosts
useradd -m -s /bin/bash -G adm,sudo probe
echo 'probe:cosmic-probe' | chpasswd
systemctl enable cosmic-greeter.service qemu-guest-agent.service
ln -sf /usr/lib/systemd/system/cosmic-greeter.service /etc/systemd/system/display-manager.service
# Always use the genuine greeter. A live ISO may carry initial-session autologin.
sed -i '/^\[initial_session\]/,$d' /etc/greetd/cosmic-greeter.toml
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
python3 - <<'PY'
import hashlib,json,pathlib,subprocess
rows=json.load(open('/var/lib/qualification/package-map.json'))
debs={}
for f in pathlib.Path('/var/cache/apt/archives').glob('*.deb'):
 fields=subprocess.check_output(['dpkg-deb','-f',str(f),'Package','Architecture','Version'],text=True)
 info=dict(line.split(': ',1) for line in fields.splitlines())
 debs[(info['Package'],info['Architecture'],info['Version'])]=f
for r in rows:
 if r['unchanged_from_signed_iso']:continue
 f=debs.get((r['package'].split(':')[0],r['architecture'],r['version']))
 if f is None:raise RuntimeError('Changed package payload not retained: '+r['package'])
 actual=hashlib.sha256(f.read_bytes()).hexdigest()
 if actual!=r['sha256']:raise RuntimeError('Signed package hash mismatch: '+str(f))
 r['retained_deb']=f.name
json.dump(rows,open('/var/lib/qualification/package-map.json','w'),indent=2)
PY
dpkg-query -W -f='${binary:Package}=${Version}\n' | sed 's/^/Package: /' > "$Q/installed-versions.txt"
# Freeze discovery after installation. The exact filesystem, ISO and signed apt
# lists plus fetched delta packages are retained outside the guest.
apt-mark hold $(dpkg-query -W -f='${binary:Package} ') > "$Q/holds.txt"
systemctl disable apt-daily.timer apt-daily-upgrade.timer || true
