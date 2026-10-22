# Halcyon

Desktop widgets and themes for COSMIC.

![Halcyon look: frosted cards over a night sea](docs/screenshots/halcyon-look.jpg)

Halcyon puts a small set of widgets on your COSMIC desktop and gives the whole
desktop a coordinated look: wallpaper, colors, windows, dock and panel.

- **Starter looks.** Six complete looks, each with its own wallpaper. Colors
  follow the wallpaper.
- **Widgets.** At a glance (clock, date, weather), system monitor, calendar,
  projects, music, notes and a focus timer. Widgets never overlap; music shows
  while something is playing or paused.
- **Styles.** Thirteen widget styles, from soft cards and frosted glass to
  outlines, paper and terminal, with matching dock and panel.
- **Gentle on your setup.** A look changes what it owns (wallpaper, colors,
  dock and panel appearance, the widgets it places). Your applets, their
  order, your panel position and the widgets you set up stay yours. Every
  change can be undone.

| First run | Choosing a look |
|---|---|
| ![Welcome](docs/screenshots/setup-welcome.jpg) | ![Looks](docs/screenshots/setup-looks.jpg) |

## Install

Halcyon is in development; the first release is being prepared.

- **Pop!_OS 24.04 (COSMIC):** a `.deb` package will be published on the
  Releases page. Download it, open it, and choose Install.
- **From source:** see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

After installing, open **Halcyon** from the app launcher. A short setup picks a
look, your dock and panel, and your city for the weather.

**Islands** (a panel split into floating islands) is an optional, separate
add-on. It only runs on the exact COSMIC versions it was tested with and
otherwise leaves the normal panel in place.

## What leaves your computer

- **Weather:** your city search text and the chosen city's coordinates go to
  [Open-Meteo](https://open-meteo.com).
- **Calendar (optional):** if you connect Google Calendar, Halcyon talks to
  Google with your permission. Sign-in tokens stay in your system keyring.
- **Album art:** artwork your music player names is downloaded from its
  public HTTPS address.

No analytics, no tracking, no account needed.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE), [ASSETS.md](ASSETS.md) for the
artwork and [THIRD_PARTY.md](THIRD_PARTY.md) for dependencies.
