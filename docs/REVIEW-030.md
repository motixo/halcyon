# Halcyon 0.3.0 review tracker

Updated 2026-10-25. This tracker covers code quality, user behavior, failure
recovery and unnecessary complexity in local 0.3.0 development. Reviewed source:
`8a74f6c..73dfecb4c426cd9eda76027101fb63948bd21ece` on
`feat/multi-monitor-030`. Published 0.2.5 and signed APT remain separate.

There are 22 tracked review items: one P1, fourteen P2 and seven P3, including
R17's optional fault-model robustness work. Round 1 identified R01 through R18;
Round 2 added R19 through R22, broadened R03 and qualified existing findings.
R01 blocks release on resource-exhaustion grounds; R02 blocks the current release
verification path. All fixes remain open. Optional reductions and an unresolved
validation question are tracked separately. Follow-up reviews may challenge,
merge or reprioritize findings; preserve the reasoning when changing status.

## Work order and completion rules

1. Bound calendar parsing and make credential cleanup cancellation-safe: R01, R03.
2. Correct display and Islands state transitions: R04 through R08, R19, R20.
3. Correct calendar interoperability and collector writes: R09 through R11.
4. Repair release dispatch and include regression suites in CI: R02, R13.
5. Address inherited Settings lifecycle issues and smaller defects: R12,
   R14 through R16, R18, R21, R22. Consider optional R17 and reductions only
   with a clear benefit; R17 is not an established production defect.
6. Complete acceptance against the resulting fresh artifacts, keeping the
   outstanding external checks below explicit.

Use **Open**, **In progress**, **Fixed locally**, **Verified**, **Deferred** or
**Rejected**. A code change alone is Fixed locally. Verified requires a fix
commit/reference, focused evidence for the trigger and relevant build checks.
Record native acceptance separately where source or fixture evidence cannot
establish real session behavior. Deferred and Rejected require a reason.
Implementation owners are unassigned; review assignments do not authorize fixes.

Preserve all `.task-*-evidence` directories and existing private evidence. There
is one Cargo slot; coordinate any necessary build/test work through the parent.
Use Rust 1.93.0, `--locked --offline` and `/tmp/halcyon-app-target` when needed.
Do not run pathological recurrence inputs without strict process resource limits.

## Finding index

Evidence labels: **Code** means a traced control path, not native reproduction;
**Probe** means a focused isolated reproduction; **Fault model** means the
repository's simulated partial-write behavior, not established Sheets behavior.

| ID | Priority | Status | Finding | Evidence |
| --- | --- | --- | --- | --- |
| R01 | P1 | Open | Recurrence expansion can exhaust resources | Code and pinned dependency |
| R02 | P2 | Open | Release gate rejects current startup policy | Probe |
| R03 | P2 | Open | Cancellation and cleanup registry races can orphan credentials | Code |
| R04 | P2 | Open | Late display identity makes Arrange save under an obsolete key | Code |
| R05 | P2 | Open | Arrange leaves other display widgets offscreen | Code |
| R06 | P2 | Open | Optional applet watch failure rejects the host | Probe |
| R07 | P2 | Open | Repeated applet restart replaces the wrong client | Code |
| R08 | P2 | Open | Failed final verification still reports Ready | Probe |
| R09 | P2 | Open | Valid DST transition timestamps reject the feed | Code and format specification |
| R10 | P2 | Open | Case-sensitive calendar tokens lose or retain wrong events | Code and format specification |
| R11 | P2 | Open | Expiring histories amplify collector writes | Probe |
| R12 | P2 | Open | Stopping widgets also kills child Settings | Code, inherited |
| R13 | P2 | Open | CI omits new regression suites | Workflow inspection |
| R14 | P3 | Open | Google source colors and names remain missing or stale | Code |
| R15 | P3 | Open | Connector preferences lose selected display highlight | Code |
| R16 | P3 | Open | Reset command leaves monitor geometry intact | Code |
| R17 | P3 | Open (optional) | Partial first report loses first observed version | Fault model; live behavior unresolved |
| R18 | P3 | Open | Desktop launcher bypasses single-instance Settings | Code, inherited |
| R19 | P2 | Open | Resume after display change discards a failed Arrange draft | Code and bounded transition model |
| R20 | P2 | Open | Layout saves fail without the widget process display snapshot | Code |
| R21 | P3 | Open | Clock Move & resize opens the wrong display | Code |
| R22 | P3 | Open | Automatic reorder ignores monitor-specific custom placement | Code |

## Findings and acceptance

### R01 Bound recurrence work and stop cancelled parsing

Source: [subscription.rs](../src/calendar/subscription.rs), `parse` recurrence
expansion near line 512, and `parsed` using `spawn_blocking` near line 97.
The pinned `rrule 0.14.0` filters the requested date window after generating
occurrences. `.all(4096)` bounds returned dates, not historical iterations or a
period's intermediate allocations. A secondly rule starting in 1970 requires
roughly 1.79 billion historical iterations to reach October 2026. A dense yearly
rule can buffer 31,536,000 dates before the outer limit applies. The four-second
guard runs between records; the async timeout does not stop a running parser.

- Fix direction: enforce work/allocation budgets before and during expansion,
  and ensure cancellation terminates parser work. A smaller output cap or an
  additional async timeout alone does not solve it.
- Acceptance: resource-limited subprocess cases for historical secondly and
  dense yearly rules return a bounded failure promptly, terminate the worker,
  and cannot accumulate workers on repeated refreshes. Ordinary recurrence
  behavior remains correct. No unbounded stress run has been performed.

### R02 Use the current policy in release dispatch

Source: [release_gate.py](../scripts/compatibility/release_gate.py), `verify`
line 18; [runtime_compatibility.py](../companion/runtime_compatibility.py),
`POLICY` line 17. Runtime declares startup-v2, but release dispatch accepts only
startup-v1. An in-memory probe of the actual dispatch rejected v2 with
`unknown_release_policy` before practical approval verification.

- Fix direction: use the authoritative policy identity without weakening
  exact artifact/approval validation or reusing historical qualification.
- Acceptance: current policy reaches the correct verifier; mismatched policy
  and wrong artifact hashes fail. Qualify the fresh payload separately.

### R03 Preserve credential cleanup across cancellation and concurrent updates

Source: [subscription.rs](../src/calendar/subscription.rs), `commit` line 163;
[calendar settings](../src/settings/calendar.rs), outer timeout line 332;
[auth.rs](../src/calendar/auth.rs), `forget` line 290. The new credential and
connection can be committed before old-item deletion completes. The timeout
can drop cleanup before its fallback ID is recorded, leaving an old feed URL
or refresh token undiscoverable by later disconnect.
This timeout encloses subscription replacement, including replacement of a
Google connection. Google sign-in's own preparation timeout ends before commit;
the same timeout regression is not established there. Completion reloads the
connection before displaying the error, so stale connection UI is not assumed.

Round 2 also identified an inherited registry race in
[calendar/mod.rs](../src/calendar/mod.rs), `retry_stale_credentials` line 233,
and [store.rs](../src/calendar/store.rs), setter line 28. A retry snapshots `[A]`
and awaits deletion while another operation appends B. Replacing the registry
with the retry's local remainder then drops B's cleanup reference. The setter
also continues after a lock error and discards persistence errors. This is
code-path/interleaving evidence, not a real-keyring reproduction.

- Fix direction: persist pending cleanup before awaiting deletion and clear it
  only after success. Account for cancellation around new-item creation too.
  Use a checked, locked read–modify–write to remove only successfully cleaned
  IDs from the latest registry, preserving concurrent additions. Surface
  persistence failures rather than reporting durable cleanup intent prematurely.
  Do not enqueue a credential still active after a failed replacement.
  Distinguish a committed connection from preparation failure in the UI.
- Acceptance: delayed keyring deletion past the outer deadline leaves a
  discoverable cleanup reference, later cleanup removes it, and the current
  connection remains coherent. Concurrent append during cleanup survives the
  retry, and lock/write failures leave intent recoverable and report failure.
  Use a fake/private keyring, never real tokens.

### R04 Reconcile display identity changes during Arrange

Source: [app.rs](../src/app.rs), `reconcile_output` line 598;
[layout.rs](../src/app/layout.rs), captured contexts and commit line 695.
An asynchronous identity result can replace `DP-2` with an EDID key while
handles/geometry stay unchanged. The early return preserves the editor's old
contexts. Saves succeed under the connector key while effective layout reads
the EDID key, losing the visible edit after exit.
The trigger requires nonempty contexts: two outputs or existing multi-monitor
configuration. A plain single-monitor configuration without that state is not
affected by this specific path.

- Fix direction: handle persistence-context transitions independently of
  surface geometry, preserving connector geometry and rebasing/pausing drafts.
- Acceptance: enter Arrange, inject identities with unchanged geometry, move,
  finish and reopen; the move remains effective and Undo remains coherent.

### R05 Reconcile widgets outside the Arrange display

Source: [layout.rs](../src/app/layout.rs), `sync_widget_surfaces` line 470.
After switching Arrange to another display, the primary probe can arrive before
an extra probe. Unavailable widgets on the other monitor start offscreen; later
probe completion hits the active-Arrange early return and never restores them.
This requires the initial rebuild before the extra probe arrives. That later
event clears geometry without setting `rebuild_pending`, so end-of-update
reconciliation does not rescue it. Extra-probe-first ordering avoids the trigger.

- Fix direction: continue surface, geometry and input-region updates for
  widgets outside the overlay's editing display.
- Acceptance: exercise both probe arrival orders and verify non-editing
  widgets have correct size, position and input before Arrange exits.

### R06 Isolate optional watch failures through admission

Source: [panel_configuration.py](../companion/panel_configuration.py), `Watch.add`
line 44 and `_configured_applet` line 332;
[verification.py](../companion/verification.py), watch transfer line 84.
Registration is retained before symlink validation succeeds. Applet discovery
catches the error, but transfer replays it without that boundary and rejects
the host. The `/bin/sh` probe retained a failing registration on merged-/usr.
The failing symlink is the `/bin` ancestor. Leaf executable symlinks already
have entry/target observation support: `/usr/bin/sh` passed the challenge probe.
Desktop-entry symlinks encounter stricter read validation separately.

- Fix direction: record successful registrations only, address native-valid
  symlink ancestors/desktop entries, and preserve optional failure
  isolation throughout transfer without dropping required integrity checks.
- Acceptance: an Admission-level fixture covers symlinked launchers/desktop
  entries and failed optional watches; core failure still selects stock.

### R07 Keep repeated applet identities stable through restart

Source: [panel_configuration.py](../companion/panel_configuration.py), duplicate
admission line 448; retained native `cosmic-panel-bin/src/space/wrapper_space.rs`
line 754 under the issue-12 native-source-final evidence. The restart callback
finds the first client by applet name. Restarting a second identical applet
replaces the first occurrence's mapping. This is an inherited native defect
newly exposed by admitting duplicates in ordinary startup.

- Fix direction: bind restart callbacks to stable occurrence identities.
  Update the reproducible native patch/source, not only an extracted tree.
- Acceptance: launch two identical applets in one section, restart the second
  twice and verify both clients/positions. Bind new native evidence to the
  resulting binary; existing smoke evidence does not cover this recovery.

### R08 Publish Ready only after final verification

Source: [install_status.py](../companion/install_status.py), inspection line 239.
`inventory` can set `host=True` before `observation.check()` throws. The handler
records an error but keeps the successful host flag. A probe of the actual
inspection function returned Ready and healthy after `configuration_changed`.

- Fix direction: publish readiness after all checks, or invalidate it on errors.
- Acceptance: final-check failure after successful inventory cannot report
  Ready/healthy; successful verification and optional applet notices still work.

### R09 Resolve explicit DST transition times correctly

Source: [subscription.rs](../src/calendar/subscription.rs), `datetime` line 280.
The pinned parser rejects explicit ambiguous/nonexistent local times, causing
the entire feed to fail, even for an old out-of-window event. RFC 5545 specifies
the first occurrence for ambiguity and the pre-gap offset for a nonexistent
explicit time. Generated recurrence instances have separate rules.

- Fix direction: resolve explicit iCalendar timestamps according to
  [RFC 5545 section 3.3.5](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.3.5).
- Acceptance: Zagreb autumn ambiguity, spring gap, and an old transition event
  alongside a normal current event sync correctly.

### R10 Compare calendar tokens without case sensitivity

Source: [subscription.rs](../src/calendar/subscription.rs), property helpers
line 237, cancellation line 479 and recurrence dispatch line 488. The parser
retains property spelling; exact comparisons silently ignore lowercase
recurrence properties and fail to hide `STATUS:cancelled`.

- Fix direction: normalize property names and enumerated tokens, preserving
  case-sensitive text, IDs and URLs as required by
  [RFC 5545 section 3.1](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.1).
- Acceptance: mixed-case recurrence, exclusion and cancellation inputs produce
  the same agenda as uppercase equivalents; text/URL bytes remain unchanged.

### R11 Batch collector history expiry writes

Source: [collector.gs](../scripts/usage/collector.gs), `currentRows` line 88 and
`write_` line 259. Expiring day histories changes many rows during ingestion,
then issues one range write per changed row under the global lock. A fixture
with 100 installations and one report produced 100 writes at HEAD versus one
at the base. Production latency/timeout impact is unmeasured.
Successful expiry persists trimmed histories, so this is not necessarily repeated
on every report; row-count changes already use bulk writes. Method-call counts
do not establish network round trips because Apps Script caches writes. Keep
this as a scalability concern; measure lock duration during collector acceptance.

- Fix direction: batch adjacent changed rows into bounded range writes,
  including maintenance, while preserving status and retention semantics.
- Acceptance: bound write-call count for simultaneous expiry across many rows;
  preserve day counts, opt-outs, deduplication and interruption recovery.

### R12 Separate Settings lifetime from the widget service

Source: [app.rs](../src/app.rs), `launch` line 1677;
[desktop-cards](../scripts/desktop-cards), `settings` line 227;
[halcyon.service](../packaging/systemd/halcyon.service). Settings launched from a
widget remains a descendant in the service cgroup. Stopping background operation
also terminates Settings under systemd's default control-group kill behavior,
bypassing normal close/flush. This is inherited and awaits native confirmation.
This applies when the service launches the owning Settings process. Forwarding
to an already independently launched owner does not move it into the service.

- Fix direction: use an independent application unit/scope; retain normal
  widget-service child cleanup. Process/session detachment alone is insufficient.
- Acceptance: open Settings from a widget, stop/restart the widget service,
  and verify the window and its pending edits survive appropriately.

### R13 Include regression suites in CI

Source: [ci.yml](../.github/workflows/ci.yml), lines 40 and 50. Explicit suite
selection omits `collector.activity.test.cjs`, `companion.test_configured_applets`
and `companion.test_practical_compatibility`.

- Fix direction: use intentional discovery or explicit complete suite entries.
- Acceptance: demonstrate that each intended suite is collected by the workflow;
  no native or deliberately ignored diagnostic is accidentally made mandatory.

### R14 Refresh selected Google calendar metadata

Source: [calendar settings](../src/settings/calendar.rs), Loaded line 238;
[google.rs](../src/calendar/google.rs), event metadata line 163. Old single
selections synthesize an empty color; fresh calendar-list metadata updates only
editor choices. Event fetching keeps using stale persisted metadata. A sole
calendar cannot be deselected to force rebuilding the selection.
The same metadata path leaves provider calendar names stale as well as colors.

- Fix direction: reconcile display metadata by stable selected ID with revision
  fencing and unchanged selection semantics.
- Acceptance: an old connection and later provider color/name changes update
  event metadata after refresh without requiring reconnect.

### R15 Keep connector selections highlighted after identification

Source: [monitors.rs](../src/settings/monitors.rs), lines 120 and 163, and the
placement display selector. Choices emit EDID keys but highlighting compares
against saved connector strings. The output resolves correctly yet no button
appears selected.

- Fix direction: compare resolved output identity, preserving Default/Automatic.
- Acceptance: a connector preference plus an EDID result highlights exactly
  the correct output for global and per-widget controls.

### R16 Reset monitor geometry with the desktop command

Source: [layout.rs](../src/app/layout.rs), `reset_layout` line 871. The command
clears only base positions/sizes; monitor layouts survive and keep overriding
the reset. The Settings reset path is separate.

- Fix direction: reset monitor geometry transactionally and refresh effective
  geometry while preserving display assignments.
- Acceptance: reset base and per-display layouts, confirm defaults after
  restart, and verify Undo restores both.

### R17 Recover first observed version after partial writes

Source: [collector.gs](../scripts/usage/collector.gs), line 128. In the existing
partial-cell fault model, interruption after column J or K persists the first
receipt timestamp without its version. Retry initializes both only when that
timestamp is empty, leaving the version permanently blank. Fixture failures
after cells 10 and 11 reproduced this; actual Sheets partial-cell behavior is
not established.
Round 2 classifies this as optional robustness against the declared fault model,
not a confirmed live Sheets corruption bug. The fixture writes cells sequentially
and throws mid-range; a failure after the complete range write preserves the
original version on retry. The platform's rectangular-write documentation does
not establish prefix-write failure semantics or prove atomicity.

- Fix direction: recover the incomplete pair from the previously persisted
  version before a later report overwrites that value.
- Acceptance: extend interruption checks to first receipt/version as well as
  counts, including a retry reporting a newer app version.

### R18 Centralize Settings instance ownership

Source: [desktop entry](../packaging/io.github.profilaz3.Halcyon.desktop), line 5;
[settings.rs](../src/settings.rs), runner line 811. The launcher calls the binary
directly, bypassing the controller lock/forwarding path. Ordinary toolkit run
does not establish single-instance ownership. Multiple Settings processes can
consume/remove the same command file. This is inherited.

- Fix direction: centralize ownership and forwarding in the native entry point
  or consistently route supported launchers through the controller. Coordinate
  with R12 so ownership and service lifetime are solved together. Avoid redundant
  Python and native ownership mechanisms; account for direct binary launches.
- Acceptance: repeated launcher, widget and direct launches leave one Settings
  owner and route each requested page correctly.

### R19 Preserve failed Arrange recovery when display contexts change

Source: [layout.rs](../src/app/layout.rs), context mismatch line 580, failure
preservation line 767 and `resume_arrange` line 817. After a failed save/cancel,
the editor retains its draft and rollback receipts with `exit_failed=true`.
If a display disconnect changes placement contexts, Resume clears that flag
before entering `arrange`. The new context-mismatch branch then discards the
paused editor, losing the unsaved draft and Retry cancel receipts. Persisted
history survives; the session recovery operation does not. This is distinct
from R04's obsolete persistence key and is supported by source tracing and a
bounded transition model, not native hotplug reproduction.

- Fix direction: validate contexts before clearing the failed state. Preserve
  the draft and receipts on mismatch; require explicit discard or safely
  restore/rebase the original editing context.
- Acceptance: construct a failed paused editor containing a sentinel draft and
  receipt, change contexts, dispatch Resume, and verify both remain recoverable.
  Cover failed Save and Cancel, and confirm explicit discard still works.

### R20 Make Layout editing coherent without a desktop snapshot

Source: [layout_state.rs](../src/layout_state.rs), `for_display` line 235;
[placement.rs](../src/settings/placement.rs), captured contexts line 247 and
save guard line 1116. With background startup disabled, a new login can have
no widget-process snapshot. Fallback geometry has empty monitor/context maps,
while Settings independently discovers two live outputs. Every Layout save then
fails with “Displays changed. Cancel this move and try again.” Cancel/retry
cannot repair the missing snapshot; monitor selection also lacks geometry.
Starting widgets supplies it, but a dirty editor retains its captured contexts
until canceled. This is distinct from R04's active Arrange identity transition.

- Fix direction: construct consistent fallback geometry and contexts from the
  live Settings registry, or gate editing with an actionable prerequisite state.
  Preserve the context guard against saving to the wrong display.
- Acceptance: initialize Settings without `layout-state.json`, deliver two fake
  outputs, select/nudge a widget and verify usable editing or an explicit
  prerequisite, rather than an endless retry instruction. Source-path evidence
  only; no native reproduction has been performed.

### R21 Route Clock layout shortcuts through display-aware selection

Source: [pages.rs](../src/settings/view/pages.rs), button line 1222;
[settings.rs](../src/settings.rs), `EditWidgetLayout` line 1814;
[placement.rs](../src/settings/placement.rs), ordinary selection line 1048.
The Clock shortcut resolves Clock to Glance where applicable, then directly
selects it without switching the preview monitor. With preview A and target B,
the requested widget is absent from the canvas and arrow movement does nothing.
Manually choosing B recovers. This is a navigation integration defect supported
by source tracing, not a native reproduction.

- Fix direction: reuse display-aware selection after resolving Clock/Glance.
- Acceptance: with preview A and Clock or Glance on B, activate Move & resize
  and verify preview B, visible target geometry and working movement.

### R22 Recognize monitor placement in automatic reorder controls

Source: [config.rs](../src/config.rs), custom-position detection line 288 and
monitor save line 607; [settings.rs](../src/settings.rs), reorder line 2411;
[layout.rs](../src/app/layout.rs), effective geometry line 307. Monitor-only
manual placement leaves base position/size maps empty. Automatic column order
checks those base maps only, admits a reorder and reports a successful save,
but explicit monitor geometry keeps the widgets in place. The existing Custom
layout explanation is absent. This is separate from R16's reset command.

- Fix direction: include relevant monitor geometry in custom-placement checks
  used by admission and explanatory copy; preserve genuinely automatic ordering.
- Acceptance: manually place Weather and Devbox with only monitor geometry,
  then verify reorder is appropriately disabled/explained. Cover an automatic
  layout that should still reorder. Source-path evidence only.

## Optional reductions

- **O01 Open:** Consolidate duplicate image caches in
  [profiles.rs](../src/settings/view/profiles.rs), lines 93 and 122. Only
  `look_image` calls `look_rgba`; both caches retain pixel copies. Removing the
  redundant copy saves 3.52 MiB per 1280x720 wallpaper, approximately 21.09 MiB
  across six visited looks, excluding renderer allocations. Keep stable handles
  and the bounded single-entry crop cache. This is bounded overhead, not a leak.
- **O02 Open:** Remove obsolete single-selection `Select`, `labels` and
  `selected` calendar editor state after moving useful race coverage to Toggle.
  Confirm all message/test callers before removing the path.
- **O03 Open, inherited:** Narrow successful-write refreshes in
  [settings.rs](../src/settings.rs), `after_desktop_change` line 3347. It refreshes
  every opened editor and replans profiles synchronously. Latency is unmeasured;
  retain correctness and measure a representative save before adding machinery.
  Prefer affected-editor refresh over a new scheduling framework.

## Validation question

**V01 Open:** `DesktopStore::recover` and `discard_recovery` change settings,
history or journals without incrementing the revision checked by
[native_jobs.rs](../src/settings/native_jobs.rs), line 169. A visible stale-state
defect is not established because watchers/page reopening may reconcile it.
Round 2 found concrete repair paths: successful Dock recovery reloads its own
drafts/recovery state; native completion refreshes global Undo unconditionally
at line 356; widget configuration reaches Tick through content watching with
a 30-second fallback; native editor pages and Profiles refresh on reopening.
A brief stale preview before notification remains possible. Do not promote the
missing revision alone to a confirmed defect.
Force the native-worker recovery path and deliver completion before watcher
events; inspect preferences, recovery flags and Undo. Promote to a finding
only with a concrete failure; otherwise record why reconciliation is adequate.

## Remaining acceptance

These are existing gaps, not additional confirmed defects:

- Live Google OAuth/provider acceptance.
- Live schema-4 collector migration and delivery; Workspace authorization was
  blocked with 403 and the current schema-2 endpoint rejects new reports.
- Physical monitor hotplug, resume and reboot, including mixed scaling.
- Fresh main/companion package acceptance bound to their actual new hashes,
  release metadata, images and short release notes.
- Actual Caffeine Flatpak and popup behavior.
- Broader GPU/mode-switch and long-running widget behavior. The confirmed
  Settings GPU improvement does not qualify these scenarios.
- RX590 lock-up report causation remains unproven; Arrange fixes do not close it.

## Review rounds

Round 1 is complete: calendar/privacy, monitor/layout, Islands, architecture,
reporting and startup/release integration. Parent checked the reported source
paths and relevant pinned dependency behavior. Focused Python/Node probes support
R02, R06, R08, R11 and R17; no native reproduction or broad test rerun was claimed.

Round 2 is complete, with three additional Codex reviews:

- **User workflows:** added R20 through R22 and confirmed metadata, highlighting,
  reset and lifecycle qualifications across everyday actions.
- **Failure recovery, complete:** added R19, confirmed and broadened R03 with
  inherited registry races, and found repair paths keeping V01 unconfirmed.
- **Challenge review:** upheld R01, narrowed R03 through R06 and R12 triggers,
  retained R11 as call amplification with unmeasured runtime impact, and
  classified R17 as optional fault-model robustness. R18 remained supported.

Round 2 used read-only source/dependency inspection and bounded probes. The
parent reconciled overlapping findings, evidence and disagreements here. No
review finding is marked fixed by review alone; no implementation, build,
native session or broad test rerun was performed for this follow-up round.
