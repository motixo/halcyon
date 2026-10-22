# Shared desktop themes

- Status: Accepted for implementation by the user on 2026-10-20 ("agreed. do it"); colors, native accents, the window outline and the Themes layout amended by [0002](0002-theme-colors-and-first-use.md)
- Date: 2026-10-20
- Scope: Widgets, dock and panel appearance

## Context

The user wants several visual systems that coordinate desktop widgets, the
dock and the panel. Current palettes mainly select widget colors. Profiles
already combine Dock, Panel, Appearance and WidgetStyle changes, but some
built-in presets also change widget visibility. Those presets are not a
suitable direct substitute for an appearance-only theme.

Widget rendering still contains fixed font choices, padding and text sizes.
The built-in widget palettes use dark surfaces. Daily overview has a
transparent treatment; ordinary cards do not have the same surface choice.
Native dock and panel styling already exposes background, opacity, corners,
padding and margins. The installed islands companion adds grouped geometry.

The relevant implementation is in `src/config.rs`, `src/widget_style.rs`,
`src/desktop/panels.rs`, `src/desktop/profiles/`, and
`companion/dock-islands-core/`. The current portable profile format is version
6, with an explicit allowlist and compatibility validation.

## Considered options

1. Add more named color palettes. This is small, but cannot express distinct
   typography, surface treatments or geometry across the three components.
2. Bundle existing controls into more profiles. This can deliver several looks
   quickly, but does not resolve hardcoded widget styles or define consistent
   colors when wallpaper-based coloring is enabled.
3. Add a shared theme definition and resolve it through the existing profile
   planner and component adapters. This needs widget rendering work while
   reusing preview, conflict detection, apply, recovery, undo and sharing.
4. Build independent renderers for each theme or replace the native shell.
   This provides more freedom but duplicates behavior and makes reliability,
   accessibility and upstream compatibility harder to maintain.

## Proposed decision and rationale

Use option 3. A theme is a data record describing semantic colors, surface
treatment, a family of corner sizes, spacing and typography. Resolve the
selected theme once, then adapt those roles to widgets and the supported
native dock/panel controls. Keep one renderer for each widget.

Use the existing profile engine as the application mechanism, with a Themes
gallery focused on appearance. Layout presets remain a separate optional
capability; choosing a theme preserves widget positions, explicit sizes,
enabled sections, account connections and content preferences.

Expose one color-source choice: theme colors or wallpaper colors. Wallpaper
colors replace the relevant accent and neutral roles while the chosen style
retains its geometry and typography. Warning and error colors retain their
meaning. Per-component overrides must remain visibly Custom and offer a
reset to the shared theme.

Matching wallpaper is optional. A hidden panel stays hidden. A theme must
adapt to installed capabilities and must not install a shell companion as an
unannounced side effect.

The first visual studies are Soft, Outline, Paper and Bare. These are design
directions, not installed or importable themes. The interactive study and
machine-readable proposal are in the sibling task artifact directory
`.desktop-theme-study-20261020/`.

## Consequences

- Extend widget styling beyond a four-color palette, with separate readable
  text roles for cards and wallpaper, real light surfaces, configurable
  typography, and consistent spacing.
- Extend the versioned profile allowlist for any new fields. Preserve existing
  profiles and keep old configurations visually unchanged until a theme is
  selected.
- Reuse the existing staged apply and recovery machinery. Pending drafts must
  be considered by the planner, not overwritten when switching themes.
- Keep rendering free of configuration writes and palette extraction work.
  Cache resolved roles and update them only when their inputs change.
- Native applets consume COSMIC theme roles. Where a requested shared accent
  requires changing a COSMIC theme variant, preview must disclose that COSMIC
  applications can also be affected. Do not promise independent per-applet
  colors where the native adapter cannot provide them.
- Arbitrary applet redesigns, icon replacement, compositor animations and
  universal backdrop blur are outside the initial scope.
- Validate each theme with small widget sizes, both display modes, bright and
  dark wallpaper, missing data, keyboard use, supported native bar variants,
  and older profile imports before release.

## References

- [COSMIC panel configuration](https://github.com/pop-os/cosmic-panel/blob/master/cosmic-panel-config/src/panel_config.rs)
- [COSMIC theme model](https://github.com/pop-os/libcosmic/blob/master/cosmic-theme/src/model/theme.rs)
