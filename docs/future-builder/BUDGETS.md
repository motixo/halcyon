# Resource and responsiveness budgets

Revision 1, 2026-10-26. These are engineering acceptance limits for the future implementation. They are not claims that the concepts already meet them. Increasing a limit needs a measured workload and a recorded decision.

## Baseline and comparisons

Reference workstation: Ryzen 7 5700G, integrated AMD Cezanne/Vega, amdgpu, powersave governor, Rust 1.93.0, pinned libcosmic. The [evidence record](evidence/baseline-20261026.json) identifies kernel, artifacts, fixtures and raw samples.

The native-tree probe measures event dispatch plus drawing and **synchronous screenshot readback**, not compositor input-to-paint latency. Four real pages, 40 warm samples each at 820×600 and 1280×800, plus 12 software samples per page at 1280×800. Three warmup draws precede sampling. Background provider tasks are not executed. First-page timings are not cold application startup.

A separate private nested compositor sampled real idle Settings processes for 60 seconds each without the diagnostic capture timer. The private output was 1280×800 at 100%, with an 852×600 application window. This establishes a starting point, not battery or long-session qualification. The nested compositor was software-rendered; its application's AMD handle alone did not prove hardware rendering. Headless GPU engine activity was verified separately. Process RSS and GPU allocations must not be added as if they were disjoint physical memory.

Measured p95 event + draw + synchronous readback, milliseconds:

| Current page | GPU 820×600 | GPU 1280×800 | Software 1280×800 |
| --- | ---: | ---: | ---: |
| Clock | 1.44 | 1.19 | 7.44 |
| Weather | 1.10 | 1.13 | 8.28 |
| Layout | 0.85 | 1.03 | 8.63 |
| Themes | 2.69 | 2.81 | 11.64 |

Native idle RSS was 239.4 MiB on Clock and 247.7 MiB on Themes. Neither 60-second sample incremented process CPU ticks at the kernel's 10 ms accounting granularity; this does not mean literally zero CPU use. Surviving-thread voluntary context-switch rates were 1.67/s and 1.42/s, a lower bound that excludes retired/new workers and is not a timer-wakeup count.

## First slice acceptance: FB05–FB09

| Measure | Limit and method |
| --- | --- |
| Warm event dispatch | p95 ≤1 ms in the current native-tree harness |
| GPU draw/readback proxy | p95 ≤4 ms per reference page at the two baseline sizes; if a paired baseline itself exceeds 4 ms, investigate the environment before accepting a regression |
| Software draw/readback proxy | p95 ≤15 ms at 1280×800 |
| Real interaction | Target p95 input-to-present ≤50 ms and p95 frame interval ≤20 ms during a 60 Hz drag/scroll; instrument actual presentation before FB09 acceptance |
| Static idle | ≤0.5% of one CPU core over a 60 s stable sample; zero self-scheduled frames while content is static; event-driven updates permitted |
| Same-workload memory | Retained RSS growth ≤64 MiB over a paired current baseline; ≤16 MiB growth between cycles 5 and 25 of open/edit/cancel/close after caches warm |
| Current maximum single-instance design | All eight standalone widgets; also exercise Glance's alternative grouped layout. The nine existing kinds cannot all be independent simultaneously because Glance replaces Clock/Weather/Calendar. No change to persistence limits in the first slice |
| Provider work | Exactly one in-flight request per full binding key; two same-key consumers must not double fetches; preview/share consumers start zero live requests |
| Cold image work | Off the interaction thread; cancellation acknowledged within 100 ms at bounded work checkpoints; no stale completion applied |

The local first-slice measurements are now recorded in [FB09](FB09.md): actual native input to visible nested-compositor pixels, sustained frame intervals, 60 s settled idle, warm memory cycles and cold-worker cancellation. All pass their scoped limits. These remain separate from the draw/readback proxy and from physical display presentation/device qualification in FB20. Context switches are not equal to timer wakeups. Active system-monitor refreshes in the maximum desktop workload are reported separately from static idle.

## Finite limits for later stages

These limits constrain the design now; implement them at the owning boundary before exposing the corresponding feature.

| Resource | Initial ceiling |
| --- | --- |
| Applied widget instances | 24 active, 64 stored including disabled; reject activation above the active limit with a useful explanation |
| One recipe | 32 native parts, nesting depth 4; no recursion, loops, commands or executable expressions |
| Entire active composition | 768 parts, 64 style tokens, 4 moment layers; limits checked before allocation/layout |
| Preview specimen gallery | 12 rendered tiles retained; other rows virtualized; at most 2 background specimen/decode jobs |
| Raster source | 16 MiB encoded, 16 million decoded pixels, maximum dimension 8192; enforce both dimensions and checked byte arithmetic before allocation |
| Ridge matte | 8.5 million single-channel pixels; shared validated transform; unsupported fits use the flat fallback |
| Image memory | 128 MiB total retained decoded/derived CPU images, including gallery images; 64 MiB additional aggregate decode/resize workspace; evict or reject before exceeding either |
| Gallery/type caches | Image thumbnails ≤16 MiB within the image budget; shaped specimen cache ≤8 MiB; bounded keys and revision invalidation |
| GPU resources | Target ≤256 MiB attributable process allocation for the maximum design; verify driver reporting and ownership, separately from RSS |
| Whole-process RSS | Target ≤512 MiB at 24 active instances on the reference workload; future limits do not loosen the first-slice paired regression limit |
| Async work | 4 concurrent provider jobs, 32 pending unique keys; coalesce obsolete generations. Existing stricter provider limits win |
| Ordinary fetch deadline | ≤15 s wall time including retries for a refresh; existing shorter deadlines win; OAuth and user interaction retain separate explicit deadlines |
| Design and legacy imports | Legacy profile stays 256 KiB. New design manifest ≤512 KiB; check bytes, nodes, depth and strings before resolution |
| Future asset package | ≤64 files, ≤64 MiB archive, ≤128 MiB extracted total, plus per-image decode limits; no path escapes, links or executable entries |

At maximum load, test two displays, 24 mixed clock/weather/text/meter instances, four moment records, two different provider bindings and 12 visible gallery tiles. The 24-instance caps do not ship until FB16; they are not an excuse to build multi-instance machinery in FB05. Library entries may exceed these counts on disk because only a bounded working set is loaded.

Static rendering has no redraw clock. Active animations follow the output frame callback, capped at 60 fps in the initial product; hidden content requests zero animation frames, and reduced motion disables nonessential animation. Provider cadence follows data need rather than frame rate.

## FB10 local result

The Type wall retains at most twelve native specimens per page. The fixed
compiled gallery cache rejects allocations above 16 MiB and shares two decode
slots. In actual native Setups windows, 26 select/discard cycles retain 0.20 MiB
(compact) and 2.49 MiB (wide) between cycles 5 and 25. The four reference pages
remain within the existing limits (GPU p95 2.76 ms, software 11.83 ms worst).
New setup-review draw/readback is a separate measured workload, up to 5.22 ms;
this is neither the reference-page workload nor physical presentation timing.
See [FB10](FB10.md) for exact scope and evidence. Later aggregate caps remain
requirements at their owning implementation boundaries.

FB11 retains at most 50 local geometry/style history entries. Restored canvas
baselines are capped at 128 KiB and style overlays at 8 KiB, within the existing
private draft envelope. Only seven supported style pointers are accepted. Busy
canvas pointer events do not grow the native writer queue. Two 26-cycle native
Apply/Undo runs grew by 0.79/0.89 MiB between cycles 5 and 25. These capture-mode
runs make no idle or physical latency claim; see [FB11 evidence](evidence/fb11-20261027.json).

FB12 bounds composition to three source snapshots, each using the existing 256 KiB
profile limit, with twelve visible source choices per page. Existing two-job media
admission and caches are retained. Final private native Mix board sessions retained
0.52/0.35 MiB across warm repeated Apply/Undo cycles at compact/wide sizes. These
capture-mode runs do not establish idle or physical frame-pacing performance.


FB13 public previews are fixed at 1280×720, with a 270,336-byte request bound,
4 MiB PNG cap, four-second worker deadline, 512 MiB child address-space and
two-second child CPU limits. Rendering and PNG encoding share the two media
slots; one editor owns at most one unfinished export. The final release child
rendered the cold public sample in 117 ms with 48.3 MiB peak RSS. This is one
bounded subprocess measurement, not a frame-pacing or physical GPU claim.

FB14 uses one 1672×941 single-channel prepared matte and at most one 2048×1024
Clock image per runtime/editor cache. It shares the two media slots and existing
child bounds. Six native Apply/Undo cycles retained 0.08/0.32 MiB after warm-up;
short five-second depth idle samples had no commits. These bounded checks do not
replace the long idle, physical-device or aggregate future-instance gates.

FB15 adds two fixed phase records with one shared family and four optional Clock
properties each. Existing 32-rule/64 KiB scheduler and canvas history/draft bounds
remain. No additional subscription or timer is introduced. Six native activation/
Undo cycles retained 0.07/0.05 MiB at compact/wide sizes; the five-second live
Clock idle sample had zero commits. These are bounded checks, not new long-idle
or battery claims.
