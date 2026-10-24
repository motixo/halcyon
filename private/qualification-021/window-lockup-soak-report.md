# Released 0.2.0 native window responsiveness soak

Status: **stopped after one additional runner allocation because initial stock preflight failed. No core canary or hour timer ran. Two additional allocations remain undispatched under the explicit failed-preflight stop condition.** No lockup, stability or resolution claim. This report supersedes no original failure: [the earlier investigation](window-lockup-investigation-report.md) and all its evidence remain intact. The source Arrange cancellation finding is assigned to the parent; no product code is changed here.

## Scope, gates and controls

Up to three additional standard public runner allocations, each at most 100min; one offline VM per job, separate clean overlay, contexts run serially. Stock must first prove native canary and enter the hour before another context is dispatched. If its initial preflight fails, stop after that allocation and diagnose retained evidence.

Infra execution commit **5c1f9a73c0623dabfdea5c8bcf730c2ae9e54ae1**. Only run **40015446836**, stock; job **120626227052**, standard ubuntu-24.04 runner, **2026-10-23 19:57:25–20:03:25 UTC (6min total)**. Main-only and Islands were not dispatched.

Native preflight requires a Type=forking editor unit with active MainPID, real mapped toplevel geometry, native pointer/keyboard focus, select/type/save, independent exact file readback, visible before/after frames, normal close and reopen, another real save, and actual editor ELF/package identity. Only after those gates does the hour timer start. Controller creates the empty disposable fixture once; it never replaces editor contents or fabricates a save. A five-second bounded readback wait permits asynchronous disk completion without retyping or retries.

The hour targets 3660sec and records native input at five-minute intervals plus a final actual save/close/reopen. Bounded minute resource samples retain process PIDs/start times, CPU ticks, RSS/VM/threads/fds. Full native process identities, compositor/kernel journals and renderer maps are captured. Optional move/resize/maximize/workspace/Files/popup/edge-menu gestures have separate assertions and failures; a coverage failure does not erase core checkpoints. Actual core input/close/reopen failure stops and preserves state without automatically attributing a product bug. Optional RTC suspend/resume occurs only after core completion and cannot overturn its result.

Local preflight: three wire/media/syntax checks and nine retained-image calibration checks passed. An isolated synthetic fork process, without host service or desktop interaction, proved the child outlives its successfully exiting parent; this is not substituted for native systemd proof. The actual guest unit properties, MainPID/Result/state/journal and native ELF are required. No clock changes, mode injection or synthetic event claims.

## Immutable inputs

Same retained complete 1.11 guest run **39963889546**, evidence **12507125311**, retained **12506756949**; no base rebuild. Base qcow2 **d3ea6c9f93c68ca0013722c4d22d77408f590ba54bf4b929a25faa50eaeb5c98**, vars **25d24fad9e38e155e239bcdcd4edb058cf033e34903613f662cf44ba19a7d352**. Signed input manifest, firmware and retained artifact digests are checked by the existing restore recipe. Original artifacts expire October 30 and remain preserved.

Existing finalized package bundle run **39966537804**, artifact **12507891179**, ZIP **2cdea1fd0ba584bc1c0891033764042b07692b370392fc4f4896d4fef7e11f65**. Only actual public 0.2.0 main and matching released Islands may be installed, with full installed payload validation:

| Public input | Package SHA256 | Expected actual ELF SHA256 |
|---|---|---|
| Main | af1ff76f654f7eaefad6b788235304fff6cc868bcc14011b3b073a99db4af847 | 0abb0f393b14ba0a5ddf791a719c391dfa9a8a04e39196a1caddc8ff7028a01f |
| Islands | 1ec258de00228f02d37fa6201bd4ac4050344594cb452ad6d05ada459ea81c5e | f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31 |

Stock has no installed Halcyon/config and no inserted candidate media. Main and Islands use the same actual public starter look selected through the visually calibrated setup UI; persisted family/palette confirms selection. This intentional product theme treatment differs from untouched stock; native apps/input sequence, pinned guest, VM topology/resources/renderer and offline conditions otherwise match. Native configuration snapshots and actual profile identity are retained; no hidden configuration writes set the chosen appearance.

Offline throughout app execution (`-nic none`), private Unix QMP/QGA/VNC, PCI-pinned topology and read-only candidate media. No owner token enters guest; controller selects profilaz3 explicitly. CI token read-only, checkout persist-credentials false. No host installation/desktop, hosthome mounts, product/default branch edits/builds, PR, public message/issue, release, paid runner, subagent or sibling cleanup.

## Results and evidence

| Context / gate | Actual result | Duration / scope |
|---|---|---|
| Stock service lifetime | **Pass for launch/lifetime** | `Type=forking`, MainPID **2655**, `Result=success`, active/running. Unit journal and subsequent process/ELF evidence agree. |
| Stock visible native editor | **Observed** | Independent retained screenshot shows the editor and empty fixture; foreign toplevel reports `com.system76.CosmicEdit`, title `lockup-input.txt - COSMIC Text Editor`. |
| Stock native geometry/state observer | **Harness failure** | Foreign title/app ID present, extension geometry/state absent in repeated short-lived snapshots. This stopped preflight. |
| Stock focus/type/save/readback/close/reopen | **Not reached** | No typed canary, no controller substitute save, no completed core assertion. Visible focus decoration alone is not counted as proof. |
| Stock one-hour input/resource/idle | **Not started** | **0sec core-hour exposure**, zero checkpoints. Only preflight failure resource/process observations; no resource-growth conclusion. |
| Main-only public0.2.0 | **Not dispatched** | Stock gate did not pass. No installed/executed public main assertion in this round. |
| Main+released Islands public0.2.0 | **Not dispatched** | Stock gate did not pass. No installed/executed Islands assertion in this round. |
| Optional gestures / suspend-resume | **Not reached** | No gesture or resume pass. |

**Service correction verified, observer readiness failed.** `editor-unit-0.txt` records an active forking service with MainPID2655 and successful result. The actual native editor ELF is **61f32f20df9c4da800b0df8054cb0b7ec6c65ccd4b9d11679f7f6c7303dc1149**, package **1.11.0~1792525544~24.04~4ac1cc8**. The editor is plainly rendered in `001-failure.png`; this resolves the prior missing-window/lifetime mismatch. It does not prove application typing or saving, which was never attempted.

The observer’s last snapshot reports the correct live editor title/app ID but lacks extension state/geometry. The executed harness misleadingly reported “did not map”; the screen proves mapping. Its actual failure is incomplete extension data. It recreated a connection, sent immediate roundtrips and closed; waiting between fresh snapshots did not wait for initial events on the original connection.

The [exact native compositor source](https://raw.githubusercontent.com/pop-os/cosmic-comp/8cb1802caada438bb5cc7670dfabdaca5fad0502/src/wayland/protocols/toplevel_info.rs) registers a requested extension handle in its constructor, then sends state/geometry from `refresh()`. `wl_display.sync` completion therefore does not establish receipt of those later extension properties. Source SHA256 **709d3889af0fc68ec919dae4f31d1d9c8926bba598878d410adfa355599b7d96**. This is a source-backed observer-readiness defect, not an observed application or compositor lockup. No raw wire trace was captured to time the native delayed events, and the corrected observer remains unverified in the guest.

**Local correction after the failed run:** keep the same read-only observer connection open and use a bounded three-second event wait for initial extension state/geometry; report completeness explicitly and detect EOF. Added a socketpair test that sends extension state/geometry after a delay and a timeout test for missing properties. **Five local tests pass**; no new native dispatch follows. The editor startup error now names incomplete observer state instead of falsely saying the editor did not map. A secondary collect-stage connection-reset error occurred during guest poweroff; local collection now syncs and leaves shutdown to the owned-VM controller, avoiding that QGA race. These local corrections are not represented as native passes.

[Run 40015446836](https://github.com/profilaz3/halcyon/actions/runs/40015446836) evidence artifact **12528826459**, `lockup-session-stock-40015446836`, **3,535,216 bytes**, ZIP SHA256 **79f0c390a2d9d94c8f8375691adbb2030d6a592ebc81077b2b9a9f75de087299**, expiry **2026-10-30 20:03:20 UTC**. ZIP downloaded locally and hash-verified before safe extraction; only small evidence downloaded. All original guest/input artifacts remain untouched.

| Key retained file | SHA256 |
|---|---|
| `001-failure.png` | 0635c0558469dd7de7ac058767244568059e46771c189fec78ce87c1ea564ffa |
| `editor-unit-0.txt` | 75fe98fbfccac2070ccd0db47dc37934537474eeff068c6bc14ce7ee4ef07548 |
| `editor-map-0.txt` | e618b9cb7d13ab27dca14bcd9196d92adddb72f3ba00993faad0510257650779 |
| `failure-identity.txt` | f88b749f02ee9c3b0940cb7e6fe24f810e66a8ecf31d406484dd6ac43a23ea67 |

Complete local file hashes are in [soak-evidence-manifest.json](lockup/soak-evidence-manifest.json), machine results in [soak-results.json](lockup/soak-results.json), and full retained files under `lockup/evidence-40015446836/lockup-session-stock-40015446836/`. Preflight artifact **12528127801** and stale progress artifacts **12528521912 / 12527693772 / 12528502087** all contain the same failure state, ZIP digest **dbd4e2f4ae017578310da49e9a25860f260a917f29e68218629ccf253b6895d8**. These minute-labelled upload steps ran after failure; they do **not** mean 20/40/61min elapsed coverage. `finished=true` in results records collection, not completion of the canary/hour.

Actual native process provenance: compositor PID1454 ELF **2399b768cc6a4599a6495d0865cf4dc6807cfee2c6e46b3d7466a800f903be5e**; stock panel PID1524 ELF **e66271e6114ac21754f2969c910a36dfea3b958f949d74d9e7e22595caf0f937**; settings daemon PID1500 ELF **0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423**. OS/kernel/Mesa/native package versions, renderer maps, resource sample, full journal and kernel log are retained. Editor startup has EGL/ZINK fallback warnings while the frame renders correctly; those warnings alone are not evidence of the hour-long symptom. There is no RX590/amdgpu test or long-term resource evidence.

## Remaining execution gate

Per the explicit instruction to stop after an initial preflight failure, no second or third runner was spent. First requalify the corrected same-connection observer against this visible native editor, then obtain real focus/type/save/readback/close/reopen proof before starting any hour. All three full-hour contexts remain required. The source Arrange cancellation finding remains with the parent; no product workaround was introduced. The user tracker remains open.

## Interpretation limits

Virtio/software-rendered VM results cannot establish RX590 correctness or resolve the reporter's hardware-specific incident. Exact reporter Halcyon/COSMIC/OS versions, Islands status, material, affected apps and input scope, preceding actions/recovery behavior, kernel/Mesa/amdgpu versions, display layout/scaling and relevant redacted incident-time logs remain missing. The tracker cannot be closed from a software VM pass alone. Publication hold remains; any future integrated package requires its own qualification.
