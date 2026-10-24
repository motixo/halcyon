#!/bin/bash
set -Eeuo pipefail
: "${WORK:?}"
# Download authenticated Ubuntu tools to a task-local prefix, NEVER install to host.
mkdir -p "$WORK/apt/lists/partial" "$WORK/apt/cache/archives/partial" "$WORK/tools"
cat > "$WORK/apt/sources.list" <<'EOF'
deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] https://archive.ubuntu.com/ubuntu noble main universe
deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] https://archive.ubuntu.com/ubuntu noble-updates main universe
deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] https://archive.ubuntu.com/ubuntu noble-security main universe
EOF
cat > "$WORK/apt/apt.conf" <<'EOF'
Dir::Etc::parts "-";
Dir::Etc::main "-";
EOF
export APT_CONFIG="$WORK/apt/apt.conf"
opts=(-o Dir::Etc::sourcelist="$WORK/apt/sources.list" -o Dir::Etc::sourceparts=- -o Dir::State::lists="$WORK/apt/lists" -o Dir::Cache="$WORK/apt/cache" -o Debug::NoLocking=1 -o APT::Get::List-Cleanup=0 -o Acquire::Languages=none -o Acquire::Retries=0 -o Acquire::http::Timeout=15 -o Acquire::https::Timeout=15)
sha256sum /var/lib/dpkg/status > "$WORK/evidence/host-dpkg-before.sha256"
timeout --kill-after=10s 180 apt-get "${opts[@]}" update -q
timeout --kill-after=10s 240 apt-get "${opts[@]}" --download-only --no-install-recommends -y --reinstall install qemu-system-x86 qemu-system-common qemu-system-data qemu-utils python3-pil seabios ipxe-qemu
find "$WORK/apt/cache/archives" -maxdepth 1 -name '*.deb' -print0 | sort -z | xargs -0 sha256sum > "$WORK/evidence/runner-tool-archives.sha256"
for deb in "$WORK"/apt/cache/archives/*.deb; do dpkg-deb -x "$deb" "$WORK/tools"; done
sha256sum -c "$WORK/evidence/host-dpkg-before.sha256"
export LD_LIBRARY_PATH="$WORK/tools/usr/lib/x86_64-linux-gnu"
export QEMU_MODULE_DIR="$WORK/tools/usr/lib/x86_64-linux-gnu/qemu"
export PYTHONPATH="$WORK/tools/usr/lib/python3/dist-packages"
"$WORK/tools/usr/bin/qemu-system-x86_64" --version > "$WORK/evidence/qemu-version.txt"
"$WORK/tools/usr/bin/qemu-img" --version > "$WORK/evidence/qemu-img-version.txt"
python3 -c 'from PIL import Image; print("Task-local QEMU and Pillow load")'
du -sb "$WORK/apt" "$WORK/tools" > "$WORK/evidence/runner-tool-bytes.txt"

# --version cannot establish device-module or ROM availability. Fail before image IO.
sudo env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin WORK="$WORK" python3 private/package-session-025/preflight.py
