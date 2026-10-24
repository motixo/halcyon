# Historical qualification infrastructure archive

Integrated 2026-10-25 from `ci/qualification-probe-021` at
`0f4f7981176168c0caa7a54666af76873757daf2`. The branch's implementation work is
closed. It contains diagnostic infrastructure and historical evidence, not
additional Halcyon application changes or acceptance of current 0.3.0 packages.

**0.2.5 is published.** Earlier reports and generated results saying “ON HOLD”
record the status at the time of those experiments. They do not override the
current release status or establish a current release blocker. Current work is
tracked in [REVIEW-030.md](../../docs/REVIEW-030.md) and
[PLANNED_WORK.md](../../docs/PLANNED_WORK.md).

## Completed scope

- Retained guest boot/replay infrastructure and recorded package provenance.
- Manual-theme and natural-sunrise development comparisons, summarized in
  [the final comparison report](theme-auto-baseline-islands-report.md), which
  supersedes earlier pending gates for those experiments.
- Three roughly 61-minute periodic editor-input sessions, each with 45 exact
  saves and 15 closes/reopens. See the
  [round-three report](window-lockup-soak-r3-report.md). Passing those input
  checks does not mean the subsequent workspace sequences passed.
- Prepared three native-blur controls with independent overlays; 13 local
  configuration tests passed. This is preparation only: no retained R2 native
  execution result was found. Summary success would still require frame review.

## Deferred historical diagnosis

Both public 0.2.0 contexts crashed the native compositor during post-hour
overview and recorded Files applet panics. R1 separately reproduced a Workspaces
client restart on stock, then stopped at a harness export assertion before the
native-blur comparison. RX 590 causation, whether an hour delay is necessary,
equivalent native-blur reproduction and the sufficiency of disabling global
frosting remain unproven. Closing this branch does not resolve those questions.

## Inert workflow snapshots

The two original workflows are retained under [workflows](workflows), outside
`.github/workflows`; they are not registered GitHub Actions jobs. Their original
contents are preserved for interpreting the executed-source records.

Do not present them as ready-to-run current tooling:

- Guest finalization uses a nonrecursive `cp` over a source directory that now
  contains subdirectories, followed by a flat retained-file hash operation.
- Theme preparation pins historical guest/package artifacts, while the old
  dispatch interface also offers incompatible newly built package runs.
- Live repair fetches the historical branch and can replace harness files
  during execution. That mechanism is not a current reproducibility contract.
- Raw evidence and some protocol/source/fixture dependencies are local archives,
  not all tracked files. Expiring Actions artifacts are not durable storage.

Any future diagnostic run needs an explicit new scope, a validated harness and
available immutable inputs. Integration did not dispatch a VM or build packages.

## Evidence custody

Tracked summaries, manifests, scripts and executed-source records are integrated
here. The complete runner checkout, including ignored ZIPs, screenshots and
source archives, is preserved on this machine at:

`/home/workstation/projects/personal/halcyon-evidence/closed-worktrees-20261025/halcyon-021-runner-probe/`

Its `private/qualification-021/` includes these locally retained reports absent
from tracked history:

- `guest-builder-report.md`
- `theme-session-report.md`
- `theme-session-followup-report.md`

The old runner path links to that archive so existing evidence references remain
usable locally. A fresh clone does not contain all inputs needed to repeat the
historical reviews. Run IDs, artifact identifiers and recorded hashes remain in
the original reports/manifests; those records were not rewritten as new results.

The R1 report referenced `glass-rendering-exploration-r1/report.md`, but that
report was not found in the retained worktrees. Treat its citation as unavailable;
the preserved R1 observations and pinned-source references stand separately.

## Integration checks

All 40 added Python scripts parse and all 10 shell scripts pass `bash -n`.
The self-contained artifact-boundary (3), native configuration (13) and media
preflight (6) tests pass from the integrated checkout. These checks do not claim
that archived screenshot-dependent suites or native experiments were rerun.

The reporting branch at `6d6f8cc` was already an ancestor of main and needed no
additional implementation. Its checkout and private evidence are preserved
under the same archive root as `cosmic-desktop-cards/`. Shared Git history is
retained separately at `/home/workstation/projects/personal/halcyon.git`; the
active checkout remains `/home/workstation/projects/personal/halcyon`.
