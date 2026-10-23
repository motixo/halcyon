# Untouched Auto baseline, natural sunrise, and missing Islands looks

**COMPLETE for the requested DEVELOPMENT scope.** Islands five-look manual
cycles pass; natural sunrise passes on untouched stock, candidate stock and
candidate Islands. Untouched COSMIC reproduces the retained candidate's clock-step
failure, closing that comparison as a pinned native-stack limitation. This is
not final 0.2.1 package/release qualification.
Only infra worktree `halcyon-021-runner-probe`, branch `ci/qualification-probe-021`.
No product/native package is built in this round. Stock manual cases already
accepted by the parent are not rerun. Repair awaits a stable integrated package.

## Fixed inputs and scope

Reviewed frozen product `0b688c832bd2d22d047bd63664a8cd7e7b35becb`, finalized
package build run `39966537804`, artifact `12507891179`, artifact ZIP SHA256
`2cdea1fd0ba584bc1c0891033764042b07692b370392fc4f4896d4fef7e11f65`.
Candidate Debian SHA256
`47abfa40383fa4422793a50403646fb7f452e812888d9ed8af80378a0d13dad5`;
installed/running Halcyon ELF SHA256
`d070ad5d8bf1362fcd063ee31a56700473744f2f03723948a8357961dd49736b`.
Released matching Islands 0.2.0 Debian SHA256
`1ec258de00228f02d37fa6201bd4ac4050344594cb452ad6d05ada459ea81c5e`;
expected running ELF
`f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31`.

Same retained complete 1.11 guest run `39963889546`, evidence artifact
`12507125311` ZIP SHA256
`47c57f02d9d2807cdabadd31b387e0e336a21441cd6acb1b99043fe043660b06`,
retained artifact `12506756949` ZIP SHA256
`e87c617f179da525d7d06e3e74b862fa87dcefabdbcbcb02f2e7258802a6ccd4`.
Base qcow2 SHA256
`d3ea6c9f93c68ca0013722c4d22d77408f590ba54bf4b929a25faa50eaeb5c98`;
base vars `25d24fad9e38e155e239bcdcd4edb058cf033e34903613f662cf44ba19a7d352`.
Signed original ISO, retained firmware, APT closure and all original file hashes
are verified before opening the base. Original artifacts expire October 30.
No base rebuild, mutable product output read, host install, product/default branch
edit, release, paid runner, subagent or upstream issue submission.

Read-only workflow Actions/contents token, checkout persist-credentials false.
Owner auth remains controller-only via the specified profilaz3 gh auth lookup.
No token is sent into a guest. PCI-pinned Q35 overlays, `-nic none`, private
Unix QMP/QGA/VNC sockets, read-only candidate media and no host-home mount.
Only small evidence is downloaded locally; the 15GB guest stays on disposable CI.

## Orchestration and local proof before dispatch

`6fe183b` defines a typed phase/lineage plan and one QEMU argument generator.
`cd5e0e79fd03c890163a9546181f5e697064db56` adds tracked-resource proof and keeps
manual cases independent of solar-source assertions. Before dispatch:

- all 26 referenced harness/source/calculation/preparation/workflow inputs tracked;
- five actual launch specifications inspected: two untouched phases have no
  candidate media; all candidate phase names, including Islands, get the same
  explicit read-only ISO;
- six local wrapper/inserted-media tests pass; eight retained-screen calibration
  tests pass; Python/shell syntax and workflow YAML parse pass;
- generated guest provenance scripts parse without invoking any guest;
- empty default DVD is allowed, while inserted removable media is rejected for
  untouched guests. Candidate QMP identity must match the exact ISO path and ro=true.

Records: `theme/acceptance/tracked-launch-preflight.json`,
`local-wrapper-verification.txt`, `local-verification.json`. No local live desktop
or native VM is driven by those tests. The actual native tests run only on CI.

First job, run `39994497276`, exact infra head `cd5e0e79fd03c890163a9546181f5e697064db56`:
serial clean `untouched-clock`, `untouched-natural`, `candidate-stock-natural`.
No stock manual mode cycle. Candidate Calm is chosen once as the natural-test
profile, through actual UI; its manual cycle is not repeated.

Second/final job `39997696992`, exact infra head
`2a4392b082f7587b2ed67f9347f11200ca39c2a3`: focused corrected
`untouched-clock`, `untouched-natural`, then independent `candidate-islands-manual`
and clean `candidate-islands-natural`. The baseline rerun addresses the observed
six-pixel native UI calibration cause, not an unexplained retry. Passed candidate
stock natural evidence is not repeated. No Auto/baseline result gate controls Islands manual
execution. Errors remain recorded per phase; a failed phase does not suppress
independent useful coverage. One QEMU process at a time, owned-process termination
confirmed before next overlay; each job hard limit 120 minutes. Natural waits
bounded to ten minutes, clock-step polls forty seconds, each exercise thirty minutes.
APT remains bounded (180-second update, 300-second install, zero retry), reusing
installed tools and the already verified retained-artifact range-transfer recipe.

## Natural boundary calculation and method

Native daemon package `1.11.0~1792526191~24.04~655b14f`, ELF
`0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423`;
COSMIC portal package `1.11.0~1792525517~24.04~f448ddf`, ELF
`73d5864b661f5e4d5995e788869b26a9b9091edf723277bae41f9cb38916d56b`.
Pinned daemon source `655b14fb9572dd7f7231ec6c343c63984e99741a` uses embedded
largest-city geodata, `sunrise::SolarDay` and monotonic solar deadlines.

The exact source geodata SHA256 is
`910a6c8b4e16ffa875a0aca47bd9c215ddee19941ab7f0012b6106fffe66b3b9`.
A small isolated **infrastructure calculator**, compiled locally with Rust 1.93
and its own locked dependencies, decodes the native bitcode GeoPosition structure
and calls the native lock's exact `sunrise 2.1.0`/`chrono 0.4.45` implementation.
It builds neither Halcyon nor COSMIC and produces no package or installed program.
The native lock checksums for sunrise, chrono and bitcode 0.6.9 match the calculator
lock exactly; all source/config/output digests are retained in
`theme/acceptance/solar-provenance.json`. Calculator source, lock and output are
tracked and copied into CI evidence. The natural exercise also requires the exact
12KB geodata blob to be found inside the pinned installed daemon ELF, not just
assumed from a timezone label. Manual Islands coverage does not depend on that
solar embedding assertion.

Decoded **Europe/London** is latitude **51.50853**, longitude **−0.12574**.
For 2026-10-23, the exact calculation gives:

| Event | UTC | Epoch |
|---|---|---|
| Sunrise | 06:37:41 | 1792737461 |
| Sunset | 16:51:44 | 1792774304 |

Natural tests set NTP off, timezone London and clock **07:29:41 BST** (eight
minutes before sunrise) **before user login**, then assert the actual epoch.
They do not set time/timezone again. Candidate installation/host setup and actual
Calm selection occur before enabling real UI Auto, in that fresh overlay. Native
Auto begins in Dark before sunrise, without a manual native mode selection.
After UI is stopped, normal time progresses across sunrise. Portal + guest
realtime/monotonic samples, actual native mode/config, before/near/after screenshots
and unchanged daemon/portal/bar identities are required. Early/late transitions,
clock drift/steps or insufficient setup time are captured as failures, never
repaired by another clock step, native mode-key write or restart.

DBus monitor runs before initial setup. A natural phase must contain exactly one
initial SetTime and one initial SetTimezone method, with neither repeated during
observation. No synthetic events, fake mode service, live timezone stimulus,
daemon restart counted as transition or inherited manual-cycle deadline cache.
Each natural test uses its own clean overlay. Startup state and actual Dark→Light
transition are reported separately.

## First job: preserved calibration failure and candidate stock natural pass

Run [39994497276](https://github.com/profilaz3/halcyon/actions/runs/39994497276),
job `120554386192`, exact executed head `cd5e0e79fd03c890163a9546181f5e697064db56`.
VM step 17:26:53–17:39:58 UTC. Package build skipped. Overall execution failed
because both untouched phases stopped before Auto enable, while the candidate
stock natural phase completed successfully. All three fresh overlays were serial.

Artifact **12520326845**, `theme-acceptance-stock-39994497276`, **28,822,378 bytes**,
ZIP SHA256 `150a54a67700687b7aa602a1d091fc0b81a176756355f77d22a19dda1a164b44`,
expiry **2026-10-30 17:39:58 UTC**. Authenticated original ZIP and extracted
screens/state/logs remain under `theme/evidence-39994497276/`. Neither failure
JSON nor any original screenshot was rewritten.

Both untouched guests passed the corrected actual inserted-media/absence checks
and logged into the real native session. Their native Settings preview was at
**y−6** relative to the themed calibration; the bottom interior probes landed
on gray preview framing. The saved error is `Dark image control pattern absent`.
The actual Auto toggler is centered **(978,457)**, compared with themed (978,463).
This is a control-placement assertion failure, not a native Auto verdict.

The correction uses all eight paired top/bottom antialiased edge probes in the
actual Dark and Light preview assets, requires one consistent vertical shift
within ±8px, then checks the original interior patterns and thumb/track pixels
at that same offset. Both retained untouched screens identify exact `[0,-6]`;
previous native stock/themed/Islands controls and wrong-screen rejection remain
valid. **Nine retained-screen tests pass**, plus six launch/media tests. Records
are `theme/acceptance/calibration-correction-verification.txt` and
`final-islands-tracked-launch-preflight.json`. Before the final dispatch all 26 referenced files were rechecked, and all seven
launch specifications across the two host plans passed media/lineage assertions.
This changes UI calibration only;
no expected native palette values or recorded app outcomes are edited.

Candidate stock natural sunrise: actual Calm selection persisted `soft/teal`,
`native_mode_palettes=1`. Exact geodata found inside actual pinned daemon ELF at
**offset 456099**, 11,958 bytes, same SHA256 as the calculated source. Before and
near-boundary frames `011`/`012` are Dark; frame `013` is Light. Actual first
sample 06:31:19.793 UTC is Dark; first observed Light is **06:37:44.638 UTC**,
**3.638 seconds after calculated sunrise**, within five-second polling resolution.
Seventy-one Dark samples precede the boundary. Maximum realtime/monotonic drift
is **0.000000741 seconds**. Exactly one initial SetTime and SetTimezone appear
in the entire phase; neither is repeated during the observation.

Native PIDs remain **daemon 1615, portal 1968, panel 1642**, with the exact
pinned ELFs. No daemon restart or mode injection. Settings/widgets are stopped
in all observed frames. Independent inside-bar clock-label pixels show **7.22:1
Dark** and **12.77:1 Light** contrast (131 / 102 near-role glyph pixels), with
actual bar/dock surfaces changing. All before/near/after full screenshots were
independently viewed; the crop is actual clock text inside the bar, not wallpaper.
The wallpaper remains unchanged, which is additional separation from bar ink.
Raw proof: `candidate-stock-natural/natural-transition-proof.json`,
`natural-samples.json`, `native-geodata-embedding.txt`, snapshots, portal assertions,
DBus trace and monotonic journal. Independent review:
`independent-acceptance-review.json`. All 22 copied harness/input hashes match the
exact executed commit and installed candidate payload hashes pass.

## Final job, authenticated evidence and actual execution

Run [39997696992](https://github.com/profilaz3/halcyon/actions/runs/39997696992),
job `120565423495`, exact executed infra
`2a4392b082f7587b2ed67f9347f11200ca39c2a3`. Preparation
17:45:40–17:48:53 UTC; VM step 17:48:53–18:25:41, overall execution **success**.
Package build skipped. Four independent clean overlays, one VM at a time:

| Phase | Actual runner UTC interval | Outcome |
|---|---|---|
| Untouched clock comparison | 17:48:53–17:53:44 | Complete; observed native clock-step failure |
| Untouched natural sunrise | 17:53:44–18:03:05 | PASS |
| Islands five manual looks | 18:03:05–18:16:16 | Five PASS, independent of Auto outcomes |
| Islands natural sunrise | 18:16:16–18:25:41 | PASS |

Artifact **12521924667**, `theme-acceptance-islands-39997696992`,
**116,315,384 bytes**, ZIP SHA256
`2460847c5681fb683c84c8c7c4d1f52bb42be9a595e922d805eb9220125b4d7f`,
expiry **2026-10-30 18:25:41 UTC**. Original verified ZIP and all extracted
screens/config/process/signal/journal evidence remain under
`theme/evidence-39997696992/`. No third job. Original failed snapshots/ZIPs remain.

Read-only ISO SHA256s (the package payloads are identical; ISO authoring timestamps
are independently recorded): first job
`3a86e6e8244e83a26c6d4e6060f4d432d1d9db4e33b1ddf6dab81e519e10b252`,
final job `fd5e3a712c23165e8636dd38c68b649ef73b7cbf39bf898e5db4cf55bdc21165`.
Both packages' approved hashes and all **33 Halcyon + 10 Islands** installed regular
payload files pass checks; actual running product ELFs match the frozen identities.
All **22 copied harness/input hashes per job** match their exact executed commits.
Independent review verifies the pinned daemon/portal/appropriate panel ELF in all
**36 stopped-UI frames** across both jobs.

## Untouched comparison: native clock-step limitation reproduced

Both final untouched phases have no Halcyon or Islands package, binary, process,
config, or inserted candidate media. `untouched-absence.txt`, actual QMP block
records, launch arguments and post-login absence review prove this. An implicit
empty DVD device exists and is correctly allowed. The appearance page and real
Auto toggle are the only native appearance actions in this baseline; no starter
look or manual native Dark/Light choice is applied.

Final untouched daemon **PID 1619**, portal **PID 1983** remain unchanged through
the entire clock diagnostic, with the pinned ELFs. The real UI Auto switch persists
true. These are observed results, not natural-transition coverage:

| Actual timedated stimulus | Untouched effective mode | Retained candidate effective mode | Result |
|---|---|---|---|
| London 23:00 configured before login; UI Auto enable | Dark | Dark | Initial night matches; not a live transition |
| Live SetTime London 12:00 | Dark for 41.942 seconds | Dark for 41.940 seconds | FAIL: expected Light, identical effective failure |
| Live SetTimezone Paris, actual daytime 13:01 | Light | Light | Live timezone/location recomputation works |
| Live SetTime Paris 23:00 | Light for 41.942 seconds | Light for 42.124 seconds | FAIL: expected Dark, identical effective failure |
| Live SetTimezone London, actual night 22:01 | Dark | Dark | Live timezone/location recomputation works |

Retained candidate comparison is run **39987753102**, artifact **12516872732**,
ZIP SHA256 `61851430b7c466719b26c985dda09edbf52abfceebcef050736c9e9e4afa82e0`.
The exact clock-protocol function AST is unchanged
(`clock-protocol-equivalence.json`). No candidate clock sequence or accepted
stock manual case was replayed in this round.

Raw key representation differs in the first two rows: untouched native Dark uses
its default and has no `is_dark` override; the prior themed/manual candidate has
an explicit `true`. Actual portal/UI modes and native keys are consistent in both.
The raw null/true values are preserved, not rewritten. Independent effective-mode
comparison is in `clock-baseline-comparison.json`; `same_observed_effective_pattern`
and `native_limitation_confirmed_for_pinned_guest` are true.

Actual DBus evidence has **30 messages**, **four PropertiesChanged signals**:
NTP false, London timezone setup, Paris timezone, London timezone. Every real
SetTime method is recorded; **none causes a clock/time property-change signal**.
Normal timedated service owner/PID changes are retained, separately from the
stable desktop daemon: at initial night PID 1127 / `:1.49`; after noon and Paris,
PID 3249 / `:1.133`; after night and London, PID 3712 / `:1.145`. Each checkpoint
records active state, invocation ID, actual `date`, timedated time, guest uptime,
native files and portal response. The desktop clock label is not treated as the
time authority: the night failure frame still displays the prior daytime label,
while actual system time is 23:00:50.389 CEST. Full failure screenshots were viewed.

The boundary is native `src/time.rs` subscribing to timedated PropertiesChanged
for WallClockChange, while `src/theme.rs` holds monotonic solar deadlines. Native
location polling correctly recomputes when timezone changes. The
[systemd 255 API](https://www.freedesktop.org/software/systemd/man/255/org.freedesktop.timedate1.html)
does not provide clock-step notification there; its documented local alternative
is timerfd. Runtime trace plus the untouched reproduction establish the limitation
in this exact pinned cohort, rather than a Halcyon-only effect. Installed systemd
is `255.4-1ubuntu8.15pop0~1778766128~24.04~85b5073`, DBus `1.14.10-4ubuntu4.1`.
Source: [time.rs](https://github.com/pop-os/cosmic-settings-daemon/blob/655b14fb9572dd7f7231ec6c343c63984e99741a/src/time.rs),
[theme.rs](https://github.com/pop-os/cosmic-settings-daemon/blob/655b14fb9572dd7f7231ec6c343c63984e99741a/src/theme.rs),
[location.rs](https://github.com/pop-os/cosmic-settings-daemon/blob/655b14fb9572dd7f7231ec6c343c63984e99741a/src/location.rs).

Minimal confirmed native reproduction is retained in
`theme/acceptance/upstream-clock-reproduction.md`. No product workaround,
upstream patch, external issue submission or injected mode event. This result
must not be generalized to every COSMIC version or confused with ordinary solar
switching, which passes separately below.

## Natural Auto: genuine solar Dark→Light transitions pass in all three contexts

Calculated sunrise is **06:37:41 UTC / 07:37:41 BST**, using actual native embedded
London geodata and the pinned native solar algorithm. After initial pre-login
clock/timezone setup, time progresses normally. These are actual transitions,
not matching startup states or daemon restarts:

| Context | Last sampled Dark UTC | First sampled Light UTC | First Light vs sunrise | Stable daemon / portal / panel PIDs | Clock-glyph contrast Dark / Light |
|---|---|---|---|---|---|
| Untouched stock | 06:37:36.748 | 06:37:42.061 | +1.061 s | 1623 / 1980 / 1649 | 13.93 / 13.02 |
| Frozen candidate, stock Calm | 06:37:39.323 | 06:37:44.638 | +3.638 s | 1615 / 1968 / 1642 | 7.22 / 12.77 |
| Frozen candidate, released Islands Calm | 06:37:38.627 | 06:37:43.939 | +2.939 s | 3355 / 3733 / 3595 | 7.22 / 12.77 |

Times bound the observed transition at approximately five-second polling
resolution; they are not precise scheduler-latency measurements. Untouched has
76 Dark samples before the boundary, stock candidate 71, Islands candidate 60.
All three have exactly **one initial SetTime and one initial SetTimezone** in the
entire phase, and no subsequent clock/timezone stimulus. Maximum computed
realtime-minus-monotonic drift is below one microsecond in every sampled window.
Native `auto_switch=true` persists and actual native mode/portal switch to Light.
Settings/widgets are stopped, with actual daemon/portal/bar ELF identities unchanged.
No direct mode-key writes, synthetic signals or restart is counted as a transition.

Released Islands setup requires an initial install/enable/relogin before observation;
this is recorded preparation. The clock was set before the first login and is not
stepped again. The actual observation starts after setup and UI Auto enable, with
one stable final desktop daemon through the boundary. Manual tests use a separate
clean overlay and do not contaminate this window with overrides/deadlines.

The actual released Islands executable runs from
`/home/probe/.local/share/cosmic-desktop-cards/companion/cosmic-panel-islands`;
its SHA256 is exactly the approved released ELF, not inferred from an installed
path or package label. Candidate Calm remains `soft/teal/native_mode_palettes=1`.
Full before/near/after screenshots on all three contexts were independently viewed.
The unchanged wallpaper and bar/dock surface/ink changes are separately visible.
Proofs: each natural phase's `solar-setup.json`, `native-geodata-embedding.txt`,
`natural-samples.json`, `natural-transition-proof.json`, native snapshots, portal
assertions, actual DBus stream and monotonic journal; independent derived review
`independent-acceptance-review.json` in each artifact root.

## Released Islands missing five manual looks: all PASS

All cases use the same finalized candidate and matching released Islands 0.2.0,
through actual UI selection and native Settings controls. No config injection.
Each profile has persisted `native_mode_palettes=1`, native mode + portal assertions
for Dark→Light→Dark, stopped product/settings UI, and unchanged actual host process.
Native Islands **PID 3596**, ELF `f4dca9…`, remains unchanged across all five cycles.

| Look | Actual persisted family / palette | Actual Dark / Light / Dark glyph contrast | Native stopped-UI frame numbers | Verdict |
|---|---|---|---|---|
| Calm | soft / teal | 7.22 / 12.77 / 7.22 | 009 / 012 / 015 | PASS |
| Evening | soft / lavender | 7.37 / 12.14 / 6.04 | 025 / 028 / 031 | PASS |
| Focus | bare / graphite | 13.87 / 11.69 / 13.87 | 041 / 044 / 047 | PASS |
| Paper | paper / amber | 14.19 / 13.89 / 14.19 | 057 / 060 / 063 | PASS |
| Rosé | glass / rose | 6.70 / 8.74 / 6.70 | 073 / 076 / 079 | PASS |

Actual inside-bar clock-label bounds `[510,17,611,30]` contain native applet letter
strokes, not wallpaper. All 15 bar states were independently viewed at original
pixel scale in `visual-review/islands-all-manual-bars.png`, a labeled, unscaled
crop/append of original screenshots; originals are unchanged. Native surfaces
and ink invert visibly with mode, with 71–129 near-role glyph pixels in the mode
observations. Contrast varies with glyph antialiasing/composition; no expectation
fixture or original heuristic was rewritten to force a pass.

Real calendar popups are opened and visibly dismissed by outside clicks for each
look. All five full Dark popup screens were independently viewed. Headline crops
inside actual native popup surfaces show respectively **7.40 / 7.39 / 14.06 /
14.19 / 7.51** contrast (196–240 near-role ink pixels), recorded in
`candidate-islands-manual/independent-popup-glyph-review.json`. Calendar Light
popup surfaces were not captured; native clock-applet bar text, bar/dock surfaces,
and native mode are observed in both modes. This is the same captured manual
scope as the already accepted stock cases, without claiming unobserved popup states.

## Remaining gates and preservation

- The requested missing manual and natural-Auto development evidence is complete.
  No accepted stock manual case was rerun. Natural uses a single Calm selection
  in clean overlays and genuine Dark→Light progression; it is a separate scenario.
- Live manual clock-step recomputation fails identically without Halcyon in this
  pinned native cohort. Treat it as a recorded upstream scope limitation, not a
  product workaround target or a passing clock-step test.
- Natural sunset/Light→Dark and suspend/resume were not exercised; no broader
  24-hour or resume coverage is claimed.
- Legacy Repair remains deferred to the stable integrated reviewed N1/R8 package.
  No old-build Repair rerun or newer unreviewed product build occurred.
- Exact final 0.2.1 package/payload requalification is still required after
  integration; these results cover only the frozen development snapshot.

Exactly two additional standard public CI jobs were used, each under 120 minutes,
one offline VM at a time. No product/default edits, release, PR, paid service,
host install/live desktop, subagent, broad cleanup or sibling file changes.
All original failed JSON/screens/ZIPs and retained guest artifacts are preserved.
Only owned disposable VM processes were stopped; no git-clean or outside staging
cleanup. No operation here could delete sibling `.superpowers`/`docs/superpowers`.

## Infra handoff

Executed first head `cd5e0e79fd03c890163a9546181f5e697064db56`; final head
`2a4392b082f7587b2ed67f9347f11200ca39c2a3`. Supporting infra commits:
`6fe183b` (isolated phases/solar source), `cd5e0e7` (tracked launch proof),
`952f61e` (full released payload/provenance review), `2a4392b` (observed native
six-pixel calibration correction, nine positive/negative retained-screen tests,
focused baseline recovery with independent manual execution). Controller reports,
comparisons and hash manifests follow in the final evidence commit. All commits
stay on `ci/qualification-probe-021`; no source/package build is added to CI.

## Final local integrity verification

Original ZIP digests were verified against the Actions artifact API before safe
extraction. All original files and independent derived observations are retained.
File manifests were generated and every retained hash rechecked after visual
review:

- Run `39994497276`: **282 files**, manifest `theme/acceptance/evidence-39994497276-SHA256SUMS.txt`, manifest SHA256 `a79af3dc44356f43c4ddf512c5537b7b470e7b5b0d2634c82ad4f34e9403ae60`.
- Run `39997696992`: **783 files**, manifest `theme/acceptance/evidence-39997696992-SHA256SUMS.txt`, manifest SHA256 `e5d36a6660d16b76313f69f415d2adf02af9ed20caac67050c869d19436cf3cb`.

Final Python syntax, shell syntax, and source/report whitespace checks pass. Raw
Actions logs preserve their original trailing whitespace. No new native test or
job was launched for these controller checks. No new platform-memory fact outside
this repository/evidence qualified.
