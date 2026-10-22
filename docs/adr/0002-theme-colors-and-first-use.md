# Theme colors, native accents and the window outline

- Status: Accepted (implementation by the user's request of 2026-10-20 to streamline Themes, fix the window outline and improve wallpaper colors)
- Date: 2026-10-20 (amended the same day after review: applied colors, Reset recovery, grey wallpapers and derived-only outlines)
- Amends: [0001 Shared desktop themes](0001-desktop-theme-system.md)

## Context

After the first release of shared themes the user reported that the COSMIC window outline did not follow theme or wallpaper colors, that wallpaper-derived colors were too dark, and that the Themes journey needed to be clearer.

Investigation found three causes:

1. COSMIC draws the active-window hint with the theme builder's `window_hint` when it is set, otherwise with the accent. Desktop Cards only wrote `accent` (and surfaces), so an outline color set anywhere, even long ago, never followed again. The Themes preview hid the sample window, so the outline was never visible before Apply either.
2. One accent was written to both COSMIC variants. Families with dark surfaces wrote pastel accents into COSMIC Light; Paper wrote a dark accent into COSMIC Dark. COSMIC's own palettes use light accents in Dark and deep ones in Light, and libcosmic adjusts only accent text, not fills, focus rings or the outline.
3. Wallpaper colors offered raw median-cut swatches. On dark pictures every swatch is dark and grey, and clicking one stored that dark pixel as the accent. Surfaces were near-black neutrals, so a wallpaper's character reached only a thin accent.

Themes and Wallpaper & colors also offered overlapping color controls with different vocabularies over the same settings.

## Decision

**One seed, toned per role.** A look has one seed color: the wallpaper's recommendation, a fixed color, or the style's own. Roles are derived in OKLCH: lightness moves per role while hue and chroma stay (with real sRGB gamut mapping), so readable ink keeps its color instead of mixing toward white or black. Each style keeps its surface lightness and character (Soft visibly tinted, Outline nearly matte, Paper a warm sheet that never turns cool, Bare transparent) with a bounded tint of the seed's hue.

**Offered wallpaper colors are usable accents.** Analysis keeps the exact swatches for polarity and previews, and separately offers up to four distinct accents from a chroma-weighted hue histogram, each toned to COSMIC's dark-accent lightness. A grey picture offers none; a theme then keeps its own color, whichever route set the wallpaper (Themes, Wallpaper & colors or the runtime). A faint tint gives one quiet steel.

**Native accents per variant.** The shared color is written as a pastel tone to COSMIC Dark and a deep tone to COSMIC Light, by every writer (theme selection, Wallpaper & colors, the runtime), so linked status stays consistent.

**Themes own the window outline.** Theme records set `window_hint` to follow the accent in both variants. An outline chosen later in Theme & fonts survives style changes as a Customized "Windows" surface with its own Reset; an outline equal to the one retained in Before Themes predates themes and follows on the next style change. Outside themes, Wallpaper & colors shows when the outline does not follow and offers **Match accent** inside the same color transaction. COSMIC also draws an outline color that only the derived theme holds (another tool can write one without the builder): it counts as Customized and survives style changes like a later choice, and only the explicit relinks (Windows Reset, Match accent, Theme & fonts "Follow accent") clear it.

**One place for theme colors.** Themes shows a live preview of the real desktop (including an active window and its outline), four specimens on the user's wallpaper, one Color row and the optional matching wallpaper. Style and color are independent choices: a style change keeps the look's color, the pending choice or else the applied one (following the wallpaper, a fixed color or the style's own), also after Settings is reopened. With a theme applied, Wallpaper & colors summarizes the color and links to Themes instead of repeating its controls. The first theme starts from the existing choice: wallpaper colors stay on, an explicit fixed color is kept, and a person who never chose starts from the wallpaper's colors when it has any.

**Compatibility.** No profile format change is needed: the outline uses the existing `outline` variant field (`"auto"`). Surfaces and accents written by the previous derivation are still recognized as linked, so they move to the current tones on the next update rather than turning Custom.

## Consequences

- A typed hex color stays exact for widgets; COSMIC receives per-variant tones of it.
- Linked detection compares against both the current and the previous derivation; the previous formula stays only for that comparison (`desktop_theme::previous_native`).
- Status attribution changes: custom native surfaces mark Dock and Panel; custom accents or a non-following outline mark Windows.
- Choosing a style keeps the look's color (pending or applied) and a pending matching-wallpaper choice; a saved variation still restores its recorded appearance exactly.
- Wallpaper & colors settles a theme's wallpaper polarity (the ink of text drawn on the wallpaper) for the wallpaper it applies, in the same Apply and Undo, as Themes and the runtime do: its Apply also tells the runtime that wallpaper's colors are settled.
- Draft recovery plans again only records that choose a whole style. A saved surface Reset is restored as it was: its scope and the color fields it leaves out are its intent, and it takes no wallpaper colors.
- Tests that encoded "recommendation equals a swatch" and "both variants share one accent" now assert hue preservation and per-variant tones instead.
