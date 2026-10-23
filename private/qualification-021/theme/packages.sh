#!/bin/bash
# Disposable CI builder ONLY. Owner credentials are never supplied.
set -Eeuo pipefail
OUT=$RUNNER_TEMP/theme-packages
mkdir -p "$OUT/media" "$OUT/build"
exec > >(tee "$OUT/build/build.log") 2>&1
SHA=0b688c832bd2d22d047bd63664a8cd7e7b35becb
git fetch --no-tags origin "$SHA"
git worktree add --detach "$RUNNER_TEMP/theme-source" "$SHA"
cd "$RUNNER_TEMP/theme-source"
[[ $(git rev-parse HEAD) == "$SHA" ]]
git status --porcelain > "$OUT/build/source-status.txt"
git rev-parse HEAD > "$OUT/build/source-sha.txt"
git rev-parse HEAD^{tree} > "$OUT/build/source-tree.txt"
sha256sum Cargo.lock > "$OUT/build/Cargo.lock.sha256"
sudo apt-get update -q
sudo apt-get install --no-install-recommends -y build-essential pkg-config pkgconf cmake just libxkbcommon-dev libwayland-dev libexpat1-dev libfontconfig-dev libfreetype-dev libinput-dev libudev-dev libdbus-1-dev libseat-dev libegl1-mesa-dev libvulkan-dev libssl-dev libclang-dev dpkg-dev
rustup toolchain install 1.93.0 --profile minimal
rustc +1.93.0 --version --verbose > "$OUT/build/rust.txt"
cargo +1.93.0 build --release --locked --bin halcyon
strip --strip-unneeded target/release/halcyon
sha256sum target/release/halcyon > "$OUT/build/stripped-binary.sha256"
python3 packaging/build-deb.py --binary target/release/halcyon --version '0.2.1~dev.0b688c8' --out "$OUT/media"
gh api repos/profilaz3/halcyon/releases/tags/v0.2.0 > "$OUT/build/public-release.json"
gh release download v0.2.0 --repo profilaz3/halcyon --pattern '*.deb' --pattern '*.deb.sha256' --dir "$OUT/media"
python3 - "$OUT" <<'PY'
import hashlib,json,pathlib,sys,subprocess
out=pathlib.Path(sys.argv[1]); release=json.loads((out/'build/public-release.json').read_text())
for a in release['assets']:
 p=out/'media'/a['name']
 if p.exists():
  assert 'sha256:'+hashlib.sha256(p.read_bytes()).hexdigest()==a['digest'], a['name']
for p in sorted((out/'media').glob('*.deb')):
 dest=out/'build'/p.stem;dest.mkdir()
 subprocess.run(['dpkg-deb','-x',str(p),str(dest)],check=True)
 subprocess.run(['dpkg-deb','--info',str(p)],check=True)
 files={str(f.relative_to(dest)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(dest.rglob('*')) if f.is_file()}
 (out/'build'/(p.stem+'-payload.json')).write_text(json.dumps(files,indent=2))
 import shutil;shutil.rmtree(dest)
PY
cd "$OUT/media"
sha256sum -c cosmic-ext-halcyon_0.2.0_amd64.deb.sha256
sha256sum -c cosmic-ext-halcyon-islands_0.2.0_amd64.deb.sha256
sha256sum -- *.deb > SHA256SUMS
cp SHA256SUMS "$OUT/build/approved-inputs.sha256"
