# Islands material foundation (Halcyon 0.2)

## 0.2.5 decorative rendering policy

Native backdrop enhancement is deferred. The companion does not bind the desktop
background-effect manager, so it neither requests bar blur nor advertises/forwards
that facility to hosted applets and popups. `material_blur()` is false and
`Capabilities.rounded_blur` stays false; `MaterialSupport { version: 1 }` still
means the four decorative finishes are supported. Tint, static gradients, edges,
Liquid radius, native opacity/maximization, input masks and cached backgrounds
remain unchanged. No global theme/effect preference is rewritten. The updated
patch must be rebuilt; an older running host still has its earlier behavior.

The native-blur descriptions and demonstrations below are historical foundation
notes, not claims about 0.2.5 rendering or release acceptance. See
[material change notes](../docs/decorative-materials-025.md).

The Task 1 foundation retains the version-1 Islands sidecar and IPC without introducing a second color owner. The Task 2 material extension is documented below; these foundation notes describe the original native contract.

- `PanelSpace::layout_island_backgrounds` in the reconstructed patch obtains color through native `self.bg_color()`. That preserves native dark/light background, configured opacity, native frosted alpha mapping, overrides, maximization and animation. Do not replace it with an independent RGB palette.
- `colors.panel_blur(config.opacity)` and the native background-effect object determine active frosting. The preference belongs to `com.system76.CosmicTheme.{Dark,Light}/v2/frosted_panel`. A material may eventually choose local rendering behavior, but neither the launcher nor a material switch should write the global theme to obtain an effect.
- `dock_islands_core::rounded_region(x,y,w,h,radius)` produces `[x,y,width,height]` logical rectangles with conservative pixel coverage. Radius clamps to half the shorter side. Identical adjacent rows coalesce. Each island contributes its own union; gaps are absent. Use the same geometry for input and blur, but never frost the added autohide reveal strips or arrangement handles. The layer surface's corner radius and padding must stay zero in Islands mode.
- `BackgroundStamp` includes layout, dimensions, scale, radius, resolved native color, frosting availability/preference and arrangement state. Add material parameters to the stamp when Task 2 adds them, otherwise changes may not redraw. Preserve the existing dirty/animation checks and caching: rebuilding Iced backgrounds on every layout creates a repaint loop.
- IPC `Capabilities.rounded_blur` is now dynamic: true only when every active panel surface owns a native blur object. It describes capability, not current activation; use native theme preference for actual frosting. It stays false before surfaces exist or when native blur is unavailable. Protocol version and settings schema remain 1.
- Unknown sidecar versions continue to preserve data and use ordinary native layout. Any new sidecar field needs deliberate schema/backward-reader handling; the current settings struct denies unknown fields. Do not silently append fields while still expecting old companions to read them.

## Native support and cost

The union was demonstrated on official cosmic-comp source revisions `742b800` and `8cb1802`. Each passed 24 isolated Dock/Panel, Together/Separate, radius 0/12/160, native frosting off/on cases. Correct frosting tests use theme schema **v2**. Sharp checkerboard gaps and clipped corners were pixel-identical to the uncovered background; the frosted island interior was smoothed. The 1.10 compositor ran against newer host libraries/applets; this is not clean 1.10 OS/session certification.

The mask is quantized at logical-pixel resolution; curved edges may differ slightly from the antialiased background, especially at fractional scale. Coalescing reduces Wayland region requests and cached unchanged backgrounds avoid a repaint loop. The compositor still blurs/captures a single layer surface's bounding area, including the span between islands internally, then clips the visible result. This is not a claim of independent small GPU blur targets or a zero-cost material. Hardware/fractional-scale/multi-monitor performance requires separate qualification.

## Compatibility boundary

`reviewed-source-v1` admits the existing whole 1.10/1.11 source triples across timestamp/distribution rebuilds, only for manifests declaring runtime probe 1. Unknown source revisions still select stock. The five-second native preflight checks directly linked native libraries and required globals without creating surfaces. Graphics libraries loaded later (for example EGL) are outside this preflight and remain subject to native startup/failure supervision. The startup supervisor requires a valid IPC reply from the actual child PID within 15 seconds. Runtime failure uses the existing crash marker and stock recovery; disable and saved layouts retain their prior behavior. Static package diagnostics do not certify the version of an already running compositor. Log out after upgrades.

## Halcyon 0.2 material extension (Task 2)

Material selection is independent of version-1 Islands layout. The optional record lives at `io.github.profilaz3.Halcyon.Materials/v1/{Dock,Panel}`:

```ron
(version:1,finish:(kind:liquid,opacity:None,tint:35))
```

`kind` is `frosted`, `clear`, `smoked` or `liquid`; an explicit opacity is `Some(45)` through `Some(100)`, and tint is 0–100. Missing records restore native rendering. Unknown versions/fields/invalid values are ignored by the renderer without rewriting them; Halcyon's writer refuses to overwrite an unknown record. Existing version-1 Islands settings and protocol-1 `Capabilities` are unchanged. Older companions ignore this new namespace and continue rendering their saved layout. The new optional `MaterialSupport` request returns `MaterialSupport { version: 1 }`; older hosts reject that request, which the UI handles as unavailable. It does not claim blur strength control.

The native background remains the hue/polarity owner. Material tint mixes a small contribution of the native accent into that background; it never stores a second RGB source or changes the theme. The finish provides opacity and a static vertical highlight. Native opacity still multiplies the finish, and maximization remains opaque. Liquid has a minimum 24 px radius, clipped to the bar dimensions. All values are in the background cache stamp (and ordinary bar cache), with notification-driven reload. The material explicitly requests **local** background blur when an effect object exists, independent of `frosted_panel`; removing the record restores the native preference. No global frosting, alpha map, or window-effect key is written.

The ordinary companion dock/panel renderer and Together/Separate Islands use the same finish. Rounded island masks still exclude gaps and handles. On stock COSMIC or an older add-on, the Appearance page explains that bar materials need the updated add-on; widget materials remain usable. There is no live installation/enable action in the chooser. The preview shows the same cached wallpaper for all finishes and approximates tint/highlight/transparency; actual compositor blur is demonstrated by native screenshots, not simulated in the Settings thumbnails.

The app's `materials` record holds one shared finish plus optional `widgets`, `dock` and `panel` overrides. An absent shared material preserves legacy rendering and customizations. Choosing a material, adjusting it or restoring inheritance is one guarded desktop transaction, including the relevant bar records. Undo touches those fields, never a later color change. Saved setups containing materials use profile format 8 and restore bar records with the widget material in the same transaction; older setups retain their earlier format. Color themes stay format 7 and do not capture or change materials.

Liquid widget surfaces have a 220 ms edge glint on pointer entry. The thin highlight moves no text/content and requests no frames after it settles. `Surface motion` under Adjust disables it. Native bars have static highlights and retain their existing native transitions. Light native themes keep their light ink/background polarity for legibility: Smoked is the denser finish, not a forced dark palette. Widget contrast assistance can increase opacity while preserving the saved transparency. This is not optical refraction and is not a per-surface blur-strength implementation.
