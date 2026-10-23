#!/bin/bash
set -Eeuo pipefail
mkdir -p "$RUNNER_TEMP/cosmic/evidence" "$RUNNER_TEMP/theme-packages"
sudo sed -i 's|http://azure.archive.ubuntu.com/ubuntu|https://archive.ubuntu.com/ubuntu|g' /etc/apt/sources.list.d/*.sources 2>/dev/null || true
if [ -f /etc/apt/apt-mirrors.txt ]; then
  printf 'https://archive.ubuntu.com/ubuntu\nhttps://apt.pop-os.org/ubuntu\n' | sudo tee /etc/apt/apt-mirrors.txt
fi
sudo apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 update -q
sudo apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 install --no-install-recommends -y qemu-system-x86 qemu-utils ovmf python3-pil aria2 xorriso tesseract-ocr
# Anchor the API artifact digests to the independently reviewed handoff.
python3 - <<'PY'
import json,os,subprocess
r=json.loads(subprocess.check_output(['gh','api','repos/'+os.environ['GH_REPO']+'/actions/runs/39963889546/artifacts']))
expected={12507125311:'sha256:47c57f02d9d2807cdabadd31b387e0e336a21441cd6acb1b99043fe043660b06',12506756949:'sha256:e87c617f179da525d7d06e3e74b862fa87dcefabdbcbcb02f2e7258802a6ccd4'}
for k,v in expected.items():
 a=next(a for a in r['artifacts'] if a['id']==k);assert not a['expired'] and a['digest']==v
PY
REUSE_RUN=39963889546 bash private/qualification-021/restore.sh
python3 private/qualification-021/artifact-fetch.py "$GH_REPO" "$PACKAGE_RUN" "theme-packages-$PACKAGE_RUN" "$RUNNER_TEMP/theme-packages"
python3 - <<'PIN'
import json,os,subprocess
r=json.loads(subprocess.check_output(['gh','api','repos/'+os.environ['GH_REPO']+'/actions/runs/'+os.environ['PACKAGE_RUN']+'/artifacts']))
a=next(a for a in r['artifacts'] if a['id']==12507891179)
assert not a['expired'] and a['digest']=='sha256:2cdea1fd0ba584bc1c0891033764042b07692b370392fc4f4896d4fef7e11f65'
PIN
cd "$RUNNER_TEMP/cosmic"
gpgv --keyring "$PWD/retain/pop.gpg" retain/SHA256SUMS.gpg retain/SHA256SUMS 2>&1 | tee evidence/theme-iso-signature.txt
(cd retain && sha256sum -c SHA256SUMS) | tee evidence/theme-iso-checksum.txt
[[ $(sha256sum base.qcow2 | cut -d' ' -f1) == d3ea6c9f93c68ca0013722c4d22d77408f590ba54bf4b929a25faa50eaeb5c98 ]]
[[ $(sha256sum base-vars.fd | cut -d' ' -f1) == 25d24fad9e38e155e239bcdcd4edb058cf033e34903613f662cf44ba19a7d352 ]]
cp -a "$RUNNER_TEMP/theme-packages/build" evidence/package-build
cp "$RUNNER_TEMP/theme-packages/build/approved-inputs.sha256" evidence/approved-inputs.sha256
cd "$RUNNER_TEMP/theme-packages/media"
sha256sum -c ../build/approved-inputs.sha256
xorriso -as mkisofs -V HALCYON_INPUTS -r -o "$RUNNER_TEMP/cosmic/candidate.iso" .
sha256sum "$RUNNER_TEMP/cosmic/candidate.iso" > "$RUNNER_TEMP/cosmic/evidence/candidate-media.sha256"
