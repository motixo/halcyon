# Native Auto diagnosis and remaining looks — DEVELOPMENT only

Status: **PARTIAL DEVELOPMENT result; two additional CI jobs used, no further run.**
Stock Paper and Rosé manual cycles pass. Candidate native Auto misses live clock
steps and responds to timezone changes. Untouched baseline comparison is blocked
by a harness assertion before login; Islands was gated off. Therefore an identical
untouched COSMIC failure is **not established**, and native Auto is not qualified.
This report does not qualify a release.

## Scope and immutable identities

Only infrastructure branch `ci/qualification-probe-021` is changed. First dispatch
`39986644245` attempted one standard Ubuntu 24.04 stock-host job. It failed before
any VM launch because the wrapper referenced three
pinned-source files that had not been committed. Corrected second/final dispatch
`39987753102` at `db6d3c44ce940eca55675ed6a8dff5fab68ad994` scheduled serial untouched stock, candidate stock, and gated candidate Islands
in one standard runner job. Only the first two overlays were actually booted. The
Islands phase is gated by actual matching baseline/candidate stock observations,
unchanged daemon PID per phase, successful night startup/live timezone changes,
and recorded real timedated methods and timezone PropertiesChanged traffic.
No third job is authorized or planned. One offline VM runs at a time.
No already accepted stock Halcyon/Calm/Evening/Focus or public v0.2.0 scenarios
are repeated. Stock remaining looks are Paper/Rosé; Islands remaining looks are
Calm/Evening/Focus/Paper/Rosé. Legacy Repair remains deferred.

Frozen reviewed product commit: `0b688c832bd2d22d047bd63664a8cd7e7b35becb`.
No product build occurs in this round. Finalized package build run `39966537804`,
artifact `12507891179`, ZIP SHA256
`2cdea1fd0ba584bc1c0891033764042b07692b370392fc4f4896d4fef7e11f65`.
Candidate Debian SHA256
`47abfa40383fa4422793a50403646fb7f452e812888d9ed8af80378a0d13dad5`,
installed ELF SHA256
`d070ad5d8bf1362fcd063ee31a56700473744f2f03723948a8357961dd49736b`.
Approved released Islands 0.2.0 input (not installed/run this round), Debian SHA256
`1ec258de00228f02d37fa6201bd4ac4050344594cb452ad6d05ada459ea81c5e`,
previously verified ELF SHA256
`f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31`.

Same retained complete COSMIC 1.11 guest run `39963889546`, evidence artifact
`12507125311`, retained artifact `12506756949` (October 30 expiry). ZIP SHA256s
`47c57f02d9d2807cdabadd31b387e0e336a21441cd6acb1b99043fe043660b06`
and `e87c617f179da525d7d06e3e74b862fa87dcefabdbcbcb02f2e7258802a6ccd4`.
Base qcow2 SHA256
`d3ea6c9f93c68ca0013722c4d22d77408f590ba54bf4b929a25faa50eaeb5c98`;
base vars SHA256
`25d24fad9e38e155e239bcdcd4edb058cf033e34903613f662cf44ba19a7d352`.
Signed ISO inputs and complete package/payload manifests are verified on runner.
No base rebuild, networking in the VM, external guest service, host home mount,
public SSH/VNC endpoint, owner token in CI/guest, default branch edit or release.

## Evidence inspected before execution

Original stock failure run `39979600771`, artifact `12514460018`, ZIP SHA256
`5004627e4bede6319970b0007bdd79e4b8deb1e8d6e4755cf2df3482ef2c46f9`
is retained unchanged under `theme/evidence-39979600771/`.
Its `automatic-blocker-journal.txt` records initial `Etc/UTC` geodata miss,
London timezone update at daytime, then a successful real timedated SetTime to
23:00. Auto remained Light through the forty-second observation. That ordering
cannot alone establish whether startup solar scheduling works or live clock
recomputation fails. Its raw UI/OCR/pixel failures are preserved.

Pinned settings-daemon source `655b14fb9572dd7f7231ec6c343c63984e99741a`,
retained `theme/upstream-daemon-{theme,time,location}-655b14f.rs` (primary source:
[time.rs](https://github.com/pop-os/cosmic-settings-daemon/blob/655b14fb9572dd7f7231ec6c343c63984e99741a/src/time.rs),
[theme.rs](https://github.com/pop-os/cosmic-settings-daemon/blob/655b14fb9572dd7f7231ec6c343c63984e99741a/src/theme.rs),
[location.rs](https://github.com/pop-os/cosmic-settings-daemon/blob/655b14fb9572dd7f7231ec6c343c63984e99741a/src/location.rs)):

- `location.rs`: initial timezone event; `/etc/localtime` polling; embedded
  largest-city coordinates for the selected timezone.
- `theme.rs`: wall-clock solar event times converted to monotonic `Instant`
  deadlines. Auto enable reads the current stored schedule. Location events and
  `TimeChange` reconstruct it; a manual override can defer mode changes.
- `time.rs`: `TimeChange::WallClockChange` is driven by actual timedated
  `PropertiesChanged`; resume is logind `PrepareForSleep(false)`. Calling
  SetTime successfully is not itself evidence that this subscribed signal fired.

Installed daemon package `1.11.0~1792526191~24.04~655b14f`, ELF SHA256
`0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423`.
Portal package `1.11.0~1792525517~24.04~f448ddf`, ELF SHA256
`73d5864b661f5e4d5995e788869b26a9b9091edf723277bae41f9cb38916d56b`.

## Method and preparation correction

Infra first attempt `30941ff2dbcc833c72683e4728451512e9f2dcad`; actual guest execution
`db6d3c44ce940eca55675ed6a8dff5fab68ad994`.
Preparation inventories existing prerequisites and installs only missing tools.
APT uses authenticated runner Ubuntu sources only, zero retries, 15-second
transport timeouts, 180-second whole update and 300-second whole install bounds.
Unrelated vendor repositories are excluded. Preparation logs are retained from
its first operation. The working retained-guest range transfer recipe is reused.

Intended baseline sequence: before native login, start actual system DBus
monitor, assert Halcyon absence, disable NTP, set London/23:00; then enable the
real native UI Auto toggle and observe startup-night state. The baseline actually
stopped at the absence assertion before clock setup or login. No baseline Auto
observation exists. The candidate completed this clock setup before login and,
after requested look cycles, the real Auto UI toggle. Without restarting its
settings daemon: SetTime noon; SetTimezone Paris; SetTime night; SetTimezone London.
Each retains actual DBus traffic, owner changes, service state, clocks, process
identities, config snapshots, portal samples and stopped-UI screenshots. No native
mode-key writes, synthetic mode events, daemon restart counted as a live transition
or Halcyon workaround. These are timedated stimuli; no natural solar boundary is claimed.

Candidate completed the intended Auto sequence after only requested remaining look
cycles. Each look requires a screenshot-calibrated actual pointer selection,
persisted family/palette/native-mode provenance, native Dark→Light→Dark controls,
portal checks, stopped settings/widgets, real bar/applet screenshots and stable
native process identities. Outside pointer clicks visibly dismiss popup grabs.
The prior one-pixel grid shift is handled by a uniform translation only when all
47 retained probes match, never by accepting a different screen. Eight local
retained-image tests pass in `theme/auto-diagnosis/local-calibration-verification.txt`.

## Initial infrastructure failure (preserved)

Run `39986644245`, execution commit `30941ff2dbcc833c72683e4728451512e9f2dcad`,
job `120527436713`: preparation succeeded 16:25:45–16:30:32 UTC. Prerequisites
were ready by 16:26:15 (31 seconds); retained 15GB ZIP verified/extracted by
16:29:27. The VM wrapper failed immediately at 16:30:32 because three referenced
pinned source files were local-only. No VM was launched, no scenario ran.
Artifact `12516625530`, 261,022 bytes, ZIP SHA256
`3d427a07ea83862964219fda88330944fb574ea32551979f00a213c61b50a619`,
expiry 2026-10-30 16:30:33 UTC. Its original verified ZIP, extracted preparation
logs and package/input provenance remain under `theme/evidence-39986644245/`;
Actions log under `theme/auto-diagnosis/actions-39986644245.log`.

Correction commit `c37c8b0` includes those immutable reference files and the
recorded-evidence gate for serial Islands; `db6d3c4` retains all control sources
and checks QEMU termination before proceeding. Local preflight verifies all 13
harness/source/calibration inputs are committed. The job cap is still two total
additional CI jobs including this pre-VM failure. The second job reuses the same
finalized packages and authenticated base, without another build.

## Known serial Islands wrapper defect (not hidden or retried)

While the second/final job was preparing, another controller read caught the
serial phase's media condition: `if [[ $phase == candidate ]]` attaches finalized
read-only media only to the stock candidate phase; the new phase name `islands`
also passes candidate lineage to Python but receives no CD. This is an
infrastructure defect introduced when combining phases under the two-job cap.
This defect was discovered while the dispatched job was running; that immutable
job could not be patched. **It was not reached:** the failed untouched baseline
made the stock evidence gate false, so no Islands VM was launched and no Islands
mount failure occurred. Correction `c4911db` uses `run_lineage == candidate` so
all actual candidate phases receive the authenticated read-only package media.
This correction has not been executed in a VM.

## Actual second-job results and evidence

Run [39987753102](https://github.com/profilaz3/halcyon/actions/runs/39987753102),
job `120531261923`, execution head `db6d3c44ce940eca55675ed6a8dff5fab68ad994`.
Preparation 16:33:37–16:39:38 UTC; prerequisites ready in 24 seconds. Retained
15GB artifact verified/extracted 16:37:43; finalized package artifact 16:39:25.
VM step 16:39:38–16:48:46, failed overall because the baseline was incomplete.
Package build job skipped. Actual overlays ran serially: untouched boot at
16:39:38, candidate boot at 16:40:17; no concurrent VM, no Islands boot.

Artifact **12516872732**, `theme-auto-stock-39987753102`, 39,118,299 bytes,
ZIP SHA256 `61851430b7c466719b26c985dda09edbf52abfceebcef050736c9e9e4afa82e0`,
expiry **2026-10-30 16:48:46 UTC**. Authenticated original ZIP and 339 extracted/
derived evidence files are retained under
`theme/evidence-39987753102/theme-auto-stock-39987753102/` (ZIP alongside).
`theme/auto-diagnosis/evidence-39987753102-SHA256SUMS.txt` hashes all 339 files;
manifest SHA256 `408398e359231410c87817e9c9c820cb7f29c411155db50fdc00c7a738b431b3`.
Read-only candidate media SHA256
`c278de5c9e6f3d0cedf7dd18ac68bed69fc778b969ffff01f2e14040deebf173`.
Only small evidence was downloaded locally; retained guest/ISO/closure stayed on CI.

`controller-provenance-review.json` confirms eight recorded harness hashes match
the exact executed commit, all 33 installed package payload files pass independent
SHA256 checks, and all 13 stopped-UI frames contain the exact pinned daemon/portal
ELFs. Actual Halcyon running PIDs 2795 (Paper), 6313 (Rosé) both map to the exact
finalized `d070ad5…` ELF. Panel PID 1590 and ELF
`e66271e6114ac21754f2969c910a36dfea3b958f949d74d9e7e22595caf0f937` are preserved.

### Untouched baseline: BLOCKED by infrastructure, not a native test failure

The exact failing command was:

```sh
test ! -e /usr/bin/halcyon && test ! -e /home/probe/.config/halcyon && ! pgrep -u 1000 -f /usr/bin/cosmic-settings-daemon && test ! -e /dev/sr0
```

QEMU supplied an implicit **empty default DVD drive** despite no candidate media
argument. `untouched/fatal-journal.txt` records QEMU DVD-ROM and `/dev/sr0` creation;
`untouched/vm-command.txt` contains no ISO/CD candidate drive. The device-node
existence assertion was wrong. `prelogin-user-processes.txt` shows no probe-user
processes and no Halcyon/Islands packages. `fatal-state.txt` has only three native
fixture config entries, no Halcyon config and no user settings daemon. The retained
`001-fatal.png` is the untouched greeter at 16:40, not a logged-in theme test.

The assertion ran before pre-login clock setup; therefore there is **no baseline
clock/timezone/Auto result and no basis to say untouched COSMIC fails identically**.
`stock-diagnostic-validation.json` correctly reports `validated:false` and
`('untouched', 'incomplete diagnostic')`; `islands-gated.txt` records the skip.
Local correction now records QMP `query-block`, rejects inserted removable media
rather than an empty device node, and prints separate binary/config/user-daemon
absence assertions. Syntax checked only, no third VM/job or claimed rerun.

### Remaining stock looks: PASS within captured scope

| Look | Persisted actual identity | Dark→Light→Dark clock-glyph contrast | Stopped-UI frames | Result |
|---|---|---|---|---|
| Paper | `paper / amber`, `native_mode_palettes=1` | 13.47 / 13.89 / 14.19 | 009 / 012 / 015 | PASS |
| Rosé | `glass / rose`, `native_mode_palettes=1` | 6.76 / 8.69 / 6.76 | 025 / 028 / 031 | PASS |

Both actual native controls and portal/native-mode assertions pass. The source
setup grid was selected through calibrated pointer input, followed by actual
persisted identities; no hidden product or native-mode writes were used. All six
full mode screenshots were independently viewed. The clock-label ROI
`[590,17,691,30]` is entirely inside the visible bar and contains real date/time
letter strokes, not wallpaper. Independent observations retain 89–128 near-role
glyph pixels for these six frames, observed surface/ink RGB and calculated contrast
in `candidate/independent-full-clock-review.json`.

Paper surfaces change `(45,40,32)` Dark ↔ `(250,242,232)` Light, with light ink ↔
dark ink. Rosé surfaces change `(76,42,46)` Dark ↔ `(212,188,191)` Light, with
`(221,181,181)` light ink ↔ `(45,33,33)` dark ink. Rosé has actual native panel
opacity `0.72`; the Light screenshot's composited background differs from the
uncomposited native base `(255,239,239)`. The original simple RGB background
heuristic remains **false** for frame 028; it has not been rewritten into a pass.
Actual glyph contrast 8.69 and full-screen observation support readable text.

Real native calendar popups were opened and visibly dismissed after each final
Dark state (`016`, `032`; dismissal `017`, `033`). Independent headline crops
inside the actual popup show Paper 196 glyph pixels / 14.19 contrast, Rosé 235 /
7.51 (`independent-popup-glyph-review.json`). Popup surfaces in Light were not
captured; native time-applet bar text and bar/dock surfaces were captured in both
modes. No broader claim of Light-popup coverage is made.

### Candidate Auto: initial night state and timezone recomputation work; clock steps fail

Auto was enabled through the real native Settings control after London/23:00 was
set **before user login**. This avoids the earlier Etc/UTC/daytime startup sequence.
The initial Dark state was already selected by the final manual look cycle, so
its matching night state is not counted as a live automatic transition or proof
of a natural solar boundary. Actual persisted Auto remains true throughout;
product identity stays `glass/rose/native_mode_palettes=1`.

| Stimulus / actual time | Native state / portal | Verdict |
|---|---|---|
| Pre-login London 23:00; later native UI Auto enable | `is_dark=true`, `u 1` | Expected initial night state; no live transition required |
| Real live SetTime to London 12:00 | stays `true`, `u 1` for 41.94 monotonic seconds | FAIL: expected Light |
| Real live SetTimezone Paris, actual 13:01 | `false`, `u 2` | PASS: live timezone/location recomputation |
| Real live SetTime to Paris 23:00 | stays `false`, `u 2` for 42.12 seconds | FAIL: expected Dark |
| Real live SetTimezone London, actual 22:01 | `true`, `u 1` | PASS: live timezone/location recomputation |

Actual settings-daemon **PID 1576** and portal **PID 1935**/their exact ELFs remain
constant across all five observations. No daemon restart is used or counted.
`*-clock-services.txt`, `*-before-*`, raw snapshots, portal samples, guest uptime,
`auto-monotonic-journal.txt` and normal journal preserve time and process evidence.

`final-dbus-traffic.txt` and `independent-dbus-review.json` retain 34 actual DBus
messages: four PropertiesChanged signals (NTP false, London timezone before
login, Paris timezone, London timezone), **none for any SetTime**. Real SetTime
method calls are captured. Service owners change normally on timedated idle
exit/reactivation (`:1.48`, `:1.154`, `:1.163`, `:1.174`); after daytime wait,
checkpoint records active PID 9869/owner `:1.163`; after night wait PID 10331/owner
`:1.174`. The desktop daemon/portal stay alive. This distinguishes timedated's
service lifecycle from a settings-daemon restart.

The live-noon screenshot `039` still displays 11:05 PM while actual `date` and
timedated show London 12:00:50. This is an additional observed stale native clock
label after the backward clock step, not evidence that SetTime failed. Frame
`041` displays 11:00 PM after the forward step but remains Light. Clock/date
labels alone are therefore insufficient evidence of actual system time here.

### Suspected native boundary and upstream reproduction scope

Pinned daemon `time.rs` subscribes to timedated PropertiesChanged for
WallClockChange; `theme.rs` otherwise keeps monotonic solar deadlines, and
`location.rs` polls timezone changes. The actual trace has timezone signals but
no SetTime signal. The [official systemd 255 timedated API](https://www.freedesktop.org/software/systemd/man/255/org.freedesktop.timedate1.html)
explicitly does not offer clock-step notification on this interface and directs
local observers to timerfd with CLOCK_REALTIME/TFD_TIMER_CANCEL_ON_SET. This is a
strong, source-supported suspected boundary in native clock-change detection;
service-owner changes are recorded, not silently assumed to be the cause.
Installed systemd is
`255.4-1ubuntu8.15pop0~1778766128~24.04~85b5073`, DBus `1.14.10-4ubuntu4.1`,
broker `35-2ubuntu0.1`. This upstream API reference is distinguished from the
exact downstream package and actual runtime evidence.

Minimal native reproduction is retained at
`theme/auto-diagnosis/upstream-reproduction.md`; actual sequence is observed in
the candidate session only. It is a **proposed untouched-native reproduction**,
not a completed upstream-only reproduction. No Halcyon workaround/product patch,
manual mode-key injection, artificial theme notification, natural-transition
pass, daemon-restart pass or external upstream issue submission is claimed.

## Results and exact remaining gates

- Stock Paper/Rosé manual scenarios are complete and pass the captured scope.
  Together with previously accepted stock Halcyon/Calm/Evening/Focus, no accepted
  case was rerun unchanged in this round.
- Untouched native Auto comparison remains blocked by the corrected-but-unrun
  empty-drive assertion. Candidate evidence strongly supports the native
  PropertiesChanged/clock-step boundary, but does not prove identical behavior
  without Halcyon.
- Islands Calm/Evening/Focus/Paper/Rosé and native Auto are **NOT RUN** in this
  round because the prerequisite baseline gate failed. The latent serial-media
  bug was also corrected locally, never treated as executed qualification.
- Natural sunrise/sunset and suspend/resume switching were not exercised. Real
  timedated timezone recomputation passes on candidate stock; live clock-step
  recomputation fails. These are separate results.
- Repair remains deferred until a stable integrated reviewed package with N1/R8
  is available. No newer product commit was built or run. Final 0.2.1 still needs
  exact finalized package/payload requalification; these inputs remain the frozen
  development snapshot, including its previously noted packaging license warning.

Two CI jobs total were dispatched, both standard public runners, each under
120 minutes. No extra retry/third job. First failure, baseline failure, gate result,
raw screenshots/config/process/signal logs and original ZIPs are preserved.
All controller writes/commits are inside this infra worktree. No command issued
here could delete sibling `.superpowers` or `docs/superpowers`; no git-clean or
cleanup outside explicit disposable runner/guest staging occurred. The product
worktree, its mutable outputs and default branch were untouched.

## Infrastructure commit handoff

- `30941ff2dbcc833c72683e4728451512e9f2dcad`: scoped dispatcher, bounded runner
  preparation, baseline/candidate controls; first pre-VM failed execution.
- `c37c8b0`: commit missing pinned source copies, gate serial Islands by stock evidence.
- `db6d3c44ce940eca55675ed6a8dff5fab68ad994`: exact executed second-job harness.
- `c4911db`: later serial-media correction and independent pixel/DBus readers;
  VM correction unexecuted, controller evidence readers actually used.
- `ae3ab3108fbd0ec102cececefe2a70f8901562d9`: empty-DVD assertion correction,
  independent provenance reader and honest upstream reproduction scope. Baseline
  correction syntax checked but not VM executed. No new dispatch after the cap.

All are infrastructure-only commits on `ci/qualification-probe-021`. Final
report/evidence commit follows these. Local verification is retained in
`theme/auto-diagnosis/final-local-verification.json`; eight retained-image control
calibration tests passed before dispatch, and final Python syntax, shell syntax
and whitespace checks pass. These local checks do not qualify unexecuted scenarios.

No new platform-memory fact outside this repository/evidence qualified.
