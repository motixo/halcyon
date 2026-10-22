# GitHub presentation assets

- `hero.png`: AI-generated cover, made with built-in OpenAI image generation
  through Codex on 2026-10-22. The only image reference was our
  `assets/wizard/welcome.jpg`. Exact prompt: [hero-prompt.txt](hero-prompt.txt).
- `looks.png`: HTML composition of the app's existing look previews.
  Source: [looks.html](looks.html), using `docs/brand/looks/shipped/`.
  These show sample widgets and simplified native surfaces, not a live session.
- `widgets.png`: HTML composition of unchanged native widget renders.
  Source: [widgets.html](widgets.html), with input images in `sources/`.
- `layout.png`: native Layout editor fixture, unmodified.

Widget and Layout inputs came from the app's native renderer at commit
`dc8bb5e`, captured on 2026-10-22. They contain synthetic fixture data,
not a person's accounts, music history, notes or measurements. Source fixture
names: `theme-frosted-weather-narrow-dark`, `compact-system-360-7-true`,
`music-browser-example`, `notes-checklist-example`,
`home-resized-820x600`.

The two HTML compositions are rendered at 1100 CSS pixels wide, at 1x device
scale. Relative image paths resolve from this directory. T3's HTML renderer
was used for the committed PNGs; no UI was drawn by the image model.

All assets are GPL-3.0-or-later, as the rest of the project.

## Editorial references

Reviewed [Zed](https://github.com/zed-industries/zed),
[Zen Browser](https://github.com/zen-browser/desktop) and
[COSMIC Text Editor](https://github.com/pop-os/cosmic-edit).
The useful shared pattern is a short product description, visible download
or installation links, and deeper technical material linked separately.
No artwork or prose was copied.

Keep captions honest: previews are previews; native fixture captures use
sample data. Do not use generated interface concepts as product screenshots.

