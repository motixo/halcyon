# Planned work

Feature requests and user reports, with implementation and investigation status.
On 2026-10-23 the maintainer requested all three entries below be included in the
current work and explicitly put the 0.2.1 release on hold. Existing reliability
and distribution work continues. Nothing below is claimed shipped or resolved
until its implementation and relevant verification are complete. Publication
remains on hold; the earlier autonomous-release instruction is superseded.

## Activity reporting: user control and opt-out counts

Recorded: 2026-10-23. Status: accepted for current work; implementation pending.

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

### Decisions to settle before implementation

- First-run opt-out before an installation identity exists: decide whether to
  send an unlinked aggregate event or create an identifier solely for recording
  that choice. Do not introduce that behavior silently.
- Offline delivery: decide whether the single status event can be queued, its
  bounded retry/expiry policy, and what is disclosed. Local opt-out must take
  effect immediately even when the collector cannot be reached.
- Re-enabling: define whether the sheet retains the previous opt-out timestamp
  and whether metrics mean current opted-out installations or opt-out actions.
  Avoid counting one installation repeatedly as different people.
- Retention: define how status records expire alongside the current 90-day
  activity retention. Do not keep identifiable opt-out records forever merely
  to preserve an aggregate count.
- Coordinate client schema, collector, sheet migration, onboarding, Settings
  and the privacy notice. Existing releases must remain compatible while the
  transition is deployed.

Current behavior is documented in [Privacy](PRIVACY.md): reporting is always on
in 0.2.0. This roadmap entry does not change that behavior or claim an opt-out
already exists. Relevant code: `src/usage.rs` and `scripts/usage/collector.gs`.

## Reliability report: windows stop responding

Recorded: 2026-10-23. Priority: high. Status: unconfirmed user report; investigation
pending. No public issue URL or reporter details were supplied with this quote.

> Thanks for sharing this software. I like the level of customization it offers.
> Unfortunately, I had to uninstall it after an hour of using because it seemed
> to cause windows to lock up such that I couldn't close or interact with them.
> If the bugs were ironed out, I would definitely give a shot again.

The reported impact is severe enough that the person uninstalled Halcyon.
The cause is not established: do not assume Islands, a particular material,
Halcyon's main process or COSMIC is responsible without evidence.

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

## Multi-monitor report: remember the chosen display

Recorded: 2026-10-23. Priority: high. Status: open user report; implementation complete; native multi-output verification pending.
Source: [GitHub issue #9 — Problems when having multiple monitors](https://github.com/profilaz3/halcyon/issues/9),
reported by Tri4c3.

Source investigation completed: widgets have no saved display preference;
their surfaces use compositor-selected outputs. Output changes during the first
five seconds can be skipped. Islands already remembers a named display, but its
disconnected-display fallback depends on enumeration order. These code findings
do not establish which surface the reporter encountered. The scoped implementation
is complete locally and is not a published fix: Settings → Layout now saves a
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

## Adding future entries

Record the user need or original report, intended behavior, current status,
priority, unresolved decisions, acceptance criteria and related issue/PR links.
Keep proposed work separate from confirmed defects and completed features.
