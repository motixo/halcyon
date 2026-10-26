# Onboarding widget choices and layout

Status: implemented locally; unpublished. Requested 2026-10-25. The native COSMIC
onboarding now includes widget selection and ordering. Keep the desktop preview central,
short copy, clear defaults and a visible Back/Continue footer.

[Interactive concept](../.task-onboarding-concept/onboarding.html)

## Flow

Welcome → Starter setup → Dock and panel → **Your widgets** → Ready.
Your widgets contains two short screens: **Choose** and **Arrange**. Retain the
existing Weather screen as an optional substep within Widgets, only when a visible
weather section needs a city. Progress shows four meaningful groups after Welcome;
it does not add a permanent navigation destination.

1. **Choose:** start with the selected setup's widgets. Eight compact choices cover
   Clock, Weather, Calendar, System monitor, Notes, Focus timer, Music and Projects.
   Each has a native symbolic icon, one short description and a clear on/off state.
   A separate **Keep clock, weather and calendar together** switch controls At a
   glance. Its child selections remain visible; no duplicate standalone surfaces.
   Initial grouping follows the selected setup/current configuration.
2. **Arrange:** show only selected desktop surfaces in a numbered list. Reorder by
   dragging, Move up/down buttons, or Alt+Up/Down. The preview moves the matching
   surface and keeps the selected row in view. Explain the order once: **Top to
   bottom, then the next column.** A local Undo move action and Use setup order
   action make experimentation reversible. Reset order does not change selection.
3. **Ready:** show the resulting desktop, selected widget count and explicit
   start-at-sign-in choice. Finish starts the existing completion flow. Calendar
   connection, project folders and further customization remain available in
   Settings; they do not block onboarding. Offer relevant setup shortcuts after
   completion, without starting providers, music or the timer from the preview.

All-off is valid. Say **No widgets selected** and allow completion without forcing
one on. On a fresh setup, do not start the widget service for an empty selection;
keep startup intent explicit. Reopening onboarding must not stop an existing
service merely because an empty local draft was previewed.

## Interaction and visual design

- Opaque, native theme-aware application surfaces; one accent for selection and the
  primary action. Use bundled wallpaper and the shared widget miniatures in the preview.
  Geometry follows the applied placement plan; content is explicitly a sample.
- At 820×600, put choices/order on the left and a compact desktop preview on the
  right. Scroll the choice area when necessary; keep the footer visible. Wider
  windows give the preview more space. The HTML concept stacks on phones for
  reviewing in chat; this is not a proposed mobile application.
- Names and on/off state remain visible without hover. Off cards stay readable.
  Use native focus rings, accessible names, switch state and reorder announcements.
  Dragging is optional; keyboard and button operation have the same result.
- Empty content uses neutral copy: **Connect later**, **Choose folders later** or
  **Open a player**. It is not an error and does not disable the choice.
- Continue from Choose opens Arrange. **Use these widgets** on Arrange commits
  the widget selection and placement together. Weather setup follows only if
  required, otherwise go directly to Ready. Back retains the draft. **Keep current
  widgets** explicitly skips this customization; avoid an ambiguous Skip label.

## Placement and state contract

The existing `widget_order` only orders Calendar, Projects, System monitor and
Weather. Clock, At a glance, Music, Notes and Focus use separate placement paths;
explicit setup positions also override automatic ordering. Reordering that field
alone would produce a misleading preview.

Introduce a bounded wizard draft with selected items, grouping, active-surface
order, a baseline and touched flags. Resolve it with a pure placement planner using
current natural sizes, the chosen display's usable area and existing minimum
sizes. Produce explicit normalized positions through the existing placement and
monitor-context machinery. Preview and commit consume the same plan. Project the
legacy four-card order where useful; do not make it the source of truth for all
widgets or add a competing runtime layout system.

Fresh setups use the current widget destination. If several displays exist, offer
**Arrange display** here to choose the preview and reorder target.
**Move selected widgets here** explicitly changes assignments; switching the
preview alone does not move anything. Draft moves on several displays commit
together. Reopened onboarding preserves
existing per-widget assignments and manual positions until the person explicitly
reorders or changes the destination. Reorder only the selected display's surfaces;
show assignments elsewhere without silently moving them. Disconnection invalidates
an in-flight drag and replans using existing fallback/hide rules.

Use natural sizes, spacing and additional columns where they fit. Never silently
omit widgets, overlap them or shrink below readable minimums to fit the miniature.
If the chosen display cannot fit the result, keep the draft, explain which items
need room and offer fewer widgets or another display before committing placement.

Group changes map Clock/Weather/Calendar choices into At a glance sections or
standalone widgets, preserving the same content choices. Turning the group off
restores separate choices and a stable order. No empty At a glance surface is
created when all three sections are off. Connections, notes, projects, timer state,
style overrides and disabled widgets' saved sizes are preserved.

Only the new widget screens are staged; this does not change the existing wizard's
look/bar Apply and scoped Cancel behavior. Commit through one guarded WidgetPatch
on the existing serialized native job queue and retain its receipt for wizard
Cancel/Undo. Do not call the immediate visibility setters for each preview click.
Guard touched fields against external edits, report conflicts and retain the draft.

Persist the bounded draft in `setup-wizard.json`, without content or provider data.
Old saved step names remain valid. Resume preserves selections/order, rebuilds
geometry from current displays and never reapplies an already committed action.
Returning to Starter setup can refresh untouched defaults; once customized, widget
choices survive a look change unless the person chooses Use setup widgets.

## Implementation order and acceptance

1. Add the draft, grouped visibility projection and pure placement planner. Cover
   every widget, partial grouping, all-off, missing content, overflow, display
   assignments and preservation of unrelated preferences.
2. Extend wizard routing/resume and the guarded transaction. Cover Back, keep current
   widgets, earlier look changes, old session files, conflicts, failed writes,
   successful retry, Cancel and Undo. Weather routing follows actual visibility.
3. Build native Choose/Arrange/Ready views with shared previews, keyboard reorder,
   focus scrolling, accessible state announcements and reduced motion.
4. Verify actual applied positions against the preview. Exercise first run and
   reopening, full keyboard operation, 820×600 and wide windows, light/dark and
   150% scale. Qualify assistive technology separately: the current build does not
   enable libcosmic's a11y feature, so the HTML concept is not native accessibility
   evidence. Physical display acceptance remains an explicit release check.

Likely touchpoints: `src/settings/wizard.rs`, a focused wizard widget planner,
`src/settings/view/wizard.rs`, native-job integration, existing placement/context
helpers and wizard/onboarding tests. No new service, provider or web runtime.

## Local implementation — 2026-10-25

- `src/settings/wizard/widgets.rs` owns the bounded selection, grouping, ordering,
  display targets and field baselines. The same pure plan supplies preview
  positions and the guarded widget transaction. Existing manual geometry stays
  until moved; newly enabled widgets use free space. Bar dimensions reserve room
  at screen edges. Oversized arrangements block Apply with a specific message.
- Native Choose, Arrange and Ready views share the Settings theme and miniatures.
  Reordering supports grip dragging, arrows, keyboard focus and Alt+Up/Down.
  Ctrl+Z on Arrange undoes its last move. Focus reveals scrolled controls.
- Draft selection, ordering and display work survive closing Settings. Drag state
  and widget content are not persisted. Old named wizard steps still resume.
  First run creates the state directory; persistence failures are reported.
- Apply runs through the serialized native worker with one receipt for Cancel.
  Outside edits and failed writes retain the draft. Repeated Next does not apply
  again. Ready returns to review when a changed widget draft has not been applied.
- Weather is conditional. An empty choice does not start the widget service.
  Setup changes preserve customized widget choices; Use setup widgets explicitly
  adopts the selected starter's choices again. Notes, folders, providers and
  disabled widgets' dimensions remain untouched.

**44 focused checks passed; the release build passed.** Native captures cover
820×600 and 1280×800, light/dark themes and 150% raster scale. Two preexisting
opt-in diagnostic probes were not run.

Validation and native captures: [acceptance record](../.task-onboarding-implementation-evidence/acceptance.json).
Captures use the production widget tree with isolated preferences and sample
content. Physical monitor hotplug, compositor scaling and assistive-technology
acceptance remain separate real-device checks. No installed package or service
was changed.
