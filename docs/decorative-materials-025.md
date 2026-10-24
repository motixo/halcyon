# Decorative materials in 0.2.5

Frosted, Clear glass, Smoked glass and Liquid retain their tint, translucency,
highlights and edges. Liquid keeps its rounded shape and optional brief widget
glint. The updated widget renderer and companion do not request native backdrop
blur; the companion also does not advertise or forward backdrop effects to hosted
applets/popups. Settings disables its automatic window/popup/system blur requests.
These are decorative finishes, not real background defocus or optical refraction.
There is no new rendering-engine preference.

Shared finishes, per-surface overrides, color synchronization and guarded Undo
remain. Transparent Clock and unfilled Glance stay transparent. **Appearance →
Adjust → Widget readability** uses the existing contrast scrim and readable ink
without overwriting the saved transparency. Clear surfaces may show sharp, busy
wallpaper; increase opacity or enable readability when needed. Material opacity,
native bar opacity and maximized-bar behavior are unchanged. Stock COSMIC bars
keep their native appearance; exact parity with the companion is not promised.

Choosing a color theme, including the legacy Frosted style, no longer enables
COSMIC window, applet or system-interface frosting. Old/imported saved setups do
not enable those effects either; explicit saved Off values still apply. Native
blur strength/glass fields remain readable for compatibility, but do not enable an
effect. Earlier gallery captures used native blur and are not exact previews of
this decorative rendering.

## Existing desktops and explicit recovery

The update does **not** automatically turn off global COSMIC effects. They may
have been enabled by a previous Halcyon action, by COSMIC Settings, or by another
application. Halcyon's journal retains before/after values and guarded history,
but cannot identify a later external write that reasserted the same value. An
automatic “repair” could overwrite an intentional preference.

To remove existing frosting explicitly, open **System appearance → Frosted
surfaces and blur**, turn the enabled surface switches off, and Apply. These
controls affect both light and dark themes. If the previous Halcyon appearance
change is still the offered Undo action and reverting that whole action is what
you want, guarded Undo remains available; it refuses conflicting changed values.
Undo is not a selective blur migration and can restore earlier effect settings.
COSMIC Settings also controls the independent desktop effects.

An already frosted COSMIC desktop or an older running companion can still use
native blur. This change avoids Halcyon's new automatic activation and local
rendering requests; it does not fix the compositor, establish an RX590 fix, or
claim that all native graphics issues are solved. A new companion binary is
required for the host behavior described here. Build and native visual smoke
results belong to the integrated package review, not this source change.
