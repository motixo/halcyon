# Planned work

## Current release and store distribution

Updated 2026-10-24: **0.2.5 is published**. PR #11 added the signed APT channel;
public enrollment, authenticated downloads, missing-key rejection and normal
metadata renewal passed. Users can opt into system updates using
[the enrollment instructions](DISTRIBUTION.md). No global/default Store listing
or native Store search result is claimed.

Next distribution work: [store listing preparation](STORE-LISTING.md). The
preferred eligibility discussion is with the COSMIC Flatpak maintainers, with
Islands remaining an optional APT package. Their contribution rules require
human-written upstream issue/PR text; no inquiry has been posted. The source audit
records the real startup, host-settings and helper changes needed before a
working Flatpak can be submitted. Existing release notes below describe the
historical preparation state, not a current publication hold.

## Islands rejects configured applets — issue #12

Recorded 2026-10-24. [Report and panel configuration](https://github.com/profilaz3/halcyon/issues/12).
**Fixed locally for 0.3.0; not released.** Ordinary startup discovers configured
applets without the fixed list, and reports individual applet problems separately.

After upgrading from 0.2.0 to 0.2.5 on COSMIC 1.11, the reporter gets the stock
panel at every login. Their Workspaces (`com.system76.CosmicAppletWorkspaces`)
and Caffeine (`net.tropicbliss.CosmicExtAppletCaffeine`) IDs are absent from the
startup policy's hard-coded list. The reporter says the layout worked in 0.2.0
and the separate native ABI probe passes.

Confirmed using production `runtime_compatibility.Session` in the existing
isolated configuration/process fixture: Audio is accepted; each reported ID
independently raises `unqualified_configured_applet` before desktop-entry
resolution. The reporter's machine and live panels were not touched.

Implemented:

- Native XDG desktop-entry discovery for arbitrary configured applet IDs,
  standalone/multiplex launchers, wrappers, repeated instances and displays.
- Host/core readiness remains required; missing or failed optional applets no
  longer force stock fallback. Executable/configuration changes still invalidate
  startup, and owned-process cleanup remains enforced.
- CLI and Settings show bounded applet-specific notices. An absent host no longer
  implies a compatibility update. Launcher failures retain a sanitized reason.
- The native host skips empty/malformed applet commands instead of panicking.
  Startup policy v2 binds its freshly built ELF; historical evidence is unchanged.

Validation: original production Session reproduction fails on the baseline and
passes after the fix. Focused discovery/status/lifecycle, owned-group cleanup,
Rust diagnostics and native command parsing pass. The real 0.2.0 package migrates
and disables correctly in isolation. A private nested COSMIC session passed the
ABI probe and IPC checks, rendered actual Workspaces and a custom Audio desktop
entry, and stayed alive with missing/invalid applets. Caffeine's ID is covered by
process fixtures; its actual Flatpak/popup and the reporter's login are not yet
verified. No live user panel was replaced.

This issue is separate from Settings rendering and monitor placement; those local
changes are preserved. Final 0.3.0 package acceptance must bind the new native
ELF rather than inherit 0.2.5's historical approval. No GitHub reply, closure or
publication was performed by this fix.

## Historical release preparation

Feature requests and user reports, with implementation and investigation status.
On 2026-10-23 the maintainer requested the tracked features and held publication
while implementation continued. The upcoming version is **0.2.5**; older 0.2.1
task, branch and evidence names remain historical identifiers.

On 2026-10-24 the maintainer asked to defer the blocked blur experiment and
accelerate release. Release preparation is resumed with a narrower focus:
finish the runtime compatibility correction, use a conservative material path,
and verify the actual packages' launch, Settings, Disable/restore and included
feature behavior before publication. Nothing is claimed shipped yet.

Further blur causation research, wallpaper-frost/GPU prototypes, physical RX 590
qualification and exhaustive platform coverage are follow-up work, not claims
of this release. Preserve the reported failures and disclose unverified scope.
Do not advertise system-update enrollment before its hosting/bootstrap works;
unavailable enrollment need not delay the GitHub release. Local opt-out is tested
and effective. The collector upgrade is deferred: its current schema rejects
0.2.5 reports and preference messages, so counts will be incomplete. This does
not block the app release; successful server delivery is not claimed.
Known failures need safe released behavior rather than a waived test.

## Activity reporting: user control and opt-out counts

Recorded: 2026-10-23. Status: client implemented and verified; live collector migration deferred. Local opt-out is accepted for release. Server opt-out counts remain unavailable until migration; pending preferences expire after seven days without re-enabling reporting. Older clients continue reporting.

### Requested experience

- A visible, clearly labelled **Share usage data** checkbox, checked by default,
  near the bottom of onboarding. Include it in an interactive installer if one
  exists; a noninteractive package install must not introduce a new prompt.
- The same preference in Settings → Privacy, so people can change it later.
- Explain briefly what is sent and link to the full privacy details. Keep the
  checkbox easy to notice and use, rather than hiding it in small print.
- Remember the choice across restarts and upgrades. An upgrade must never turn
  reporting back on after someone disabled it.

### Reporting and the private sheet

The request is to count installations that opt out and record when they do so.
Use installation terminology, not “accounts”: the current reporting identifier
is a random installation token, not a Google account or a person's identity.

Proposed sheet additions:

| Field | Meaning |
| --- | --- |
| Reporting status | Reporting enabled or opted out |
| Opted out at (UTC) | Time of the user's opt-out action |
| Status received at (UTC) | Collector receipt time, distinct from action time |

- When the user unchecks the option, stop daily activity reports and country
  lookups immediately. Cancel pending activity work and prevent stale requests
  from overwriting the new status.
- Send one minimal, explicitly disclosed opt-out status event to update the
  sheet. This is still data transmission: the UI and privacy document must say
  it happens. It is not permission to continue daily reporting after opt-out.
- Use the existing pseudonymous installation identity when available; do not
  add an account identity, fresh country lookup or widget content to this event.
- Keep opt-out counts separate from active-installation counts. Receiving an
  opt-out event must not refresh the installation's last-active date. Summaries
  should exclude opted-out installations from the current reporting population
  without erasing or misrepresenting earlier aggregate activity.
- Make repeated event delivery idempotent. Delayed activity events must not
  change an opted-out record back to enabled; re-enabling must be an explicit
  newer preference change.

### Implemented decisions and verification

- First-run opt-out creates a random pseudonymous identity solely for the
  explicitly disclosed minimal status update. It performs no country lookup.
- `usage.json` stores the default-on choice independently of config, themes,
  portable setups and Undo. Existing identity migrates; damaged state fails
  closed. Older strict state readers cannot silently re-enable it.
- The latest preference status alone retries for at most seven days / 28
  attempts. Off takes effect without waiting for delivery; activity is never
  backfilled. File locks do not span networking, and Settings saves on a worker.
- Revisions order explicit choices and reject stale responses/status/activity.
  In-flight work is cancelled where possible; already transmitted requests
  cannot be unsent. The latest opt-out timestamp survives re-enable locally and
  in retained collector records when that opt-out was delivered.
- Schema 3 supports activity and minimal status; schema 1/2 activity remains
  compatible but cannot re-enable retained opted-out rows. Additive, idempotent
  Sheet migration preserves original rows and unrelated settings. Status-only
  records never advance activity. Current summaries and countries exclude off.
- Identifiable opted-out rows expire within 90 days of status receipt. The
  current opt-out count explicitly names that window; it is not lifetime users.
  Historical aggregates are untouched. Expired identities have no permanent
  revision tombstone. Local preference remains off after queue/record expiry.

Verification: loopback Rust client tests cover cancellation before/through
requests, restart, another process, retry/expiry, damaged state, re-enable and
stale completion. Native widget-tree clicks verify the Settings and onboarding
checkbox, persistence, save failure, theme/setup import and Undo isolation;
actual captures cover 820×600 and 1024×760. Node fake-Sheet tests cover schemas,
ordering, deduplication, retention and interrupted additive migration. Release
health tests are mocked and require schema 3. Local release-build evidence and
exact commands/hashes are in the usage-optout implementation handoff.

[Privacy](PRIVACY.md) explains the new behavior and its pending-release status;
[scripts/usage/README.md](../scripts/usage/README.md) documents client/collector
compatibility, deployment order and rollback that preserves accepted opt-outs.
No live collector/Sheet operation, publication or host installation was done.
Independent review accepted the collector migration corrections: an unrelated
Reporting tab and occupied blank-header extension columns are preserved before
any preference-state writes. The client/UI had no additional material finding.
Live verification and deployment remain pending; 0.2.1 stays on hold.

## Reliability report: windows stop responding

Recorded: 2026-10-23. Priority: high. Status: investigation open; native compositor
crash reproduced separately, reporter causation unconfirmed. No public issue URL
was supplied with this quote.

> Thanks for sharing this software. I like the level of customization it offers.
> Unfortunately, I had to uninstall it after an hour of using because it seemed
> to cause windows to lock up such that I couldn't close or interact with them.
> If the bugs were ironed out, I would definitely give a shot again.

The reported impact is severe enough that the person uninstalled Halcyon.
The cause is not established: do not assume Islands, a particular material,
Halcyon's main process or COSMIC is responsible without evidence.

Investigation update: source inspection of both 0.2.0 and the current branch
found that a failed Arrange cancellation can leave the full-screen input-capture
surface active. Escape takes this same path. A local fix now releases the layer
on failed Cancel/Done exits, retains explicit recovery in the edge menu, refuses
contended settings locks promptly, and rejects stale events that could reopen
Arrange. Focused RED/GREEN tests and a release build pass. Review found and then
verified a correction for display-pin changes during recovery; full-session
acceptance remains pending. This is a demonstrated code fix,
**not proof of the reporter's lock-up cause or its resolution**.

The corrected native comparisons completed about 61m31s each on stock COSMIC,
public 0.2.0 main-only, and main plus Islands. Each passed 45 exact saves and
15 normal closes/reopens including preflight. These are periodic input tests,
not continuous typing or complete inactivity tests.

Both Halcyon contexts then crashed the compositor while opening workspace
overview, before suspend: `Custom Shaders not initialized` at
`src/backend/render/wayland/blur_effect.rs:113`. Native Files applet panics also
occurred during their otherwise successful core hours. Stock passed its
workspace/resume sequence, but did not have equivalent native blur enabled.
The product contexts used Frosted/Ocean. Islands is therefore not necessary for
this observed crash; blur causation and the need for an hour delay still need
isolation. The pinned source suggests missing shader initialization on a
software/offscreen renderer, but that is not yet a causal qualification.

Evidence: `../halcyon-021-runner-probe/private/qualification-021/window-lockup-soak-r3-report.md`.
Runs: stock 40018232557, main 40027094006, Islands 40035187607.
The first short native-blur comparison stopped on a test export assertion
before the blur controls ran. Stock opened its first overview, but a second
overview crashed/restarted the separate Workspaces application; compositor and
editor input survived. This is distinct from the Frosted compositor panic.
Corrected assertions are prepared. The follow-up agent was stopped by an
automatic provider safety flag; no native comparison result was returned.
The three missing controls remain unverified.
Keep the compositor crash open as a release concern; do not describe the
complete sessions as passed.
Software rendering does not qualify the RX 590 or resolve the original report.

### Reported hardware

- CPU: AMD Ryzen 7 5800XT.
- GPU: AMD Radeon RX 590.

Supplied by the maintainer as the affected user's machine. This is reproduction
context, not evidence that either component caused the lock-up. Kernel, Mesa,
graphics-driver versions, display configuration and session logs remain unknown.

### Investigation to queue

- Establish Halcyon and exact COSMIC package versions, whether Islands was
  installed/enabled, active theme/materials, and affected applications.
- Determine whether one window, all application windows or the whole session
  stopped responding; whether keyboard shortcuts still worked; and whether
  stopping Halcyon, disabling Islands or logging out restored interaction.
- Request only relevant, redacted logs around the failure and approximate time.
  Do not collect private widget contents, credentials or an entire home folder.
- Include the Ryzen 7 5800XT / Radeon RX 590 configuration in hardware test
  targets where available. Record kernel, Mesa and graphics-driver versions,
  monitor count, resolution and scaling; inspect relevant GPU reset/hang and
  compositor errors around the incident. A software-rendered VM pass does not
  establish that this hardware-specific report is resolved.
- Reproduce in a disposable full session. Compare stock COSMIC, Halcyon without
  Islands, and Halcyon with Islands, holding other settings constant. Exercise
  focus, click-through, close/move/resize, popups, workspaces and idle/resume over
  at least the reported hour. Check input routing, surface stacking, event-loop
  stalls, resource growth and compositor errors as hypotheses, not conclusions.
- Preserve reproduction steps and environment, add a regression for the actual
  cause, and link the eventual issue, fix PR and verified release here.

Acceptance: the identified reproduction remains responsive through the relevant
duration and interaction sequence, with evidence that the fix addresses the
cause. A short smoke test alone cannot dismiss an hour-long lock-up report.
If this problem is reproduced during current release testing, treat it as a
release blocker rather than hiding it in the backlog.

## Multi-monitor: 0.3.0 local implementation

Updated 2026-10-24. Implemented locally on `feat/multi-monitor-030`; not installed,
pushed or released. This extends the earlier global display selector below.

- Layout shows the connected screens in their logical arrangement. Select one
  to edit its widgets; Identify briefly labels the physical screens without
  taking keyboard or pointer input.
- Each widget can inherit the default display or choose a separate display.
  These controls also appear on its own settings page, so a hidden widget can
  be recovered without finding its card on the desktop. Missing displays temporarily use an available display, or hide the widget
  when **Hide if disconnected** is enabled. The original preference survives.
- Position and size are saved separately for each widget/display. Editing a
  temporary placement does not overwrite its layout on the preferred screen.
- Arrange on desktop opens on the screen selected in Layout. Other screens'
  widgets remain outside that editing overlay. Disconnect/geometry changes
  cancel in-progress drags and retire their input surfaces.
- Compositor-provided logical sizes drive each widget's geometry independently.
  Mixed scaling and portrait layouts use those logical dimensions, rather than
  treating every screen as the first screen's pixel size.
- When readable, unique EDID serials provide a local hashed display identity
  across connection changes. Displays without an unambiguous identity keep
  connection-name matching; identical/ambiguous devices are not guessed.
- Saved Setups capture the effective layout without physical display identities.
  Applying a layout updates saved display variants and supports Undo, preserving
  display assignments and disconnect choices.
- Dock & panel links from Layout to its existing native placement controls.
  Resolution, rotation and OS display arrangement remain in COSMIC Settings.

Validation uses isolated Wayland output handles, native widget interaction and
small/large Settings captures. These checks do not qualify real docking,
resume, GPU behavior or a physical mixed-DPI setup. The requested laptop could
not be inspected: its local hostname did not resolve, and the existing trusted
Tailscale laptop address refused SSH on port 22. No remote preferences changed.
Physical laptop verification remains the next acceptance step when accessible.

Local validation: the full app suite passed 989 tests with eight existing skips.
After the final first-use and hidden-widget UI refinements, all eight dedicated
multi-monitor tests, 30 placement tests and four display Settings tests passed.
The production compile, formatting and whitespace checks passed. Two existing
unused-code warnings in Calendar and widget styling remain unchanged.

## Multi-monitor report: remember the chosen display

Recorded: 2026-10-23. Priority: high. Status: open user report; implementation and review corrections complete locally; native multi-output verification pending.
Source: [GitHub issue #9 — Problems when having multiple monitors](https://github.com/profilaz3/halcyon/issues/9),
reported by Tri4c3.

Source investigation completed: widgets have no saved display preference;
their surfaces use compositor-selected outputs. Output changes during the first
five seconds can be skipped. Islands already remembers a named display, but its
disconnected-display fallback depends on enumeration order. These code findings
do not establish which surface the reporter encountered. The initial scoped implementation
is local and is not a published fix: Settings → Layout now saves a
connection-name Widget display choice. Every widget, tool, Arrange, probe and
edge surface uses the selected compositor handle. Startup metadata is processed
without the former five-second gap. Disconnect uses built-in/lexical-name
fallback, retains the preference and normalized layout/sizes, and restores the
preferred connection when it returns. Epochs and fresh IDs reject old surface
events; a new output's logical probe size gates visible creation. Active edits
release grabs and retain drafts. Unnamed or duplicate-named outputs cannot be
pinned; identical models with unique names remain distinct.

The Islands change only orders fallback for already-named Islands bars.
Native Dock/Panel All/Active/Name policies are unchanged. Local widget pins take
part in save/Undo but are excluded from portable profiles.

Verification: seven application/registry/settings tests (including real emitted
layer actions and native selector interaction), eight existing layout tests,
twelve existing tool tests, one probe-budget regression and the focused Islands
core regression pass. Native Settings captures were inspected at 820×600 and
1024×760. Source and build evidence are retained in the monitor implementation
handoff. Full multi-output COSMIC guest testing (restart/login/reboot, hotplug,
scaling and input routing) remains required; these tests do not establish the
reporter's component or resolve the public report. 0.2.1 publication stays on hold.

Independent review identified two corrections, now fixed locally: enabled Islands fallback
was counted as zero applets by the compatibility resolver, and display migration
during a sidebar reorder dropped its cancellation baseline. The resolver must
watch and fingerprint the Islands sidecar and model the actual enabled fallback;
migration must restore the original order before rebuilding surfaces. The sidebar cancellation correction passed independent review. The compatibility
parser correction now passes a scoped parent review and focused regressions
using actual Rust-serialized/loaded fixtures. Native acceptance remains required
for the combined behavior.

Chosen scope: add a **Widget display** selector using compositor output names,
preserve the selection while disconnected, and use a deterministic fallback.
The pin follows the connection, not a physical monitor moved between ports.
Keep existing Dock/Panel All/Active/named-display choices separate; correct only
Islands' fallback ordering. All widget and editing surfaces must migrate together
without rewriting saved layout or sizes.

> When turning on or restarting the plugin is not blocked on a single screen
> and is put on the one that loads first in the composer. Screen lock is needed.

The reporter describes placement following whichever monitor becomes available
first at startup/restart. The requested behavior appears to be pinning placement
to a chosen display. “Screen lock” is interpreted here as display assignment,
not session locking; the affected component (widgets, Islands, panel or dock)
and exact reproduction still need confirmation.

### Investigation and intended behavior

- Establish Halcyon/COSMIC versions, the affected component, monitor connections,
  primary-display setting, resolutions/scaling and startup order. Check whether
  the problem occurs after app restart, login, reboot, resume or hotplug.
- Inspect saved output selection and compositor output discovery. Test whether
  placement depends on enumeration order or a temporary output index instead
  of a persistent display identity; this is a hypothesis, not a confirmed cause.
- Remember the user's chosen display across restarts and output-order changes.
  Define a predictable fallback when it is disconnected, retain the preference,
  and restore placement when that display returns without losing widget layout.
- Test two or more displays, delayed initialization, primary-display changes,
  unplug/replug, mixed resolution/scaling and duplicate display models. Keep
  widgets visible and interactive without duplication or off-screen placement.
- Link the eventual reproduction, fix PR and verified release here. Current
  single-output VM results do not resolve this report.

Acceptance: placement stays on the selected display through restart/login/reboot
regardless of discovery order, and disconnect/reconnect follows the documented
fallback and restoration behavior. Scoped implementation complete; full native acceptance remains pending.
No verified release or public issue-resolution claim is made.

## System updates

Target: **0.2.5**. Status: local implementation independently approved; full
COSMIC/PolicyKit/Store and final-package verification pending. Publication is
on hold.

- App settings shows the running installation and offers a manual update check
  only for a verified installation enrolled in the Halcyon APT source.
- Checks run asynchronously and can be cancelled. Installation remains with
  the operating system; the app opens COSMIC Store for updates.
- Manual downloads, source builds and replaced executables show appropriate
  information rather than offering to update a different installation.
- Actual disposable package installation satisfies the checksum verification
  contract; altering the installed key disables update checks.
- Public enrollment remains unavailable until its source and bootstrap packages
  are ready and verified. No hosting, enrollment or release is claimed complete.

## Development package session observations (October 24)

Status: investigating exact `0.2.5~dev.integration.1` packages, source `b6419c70`.

- Applying Outline in the software-rendered COSMIC guest produced colored
  fragments around the stock dock. They persisted after Undo and disappeared
  after relogin. Desktop input remained usable. Cause and physical-GPU prevalence
  are unproven; this does not establish the RX590 report's cause.
- Settings worked through interactive checks, then logged broken pipes and a
  segmentation fault during deliberate login-session termination. Review
  disconnect handling; do not describe this as a spontaneous in-session crash.
- Dock & panel says to install Islands when it is already installed but
  unavailable. Make the warning match the known installation/compatibility state.

Evidence: `halcyon-021-evidence-recovery/package-session-acceptance-r3/report.md`
with screenshots and exact receipts. Focused diagnosis is underway. Core package
interaction checks passed; active Islands, multiple outputs and live collector
acceptance are separate outstanding checks.

## Adding future entries

Record the user need or original report, intended behavior, current status,
priority, unresolved decisions, acceptance criteria and related issue/PR links.
Keep proposed work separate from confirmed defects and completed features.
