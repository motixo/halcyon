# Released 0.2.0 window-lockup investigation

Status: **source audit completed; native soak blocked by infrastructure failures and the three-runner cap is exhausted. No hour-long responsiveness or resource-growth result was obtained.** No root cause of the reporter’s incident, fix, hardware resolution or release acceptance is claimed. No product code was changed or built.

## Source scope and finding

The report in PLANNED_WORK describes windows becoming inaccessible after roughly an hour on Ryzen 7 5800XT / Radeon RX 590. Exact affected versions, Islands status, theme/material, app scope and recovery behavior are unknown. It does not establish a cause.

Audited immutable released tag v0.2.0 at **51e2bfe4c180bafb7b04bd9d3fa0575927aab30e**, main surface/input, blur, animation, event loop, watcher, resource and companion paths. Pinned companion upstream **ca20ce3185b3b4404f429d59d429c40b58a2ddef** archive SHA256 **27854c7d3fc54cb29bc866b8421a078aa6b396ea73e2d88fe4ec0cf4fdd55704** was prepared with released patch only; no compilation. libcosmic **dc11d6a11f601692fb8fd67fdc99775f0a9eb899** dependency was read. Detailed audited boundaries are in [source-audit.md](lockup/source-audit.md); individual source hashes are in [source-provenance.json](lockup/source-provenance.json).

No hour-triggered root cause was established by source inspection. One actionable defensive boundary: Arrange is a full-output Overlay with Exclusive keyboard/unrestricted input; if cancel rollback fails, `finish_arrange(false)` returns without releasing that surface. Escape follows this same path. This is confirmed source control flow, not an observed native rollback failure or proof of the reported symptom. Product consideration: cancellation should release input even if rollback needs recovery. Blocking configuration/store flock operations are another conditional stall boundary, not a demonstrated self-deadlock. No product workaround or patch was made.

Other inspected paths include Notes' intentional exclusive editing focus and pause/hide release; coalesced byte-comparing watcher; finite drawer animation; bounded 300sec/301-sample histories; deduplicated font interning; offloaded guarded metrics; threaded usage requests; Islands background-stamp cache, background drain/unmap, separate rounded input/blur regions, bounded IPC and arrangement leases. These observations exclude no hardware/compositor failure.

## Frozen inputs and security

- Retained complete 1.11 guest run **39963889546**; evidence **12507125311**, retained **12506756949**. Original ZIP digests respectively `47c57f02d9d2807cdabadd31b387e0e336a21441cd6acb1b99043fe043660b06` and `e87c617f179da525d7d06e3e74b862fa87dcefabdbcbcb02f2e7258802a6ccd4`. Original artifacts remain intact, expiring October 30.
- Base qcow2 `d3ea6c9f93c68ca0013722c4d22d77408f590ba54bf4b929a25faa50eaeb5c98`; base vars `25d24fad9e38e155e239bcdcd4edb058cf033e34903613f662cf44ba19a7d352`. Existing signed ISO manifest and pinned retained firmware verified in runner evidence. Base not rebuilt.
- Finalized input bundle run **39966537804**, artifact **12507891179**, ZIP `2cdea1fd0ba584bc1c0891033764042b07692b370392fc4f4896d4fef7e11f65`. Only actual public released packages are installed in this investigation. The bundle also contains an older development package, which is not installed or run.
- Public main package `af1ff76f654f7eaefad6b788235304fff6cc868bcc14011b3b073a99db4af847`; ELF `0abb0f393b14ba0a5ddf791a719c391dfa9a8a04e39196a1caddc8ff7028a01f`.
- Public Islands package `1ec258de00228f02d37fa6201bd4ac4050344594cb452ad6d05ada459ea81c5e`; ELF `f4dca9b2d6fff12c37657b4dfc8c8fb7de669fb17bb409153011df156e54dc31`.
- Fresh disposable overlays; pinned PCI topology; QMP USB keyboard/tablet input; QGA readback; read-only public media, no inserted media on stock; `-nic none` throughout app execution; private Unix sockets only. No host installation, host home mounts, live account data or owner credentials/tokens in the guest. Controller owner authentication selected profilaz3 explicitly, never printed; CI checkout credentials not persisted and GITHUB_TOKEN read-only.
- No product/default branch changes, release, PR, public issue, paid runner, subagent or sibling cleanup. No commands here could delete ignored sibling `.superpowers`/`docs/superpowers` directories. All new source extraction and staging is within this infra private tree or disposable runner temp.

## Runner budget and preserved failure

Infra commit **a9ec93b953d9099bc3dabc631eab58b9caa4b47a** dispatched run **40012098859**. Stock failed before login: consecutive timedated operations returned `Failed to set time: Previous request is not finished, refusing.` No app responsiveness evidence exists from this attempt. Main had started prerequisite preparation when the run was cancelled; Islands remained queued and never allocated a runner. No baseline failure is attributed to COSMIC window responsiveness.

Verified small original artifacts:

| Artifact | ID | ZIP SHA256 | Size |
|---|---:|---|---:|
| lockup-session-stock-40012098859 | 12525984914 | 0e6ccd6ba72c721c5bb3ee0dbdde8424bcf785b62b7603992ed731ee7b594d19 | 1,367,125 |
| lockup-session-main-40012098859 | 12525819906 | 83376bdcde97f97a8c21e21caa612d9718694031743e04cda9a1e4dbec0eb78d | 3,126 |

Progress artifacts 12527630247 / 12527600566 / 12526733292 all preserve the same initial failure; their misleading minute labels do not mean elapsed soak coverage. Their common ZIP hash is `40cee72718056420266046b9db6b4f7df532ec095194287726675e5396864dec`.

Infra commit **72c35c4c81f4cc22b01cef0ee9bd2f109a16ddb5** removes unnecessary clock mutation and dispatches final run **40013237899**, one runner, maximum 100min (native step bounded 83min). This is the third runner allocation; no further jobs will be dispatched in this round. The corrected scope is serial clean stock and main-only short controls, then a full hour on public0.2.0 + matching Islands. This explicitly leaves equal-duration stock/main-only comparison incomplete. All referenced tracked files and media conditions were locally checked before dispatch; local wire/schema/media tests 3/3 and retained screenshot calibration tests 9/9 passed.

## Native results — incomplete, not a product failure

Final [run 40013237899](https://github.com/profilaz3/halcyon/actions/runs/40013237899) ended in the **stock initialization** before any input canary or timed soak. Artifact **12527103197**, `lockup-session-serial-40013237899`, ZIP SHA256 **62148080e6ede507b7541b364ce9cbc4bdcc937d6fbec3d8e6adbd14d46b2c2d**, **3,966,988 bytes**, expiry **2026-10-30 19:45:48 UTC**. The complete small ZIP was downloaded and independently hash-verified; original remains on Actions. Local copies and artifact metadata are in `lockup/evidence-40013237899/`. First-run ZIPs/metadata are likewise preserved under `lockup/evidence-40012098859/`.

| Requested observation | Result | Evidence / limit |
|---|---|---|
| Untouched stock native login | Observed | Real greeter keyboard login, full native desktop screenshot and actual session/compositor/panel/daemon/portal PIDs/ELFs. Public main binary/config absent; no inserted media. |
| Stock editor open/input/save/window gestures | **Harness blocked, not pass** | Unit launch returned success, native observer returned zero windows, screenshot shows no editor, process capture has no cosmic-edit. It stopped before typing or saving. |
| Public 0.2.0 main without Islands | **Not run** | First run’s main job cancelled during runner preparation; final serial run never reached this overlay. Frozen package hashes above describe authenticated inputs, not actual execution in this round. |
| Public 0.2.0 with matching Islands | **Not run** | First run’s queued job cancelled before runner allocation; final serial run never reached this overlay. No chosen look or Islands active-layout assertion was obtained in this round. |
| One-hour input/idle/resource growth | **Not run** | No completed checkpoint, no hour exposure, one stock resource sample only. No trend, leak, stability or absence-of-lockup conclusion. |
| Move/resize/close/workspaces/popups/click-through | **Not run** | Only login and launch attempted. The implemented sequence below remains unexecuted. |
| Native suspend/resume | **Not run** | Finalization was not reached. |
| RX590 hardware behavior | **Not tested** | Virtio/software rendering is a different graphics stack. |

**Second infrastructure failure diagnosis.** At 19:45:26 UTC, systemd logged `Started lockup-edit-0.service`; at approximately 19:45:32 the read-only observer successfully bound the actual compositor’s version-3 cosmic toplevel protocol and version-1 foreign toplevel protocol but returned an empty window list. The 19:45:33 screenshot independently shows only the stock desktop. This is not an OCR miss. Screenshot SHA256 **6e160075af9c7199ace4bcb3dc5b52968d8caf726cdd0e617f0a4cca5c28c796** (`stock/001-failure.png`). No cosmic-edit process remained in the subsequent resource/process capture.

The exact installed editor package is `1.11.0~1792525544~24.04~4ac1cc8`. Its [pinned source](https://raw.githubusercontent.com/pop-os/cosmic-edit/4ac1cc8/src/main.rs) immediately daemonizes on Unix: the parent exits successfully and a child creates the GUI. The harness used a default simple transient service. [systemd’s documented service lifetime](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#Examples) requires a forking service for that behavior; the default simple main-process exit terminates remaining service processes. Thus the source establishes a launcher/lifetime mismatch, strongly consistent with the missing process/window, rather than a COSMIC global-input failure. The initial local preflight missed this service-lifetime requirement. No explicit child-termination signal or unit Result/MainPID was captured, so a runtime causal requalification remains necessary.

Local correction adds **Type=forking / GuessMainPID=yes** for cosmic-edit only and captures unit type, MainPID, result, state and unit journal after launch. This correction has syntax/local wire checks but **has not been validated in a VM**. The exact executed code remains archived within each artifact and at infra commit 72c35c4; the later local correction must not be confused with the tested harness. No extra dispatch or retry was made after the third allocated runner. A future allowed run should prove editor lifetime and an actual save first, then start the long schedule.

**Native identities actually observed** (same PIDs/hashes at baseline and failure capture; short interval only):

| Native component | PID | ELF SHA256 |
|---|---:|---|
| cosmic-session | 1252 | ba4b4a44d0a9ae740c80182f5fec907618dce50a8f3fe647c7c2398ed9f68a4a |
| cosmic-comp | 1449 | 2399b768cc6a4599a6495d0865cf4dc6807cfee2c6e46b3d7466a800f903be5e |
| cosmic-settings-daemon | 1496 | 0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423 |
| cosmic-panel (stock) | 1522 | e66271e6114ac21754f2969c910a36dfea3b958f949d74d9e7e22595caf0f937 |
| xdg-desktop-portal-cosmic | 1857 | 73d5864b661f5e4d5995e788869b26a9b9091edf723277bae41f9cb38916d56b |

OS Pop!_OS 24.04; kernel **7.2.6-76070206-generic**; compositor package **0.1~1792508925~24.04~8cb1802**; panel **1.11.0~1792526322~24.04~ca20ce3**; session **1.11.0~1792525615~24.04~4b0a0a2**; Mesa packages **26.1.6-1pop0~1787580452~24.04~a5619ea**. Native process maps show LLVM20.1 and Mesa EGL; kernel uses virtio_gpu, not amdgpu/RX590. Kernel/compositor boot logs are retained, with no GPU-hang/reset match in this short captured kernel interval. That is not an hour-long negative result.

Only stock failure resource sample: guest uptime **93.95sec**, cosmic-comp RSS **227,668KiB**, **33 threads / 150 fds**; stock panel RSS **133,340KiB**, **21 threads / 250 fds**. One sample cannot establish resource growth. Editor ELF identity was not captured because no editor process remained; its installed package version and pinned source are recorded instead.

**Designed but unexecuted one-hour protocol.** Input probe uses actual native Cosmic Edit file open, pointer focus, select/type/save, QGA independent exact file readback, native toplevel active state/geometry and rendered screenshot/OCR. It never substitutes controller writes for editor saves. Each five-minute sequence exercises maximize/restore, titlebar move, border resize, workspace overview and switching, Files open/focus/pointer close, native popup dismissal, Halcyon edge menu, editor close/reopen and repeated canaries; approximately minute samples track resources. Save failures halt and retain exact state; uncertain gestures remain coverage failures rather than become socket-health passes. Idle spans occur between sequences; a bounded actual RTC/suspend attempt is recorded separately from proof of resume.

## Minimal missing reporter information and remaining gates

1. Exact Halcyon/public package, COSMIC/OS versions; Islands installed/enabled; active look/material and whether Arrange/Notes editing preceded the incident.
2. Which applications/windows, whether mouse, keyboard shortcuts/workspaces and compositor remained responsive; whether stopping main, disabling Islands or logging out restored input.
3. Kernel/Mesa/amdgpu versions, monitor count/resolution/scaling; approximate incident time and relevant redacted compositor/kernel GPU hang/reset logs around it.

The software-rendered virtio VM cannot establish RX590 correctness or reproduce the reporter's native GPU/display stack. First validate the corrected native launcher and real save/render canary; then obtain all three one-hour controls, target hardware reproduction and a deterministic reported action sequence. These remain required for a resolution claim. No 0.2.1 release is authorized; future integrated packages require their own exact-package qualification. No previously accepted theme cases are repeated here.
