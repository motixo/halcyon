# GitHub presentation assets

The main README uses the maintainer's unmodified desktop screenshots captured
on 2026-10-23. Calendar and media details are included with their permission.
These are real desktop captures, not generated UI or sample-data renders.
The screenshots include optional Islands and an existing Calendar connection;
Google Calendar sign-in is not configured in the public 0.1.1 package.

## Desktop screenshots

![Purple desktop with widgets and optional Islands](../screenshots/desktop-purple.png)

![Light desktop with widgets](../screenshots/desktop-paper.png)

![Misty desktop with transparent widgets](../screenshots/desktop-mist.png)

![Colors and theme settings](../screenshots/settings-colors.png)

![Dock and panel settings](../screenshots/settings-panel.png)

The wallpaper artwork is from Halcyon. App icons and media artwork visible
in the captures belong to their respective owners. The screenshots are
documentation of a running desktop, not separately licensed copies of those assets.

## Earlier presentation assets

- Cover: the original [app welcome artwork](../../assets/wizard/welcome.jpg),
  reused without changes or a slogan. Provenance: [ASSETS.md](../../ASSETS.md).
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

Project-created artwork and UI are GPL-3.0-or-later. Third-party material
visible in desktop screenshots retains its original rights.

## Editorial references

Reviewed [Zed](https://github.com/zed-industries/zed),
[Zen Browser](https://github.com/zen-browser/desktop) and
[COSMIC Text Editor](https://github.com/pop-os/cosmic-edit).
The useful shared pattern is a short product description, visible download
or installation links, and deeper technical material linked separately.
No artwork or prose was copied.

Keep captions honest: previews are previews; native fixture captures use
sample data. Do not use generated interface concepts as product screenshots.

