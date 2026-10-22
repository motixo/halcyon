# Settings as direct actions over shared desktop state

- Status: Accepted by the user on 2026-10-20: "good, accepted. decide whether you implement this, astra agent on max or opus on max"
- Date: 2026-10-20
- Would amend: [0001 Shared desktop themes](0001-desktop-theme-system.md) and [0002 Theme colors](0002-theme-colors-and-first-use.md)
- Evidence: independent [Claude exploration](../../../.settings-architecture-20261020/claude-exploration.md) and [Astra exploration](../../../.settings-architecture-20261020/astra-exploration.md), plus parent source verification

## Context

The user wants the entire settings architecture reconsidered. Individual pages work reasonably well, but completing one action often requires applying or discarding changes elsewhere. The screenshot shows Themes blocked by pending Dock & panel, Theme & fonts, Wallpaper & colors, and a recovered theme selection.

The problem is ownership, not simply navigation. `Settings::profile_overlaps` treats dirty page domains as competing transactions. Each editor preserves a separate baseline while the profile planner reads the applied desktop. Some independent edits affect the same generated COSMIC files. A user is asked to resolve this coordination by visiting pages.

The same private draft records also influence the runtime: `session_drafts::wallpaper_colors_pending` and `wallpaper::automatic::paused` can stop automatic colors after Settings closes. This was verified in source; Claude additionally reported matching live service logs. Separately, automatic wallpaper-color transactions use the same single latest-change record as manual actions, weakening the current Undo experience.

The existing adapters, validation, guarded transactions, journal, bounded image analysis, source stamps and recovery are valuable. Replacing the native UI toolkit or the desktop shell is unnecessary. The inspected local COSMIC Settings reference writes ordinary appearance and panel changes directly, including committing panel size on gesture completion.

## Considered options

1. **Retain page drafts and add Apply all.** Small visible change, but independently generated patches can conflict and sequential Apply can leave partial results and multiple Undo records. It preserves the ownership problem.
2. **One shared desktop draft and Apply.** Astra's recommendation. This can correctly compose appearance, arrangement and native settings and separate applied state from pending state. It does not inherently require pausing automatic colors. It is suitable for deliberate desktop composition, but brings a persistent editing session, global Apply scope, recovered drafts, and a staged/immediate boundary into ordinary settings tasks.
3. **Apply every control immediately, including theme tiles.** Claude's recommendation. Direct settings and Undo are a small mental model. However, merely browsing themes would repeatedly recolor applications and reload native surfaces. Reliable Undo and same-field external conflicts still need substantial design; this is not a mechanical replacement of Apply handlers.
4. **Direct settings with self-contained theme/setup choosers. Recommended.** Ordinary edits commit when completed. A whole theme or saved setup is browsed locally, then used through one explicit action. A chooser never becomes an unfinished global desktop project.

## Proposed decision and rationale

Use option 4, retaining Astra's shared ownership and semantic composition beneath a primarily direct interaction model.

The user-facing rule is: **change a setting and see it; choose a complete look before using it; Undo takes back your last action.** No page owns an independent Apply queue.

### Navigation and ownership

Use four primary destinations, with App settings anchored below:

| Destination | Responsibility |
|---|---|
| Appearance | Theme, wallpaper, color source, native coordination scope, mode, surface customization, fonts and icons |
| Widgets | Available/enabled widgets, prominent Arrange action, positions/sizes/grouping, and each widget's content |
| Desktop | Dock & panel, windows & workspaces, desktop icons and shortcuts, with object inspectors rather than an extra hub of links |
| Setups | Saved desktop combinations, import/export, schedules and display rules |
| App settings | Connections, sign-in/background operation, app motion, recovery and app information |

First use opens Appearance; subsequent use restores the last destination or an explicit deep link. Existing page keys and search targets redirect to the appropriate inspector.

A setting has one owner, but may have several useful entry points. Appearance → Dock and Desktop → Dock open the same inspector, with Style, Position and Contents sections as appropriate. Editing a color there must not say “finish this in Appearance.” A widget selected on the arrangement canvas opens the same controls as its list entry. Connection setup opens within the relevant widget flow.

Appearance is the complete starting workflow: choose a theme, keep or change the wallpaper, choose Wallpaper / Theme / Custom as the color source, and customize the parts that matter. Essential scope remains visible as **Whole desktop / Widgets only**. The latter must remain a real coordination policy, not be renamed “own accent,” which does not describe all native styling affected.

Use **Theme** for the coordinated visual recipe and **Setup** for a saved combination that can also include arrangement or behavior. Light/Dark/Automatic is **Mode**. Save a look and save a setup use the same library with explicit included categories. Existing presets remain available under saved/legacy items; this proposal does not silently delete them.

### Interaction contract

| Action | Behavior |
|---|---|
| Toggle, segmented choice, selected city | Commit the completed choice directly; one scoped Undo |
| Slider, drag or resize | Show local movement during the gesture; commit on release or completed keyboard gesture; one Undo per gesture |
| Color or number text | Keep partial text locally; commit valid input on Enter or field completion, not each valid-looking prefix |
| Theme chooser | Browse specimen/desktop previews locally; **Use theme** commits one coordinated appearance action; Cancel writes nothing |
| Wallpaper chooser | Browse locally; **Use wallpaper** commits the image and linked derived colors together |
| Setup chooser | Show affected categories beside **Use setup**; commit one guarded action |
| Shortcut, rule, schedule or note form | Keep a normal local form and its own Save/Cancel; never Save to draft followed by another Apply |
| Arrange on desktop | Commit completed gestures; Done exits. Cancel conditionally reverses this arranging session's changes only |

Form state may be recovered where losing meaningful input would be harmful. It is private form state, has no runtime effects and cannot block other settings. Do not discard notes or complex form text merely to claim “no drafts.”

The ordinary settings footer contains a labelled Undo when available, such as “Undo dock opacity.” No permanent saved notices, list of dirty pages, or mandatory Review changes. Invalid input and failures are shown at the relevant control.

There is no timed live “try-on” session in this proposal. Local previews avoid the lease, timeout and crash-rollback machinery that a real desktop trial would require.

### One model, one command path

Introduce a shared settings controller and typed semantic actions over the current applied desktop. Views keep only transient UI/form state. The controller:

1. Reads the latest compatible observations through existing adapters.
2. Resolves the action's intent, scope and dependencies.
3. Builds native appearance once, each affected panel once, and widget/wallpaper changes from the same candidate.
4. Validates and preflights the composed transaction.
5. Commits through the existing guarded store, records the action, then refreshes all views.

This is not a universal configuration framework. Existing control IDs, domain validators and typed field paths supply most of the structure. Keep unknown native values and existing allowlists.

Do not concatenate independently derived native theme patches with last-writer-wins. Shared generated files require semantic composition before serialization. Separate look/setup library state from transaction state; remove the Themes/Presets editor's navigation-dependent mode.

Settings, automatic colors and scheduled setups use the same resolver. App IPC or the existing writer lock serializes mutations. Each async action has a generation/source stamp so an older image analysis or slider completion cannot overwrite a newer choice.

### Theme defaults and custom choices

Represent appearance as a theme recipe, color policy, coordination scope and explicit per-property overrides. This is an internal model, not a new “layers” editor for users.

- Changing theme updates defaults and retains intentional overrides.
- Reset clears the named override and follows the current theme again.
- Persist the user's Wallpaper / Theme / Custom choice even if the resolved RGB happens to equal another option.
- Use observed differences and last-generated values to adopt external COSMIC changes as overrides. Inference remains necessary for outside writers, but should not be the only record of explicit choices made in this app.
- Show changed properties specifically where useful; avoid declaring an entire surface customized because one unrelated value differs.

COSMIC colors are shared across several shell/app surfaces. Expose that coupling honestly. Do not offer arbitrary independent applet accents. A focused outline can retain its supported Follow accent / Custom choice. Global roundness/density and bar-specific exceptions must pass through the same model so that this app does not fight COSMIC Settings.

Theme choice does not change widget content, enabled widgets, geometry, applets, native mode, native fonts, accounts or schedules unless a separately scoped setup explicitly includes them. Previews show the selected native mode accurately. Supporting every theme in both modes is a distinct completeness task; do not imply that selecting Paper switches the whole OS to Light.

### External changes, automatic colors and schedules

Applied appearance policy drives automatic colors regardless of open pages, unused theme previews or recovered forms. A saved proposal is not an automation lock.

Opening Settings or browsing a chooser has no runtime effect. A wallpaper chosen in this app resolves wallpaper and colors as one action; an external/slideshow change updates only still-linked roles. Missing assets or failed analysis retain the last valid colors with an accurate local status. “Automatic” is not a guarantee that IO and platform failures cannot occur.

On a write conflict, rebase independent fields. A newer explicit action can set its own field on fresh state, but never silently overwrite a same-field external change that occurred during an older in-flight gesture or change the scope of a previewed setup. Show a concise current/proposed choice locally when necessary.

Schedules may be briefly deferred while an actual overlapping gesture or compound commit is in progress, using a liveness-bound lease. Idle Settings windows and abandoned forms do not hold schedules. Preserve manual-choice precedence; cancel an older deferred switch when a later manual action supersedes it. A coordinated look is not split into partial scheduled transactions.

Separate **Show widgets** from **Run in background**. Hiding cards must not stop already-enabled color following or schedules. Reuse the existing service, not a new daemon.

### Undo is part of the feature

Ship a bounded durable manual-action history with the direct model. Preserve the latest useful manual action across reopening and automatic recoloring. Background derivation must not replace it; distinguish manual, scheduled and derived actions for both history and scheduler precedence.

Coalesce one gesture into one entry. A theme/setup/wallpaper action is one entry. Text editors retain their normal Ctrl+Z; otherwise the visible labelled Undo addresses settings history.

Undo restores the action's semantic intent and touched scope, then re-derives against compatible current inputs. It does not restore an unrelated external wallpaper or overwrite newer external settings. For a conflicting reversal, explain which value changed and allow a scoped resolution/preview; never promise unconditional byte-for-byte rollback after other actors have written.

Keep the existing transaction journal and conditional recovery. Durable intent/history metadata must share the recoverable boundary with the writes they describe. Do not automatically roll back an interrupted transaction over unrelated later edits.

### Migration

Treat existing saved edits as legacy proposals, not active locks. Read and retain the original records; convert actual changed settings and original baselines, not entire page snapshots.

Present one optional recovery sheet containing those choices. Disjoint edits can be applied together or individually in place. Incompatible proposals for the same field remain explicit choices. Do not replay four old page handlers sequentially against stale baselines and recreate the same conflict problem.

Keep records until their replacement has been safely written or the user discards them. Preserve malformed/unrecognized records for recovery. Migration is idempotent, versioned, and does not change the live desktop on upgrade.

Retain current layouts, explicit native/custom colors, widgets-only scope, portable presets, automation references, notes and meaningful form recovery. Read-only “follow COSMIC colors” and “sync colors to COSMIC” remain distinct to avoid a feedback loop.

## Consequences and rollout

- This removes ongoing page drafts and global dirty-page gates, but keeps transient candidate models and local form validation.
- Reliable immediate behavior requires shared composition, action ordering and stronger Undo. It is not a small footer edit.
- Broad batch composition is available through editing/saving a setup when deliberately requested; ordinary Settings has no global pending basket.
- All object entry points share the same state and inspector. UI duplication is prevented in code rather than by sending the person to another page.
- Do not separately ship an unconditional deletion of `paused()` while other writers still depend on old baselines. Applied/pending separation, external reconciliation and source validation must accompany that change.

Implement in coherent vertical slices:

1. Shared command/resolution path, manual history versus derived actions, migration scaffolding and applied-policy isolation; exercise against fixture copies.
2. Complete Appearance journey, local theme/wallpaper choosers, reusable surface inspectors and automatic colors. Convert all overlapping appearance writers together.
3. Convert remaining desktop/layout controls; unify navigation, search and setup flows. Retain useful specialized editors and keyboard accessibility.
4. Native interaction trials and migration verification, then install as one coherent release. Additional themes, effects and mode-complete recipes follow verified core behavior.

No existing presets or controls are retired simply because an explorer suggested it. No live desktop trial is performed as part of this exploration.

## Acceptance scenarios

1. Choose Outline, retain wallpaper, adjust dock opacity, change its position from the same inspector. Each action finishes there, with accurately scoped Undo.
2. Browse themes and cancel; open/close Settings; leave an invalid hex field. Automatic colors continue following the applied policy.
3. Change wallpaper with following enabled. Image, accents, outline, surface tones and text polarity use the same source; overrides and widgets-only scope survive.
4. Change a color externally during analysis, rapidly choose two wallpapers, and interleave a schedule. Old work cannot overwrite the newer choice; errors resolve locally.
5. Move/resize a widget, edit its content, navigate, reopen and Undo. Gesture history and document/text Undo have predictable scopes.
6. Apply a theme, let automatic colors run, then Undo. The useful manual action remains available and unrelated external changes survive.
7. Recover copies of the user's four old drafts, including conflicting proposals and incomplete text. Resolve them in one place without locks, silent loss or live writes on upgrade.
8. Hide all widgets while color following/schedules remain enabled. Those services continue; stopping background operation clearly identifies what stops.

## Evidence and limits

Parent verification read `src/settings.rs` (`profile_overlaps` and service toggle), `src/settings/session_drafts.rs` (`wallpaper_colors_pending`), `src/desktop/wallpaper/automatic.rs` (`paused`, `commit`), `src/desktop/store.rs` (`apply_owned`, `undo_latest`), and `src/automation/runtime.rs` (`manual_at`). It also checked the local COSMIC reference under `.cosmic-ux-20261019/cosmic-settings-reference/cosmic-settings/src/pages/desktop/`.

Claude's claim that staged models necessarily pause automatic colors is not accepted; Astra explicitly separates applied and pending intent. Conversely, Claude's immediate approach does not eliminate semantic composition, external-write conflicts or the need to strengthen Undo. The recommendation weighs the user's ordinary tasks rather than claiming either approach is universally superior.

This ADR records the accepted direction. The research concept demonstrates the interaction contract; implementation and release verification are tracked separately.
