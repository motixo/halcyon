#!/bin/bash
set -Eeuo pipefail
: "${WORK:?}"
mkdir -p "$WORK/evidence"
missing=()
for pair in qemu-system-x86_64:qemu-system-x86 qemu-img:qemu-utils; do
 command -v "${pair%%:*}" >/dev/null || missing+=("${pair#*:}")
done
python3 -c 'from PIL import Image' 2>/dev/null || missing+=(python3-pil)
if ((${#missing[@]})); then
 test -f /etc/apt/sources.list.d/ubuntu.sources
 sudo sed -i 's|http://azure.archive.ubuntu.com/ubuntu|https://archive.ubuntu.com/ubuntu|g' /etc/apt/sources.list.d/ubuntu.sources
 if [[ -f /etc/apt/apt-mirrors.txt ]]; then printf 'https://archive.ubuntu.com/ubuntu\n' | sudo tee /etc/apt/apt-mirrors.txt; fi
 opts=(-o Dir::Etc::sourcelist=/etc/apt/sources.list.d/ubuntu.sources -o Dir::Etc::sourceparts=- -o Acquire::Retries=0 -o Acquire::http::Timeout=15 -o Acquire::https::Timeout=15 -o DPkg::Lock::Timeout=20)
 sudo timeout --kill-after=10s 180 apt-get "${opts[@]}" update -q
 sudo env DEBIAN_FRONTEND=noninteractive timeout --kill-after=10s 300 apt-get "${opts[@]}" install --no-install-recommends -y "${missing[@]}"
fi
