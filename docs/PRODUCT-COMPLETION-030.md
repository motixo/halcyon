# Updates, support, recent changes and release presentation

Status: implemented and locally validated, 2026-10-25. The user requested items 3, 5, 6 and 7 from the
product-gap review, with a plan first. This document covers those four items.
Implementation is local and unpublished; see the acceptance record below.
Updated after the product-quality review: clearer primary actions, problem-first
support, readable change details, consistent release assets and explicit usability
acceptance.

Follow-up decision, 2026-10-25: clean main-package installs now enroll in system
updates by default, including clean upgrades of older unenrolled manual packages.
This supersedes the opt-in default below; the interactive path remains for eligible
older manual installations. Existing disabled/deleted/modified sources and OS
installation policy are preserved. See [distribution behavior](DISTRIBUTION.md)
and `.task-default-updates-evidence/acceptance.json`.

## Intended experience

People can opt into system updates, understand whether Halcyon is working,
explain a desktop change, and see documentation that accurately represents the
build they can download.

Keep the six Settings navigation roots. Within App settings, retain Startup,
Interface, Privacy and Updates; add **Help**, and extend **Recovery** with a
**Recent changes** entry. Recent changes is a focused subpage with Back, not
another root or another row of permanent tabs. Search reaches both destinations.
The six short App settings tabs must fit at 820×600. Existing routes and saved
navigation keys keep working.

Use native COSMIC controls, quiet grouped rows, contextual recovery actions and
short explanations. Healthy states should not fill the screen with banners.
Keep status, troubleshooting and the current operation's progress distinct.
Use one primary action per task. Keep package verification, transaction IDs and
configuration keys in technical details rather than routine user-facing copy.

## 1. System updates — review item 3

### Reuse and current gap

`src/settings/updates/{identity,packagekit}.rs` already verifies the running
executable, dpkg ownership and the exact owned APT configuration. It has bounded,
cancelable PackageKit discovery and a verified Store launcher. The current manual
package screen still says enrollment is unavailable. The explicit published
bootstrap and its checksums are recorded in `scripts/apt/distribution.json` and
`docs/DISTRIBUTION.md`.

### User flow

1. Show **Running version**, **Installed version** when different, and the actual
   update channel. Replaced binaries get a restart/recheck explanation, not a
   misleading current-version result.
2. For an eligible manual package, offer **Enable system updates…**. Explain that
   this adds Halcyon's package source and public key; the OS manages updates.
   Islands remains separately installed and activated.
3. Prepare the two exact bootstrap packages on a worker, verify them, then offer
   **Continue with system installer**. Authentication belongs to the OS. Report
   download, verification, authorization, installation and recheck separately.
4. Confirm success only after re-reading the installed source/keyring/ownership
   and passing the existing APT identity checks. Interrupted or partial installs
   remain explicitly incomplete and can be rechecked.
5. Enrolled users keep **Check for updates** and **Open COSMIC Store**. Retain
   manual-download and source-build paths. Preserve deliberately disabled or
   administrator-modified sources; display their reason and appropriate guidance.
6. Give each state one primary action: **Enable system updates…**, **Check for
   updates**, **Update in COSMIC Store**, or restart/reopen guidance. Do not label
   a Store handoff as an installation completed by Halcyon. Any restart action
   must preserve pending edits and report what needs restarting.
7. Show the last successful check and version-specific release notes when the
   installed or available version is verified. A failed check retains the previous
   successful timestamp and clearly reports that the latest attempt failed.

### Implementation decisions

- Add structured enrollment states: not enrolled, verified/enrolled, explicitly
  disabled, partial/conflicting, source build and unknown. The existing
  `ManualDeb` result currently collapses absent and disabled sources; separate
  them before offering installation. Ordinary refresh must never re-enable a
  source. Re-enrollment of a deliberately disabled source requires an explicit
  product action with a clear explanation.
- Use packaged, reviewed bootstrap metadata derived from the distribution
  manifest, with exact asset identity, SHA256, size and trusted origin. Do not
  resolve a mutable “latest” release or treat a same-download checksum as trust.
  Allow only the documented HTTPS release redirect path; bound download size,
  redirects and deadlines. Private temporary files must remain stable through
  verification and installer handoff; reject symlinks and replacement races.
- Prototype PackageKit `InstallFiles` with both packages in one transaction in a
  disposable environment first. The installed Noble interface documents local
  package authorization and distinguishes it from repository trust. Inspect its
  supported role and preserve normal PolicyKit authentication. Do not silently
  downgrade a rejected repository trust check or create a general root helper.
- If the backend cannot perform local installation, present the verified files
  and exact system-package instructions as an explicit manual fallback. Merely
  opening a guide is not an enrollment success.
- PackageKit may disallow cancellation once installation starts. Respect
  `AllowCancel`, explain that state, and never kill a package-manager transaction.
  After Settings closes or the bus disappears, a later inspection determines the
  outcome. Keep staging files until the installer is finished with them.
- Keep scoped `Signed-By`, package pins, checksum checks and existing OS ownership
  protections. No application self-updater or password input is introduced.

### Acceptance

- Cover enrolled, clean manual, explicitly disabled, modified source/key, partial,
  source-build, replaced-executable and unknown-installation states.
- Exercise hash mismatch, truncated download, unsupported backend, denied/canceled
  authorization, installer failure, restart during a transaction and recheck.
- Verify local package installation and source enrollment in a disposable OS
  environment; use a private D-Bus service for deterministic signal/race tests.
  The installed workstation source is not a test fixture.
- Success implies verified installed enrollment. Failure never claims an update
  was installed. Optional Islands and unrelated APT configuration are preserved.
- Each state has one clear primary action. Last-checked information cannot imply
  freshness after a failed check; notes match the displayed version. Restart or
  installer handoff does not discard pending edits.

## 2. Help & diagnostics — review item 5

### User flow

Add **App settings → Help** with the title **Help & diagnostics**. Show compact
rows for the app/version and update channel, widget background service, Islands,
Calendar, display layout/scaling and automation. Weather/provider freshness may
be included where existing cached observations support it. Use **Not checked**,
**Unavailable** and **Stopped** distinctly.

List actionable problems first, with one relevant action per problem: Open
Calendar, open Updates, check Islands, review recovery, identify displays or open
background controls. Keep healthy components quiet and technical detail collapsed.
An optional component that is uninstalled, disconnected or stopped by choice is
a neutral state, not an error. An enabled but unhealthy feature retains its
selected intent and displays the actual failure.

Existing pages continue to own their mutations. **Refresh status** is secondary.
**Preview report** opens a readable report with **Copy report** and **Save report…**
actions, rather than putting every report action on the overview. Include
**Report a problem ↗** without sending or prefilling private diagnostics; the user
chooses what to share.

### Implementation decisions

- Introduce a typed, bounded status snapshot assembled from the current service,
  installation, Calendar, update, display and automation adapters. Include an
  observation time and an explicit reason when a component cannot be read.
- Reuse existing native-worker admission, cancellation and generation ordering.
  Refresh runs off the UI thread and does not add continuous polling. Opening
  Help does not sign in, refresh remote accounts, start widgets, run updates or
  change the desktop. Network checks remain explicit component actions.
- Create a versioned report from an allowlist of fields: app/package/platform
  versions, verified channel/status codes, display dimensions/scales with generic
  labels, renderer information if actually known, and sanitized component errors.
  Reuse the existing Islands diagnostics sanitizer; exclude arbitrary error text.
- Exclude credentials, calendar/feed URLs, account addresses, notes, project
  paths, home paths, wallpaper filenames, commands, device serials, reporting
  identifiers, custom setup/rule names and raw history/configuration/journals.
  Local screen details and exportable report fields are different projections.
- Preview exactly the report that will be copied or saved. Save through the
  existing file-dialog flow and a bounded atomic private-file writer. No report
  upload or telemetry is added. Usage sharing, if shown, reflects the recorded
  local choice and verified delivery state only; unknown delivery stays unknown.

### Acceptance

- Useful healthy, stopped, unavailable, stale and failed states at compact/wide
  sizes, light/dark/high contrast and 150% scale.
- Problems precede technical data; each has a relevant next action. Optional
  unused features do not produce warnings. Report preview is readable without
  understanding JSON or internal status codes.
- Slow or failed adapters cannot freeze the window or overwrite newer results.
- Inject private sentinel strings into every source and verify they cannot appear
  in copied/exported reports. Confirm preview/copy/export use identical content.
- Actions reach the existing owning page and preserve drafts. Status viewing is
  read-only and never triggers a provider or privileged operation.

## 3. Recent changes — review item 6

### User flow

Add **App settings → Recovery → Recent changes**, also reachable from Help and
search terms such as “why did my desktop change”, “history” and “automatic”.
Show time, a plain-language action, affected areas and cause: **You**, **Schedule**,
**Display rule** or **Wallpaper colors**. Examples:

- “Applied Deep work — scheduled for 09:00.”
- “Updated colors from the wallpaper — Appearance and widgets.”
- “Changed panel spacing — you changed this in Dock & panel.”
- “Kept your manual changes — the scheduled switch was skipped.”

Link to the responsible settings/rule. Show the current automation pause or
manual-override explanation beside relevant entries. Offer Undo only for the
action currently eligible through the existing guarded Undo API. Older entries
explain what happened and offer navigation; they are not arbitrary restore points.
Group entries by local day, newest first. Each entry answers **what changed**,
**why**, and **whether Undo is available**. Expand details to show safe before/
after values when known, such as “Panel spacing: 6 → 12 px”. Use friendly setting
names; keep transaction IDs, native keys and raw configuration out of the view.

### Reuse and persistence

The store already retains up to 24 transaction records, with `Manual`, `Derived`
and `Scheduled` provenance. Automation retains six outcomes including waiting,
failed, replaced and kept-manual states. These are useful foundations, but the
Undo history preserves mainly manual actions; intermediate derived/scheduled
actions can disappear. It is not a complete chronological activity log.

- Add a compact metadata summary to the existing journaled history record for
  future committed actions. Keep at most **50 summaries / 64 KiB**; store IDs,
  timestamps, cause, action kind and affected domains, without copying raw before/
  after configuration. Preserve the existing 24-entry Undo retention separately.
- Permit bounded before/after details only for explicitly allowlisted scalar
  settings, such as spacing, color, on/off and named options. Exclude content,
  paths, URLs, commands, account/device identifiers and arbitrary strings. Values
  unavailable in older records remain unavailable; never reconstruct them from
  today's configuration. These details count toward the same 64 KiB limit.
- Commit summaries with the successful existing transaction boundary, including
  Undo/Restore. A rejected or rolled-back Apply cannot leave a successful entry.
  Preserve summaries through the existing empty Undo record when all reversible
  actions have been undone. Avoid a second independently written event log.
- Make the field additive for legacy records. Unknown/damaged optional summaries
  must not make an otherwise valid Undo record unusable. Validate count, bytes,
  string lengths and timestamps independently. Do not backfill invented events;
  show available older observations and disclose that earlier history is partial.
- Merge successful transactions with existing automation outcomes. Add an optional
  committed transaction ID to new applied outcomes if needed for reliable
  correlation. Deduplicate on proven identity, not similar labels or timestamps;
  unlinked older automation observations remain identifiable as such.
- Coalesce at the existing completed-action boundaries, not every slider frame or
  provider refresh. Do not infer outside-application changes or record note edits,
  keystrokes, visited settings pages or private provider content.
- Expose a validated read-only summary API rather than parsing internal Undo JSON
  in the view. Load on a worker and refresh using existing revision notifications.
  The history remains local and is excluded from usage reports/support exports.

### Acceptance

- Manual, derived, scheduled and display-triggered changes receive accurate causes
  and timestamps; failed/waiting/no-op schedule outcomes are not labeled Applied.
- No duplicate successful entry across commit/completion/restart; no success entry
  after interrupted rollback. Limits and legacy/damaged-summary handling pass.
- Undo still uses the exact shown eligible ID and existing conflict review. External
  edits, a newer transaction, failed recovery and stale workers cannot be overwritten.
- History browsing itself makes no changes. Current pause/manual-override and
  schedule semantics remain intact; old entries never promise unsupported reversal.
- Day grouping and readable value changes remain correct across timezone changes.
  Details contain only allowed values; older entries do not invent missing values.

## 4. Release presentation — review item 7

### Copy and assets

Audit README, screenshot/gallery provenance, AppStream, distribution/Updates
documentation, release notes, changelog and the release template against actual
behavior. Concrete existing problems include the template's “no settings toggle”
reporting claim, download-only update copy, historical native-blur images, and
AppStream's “widgets that never overlap” and unreleased-0.2.5 wording.

- Lead with the actual desktop result, followed by customization and the short
  installation path. Keep detailed compatibility, diagnostics and package
  instructions linked separately. Use direct descriptions of demonstrated
  behavior rather than superlatives or unsupported performance claims.
- Prepare concise 0.3.0 notes covering the redesigned Settings, six Setups and
  Islands compositions, displays, Calendar capability, Updates, Help and history.
- Describe decorative finishes accurately; distinguish main-app functionality,
  installed/active companion requirements, optional enrollment, configured Google
  sign-in, calendar subscriptions and confirmed collector delivery.
- Capture a small coherent set from the final local implementation: an Everyday
  desktop with Islands, Writing desk/light, a standard-panel desktop, Setups,
  Appearance, Dock & panel, Layout, Updates, Help and Recent changes. Include the
  four finish comparisons only if the presentation continues to promise them.
- Use native windows/desktops and synthetic content. Settings' illustrative scene
  is labeled as a preview; it is never presented as a live desktop capture. No
  image-generated UI substitutes. Record build hashes, fixture state, sizes and
  scope; inspect compact/wide and 150% captures. Capture the public set from the
  same final app build and matching companion, with consistent framing, scale,
  typography and sample content. Label optional Islands explicitly. Historical
  evidence stays separate from the current product gallery.
- Keep public PNG/WebP assets and concise provenance; retain large raw captures
  locally. Check links, alt text, image sizes and AppStream validity. Release links
  are tag-pinned; screenshots/docs on an old published release are not relabeled.
- Draft upcoming-0.3 material separately from released-0.2.5 claims until final
  package qualification. Any Store/listing copy must not imply accepted listing,
  full hardware qualification, or configured sign-in without evidence.

### Acceptance

The same claims appear consistently in the app, README, distribution guide,
AppStream and generated release text. Check the actual release-notes generation
path rather than assuming editing the template alone changes publication.
All screenshots reflect the implemented states and all links resolve in their
intended version. Publishing/tagging/deploying and final package qualification
remain separate release work; this pass prepares reviewable local material.

## Shared product acceptance

- Native COSMIC theme, spacing and controls; readable light/dark/high-contrast
  states and enlarged text without clipped labels or primary actions.
- Complete each flow using only the keyboard. Focus stays visible and scrolls
  into view automatically; tests must not use mouse scrolling to reveal it.
  Verify screen-reader names, roles, state changes and focus order in a real session.
- Progress appears during long work; cancellation, retry and recovery accurately
  reflect the underlying operation. Disabled controls have an adjacent reason
  or explanation reachable by keyboard, rather than an unexplained dead end.
- Measure interaction responsiveness and sustained idle/resource behavior on
  reference hardware. Record measured results separately from targets; native
  screenshots and unit tests do not establish battery life or input latency.
- Final acceptance still requires real-device update/recovery and longer-session
  battery/performance checks. Where hardware or accessibility support is missing,
  record the exact unverified behavior instead of claiming completion.

## Work order and checks

1. Finish structured installation/enrollment states and prove the OS installer
   handoff. Implement Updates with the verified manual fallback where required.
2. Define the shared status/report model and bounded history summary API. Integrate
   changes into the existing workers and journal without changing Undo semantics.
3. Implement Help and Recent changes together, with search/navigation links,
   accurate empty/error states and report preview/export.
4. Run focused installation, sanitization, history/journal, automation and Undo
   regressions. Then run the affected Settings suite once the UI is integrated.
5. Perform private native interaction checks at 820×600, 1280×900 and 150%, plus
   enlarged text, keyboard-only navigation and direct-launch routes. Exercise
   success, rejection, cancellation and restart paths against the resulting binary.
   Record screen-reader, real-device update/recovery and longer-session hardware
   results separately; retain any outstanding acceptance gaps.
6. Capture final screens and update presentation copy/metadata. Bind evidence to
   the resulting artifacts and record remaining external release checks.

Use the established single Cargo slot, Rust 1.93.0, locked/offline builds and
`/tmp/halcyon-app-target`. Preserve prior dirty work and evidence. No repeat of
unchanged broad/native suites solely for paperwork. No new dependency is expected
unless the installer proof reveals a justified need.

Full personal backup, mixed Calendar providers, translation, Flatpak/Store
submission, live collector deployment and physical hotplug qualification are
outside these four implementation items. Existing obligations remain tracked in
`PLANNED_WORK.md`; they are not marked complete by this plan.


## Local validation — 2026-10-25

Implemented Updates enrollment, Help and diagnostics, Recent changes and the
release presentation changes. No package was installed into the desktop session;
no release, tag, upload or deployment was made. Published 0.2.5 remains unchanged.

- **Installer:** both pinned public packages passed download/hash verification and
  real Noble PackageKit `InstallFiles` in a disposable Ubuntu 24.04 container.
  The source and keyring were verified afterward; neither main app nor Islands
  was installed. Private-bus/identity tests cover denied or canceled authorization,
  early signals, wrong senders, altered files, missing/disabled/partial enrollment
  and retained last-successful status. A failed or unknown outcome never means
  enrollment succeeded.
- **History:** the existing journal commits bounded summaries with the desktop
  change. Store tests cover rollback, Undo, legacy/damaged optional metadata,
  retention and automatic changes. Palette acknowledgements without setting
  changes are omitted. Automation observations use exact committed IDs for
  deduplication; no timestamp/name guessing is used.
- **Help:** direct opening works before any desktop editor has been opened.
  Reports are allowlisted and previewed before copy/export. Sentinel privacy tests
  and generation ordering cover unavailable/stale results. No report upload exists.
- **Settings:** the affected suite recorded 384 passing cases and one stale search
  expectation; the corrected search/navigation module then passed all eight cases.
  Three pre-existing ignores remain. Focused product and keyboard checks passed.
  Tab navigation scrolls the focused control into view without mouse input.
- **Native:** private COSMIC windows exercised compact 820×600, wide 1280×900,
  light/dark and compositor-verified 150% scale, repeated launch forwarding,
  unchanged widget selections, a real fixture bar edit and clean Settings exit.
  Desktop captures use actual widgets, background, settings daemon and companion.
  Writing desk's missing wallpaper contrast setting was corrected and tested
  through setup Apply and a subsequent dark-setup switch.
- **Presentation:** the new gallery records image and binary hashes; release notes
  render locally and AppStream passes offline validation. Published-release
  screenshots and changelog history are distinguished from development material.

Evidence: `.task-product-completion-evidence/acceptance.json`, the test/build logs,
`packagekit/result-2.log`, `native-gui-result.json` and
`native-desktops-result.json`. Public captures and their exact scope are in
[screenshots/030](screenshots/030/README.md).

Installer policy: Halcyon does not expose cancellation after handing the files to
PackageKit and never kills a package-manager transaction. The OS authorization
prompt can cancel before installation. Closing Settings or losing the bus leaves
staging available and requires a new installed-state inspection. This avoids
promising cancellation at a point where the backend may reject it.

Still unverified: normal-user graphical PolicyKit authentication; physical
hotplug/resume and mixed-monitor hardware; enlarged text and a dedicated
high-contrast configuration; screen-reader semantics (the pinned libcosmic `a11y`
feature is not enabled); sustained hardware latency, battery and long-session
behavior. Five-second software-renderer resource samples are fixture observations,
not performance qualification. Live Google provider acceptance and collector
delivery remain separate tracked work. Final packaged app/companion qualification
is required before publishing 0.3.0.
