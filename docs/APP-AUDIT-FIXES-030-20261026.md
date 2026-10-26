# App audit fixes — 2026-10-26

**Status: all 40 findings fixed and locally verified.** This follows the [40-finding audit](APP-AUDIT-030-20261025.md). Changes are local and unpublished.

## Corrections

| Findings | Result | Regression evidence |
|---|---|---|
| REL01 | Published 0.2.5 renewal accepts only the pinned manifest, approval and artifact bytes. New artifacts still require current-policy admission. | Actual retained publication verified; modified artifacts and historical-policy candidate admission rejected. |
| U01–U03 | Update enrollment publishes complete files without replacement, respects administrator changes during retry, and preserves keys/pins still referenced by active sources. | Enrollment faults, renamed/disabled sources and concurrent administrator-file fixtures. |
| REL03–REL04 | Controller migration precedes configuration creation. CI executes the real Rust/controller transaction module. | Isolated legacy-preference migration and controller transactions. |
| W01, SET01 | Project metadata reads are bounded regular-file reads. Fill previews crop before resizing. | Device symlink, FIFO, oversized metadata and extreme-aspect-ratio image regressions. |
| RUN01 | Notes pauses editing and releases keyboard ownership before moving offscreen or recreating its surface. | Real layer-action ordering, retained draft and explicit Resume checks. |
| RUN02–RUN04 | Failed Arrange sessions retain their original display, draft and transaction receipts through identity/geometry changes and reentry. Resume waits for valid geometry. | Isolated output proxies, failed writes, alias changes, removal and recovery transitions. |
| OB01–OB04 | Staged destinations and per-display ordering are independent of preview selection. Planner/runtime share effective content sizing and edge clearance. Resume recaptures bar reservations. | Two-display draft round trips, production Cards parity, small-output limits and named-panel fixtures. |
| OB05–OB06 | Successful Undo synchronizes wizard receipts. Rendered wizard actions are bound to their step and view identity. | Undo/Cancel and duplicate queued Next/Apply regressions. |
| TX01–TX02 | Restore includes per-display geometry; exact receipt retirement preserves newer history and journal rollback. | Restore/Undo, older receipts, derived changes, conflicts and interruption faults. |
| TX03, PARENT01 | CLI palettes update the active color policy; workspace edits detect changed inherited defaults. | Actual palette/Undo and inherited-value conflict regressions. |
| SET02–SET04 | Wallpaper-only setup changes retain automatic recoloring; offline wallpaper targets have baselines; edited automation rules invalidate old waiting work. | Profile transactions, offline-display Undo and queued-job/restart checks. |
| UX01–UX05 | Editor errors remain visible, search reveals active controls, Back retains a theme candidate, and wallpaper caches revalidate same-path replacements. | Headless native interaction and bounded image-cache regressions. |
| W02–W04 | Global widget visibility stops grouped-provider work; saved weather shows age/failure; Focus credits the actual completion date. | Runtime scheduling, both weather views and persisted overnight timer regressions. |
| CAL01–CAL02 | Calendar nominal-day endpoints resolve DST transitions; generated nonexistent recurrence times are skipped before COUNT. | Calendar parser/isolation tests and vendored recurrence-library tests. |
| IB01–IB02 | Restart retires consumed session streams and reconnects; disabled installation compatibility remains unknown until measured. | Private socket reconnect, descriptor ownership and installation-state tests. |
| IN01–IN03 | Another visible bar cannot suppress fallback creation. Runtime status queries use configured identity. Islands reorder/gap edits have completion boundaries. | Private native two-bar toggles and Settings transaction regressions. |

Two targeted integration reviews found three follow-up issues. All are closed: pending Arrange Resume retains recovery protection, successful Undo updates an unfinished wizard even while it is closed, and existing restart fixtures follow the new descriptor-ownership contract. Four Arrange and six closed-wizard regressions passed. Preview Undo remains isolated from the real desktop's wizard state. Review reports, corrections and original failure logs are retained in the evidence directory.

## Validation

| Check | Result | Evidence |
|---|---|---|
| Rust application suite | **1,125 passed, 0 failed, 11 existing ignored**; 163.23 seconds | [Final full log](../.task-app-fixes-20261026/rust-acceptance.log) |
| New focused application regressions | **67**, all included in the full run | [Test names and binary hash](../.task-app-fixes-20261026/new-regressions.json) |
| Patched recurrence library | **300 passed, 0 failed** | [Standalone dependency tests](../.task-app-fixes-20261026/rrule-tests.log) |
| Python helpers, companion, enrollment, release and APT | **212 passed, no skips**, including 9 real controller tests and 40 private-namespace APT tests | [Suite summary](../.task-app-fixes-20261026/python-summary.json) |
| Activity collector | **37 passed** | [Collector log](../.task-app-fixes-20261026/collector-tests.log) |
| Main and native builds | Rust 1.93.0, release, locked, offline: **passed** | [Artifact hashes](../.task-app-fixes-20261026/artifacts.json) |
| Source and workflow checks | **471 source hashes recorded**, no audited source removed; whitespace and CI YAML checks passed | [Source inventory](../.task-app-fixes-20261026/final-source-hashes.json) |

All application checks use the integrated rebuilt source. Initial failures and their corrections remain documented; final counts above come from successful runs, not sums of repeated attempts. The builds retain unused-code and vendored dependency documentation warnings.

Main executable SHA-256: `48702eda0d99fea1fa710a9bf71c5c2ea3af1176e8b278f20310771c058571ba`.

The native companion passed three enable/disable cycles with a visible Panel and a Dock assigned to an absent display. The previous binary failed the first fallback in the same fixture. The patched build matches all 151 files from a fresh zero-fuzz source preparation. Companion SHA-256: `120b5ec27f679d42932be21664873626ebb503c5066c20181dd48d5e2346ffd9`.

The calendar dependency patch and its licenses are under [vendor/rrule](../vendor/rrule/HALCYON-PATCH.md). That directory must ship with the manifest and lockfile in the final source package. The application build validates the root lockfile; the standalone dependency-test lock is retained separately in the evidence directory.

## Separate release acceptance

Physical hotplug, mixed scaling, suspend/reboot, live Google authorization, schema-4 collector delivery, assistive technology and final signed-package qualification remain outstanding. The nested native fixture does not qualify physical devices or resolve the previously reported RX590 session lockup. This work does not install or publish 0.3.0.
