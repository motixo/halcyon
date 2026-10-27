# Reproducing the foundation probes

Run from the repository root with the retained Rust 1.93.0 cache. Use a fresh output path for every run. One Cargo slot; no installed service or preferences are changed.

```sh
CARGO_TARGET_DIR=/tmp/halcyon-app-target cargo +1.93.0 test --release --locked --offline --no-run --bin halcyon
python3 docs/future-builder/probes/settings-baseline.py /tmp/halcyon-app-target/release/deps/halcyon-ef357a986c642533 /tmp/halcyon-tree-new
```

Use the actual test executable reported by Cargo if its name changes. The ignored `settings::future_builder_probe::measure_current_native_tree` requires an explicit private root. The wrapper creates isolated XDG directories and a private session bus, preserves HOME, records hashes and watches owned descendants' DRM counters. It runs no live providers. PNGs are real offscreen widget-tree captures.

Typography uses the compiled pinned Iced dependency:

```sh
rustc +1.93.0 --edition 2024 docs/future-builder/probes/fonts.rs --extern iced_graphics=/tmp/halcyon-app-target/release/deps/libiced_graphics-1329af2fb6329b61.rlib -L dependency=/tmp/halcyon-app-target/release/deps -o /tmp/halcyon-font-probe
/tmp/halcyon-font-probe > /tmp/halcyon-font-probe.csv
```

Choose the matching dependency artifact after a rebuild. This is a native paragraph probe, not an independent model of shaping. It emits requested and resolved faces, glyph counts and measured widths.

`native-idle.py BINARY TOOLS OUTPUT` needs stock `cosmic-comp`, `cosmic-panel`, `cosmic-randr`, `dbus-daemon`, ImageMagick `import`, and a TOOLS directory containing Xvfb and xdotool with its adjacent libraries. It starts a private X server, bus and compositor, verifies stock left/right layer-shell anchors, captures private windows and reaps its child processes. Runtime sockets use a short private `/tmp` directory because Unix sockets have a path-length limit. It does not install anything. Idle samples are deliberately separate from application screenshot mode.

The current machine's retained tools were at `.dock-islands-20261019/review-r2/native-tools/root/usr/bin` under the older scratch workspace. Evidence records the exact source/artifact hashes. This path is local infrastructure, not a portable dependency or part of the app.

The focused existing tests listed in the evidence record run directly from the built test binary, each with `--exact --test-threads=1`. They cover old-profile rejection, missing capabilities, stale plans and native-format round trips. They do not validate the proposed future migration.

## FB05 Clock implementation

Use the same build command above, then run `foundation_`, `widget_style::`, `app::clock::`, `app::glance::`, `desktop::profiles::`, `settings::preference_undo::`, `settings::navigation::` and the exact `settings::tests::clock_native_controls_and_center_actions_fit_small_settings_windows` from that test executable. Each supports `--test-threads=1` and uses existing fixture isolation where preferences are written.

The Clock capture uses two presentation fixtures with no provider execution:

```sh
python3 docs/future-builder/probes/settings-baseline.py /tmp/halcyon-app-target/release/deps/halcyon-ef357a986c642533 /tmp/halcyon-clock-new --test-name measure_clock_foundation
```

For the separate real-window idle check, build the ordinary release binary and run `native-idle.py BINARY TOOLS OUTPUT --idle-seconds 60 --analog-clock`. This variant seeds a private Paper/analog Clock, opens only Clock Settings and skips the unrelated stock panel probes. The test binary is not an application binary. As above, the compositor is private and software-rendered; an application GPU handle alone is not proof of hardware rendering.

Final records are in [FB05 evidence](../evidence/fb05-20261026.json). Earlier failed build/test logs and the first visual iteration remain in the local evidence folder; they are not counted as passing acceptance.

## FB06 component presentation

Run `fb06_`, `app::clock::`, `app::glance::`, `app::utility_tests::`, `app::auditfix_tests::`, `app::layout::`, `settings::view::profiles::image_tests::`, `settings::ux_tests::`, `settings::theme_journey_tests::`, and the compact Clock controls test listed above from the freshly built test executable with `--test-threads=1`.

```sh
python3 docs/future-builder/probes/settings-baseline.py /tmp/halcyon-app-target/release/deps/halcyon-ef357a986c642533 /tmp/halcyon-components-new --test-name measure_component_foundation
python3 docs/future-builder/probes/settings-baseline.py /tmp/halcyon-app-target/release/deps/halcyon-ef357a986c642533 /tmp/halcyon-component-timing-new
```

The first captures both Clock faces in Paper and Tidal fixtures at 100%, 150% and 200%, Weather samples, and compact/wide Settings scenes. The second repeats the existing GPU/software draw-readback workload. Run timing without concurrent compilation. Output scales exercise renderer geometry, not physical display transitions.

For real windows, build the ordinary release binary and run `native-idle.py BINARY TOOLS OUTPUT --idle-seconds 12 --component-pages`. This opens Clock, Weather and Themes in the private compositor and skips panel probes. Twelve seconds is a smoke observation, not the 60-second idle-budget qualification. The [FB06 evidence](../evidence/fb06-20261026.json) records the actual duration and scope.

## FB07 provider demand

Build the test executable using the command above. Run `fb07_` first with `--test-threads=1`. The cases use a private loopback HTTP server, acknowledged asynchronous worker fixtures, private cache files and subprocess-isolated runtime/display fixtures. They do not query a live weather service.

The integrated filters are `fb06_`, `app::tests::`, `app::auditfix_tests::`, `app::glance::`, `app::display_tests::`, `app::utility_tests::`, `app::layout::`, `app::clock::`, `schedule::tests::`, `data::tests::weather_cache`, `app::theme_tests::`, and the compact Clock controls test listed above. Final counts deduplicate overlap. Build the ordinary release binary separately with `cargo +1.93.0 build --release --locked --offline --bin halcyon` using the same target directory.

[FB07 evidence](../evidence/fb07-20261026.json) records this source's results. No new compositor performance, physical hotplug, live-provider or package qualification is implied by these focused checks.

Full raw captures/logs remain in `.task-future-builder-baseline-20261026/`. Small numerical results and source hashes are bundled in `../evidence/`. Failed harness attempts are preserved locally: native-r1 exceeded the Unix socket path limit; native-r2 established initial idle/panel behavior; native-r3 found Pillow unavailable; native-r4 used ImageMagick but aggregate thread counters fell when workers retired. Native-r5 uses `--idle-seconds 60` and matches surviving thread IDs for a clearly labelled lower bound. These are harness revisions, not hidden application failures. Use `collect.py` to rebuild the bundled evidence from these retained final runs.

## FB08 layout and input

Build the release test target, then run `fb08_`, `app::layout::`, `settings::placement::`, `layout_state::`, `app::display_tests::`, `settings::preference_undo::` and the existing transaction/profile regressions. The new column check measures actual native tree bounds; output tests use synthetic protocol objects and isolated configuration. [Exact filters and results](../evidence/fb08-20261026.json) distinguish these checks from physical/native acceptance in FB09/FB20.

## FB09 complete foundation

Run `fb09_` with four test threads. The worker tests serialize their shared two-slot
fixture internally. The private child entry is ignored by ordinary test discovery
and invoked explicitly by these cases. Run the affected profile, wallpaper, theme
journey, native queue, provider, layout, Undo and store filters recorded in
[FB09 evidence](../evidence/fb09-20261026.json).

Build the ordinary application binary separately. `foundation-session.py BINARY
TOOLS OUTPUT --style paper` exercises real native scroll input, visible composite
pixels, 26 page cycles, 26 control/keyboard-Undo cycles and 60 seconds of settled
idle. Repeat with `--style tidal`. Use a retained published binary and `--baseline`
for the same page-cycle comparison; it does not run new-control edit cycles.

`--desktop` captures actual widgets with a matching private COSMIC background.
`--maximum` enables all eight standalone kinds and starts `paused-player.py` on the
private session bus so Music is actually visible. It needs the system Python DBus
and GLib bindings. Glance's grouped layout replaces three standalone kinds; this
probe does not invent a ninth independently visible widget. Default system metrics
still refresh, so this maximum run reports active-work CPU/frame counts separately
from static Settings idle. `--inspect` performs a short capture without the timing
or cycle workloads. Every run saves its probe source, fixture, result and logs.

The X server, user bus, compositor, background, sample player and application are
private owned processes and are reaped on exit. The tools never inject input into
the user's X/Wayland server. Pixel timing includes X readback/poll overhead; it is
not physical scanout. Actual GPU engine activity is checked separately by
`settings-baseline.py` using the release test executable.

`worker-crash.py BINARY RESULT_JSON` kills its owned worker parent and adopts/reaps
the exact child. The probe keeps the pipe writer open, so parent-death handling is
required for success. Run `collect.py` in the retained FB09 raw directory only
against its recorded sources/binaries; later implementation overwrites the shared
build artifacts and must not be relabelled as the earlier qualification.

## FB10 presets and typography

Run `fb10_` plus the preset, profile-editor, UX, wizard and image-worker filters
listed in [FB10 evidence](../evidence/fb10-20261027.json). The opt-in
`measure_curated_presets` test is supported by `settings-baseline.py`; it captures
all four real recipes, selected review pages and installed-font specimens.
First GPU readback may precede texture upload: use its `*-warm-*` captures for
settled visual review, retaining cold captures as diagnostics.

`foundation-session.py --gallery [--islands-fixture] --size 820x600` exercises the
actual native release window, four setup candidates and 26 select/discard cycles.
Repeat with `--size 1280x800` for the wide layout. It uses the existing alternate
configuration/capture gate. All desktop observations, applets and optional
Islands capability are private fixtures; it neither starts nor qualifies a
companion host. The script owns and reaps its native session. `convert` is needed
for RGBA capture conversion. Frozen raw probes/results live in the FB10 evidence
directory; do not regenerate that stage's manifest after changing its sources.

## FB11 canvas

Run `fb11_`, `settings::placement::`, `settings::ux_tests::`,
`settings::session_drafts::`, `settings::legacy::`, `settings::native_jobs::`,
`config::tests::` and the compact Clock controls test against the new release test
binary. Set `DESKTOP_CARDS_LAYOUT_EVIDENCE` to retain the real-tree captures.

`foundation-session.py BINARY TOOLS OUTPUT --canvas --style tidal --size 820x600`
and the Paper/1280x800 variant use private native release windows. They check
cancel plus 26 combined Apply/Undo cycles, exact configuration restoration and
warm RSS growth. Capture mode is deliberately excluded from idle measurement.
The final FB11 records use the r10 binary and retain earlier review iterations.
Do not regenerate frozen evidence after a later implementation changes sources
or artifacts.

FB12 adds `foundation-session.py --mix`, with three source choices, Cancel and
26 Apply/Undo cycles in private release windows. Its fixture write path is distinct
from the separate forced-worker Rust regression. The final FB12 tool extraction
is retained under `.task-future-builder-fb12-20261027/tooling/`; older scratch tool
paths disappeared during validation. No packages were installed on the host.


## FB13 sharing and Spine

Run `fb13_`, profile/editor/native-view suites and the image-worker ownership
regressions. `HALCYON_SHARE_EVIDENCE` retains public sample PNGs;
`DESKTOP_CARDS_LAYOUT_EVIDENCE` retains native action captures. The raw FB13
worker probe exercises the actual release `--share-worker` entry, bounded malformed
input, private-source omission, latency and peak RSS.

`foundation-session.py BINARY TOOLS OUTPUT --spine --style tidal --size 820x600`
(and `1280x800`) applies the studio setup in an isolated native window. It copies
committed panel settings into a separate private stock-panel configuration, runs
one actual Clock applet, checks vertical Wayland anchors, then waits for keyboard
focus before Undo. All owned processes are reaped. This is distinct from vertical
Islands or physical-device acceptance. Frozen evidence is in FB13; do not rerun
its collector after later source edits.
