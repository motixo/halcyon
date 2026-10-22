Desktop Cards performance audit — 2026-10-17

Environment: Pop!_OS 24.04 / COSMIC Wayland, Ryzen 5700G, 32 GB RAM,
1920x1080 at 100% scaling. Baseline: installed unoptimized development binary.
Revised: optimized release binary, the same preferences and data.

Observed bottlenecks and changes

- The whole native renderer was unoptimized. Install release builds from now on.
- Settings reloaded preferences and rebuilt its view every second while idle.
  Directory notifications now catch atomic saves and commands, coalesced for 25 ms.
  A 30-second background contents check recovers missed events and emits nothing
  when unchanged. Capture-mode timers remain opt-in development features.
- Appearance used full 1920x1080 wallpapers for all four preview images. The
  software renderer copies images to round their corners during drawing. Bundled
  960x540 and 320x180 previews, with stable handles, reduce this pixel processing
  and remove per-view path checks. Actual desktop wallpapers retain their resolution.
- Mouse movement over ordinary cards caused application updates across all five
  surfaces. Global pointer handling now filters for the sidebar/edge and active
  drag releases. Native widget clicks and hover handling remain with the toolkit.
- Hidden widgets launched project scans, calendar/weather requests and system
  samples. These jobs now pause while disabled and resume when due after enabling.
- Settings service checks run only on General, every 30 seconds or after actions.
  Project scans and external network/service commands already run off the UI thread.

Measurements

Warm Appearance page, 12-second samples:
  Before: 1.75% CPU of one core, 80.95 MiB RSS.
  After: no measurable CPU at the /proc counter resolution, 35.51 MiB RSS.
  A separate 15-second baseline sample was 1.53% CPU and 81.66 MiB RSS.

Six-page settings navigation (Widgets, Calendar, Appearance, Weather, Calendar,
Appearance), each followed by a two-second settling period:
  CPU time: 7.96 s before, 0.39 s after (95.1% lower in this check).
  File-command consumption: 51–659 ms before, 31 ms after.
  The latter measures receipt of a page/focus command, not click-to-paint latency.

Desktop widget process, 15-second samples, Clock/Calendar/Weather/Devbox enabled:
  Before: 1.20% of one core, 58.69 MiB RSS.
  After: 0.13% of one core, 36.05 MiB RSS (native capture mode enabled).

Project scan: 70 ms wall time, five checkouts, no errors. No scan redesign needed.
Settings Calendar page: no measurable CPU in the revised 12-second idle sample.

Method: cumulative user+system CPU counters and RSS from /proc/PID/stat.
These are short local samples, not a frame-rate benchmark or performance guarantee.
The desktop remained in normal use; initial compilation overlapped some baseline
work. External weather/calendar response times were not benchmarked. The new file
watcher also makes preference changes propagate immediately instead of waiting
for the next three-second widget tick.

Validation

30 Rust and 8 Python checks passed. New tests exercise actual filesystem atomic
replacement, unchanged-write suppression, hidden-widget job suspension and resume,
and prevention of duplicate in-flight refreshes. Existing calendar, sidebar drag,
layout reconciliation, settings, controller and integration tests still pass.
All six native settings pages were captured. Today-only and Today+Tomorrow changes
propagated to both windows; the agenda stayed centered as its height changed.
Selected calendar dates now use a subtle neutral palette tint and readable light
text, with a slightly stronger outline for the current day.

Raw audit artifacts: /tmp/desktop-cards-performance/


Compact cards, refresh scheduling and clock reliability — 2026-10-17

Clock disappearance was a stacking problem, not a process crash. Calendar's
full-height COSMIC backdrop covered the earlier Clock surface, including the
visually empty area above the calendar card. Creating the decorative Clock last
in the same bottom layer fixes this while retaining click-through input and
keeping ordinary application windows above it. Verified on the actual compositor
across three fresh preview starts and the installed service, with Calendar enabled.

Weather now uses a city/time header and no separate update footer. Devbox removes
its title row and places uptime in its status footer. Their combined height falls
by 52 pixels. Calendar's footer shows the account, time and refresh icon. Full
native captures with live weather and calendar data show no content clipping.

Each widget has an independently configurable interval in its Settings page:
Devbox 3 s, Clock 60 s, Projects 30 s, Calendar 5 min, Weather 15 min by default.
Clock ticks align to wall-clock boundaries; meeting highlights update locally
without requesting Google data every minute. Hidden widgets pause their jobs.

The desktop now sleeps until the next actual deadline instead of running every
job from a shared three-second tick. Settings stay in memory between file-change
notifications, and invalid reloads preserve the last valid configuration. Refresh
settings patch only their own fields, preserving unrelated preferences.

System sampling runs off the UI thread. The CPU sensor path and HTTP client are
reused. T3 service checks run separately, at most every 15 seconds, with a timeout.
In-flight jobs cannot duplicate themselves; obsolete project and calendar results
are discarded. Weather retains a location-checked atomic disk cache across starts.
The installed service encountered an Open-Meteo 503 during validation: the saved
forecast stayed visible with an amber timestamp indicator, rather than going blank.

Validation: cargo fmt --check, 38 Rust tests, and 8 Python integration checks pass.
All eight native Settings pages were captured. The release binary was installed
atomically; its running executable hash matches the tested build. User preferences
were preserved and the service reported zero automatic restarts after installation.

Steady desktop samples using /proc/PID/stat, Clock/Calendar/Weather/Devbox enabled:
  Previous release, 58.50 s: 0.120% of one CPU core, 36.74 MiB RSS.
  Revised release, 60.46 s: 0.149% of one CPU core, 36.70 MiB RSS.
Resource use remains essentially unchanged in these short samples. These results
do not establish a CPU reduction; the changes reduce repeated work and separate
background jobs from UI interaction while retaining a similar memory footprint.

Raw audit artifacts: /tmp/desktop-cards-compact-audit/


Settings and appearance audit — 2026-10-17 (evening)

Three parallel reviews covered preferences/controller persistence, all widget
settings pages, and runtime reliability. Appearance now includes a clickable
layout preview, coordinated native controls, Soft/Solid/Minimal presets, opacity,
corner radius, outline strength, and independent wallpaper matching. Clock gains
12/24-hour format and an optional date. Widget pages gain visibility controls,
wrapping refresh choices, a per-widget default reset, and clearer paused states.

Concrete fixes:
- Rust and Python share config.json.lock around read/patch/atomic-replace, so
  concurrent settings edits preserve each other's changes. Nested fields and
  unknown future keys remain intact. Invalid files are not silently replaced.
- Failed theme writes roll back wallpaper, wallpaper collection and preferences
  when the affected files remain writable. Original wallpaper backups are kept.
- Sliders save keyboard/wheel edits as well as mouse releases. A 350 ms one-shot
  debounce coalesces changes; page navigation and both native close paths flush
  drafts. Obsolete debounce generations cannot save over a newer edit.
- Hidden sidebar hover/dismissal delays use one-shot timers; 60 fps animation
  runs only during movement. Project-folder changes clear old rows immediately.
- General controls wait for service status; duplicate city-search submits and
  folder chooser requests are ignored. Small-window navigation remains reachable.

Validation: cargo fmt --check; 45 Rust tests and 14 Python tests pass. Native
settings pages captured at 1020x720 and 820x600; lower controls remain reachable
by scrolling. Appearance verified in all three palettes, neutral-background
mode, and with no widgets selected. Native desktop captures verified opacity,
0/32 px corners, outlines, 12-hour time and hidden date across all card surfaces.

The optimized release was installed atomically and Settings reopened on Appearance.
The running executable hash matches the build; existing preferences were preserved
byte for byte. Desktop and Settings services reported zero automatic restarts.
Restore point: ~/.local/share/cosmic-desktop-cards/backups/20261017-202335-settings-appearance
Raw audit artifacts: /tmp/desktop-cards-appearance-audit/

Observed 40-second sample after installation (all five desktop widgets enabled):
  Desktop: 0.150% of one CPU core, 36.71 MiB RSS.
  Open Appearance settings: 0.125% of one CPU core, 37.10 MiB RSS.
These short observations overlapped normal desktop activity and screenshot
capture. They are not a performance guarantee or a controlled speedup benchmark.
A final appearance-only spacing adjustment followed this sample.


Settings usability and freely positioned widgets — 2026-10-17 (21:20)

Appearance now uses compact palette swatches and a concise style preview. All
primary Appearance, Widgets and Devbox controls fit the 820x600 minimum window.
Nine settings pages were captured at both 820x600 and 1020x720. Calendar overview
switches and the desktop Today/Tomorrow agenda use vertical rows. Desktop cards
have no hover tooltips. Weather search debounces input, labels region/country,
and invalidates obsolete responses; the live geocoder returned Zagreb, Croatia.

Arrange on desktop provides a fullscreen drag editor with Save, Cancel, Reset,
keyboard nudging and normalized persisted positions. Settings also offers drag
reordering of the default right column. Native pointer/keyboard checks verified
movement, Save, Escape cancellation, an unchanged save round trip, an unrelated
settings edit without position loss, and restoration of the default layout.
Independent review caught captured Enter handling and cancellation edge cases.

Native validation found the pinned libcosmic layer Size action synthesizes a
square Configure by using width for both dimensions. Initial size zero correctly
means compositor stretching, but resizing an existing surface to zero can also
create an invalid viewport. The app now takes output measurements only from its
fullscreen probe, avoids Size actions entirely, and recreates card surfaces only
when actual size/layout mode changes. Position-only changes use margins. The
Clock is recreated last to retain its proper desktop stacking. Arrangement hides
old stretched surfaces without changing their required opposing anchors.

Devbox has independent CPU, memory, disk, network, GPU, uptime and service switches.
Network uses real elapsed-time deltas and physical active interfaces. AMD GPU
utilization, VRAM and temperature were verified on this machine. NVIDIA parsing
and unavailable-device behavior were tested; no NVIDIA device was available for
a hardware check. The all-sections-off state was captured and settings restored.
Projects group linked worktrees by repository and show human titles, concise Git
state and server links without generated paths. The local five checkouts now
render as one Voice Agent project.

Validation: cargo fmt --check, 81 Rust tests and 20 Python tests pass. Native
captures show full Weather/Devbox/Projects cards and a compact vertical Calendar
in default and custom positions. The Pop!_OS page opens supported COSMIC system
settings for appearance, wallpaper, dock, panel, workspaces, tiling, keys and
screens. This is a local installation; public account onboarding and distribution
packaging remain future work.

Raw audit artifacts: /tmp/desktop-cards-production-audit/
Restore point: ~/.local/share/cosmic-desktop-cards/backups/20261017-production-205358

The release, controller, project scanner and source were installed. The running
binary SHA256 matches the verified build; desktop and Settings services are active
with zero automatic restarts. Preferences were preserved byte for byte. Settings
was reopened on Appearance. Desktop RSS after startup was 58.5 MiB; this is a
single observation, not a benchmark against the earlier release.


Grid editor, Devbox consistency and coordinated themes — 2026-10-17

Devbox uses six embedded 16-pixel outline icons with consistent stroke weight.
Every metric has the same header/indicator spacing; Network and Uptime use neutral
rules instead of invented utilization bars. All enabled sections with one GPU
fit a 281-pixel card.

Settings Layout uses the desktop's actual geometry in an aspect-correct miniature.
Both editors share Spacious/Balanced/Fine snapping and keyboard step sizes. The
right/bottom inset boundaries compete with regular grid points, so an incomplete
final cell no longer leaves a large gap. Layout saves merge only edited positions
and grid fields; Reset explicitly covers visible cards and preserves hidden ones.
A native refresh previously cleared the pointer before a press; preserving it fixes
interrupted drags. Save errors remain visible through idle pointer/release events.

Six palettes have all twelve 4K/1080p wallpapers bundled offline. The installer
validates PNG data and preserves identical installed copies; theme selection repairs
missing or damaged wallpaper copies from the installed source bundle. The two
Appearance switches independently match wallpapers and COSMIC desktop accents.
COSMIC matching is opt-in, updates both modes using the pinned official builder,
preserves unrelated customization, keeps a first-apply backup and rolls back failures.

Native checks at 820x600 verified Appearance controls fit, uniform Devbox metrics,
miniature drag/save propagation to the desktop, and right/bottom edge placement.
A saved Weather move reached [726,540] on the exact shared grid without moving the
other cards. The final edge drag reached the inset boundary and cancellation left
the saved user layout unchanged. Later user changes to preview themes and layout
are preserved for installation.

All six COSMIC palette changes were applied to isolated copies of the host's actual
theme files. Mode, spacing, backgrounds and unrelated settings were preserved; a
repeated apply was stable, the first backup stayed unchanged, preview writes were
refused, and the real host configuration remained untouched during those checks.

Validation: 117 Rust tests, 37 Python checks, formatting and optimized release build.
New tests cover edge geometry, reset/concurrent saves, failed-save feedback, default
and sparse COSMIC files, exact rollback, opt-in isolation, and offline wallpaper repair.
Audit artifacts: /tmp/desktop-cards-grid-audit/
Restore point: ~/.local/share/cosmic-desktop-cards/backups/20261017-213006-grid-themes

The verified release, controller, full source and all12 wallpapers were installed.
Desktop and Settings are active with zero automatic restarts; the running binary
SHA256 matches the tested build. The user's saved preview layout and appearance
were merged into the installed preferences. A subsequent live Ocean palette
selection applied ocean-4k.png successfully. Settings was reopened on Appearance.
