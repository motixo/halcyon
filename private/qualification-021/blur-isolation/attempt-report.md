# Native overview / blur causal isolation — R1

**Outcome: causal comparison incomplete; the single authorized allocation is consumed.** Untouched stock opened its first overview and retained real editor input. Its second overview attempt produced a **separate native cosmic-workspaces WGPU panic/restart**, with the compositor still alive. The next fresh overlay successfully enabled the three intended native frosting flags, but my overly strict export round-trip guard stopped it before editor/overview stimulus. Public Frosted and public-with-native-frosting-off were **not run**. No second allocation, hour soak, package build, product/native patch or release occurred. **0.2.5 remains ON HOLD.**

## Execution and scope

[Actions run40044224132](https://github.com/profilaz3/halcyon/actions/runs/40044224132), executed infra **d3aeeb82bb1499a10dde8c04f911c1c6c83ea557**, branch `ci/qualification-probe-021`. One allocated `ubuntu-24.04` KVM job120722104020, **00:17:24–00:27:18UTC on2026-10-24 (9min54sec)**; <=90min. Package and old session jobs were skipped. Two fresh overlays actually started, serially; the two public overlays did not start. VM restriction: 4vCPU/6GiB, pinned q35/PCI/firmware,1280x800 virtio software graphics, `-nic none`, private Unix QMP/QGA/VNC, no hosthome mounts or guest credentials. The stock cases had no inserted product ISO, Halcyon package/config/process. No clocks/mode events were injected. The original guest was restored rather than rebuilt. No ci/qualification-inputs-025/new1783 guest work was accessed.

Artifact **12540977061**, `native-blur-isolation-r1-40044224132`, **21,194,197bytes**, ZIP SHA256 **a4111db1930f789c626c7443a9991ceddac6312ffcd60aba435b3a0596f00263**, expires **2026-11-07 00:27:13UTC**. The small ZIP was downloaded, API-digest verified and extracted here. Original ZIP, complete failed-step log, raw case results, screenshots, unit logs, native journals, identities, native config snapshots and executed source copies are preserved. [findings.json](findings.json) independently checks saved-file bytes/source copies and overrides misleading runtime success flags. [run-metadata.json](run-metadata.json) records runner, serial times and skipped builds.

## Per-scenario result

| Scenario | Native setup / actual stimulus | Observed outcome |
|---|---|---|
| Untouched stock, immediate overview | Native frosting false; real editor save; Super+W18.064sec after unchanged export | **PASS for opening overview and subsequent editor input.** Overview visibly opened. Compositor1505/starttime3966 unchanged. |
| Same stock, preserved R3 maximize/restore/header-target/resize then overview | Second Super+W60.331sec after unchanged export | **FAIL for overview.** Native cosmic-workspaces panicked and restarted1619→3434; second overview is absent from its retained frame. Compositor stayed1505/starttime3966; editor still saved. |
| Stock with equivalent native frosting | Native Settings imported windows/system-interface/applets=true, panel=false, Medium strength | **SETUP BLOCKED before editor/overview.** Re-export projection changed derived text_tint; strict byte-equality guard rejected it. No input/overview result. |
| Public0.2.0 Frosted, immediate overview | Not started | **NOT RUN.** No new inference about hour-delay necessity. |
| Same public treatment, native frosting off | Not started | **NOT RUN.** No verified mitigation result. |

Stock input is independently **5/5 byte-exact fresh saves**, tokens `bstockbaseline001`…`005`, **2 normal closes/reopens**, actual native editor ELF and mapped-window evidence. Only one empty fixture was initially created; no controller content writes or repeated retyping manufactured a save. `005-immediate-overview.png` visibly shows the actual overview with token001; `018-reopened-2-input.png` visibly shows token005 after the later native app failure. Initial Type=forking unit Result=success, active/running, MainPID snapshot0; actual editor PID2747/starttime8029, correct ELF and native typing/save prove the child survived. Zero MainPID metadata was not treated as app absence.

The first overview frame also contains visible horizontal colored striping near the dock. It is retained; this is not a pixel-perfect rendering acceptance. The second frame `013-after-preserved-sequence-overview.png` shows the ordinary editor instead of overview, so the runtime claim that both overview sequences passed is **not accepted**.

## Separate untouched native Workspaces failure

The first overview before/after journals have no panic. The second **before** journal has no panic; its **after** journal first contains at guest monotonic139.417692:

`cosmic-workspaces ... vendor/wgpu/src/backend/wgpu_core.rs:3865:18` — `Surface::configure`, `Surface does not support the adapter's queue family`.

This is preceded by `ERROR_SURFACE_LOST_KHR` and capture-session protocol error1 on `ext_image_copy_capture_session_v1`. Actual Workspaces executable **7e3cf670bb3dbd63c5b5245bcfba893f42f8a07bab70073cc772e07b0ece5f3d**, package **1.0.12~1790783807~24.04~cb953ea**, baseline PID1619, final PID3434, same ELF. The compositor PID/starttime/ELF remained constant. Five editor saves still passed. This is **not** R3's `cosmic-comp` `Custom Shaders not initialized` panic and is not evidence of all windows losing input. Observed order: open/save editor → Super+W → Esc → fresh save → normal Meta+Q close/reopen/save → Meta+M maximize/restore → preserved header target → bottom-right resize → Super+W again. The panic first appears between the second before/after journals. This is a bounded observed reproduction, not proof that every preceding action is necessary. Its prior overview/close history differs from R3's first post-hour overview; no additional isolation/retry was performed.

There are **zero captured compositor panics and zero Files desktop applet `Create pixel map` panics** in these two short overlays. That is a narrow observation, not a replacement for the R3 long-session failures. The executed harness tracked compositor failure only, so its stock `complete=true/native_failure=false` flags missed the Workspaces client crash. Independent log/frame/process review caught it; a green per-case flag is not a pass.

## Setup guard failure and local repair

I assumed the native Settings export was a raw ThemeBuilder round-trip. That was an avoidable harness mistake. Actual `/usr/bin/cosmic-settings appearance import` exited successfully, and the generated theme and builder contain **frosted_windows=true, frosted_system_interface=true, frosted_applets=true**, **frosted_panel=false**, **frosted=Medium**. The raw stored builder `text_tint` remained the requested `Some("#E7E7E7FF")`. Re-export differed in exactly one field:

```diff
- text_tint: Some("#E7E7E7FF")
+ text_tint: Some("#F5F5F5FF")
```

Actual generated `Theme.Dark/v2/background` has outer `on: "#F5F5F5FF"`. Pinned native [ThemeCustomizer::from](https://github.com/pop-os/cosmic-settings/blob/f1d00fc06e18203e880771617657067c10c43321/cosmic-settings/src/pages/desktop/appearance/theme_manager.rs#L67) reconstructs the exported builder using generated theme colors, including `.text_tint(theme.background(false).on.color)`. [Import/export](https://github.com/pop-os/cosmic-settings/blob/f1d00fc06e18203e880771617657067c10c43321/cosmic-settings/src/pages/desktop/appearance/commands.rs) therefore does not promise byte-identical round-tripping. The foreground difference is a native build/export projection, not proof the requested blur flags failed. No unchanged-import control was run, so this report does not attribute that projection specifically to enabling blur.

The CLI also logged `1:1: Expected identifier` while loading a builder, already present on the initial export. It did not prevent successful import/re-export. The error log does not name the failing key; its exact separate origin is not established here. It is not being misreported as the stopped test's cause.

The locally prepared infra repair validates raw stored input plus the source-defined exported foreground against the **actual generated background.on**, rejects any other export change, snapshots before checking, and tracks native Workspaces PID/starttime/ELF and its panic logs as well as the compositor. Eleven local safety tests pass, including retained real exports/raw native fields and deliberately corrupted inputs. No expected palette is rewritten into a fixture and called app behavior. **This repair has not run in a VM.** The original executed d3aeeb8 source, failure and all artifacts remain unchanged. Overview frames still require independent visual review.

## Exact treatment and source boundary

The native-only control used actual supported native Settings CLI, not panel frosting or hidden synthetic mode writes:

```sh
/usr/bin/cosmic-settings appearance export /var/tmp/blur-export.ron
# Change only frosted_windows, frosted_system_interface, frosted_applets to true.
/usr/bin/cosmic-settings appearance import /var/tmp/blur-treatment.ron
/usr/bin/cosmic-settings appearance export /var/tmp/blur-export.ron
```

Only active Dark was targeted. Medium is step6; default alpha_map.medium is0.76154. All alpha-map entries and native panel=false were retained. Strength step0 is ExtremelyLow, **not off**; enabled compositor blur uses enum+1. The preserved `controlled-native-input.ron`, native before/after exports and `native-export-comparison.diff` contain the exact commands' data. This configured state is reproducible, but **no native-blur overview reproduction commands are claimed validated**, because the guard stopped before that stimulus.

Current [glass rendering research](../glass-rendering-exploration-r1/report.md), “What the inspected product actually does” and “Surface policy”, corroborates the three global Frosted effects and explicitly excludes panel frosting. Widgets/Islands request native blur separately for all four finishes; Settings avoids automatic window blur but retains popup/system routes. This experiment did not alter those renderers or import current product code. Its planned public-native-off case would disable only the three global effects while retaining widget blur requests, not all native blur. Disabling widget blur alone would likewise leave the legacy global effects active. No rendering research was duplicated.

Pinned [software_renderer](https://github.com/pop-os/cosmic-comp/blob/8cb1802caada438bb5cc7670dfabdaca5fad0502/src/backend/kms/mod.rs#L293) constructs an EGL/Glow context without custom `init_shaders`; [offscreen_renderer](https://github.com/pop-os/cosmic-comp/blob/8cb1802caada438bb5cc7670dfabdaca5fad0502/src/state.rs#L422) can select it, and [BlurShaders::get](https://github.com/pop-os/cosmic-comp/blob/8cb1802caada438bb5cc7670dfabdaca5fad0502/src/backend/render/wayland/blur_effect.rs#L108) expects context shader data. That remains a source-supported explanation consistent with R3's panic. **This run neither confirms nor refutes the exact failing-context hypothesis.** No native binary was instrumented/patched/rebuilt; no equivalent-blur overview was executed.

## Mitigation options and next gate

No tested Halcyon mitigation is established by this attempt. Parent can consider these options after further qualification:

- A conservative policy scoped to known affected native version/ELF and software/offscreen renderer conditions could avoid newly activating the global frosting flags and resolve Halcyon-owned blur requests to the researched decorative/wallpaper-backed material paths. Preserve hue, readable tint/scrim and finish intent; do not blindly clear system effects explicitly enabled by the user/another application. Migrating prior Halcyon-owned global settings requires provenance. Mere protocol availability or GPU vendor is not a safety test, and a destructive live-user “try blur and see” probe is inappropriate.
- Disabling only global frosting while retaining native widget blur is the still-unrun comparison. Even if it avoids overview failure, other capture paths involving widgets/bars/popups would remain unqualified. Disabling only widget blur does not test the actual global Frosted route. Blur strength0 is not a mitigation.
- A responsible native fix candidate is initializing custom shaders for the software/offscreen context and handling unavailable shaders non-fatally. It requires a patched exact native package, direct repro/requalification and independent handling of the separate Workspaces WGPU/capture failure. No upstream message/issue or product workaround was submitted/implemented.

**Concrete next step:** use the locally repaired native-state assertions and run the three missing fresh-overlay comparisons under a newly authorized budget; immediate overview first. If the preserved preceding sequence is needed, put that variant in another fresh overlay without a prior overview, so the newly observed repeat-overview/closed-window history cannot confound it. Track Workspaces separately so its stock client restart cannot be called a successful overview or confused with the compositor shader panic. No further allocation was started in this round. The questions “native blur alone?”, “hour delay needed for R3?”, and “global blur off sufficient with usable public material?” all remain **unanswered**. R3's concrete release concern stays open.

## Immutable input and native identities

Original guest run **39963889546**, retained artifact **12506756949**, ZIP **e87c617f179da525d7d06e3e74b862fa87dcefabdbcbcb02f2e7258802a6ccd4**; evidence12507125311 ZIP **47c57f02d9d2807cdabadd31b387e0e336a21441cd6acb1b99043fe043660b06**. Base qcow **d3ea6c9f93c68ca0013722c4d22d77408f590ba54bf4b929a25faa50eaeb5c98**, vars **25d24fad9e38e155e239bcdcd4edb058cf033e34903613f662cf44ba19a7d352**. Signed ISO/firmware/input checks are preserved. The original ZIP remains in native-input-retention-r1 and was not cleaned or rewritten.

Finalized bundle run39966537804/artifact12507891179, ZIP **2cdea1fd0ba584bc1c0891033764042b07692b370392fc4f4896d4fef7e11f65**; public main deb **af1ff76f654f7eaefad6b788235304fff6cc868bcc14011b3b073a99db4af847**. Bundle preparation/identity was verified; **no public package was installed in the two actually executed stock overlays**.

Actual kernel7.2.6-76070206-generic; native compositor0.1~1792508925~24.04~8cb1802, ELF **2399b768cc6a4599a6495d0865cf4dc6807cfee2c6e46b3d7466a800f903be5e**; editor1.11.0~1792525544~24.04~4ac1cc8, ELF **61f32f20df9c4da800b0df8054cb0b7ec6c65ccd4b9d11679f7f6c7303dc1149**; native Settings1.11.0~1792526230~24.04~f1d00fc, ELF **71202858ec33faa9f7ec925576d93c091ce884371f33c4e60d050cd397c854e8**. Graphics maps/journals are preserved; this is software virtio/Mesa, not RX590. No hardware causation/resolution claim.

Reference R3 execution7840d56: stock run40018232557/artifact12532668378 ZIP **6fe460119a371dc59abed53c87e448d23d89fe7e3a835b2d01a1287ffcdc0e29**; main40027094006/artifact12536019039 ZIP **cf6f934b34e96f2e76cd5eb1402a933bb1aec3789ac38b5620a8093e9d040ae5**; Islands40035187607/artifact12539711512 ZIP **678d8826ff13993ef924dbc52035d17b434d41acc196e57b9937579c6ea5e56b**. Those hours/failures remain accepted and were not repeated. Source archive/file digests and original R3 package/process pins are in [source-provenance.json](source-provenance.json), `source/`, and [findings.json](findings.json).
