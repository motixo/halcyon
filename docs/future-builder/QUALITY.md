# Production readiness gates

A concept image and a passing unit suite do not qualify a release. Evidence must identify source revision, artifact hash, environment, fixture/live data, measured result and remaining limits. Store concise reproducible results in this folder's `evidence/` when implementation starts; keep large/private captures outside Git and reference them explicitly.

## Required behavior

| Area | Minimum proof |
| --- | --- |
| Model and migration | Existing profiles retain behavior; stable instance IDs survive save/reopen; invalid/newer designs do not silently lose fields; one applied source of truth |
| Styling | Inheritance, reset, per-moment/instance precedence and accessibility constraints resolve identically in preview and live output |
| Rendering | Shared measured geometry; correct text/font fallback; same-path image invalidation; no I/O or provider startup in view construction |
| Layout and input | Insets applied once; automatic sizes restored when overrides are removed; clipping and hit regions agree; hidden/paused objects release input |
| Displays | Compact landscape, portrait and mixed 100/150/200% scale; unplug/reconnect while editing; missing saved output, resume and reboot preserve intent and recoverable drafts |
| Providers | Two consumers with the same key share work; different keys stay isolated; cancelled/stale responses cannot replace current data; no extra demand from previews |
| Data states | Loading, empty, stale with age, failed and disconnected states remain readable; account/content removal is respected immediately |
| Commit and recovery | Preview and Cancel write nothing; one completed action has one scoped receipt; external edits conflict; crash/failure at transaction boundaries preserves recoverability |
| Automation | Fixed time, DST, sleep, clock/time-zone changes, manual edits, restore and queued-old-work cases; preview scrubbing has no scheduler effect |
| Accessibility | Full keyboard selection/arrangement, visible focus, names/actions exposed by native controls, usable large text and reduced motion; real screen-reader acceptance for shipping controls |
| Imports and assets | Finite byte/decoded-pixel/node/depth limits; confined asset resolution; no commands, secrets or personal content in shared packages; Undo/recovery assets survive cleanup |
| Native integration | Missing/unhealthy Islands remains distinct from Off; standard fallback, relevant orientations and active applets are qualified on the packaged artifact |

## Performance contract

FB03's initial [numeric budgets](BUDGETS.md) and [baseline evidence](evidence/baseline-20261026.json) are recorded. Native-tree draw/readback is a regression proxy; it does not satisfy the separate input-to-present or frame-pacing gate. The full first-slice acceptance still requires these measurements and memory/provider checks. Historical [PERFORMANCE.md](../PERFORMANCE.md) measurements are not a guarantee for a new workload.

Record warm input-to-paint latency (including p95), sustained frame times during drag/scroll, idle process CPU/wakeups, retained CPU/GPU memory where measurable, cold/warm asset costs and provider request counts. Report compositor, GPU/driver, resolution, scale, power mode, sample duration and workload. Compare identical source/fixtures on the designated integrated-GPU baseline; add hybrid/other supported hardware for release acceptance.

Budget choices must include: instance/node/depth limits, decoded image and matte limits, font/gallery preview cache bounds, request concurrency/deadlines, dirty-region redraw frequency and a representative maximum design. Set thresholds from baseline and product targets; do not invent a performance claim from a screenshot.

Structural requirements already settled:

- No continuous redraw for a static desktop and no second weather request solely because another view uses the same binding.
- Hidden consumers release demand unless an explicit feature such as reminders still owns it. Preview and share rendering never create live demand.
- No unbounded parsing, decoding, recipe recursion, catalogue rendering or task queue growth.
- Cache entries are versioned, bounded and evictable. Repeated open/close/apply cycles must reach a stable memory range.
- Asset decode, native discovery and blocking writes stay off the interaction path. Late work cannot overwrite the current draft or topology.

## Usability and visual acceptance

Use real native captures at 820×600 and a wide window, light/dark themes and the supported scale factors. Inspect each shipping preset and the failure states users actually encounter. Include a keyboard-only pass and a small task walkthrough: choose a look, change its clock, mix a layout, save, apply and undo. Success means people find the action, understand its scope and can recover without knowing the implementation.

Use the concept art for composition and comfort; generate final evidence from the actual application. Named fonts, dates and artwork inside generated concepts are illustrative and are not release assets or capability evidence.

## Release gate

Relevant focused regressions first, then integrated/native and package acceptance for the final artifact. Re-run broader checks when changes or failures justify them. Physical-display, GPU and live-provider gaps stay separately visible. A task cannot become `validated` merely because its source compiles or a mocked service passes.
