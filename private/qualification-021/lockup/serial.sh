#!/bin/bash
set -Eeuo pipefail
SRC=$(realpath private/qualification-021)
ROOT=$WORK
for COHORT in stock main islands; do
 export COHORT WORK="$ROOT/lockup-$COHORT"
 mkdir -p "$ROOT/evidence/$COHORT" "$WORK/progress"
 ln -s "$ROOT/evidence/$COHORT" "$WORK/evidence"
 ln -s "$ROOT/base.qcow2" "$WORK/base.qcow2"
 ln -s "$ROOT/base-vars.fd" "$WORK/base-vars.fd"
 ln -s "$ROOT/retain" "$WORK/retain"
 ln -s "$ROOT/candidate.iso" "$WORK/candidate.iso"
 cp -a "$ROOT/evidence/package-build" "$WORK/evidence/package-build"
 trap 'python3 "$SRC/lockup/stop.py" "$WORK"' EXIT
 bash "$SRC/lockup/start.sh"
 if [[ $COHORT = islands ]]; then target=3660; else target=360; fi
 python3 "$SRC/lockup/session.py" "$WORK" public "$COHORT" run "$target"
 python3 "$SRC/lockup/session.py" "$WORK" public "$COHORT" finish
 python3 "$SRC/lockup/stop.py" "$WORK"
 trap - EXIT
done
