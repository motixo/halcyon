# Settings structural review

2026-10-27 · Implemented and locally validated. Not installed or published.

The current screens expose the features, but their composition is uneven. Several
editors spend the first viewport on navigation, explanation and a small preview;
the controls that make the preview useful appear below it. Fix the shared page
structures before doing another round of individual visual tweaks.

## Review scope and evidence

Reviewed the 30 `Page` IDs in `src/settings.rs` (27 canonical routes and three
aliases), all six App settings sections, the wizard and the additional setup,
sharing, font, migration and recovery views. Current view code determines the
available functions. Native captures from October 25–27 show their presentation;
older captures are not evidence of current copy or current provider availability.
This is a structural and visual review, not a fresh runtime or accessibility audit.

- Latest screen collection: [capture manifest](../../.task-future-builder-screenshots-20261027/manifest.json).
- Earlier screens: `.task-settings-polish-evidence/gui-final-compact/captures/`
  and the corresponding wide captures/contact sheets.
- Onboarding: `.task-wizard-stall-20261026/headless/`.
- Installation: `.task-install-implementation-20261026/native/`.
- [Interactive concepts and manifest](../../.task-structural-pass-20261027/manifest.json).

## Findings

1. **Useful columns start too late.** `view/preview.rs` and `view/recipes.rs`
   wait until 1100 px; Canvas waits until 1120 px. Below that, the inspector
   follows the preview. Above it, the shared editor still fixes the preview at
   360 px while expanding the controls. Extra width often goes to the wrong part.
2. **Setups has no real side inspector.** `selection_section()` builds a column
   and limits preview height with window height minus 380/430 px. At 820×600,
   normal setup review is capped at 220 px high; Remix at 170 px. Included
   changes, compatibility and library actions trail below the scene.
3. **Navigation and explanations compete with work.** Root selection, destination
   tabs, back links, a title, subtitles, status and preview controls frequently
   precede the actual task. Keep one navigation hierarchy and one page heading.
4. **Actions do not have consistent homes.** Builder and package export start with
   primary actions; setup review uses a footer; direct controls use Undo; Notes
   saves content independently. Preserve these different commitment rules but
   make their placement and wording predictable.
5. **Help already exists, but is applied unevenly.** `Settings::help()` has a
   question-mark trigger, hover tooltip and click-expanded explanation. Some
   nearby screens still repeat full paragraphs or technical state labels. Extend
   this helper; verify focus, Escape, pointer travel and touch access explicitly.
6. **The visual result feels secondary.** Small centered scenes and several nested
   surfaces reduce the impact of the actual desktop designs. Use the wallpaper
   and real widget renderer as the main visual; keep the surrounding controls quiet.

## Four reusable layouts

| Layout | Structure | Used for |
| --- | --- | --- |
| Preview + inspector | Flexible preview left, bounded controls right; local action footer when required | Setups, Remix, Canvas, Builder, Moments, appearance and visual widget editors |
| Gallery | Compact search/actions, fluid equal cards, adaptive number of columns | Discover, saved setups, themes, wallpaper choices and My widgets |
| List + details | Searchable/selectable list, adjacent details on wide screens; focused detail page when narrow | Calendar connections, schedules, shortcuts, floating rules, fonts, applets and change history |
| Grouped settings | Related rows in bounded groups; independent groups share a row when both fit | Updates, Startup, Interface, Privacy, Accessibility, recovery and diagnostics |

Notes is a content editor with a settings inspector. Music and Focus use live
controls, labelled as such. These do not become decorative previews.

### Width and height

- Start with the smallest supported 820×600 window. A proposed 164 px sidebar,
  20 px content margins and 16 px gutter leave about 350 px for a scene and
  248 px for an inspector. The prototype verifies this arrangement in CSS;
  native font metrics and localized labels must be checked before adopting it.
- At 1280×800, use an approximately 184 px sidebar and 312 px inspector. Give
  the remaining width to the preview, document or list. Do not stretch a toggle
  row to the width of a cinema display.
- Split based on **available content width and minimum useful child widths**,
  not assorted window-width constants. Collapse when zoom, localization or font
  size makes the minimums impossible. The layout may stack at the same nominal
  window width with larger text. This proposal does not expand the app's supported
  minimum window size.
- Keep the preview visible while a long inspector scrolls. Preserve focus and
  scroll on refresh. On narrower layouts use a clear Preview/Controls switch or
  a short preview above the controls, with the same actions reachable.
- Full desktop previews preserve the monitor aspect ratio. Widget previews may
  use a purpose-built neutral stage. Preview fit and user zoom must not alter
  the desktop's saved geometry.
- Galleries begin near the top of the content area. Increase column count when
  cards fit; retain readable names and useful thumbnails. Empty states do not
  need decorative placeholders filling the entire window.
- The footer is reserved for pending work, in-progress operations or a useful
  Undo. Healthy maintenance pages do not need an always-empty action bar.

### Text and help

- A label names the thing; a button names the action. Use “Remix”, “Font”,
  “Include”, “Apply setup”, “Save widget”, “Save note”, “Save rule”. Avoid a
  subtitle that restates the title.
- Use one term consistently: “Follow setup” for style inheritance, “Preview”
  and “Current” for comparison, and explicit “Display” labels only where scoped.
- Add “?” only for non-obvious behavior. Hover after a short delay, focus, or
  click opens the same explanation. Click/tap can pin it; Escape and a second
  click dismiss it. Users can move the pointer onto the popup. Opening it does
  not shift settings rows. Link-bearing help uses an accessible popover/dialog,
  not a tooltip with focusable controls.
- Keep the trigger next to its label, with a stable hit target and accessible
  name. Limit help to one or two short sentences, with a separate details path
  for longer information. Keyboard order follows the visible reading order.
- Keep errors, unavailable controls and their reasons, installation consequences,
  permission choices, privacy fields, destructive effects and data-loss warnings
  visible. A question mark must not be required to discover what Apply will do.

| Current copy or pattern | Proposed visible text | Help or details |
| --- | --- | --- |
| “Make a Clock or Weather widget from native parts” | “Widget builder” + Clock / Weather selector | What can be built; Save versus Apply |
| “Sources are copied…” below every mix | “Sources” section | Sources are copied; later edits do not change the mix |
| Long inheritance description | “Follow setup” | Which base values are inherited |
| “Illustrative content · Standard panel…” | “Sample desktop” + relevant panel status near panel controls | What the preview can represent |
| “Choices apply directly. Adjustments finish…” | Brief saved/changed status when relevant | Direct-edit behavior beside unfamiliar controls |
| Depth requirements paragraph | “Behind landscape”; visible “Requires Ridge” when unavailable | Full supported combination and fallback |
| Repeated Save/Apply instructions | Specific action label and local state | Explain library copy versus desktop use |

### Consistency and motion

Use COSMIC theme colors, spacing, native symbolic icons, focus and selected
states. Keep Settings chrome opaque. A wallpaper belongs inside its preview,
not behind a form. Use 16/24 px group spacing, roughly 40–48 px setting rows and
readable 13–14 px control text as starting values; derive final values from the
native theme. The mockups are not a replacement visual toolkit.

“Dynamic” means responsive layout, immediate local preview, stable selection and
clear operation progress. Use brief 120–180 ms feedback where it clarifies a
change; honor reduced motion. No background animation loops, new polling, repeated
wallpaper decoding or unsolicited provider requests. Do not animate a panel
transition that hides an error or changes where a primary action is located.

## Screen-by-screen disposition

| Screen / view | Current structural issue | Proposed treatment |
| --- | --- | --- |
| Theme & colors | Preview and introduction push color controls down; scope and direct mode changes compete | Preview + inspector; color source/scope together; mode separate; retain local candidate Apply |
| Theme/style browser | Repeated introduction and descriptions reduce gallery density | Compact search and gallery; names and useful visual differences; details on selection |
| Surface finishes | Scope, four specimens, preview and Adjust stack vertically | Flexible live specimen + finish/adjust inspector; scope stays visible; decorative capability stays accurate |
| Wallpaper | Large preview section precedes source, fitting and display controls | Scene + source/fit/display inspector; thumbnails below scene; fixed Apply/Cancel |
| Fonts & icons | Also contains colors/window appearance; first viewport is preview-heavy | Short text specimen, searchable font chooser; clear Fonts, Icons, Window appearance sections |
| Font chooser | Long list/search can lose the comparison specimen | Search and result list beside a stable sample; current family always visible; reveal exact result |
| Dock & panel | Preview, object selection and placement/style groups stack | Scene + inspector; Dock/Panel selection in header; Applets section beside/below scene |
| Islands controls | Intent, package status and host health are easy to conflate | Preserve selected style; separate package/activation/runtime state; visible fallback and recovery action |
| Applet editor | Many named actions can form a wall of controls | Ordered list, selected applet detail; accessible move arrows and removal; no hidden unavailable items |
| Windows & workspaces | Tall schematic preview before common tiling controls | Compact schematic beside Tiling/Focus; independent Workspace group; consequential changes visible |
| Floating apps | Sparse empty state; nested explanations | Searchable rule list with one Add action; right detail editor only when selected |
| Desktop icons | Large sparse icon specimen wastes height | Small specimen beside size/spacing and item toggles |
| Shortcuts | Empty/list/edit/replace states vary in hierarchy | Searchable command list with detail editor; conflict confirmation next to the binding |
| My widgets | Information and Tools remain stacked until wide breakpoint | Responsive two-column groups; name, enabled state and detail affordance; no fake live data |
| At a glance | Sections and appearance stretch into a long list | Small combined sample with sections and density inspector; explain replacing separate cards when relevant |
| Clock | Preview precedes several style groups; font and face controls scattered | Clock sample + Face/Time/Style inspector; typography nearby; depth availability visible |
| Calendar | Connections, privacy and appearance share a long column | Connections/events area + agenda/privacy inspector; disconnected state leads with connection action |
| Calendar connection forms | Provider limitations and setup instructions can dominate | Focused form with only relevant provider fields; visible availability/error; technical setup under details |
| Weather | Missing location and healthy display need different emphasis | City search leads when empty; healthy sample beside units/details/style; stale age remains visible |
| System monitor | Preview and readings list separate selection from result | Preview + reorderable readings inspector; selected reading reveals its options |
| Projects | Folder entry, explanatory text and styling pile up | Folder list with Add/Browse; preview or scan state adjacent; errors attached to the folder |
| Music | Live actions look like a sample preview | Clearly labelled live player area + player/style controls; bus/player error remains visible |
| Notes | Preview can compete with the real document | Document gets the main width; Save/Revert beside it; size/style/location inspector; no global Apply |
| Focus timer | Live session controls and configuration use similar framing | Live timer with unmistakable Start/Pause + duration/behavior inspector; hiding does not imply stopping |
| Layout & displays | Several scopes and controls precede a small scene | Scene + selected widget inspector; separate preview display, default destination and override |
| Desktop canvas | Two columns start at 1120 px; fixed scene heights | Fit scene to available stage, independent inspector, visible object list and pending action footer |
| Discover / saved setups | Navigation and search take much of first viewport | One gallery toolbar; 2/3/4 columns as useful card width allows; important metadata on cards |
| Setup review / imported setup | Scene centered with unused side space; included changes below | Scene + included domains/compatibility inspector; visible fallback; Apply/Cancel always in reach |
| Remix | Source slots consume top row; preview below; choices move away | Stage left, Picture/Style/Layout source rows right; source picker replaces only that inspector |
| Remix source picker | Source grid replaces the work context | Search/list inside inspector; chosen component and desktop preview remain visible |
| Export package | Primary action appears before fields; no clear package summary | Setup thumbnail/contents summary + credits/picture form; Save package footer; sharing disclosure visible |
| Sources & revisions | Repeated hashes and long URLs dominate | Collapsed source rows with name/author/license; full revision and URL in accessible details |
| Save preview / library operations | Many equal-weight text actions | Secondary menu for rename/duplicate/delete/export; keep Remix and Apply prominent; preserve confirmation |
| Moments | Activating, previewing and saving style are spread vertically | Day/Night selection beside persistent preview; style inspector; separate explicit activation; no wallpaper switch |
| Schedules & display rules | Status, rule list, edit form and history can form a very long page | Rules + selected rule editor; next switch visible; recent switches disclosed below |
| Widget Builder | Actions and sample fill compact first viewport; parts below | Preview + parts inspector at supported compact width; Save library copy distinct from Apply |
| Builder empty/missing states | Easy to confuse sample content with provider status | Sample/Missing-location control within preview toolbar; required primary reading visibly retained |
| Accessibility | Unsupported capability descriptions dominate otherwise useful links | Actionable Motion/Readability groups first; accurate capability explanation in details, reason visible |
| Startup & widgets | Independent operating choices and onboarding links stack | Operation group beside Getting started; preserve global visibility/background/startup distinctions |
| Interface | Short controls stretch across large widths | Bounded Edge menu and Settings groups; no decorative preview or empty status footer |
| Privacy | Dense explanatory text but some disclosure is essential | Reporting choice and visible data summary beside content exclusions; detailed policy link |
| Updates & installation | Updates and optional Islands share a long vertical page | Separate adjacent groups, each with its own observed state/action; details expandable |
| Islands install/activation | Confirmation can look like generic settings | Local staged confirmation; explicit system authorization, separate enable-next-login state |
| Restore & recovery | Routine backups, migration and interrupted work compete | Active problem first, then Backup/Restore groups; focused review for each risky action |
| Migration / design conflict | Long technical copy with multiple equal actions | One decision per focused review; retained data and compatibility consequences visible; details optional |
| Recover unfinished work | Empty state fills a large page; multiple recoveries difficult to scan | Compact empty state; otherwise selected draft list + change/conflict review |
| Help & diagnostics | Mixed local observations and support operations | Attention first, healthy status second; review export before sharing; refresh stays explicit |
| Recent changes | Long record detail may bury what happened | Readable timeline/list with outcome, cause and time; details on selection; Undo eligibility accurate |
| Wizard welcome/look | Large explanation before visible choices | Short welcome; setup gallery as the main object; progress and optional Skip visible |
| Wizard bars/widgets | Preview and choices need to be read together | Persistent preview + selectors/list; connectivity requirements adjacent to affected items |
| Wizard arrange/ready/weather | Step transitions and optional substeps must remain clear | Stable Back/Skip/Next; order list + preview; final change summary; inline errors and progress |
| Search, pickers and dialogs | Reveal and navigation must survive the restructuring | Same routes/anchors; search opens the exact disclosure and focus target; preserve local drafts |

## Preservation rules

This is presentation work. Keep the current six roots, canonical/legacy routes,
search anchors, per-display identity, cancellation, Undo, conflict handling,
receipts, library/import/export and installation trust paths. Do not introduce a
global Save basket. Direct choices remain direct; setup/theme/wallpaper/canvas
candidates retain explicit Apply; notes/rules/library objects retain their own
Save. Do not silently activate Islands, change desktop mode with a theme, move
widgets when changing preview display, or switch wallpaper with a moment.

Mockups use repository wallpapers and illustrative widget content. They represent
structure, not precise native widget rendering, native focus verification, package
availability or measured desktop performance. The 24 interactive views demonstrate
the shared layouts; the table covers additional screens that use those layouts.

## Proposed implementation order

1. Shared layout primitives, compact header, help behavior and operation footer.
   Validate native 820×600, 1280×800, enlarged text, long labels and dark/light.
2. Setup review, Remix and discovery. Review before continuing: included domains,
   source picker, current/candidate comparison, fallback and library operations.
3. Canvas, Builder, Moments and visual appearance editors. Review geometry,
   pending state, live preview behavior and unchanged operation semantics.
4. Widget content, list/detail screens and maintenance. Review empty/error/live
   states, privacy disclosure, Notes Save and installation/recovery sequences.
5. Onboarding and full navigation/search pass. Review keyboard traversal,
   tooltip accessibility, blocked actions, resize/reflow and display scaling.

Each slice gets a focused review and correction before the next. Reuse the
existing Rust/native harness and captured fixtures. Do not run unrelated broad
test campaigns or reinstall the host merely to validate layout work.

Acceptance: at the supported compact size, no horizontal overflow or clipped
primary actions; preview and useful controls share the first viewport where the
chosen layout permits; no explanation is hidden that is necessary for consent
or recovery; focus and drafts survive reflow/refresh; help works on hover, focus,
click/tap and Escape; no new idle activity; only real retained capability states
appear in native screenshots. Current implementation validation is recorded below. Physical display and screen-reader qualification remain separate.

## External references

Prototype checks: all 24 views were checked at 820×600 and 1280×800 (48 layout
cases), with no measured horizontal overflow in their content, groups or action
bars. Help focus, Escape, click-open and click-dismiss checks passed on all six
boards. Compact mobile embedding and selected wide/light views were inspected.
These checks cover the HTML proposals only. [Results](../../.task-structural-pass-20261027/validation.json).

- [COSMIC Toolkit](https://pop-os.github.io/libcosmic-book/): native components,
  responsive library and theme integration. The implementation remains libcosmic.
- [GNOME adaptive layout guidance](https://developer.gnome.org/hig/guidelines/adaptive.html):
  constrained-size-first layouts and useful container widths. This informs sizing;
  it does not replace COSMIC styling.
- [GNOME tooltip guidance](https://developer.gnome.org/hig/patterns/feedback/tooltips.html):
  useful supplementary help; essential information stays visible.
- [W3C content on hover/focus](https://www.w3.org/WAI/WCAG22/Understanding/content-on-hover-or-focus.html):
  dismissible, hoverable and persistent help. Use these interaction requirements
  as an accessibility baseline, not as a claim of native WCAG certification.


## Implementation record

The five slices are implemented in the existing Rust/libcosmic Settings UI.
Shared columns use available content width and minimum useful child widths; their
widget trees remain stable when they stack. Visual inspectors scroll separately.
The native color picker has its own wider minimum, including surrounding padding.
At the compact size it stacks rather than extending outside the window.

Setups and Remix keep their scene beside included changes or source choices.
Gallery columns adapt to the available space. Canvas, Builder, Moments and visual
editors use the same stage/inspector structure. Builder and package export keep
pending actions in their local footer. Notes gives the document the main width;
Calendar and Projects lead with their actual content and connection/folder actions.
Independent maintenance groups share a row when they fit. Active recovery reviews
retain their full-width presentation.

Supplementary help opens on hover, keyboard focus or click and closes with Escape
without shifting rows. It has no polling loop. Search and keyboard focus reveal
controls through nested scroll containers. Consent, errors, capability limits and
recovery consequences remain visible. View-only help/disclosure changes bypass
the native work queue and do not create persisted draft checkpoints.

Onboarding uses the same responsive structure for welcome, selection, ordering
and final review. Dock/panel choices retain two compact columns so every choice
remains above the footer at 820×600. Rendered-step action fencing and the existing
Next/Skip, Apply, Cancel and Undo paths are unchanged.

Implementation scope: Settings state/routing/view code and focused regressions.
No dependency, desktop runtime, provider, transaction-store or packaging changes.
No installation or publication is part of this pass. Baseline hashes and the
file-level record are retained in
[implementation evidence](../../.task-structural-implementation-20261027/implementation-files.json).

Validation: **75 focused tests passed; two existing performance probes ignored**.
Native headless captures cover all 27 canonical pages, six App settings sections
and onboarding, with compact/wide and dark/light/high-contrast cases. Three private
COSMIC release-window runs passed at 820×600, 1280×800 and 150% scaling; each kept
fixture preferences unchanged and cleaned up its processes. Enlarged renderer text,
long controls, nested search/focus, errors, Apply/Save/Undo and help dismissal have
focused checks. Formatting and whitespace checks pass.

[Full implementation evidence](../../.task-structural-implementation-20261027/report.md)
· [Native screenshots](../../.task-structural-implementation-20261027/gallery.html).
Physical-display, screen-reader and final packaged-release qualification remain
separate; these results do not claim them.
