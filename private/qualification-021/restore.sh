#!/bin/bash
# Only read-only GITHUB_TOKEN in this controller-on-runner step; never in guest.
set -Eeuo pipefail
[[ ${REUSE_RUN:?} =~ ^[0-9]+$ ]]
: "${RUNNER_TEMP:?}"
WORK=$RUNNER_TEMP/cosmic
mkdir -p "$WORK/retain" "$WORK/import-evidence" "$WORK/evidence"
gh run download "$REUSE_RUN" -n "cosmic-evidence-$REUSE_RUN" -D "$WORK/import-evidence"
gh run download "$REUSE_RUN" -n "cosmic-retained-$REUSE_RUN" -D "$WORK/retain"
cd "$WORK"
# Check the complete immutable file digest set before the image is opened.
sha256sum -c import-evidence/retained-hashes.txt | tee evidence/import-hash-check.txt
for f in package-map.json installed.tsv before.tsv holds.txt apt-signatures.txt boot-hashes.txt iso-signature.txt iso-checksum.txt pop-key.txt build-api.json; do
  cp "import-evidence/$f" "evidence/$f"
done
printf 'Retained input origin: Actions run %s\n' "$REUSE_RUN" > evidence/import-origin.txt
sha256sum retain/cosmic-1.11.qcow2 >> evidence/import-origin.txt
mv retain/cosmic-1.11.qcow2 base.qcow2
cp retain/cosmic-vars.fd base-vars.fd
