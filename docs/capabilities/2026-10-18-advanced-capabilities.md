# Advanced desktop capabilities on this COSMIC build

This document covers animated wallpapers, desktop motion, and session and window restoration. It records what this Pop!_OS 24.04 COSMIC build supports and what Desktop Cards does with that. It was checked on 2026-10-18.

The evidence comes from the installed package versions, upstream source at those exact commits, and read-only queries of the live session. Nothing was changed on the live system to gather it.

## Installed versions (dpkg)

| Component | Package version | Upstream commit |
| --- | --- | --- |
| cosmic-comp | `0.1~1791491785~24.04~4d14528` | [`4d14528`](https://github.com/pop-os/cosmic-comp/tree/4d14528) |
| cosmic-bg | `1.10.0~1791316428~24.04~91d525b` | [`91d525b`](https://github.com/pop-os/cosmic-bg/tree/91d525b) |
| cosmic-session | `1.10.0~1791314577~24.04~8093b59` | [`8093b59`](https://github.com/pop-os/cosmic-session/tree/8093b59) |
| cosmic-settings | `1.10.0~1791394725~24.04~58aeb65` | [`58aeb65`](https://github.com/pop-os/cosmic-settings/tree/58aeb65) |
| cosmic-panel | `1.10.0~1791315703~24.04~3ce6ce1` | [`3ce6ce1`](https://github.com/pop-os/cosmic-panel/tree/3ce6ce1) |
| xdg-desktop-portal-cosmic | `1.10.0~1791324839~24.04~9d35e63` | [`9d35e63`](https://github.com/pop-os/xdg-desktop-portal-cosmic/tree/9d35e63) |
| xdg-desktop-portal | `1.18.4-1ubuntu2.24.04.3` | [`1.18.4`](https://github.com/flatpak/xdg-desktop-portal/tree/1.18.4) |
| GTK 4 | `4.14.5+ds-0ubuntu0.10` | [`4.14.5`](https://gitlab.gnome.org/GNOME/gtk/-/tree/4.14.5) |
| gsettings-desktop-schemas | `46.1-0ubuntu1` | [`46.1`](https://gitlab.gnome.org/GNOME/gsettings-desktop-schemas/-/tree/46.1) |
| libcosmic (this app, Cargo.lock) | n/a | `dc11d6a` |

## 1. Animated wallpapers: not supported by COSMIC's background service

**What COSMIC does.** cosmic-bg is the only background renderer. It draws on the layer-shell background layer, one surface per output.

- **It decodes one still frame.** On a source change, `Wallpaper::draw` decodes the file once with `ImageReader … decode()` ([`src/wallpaper.rs:126-142`](https://github.com/pop-os/cosmic-bg/blob/91d525b/src/wallpaper.rs#L126-L142)). It caches the result as `current_image` and scales it per output.
- **It never animates.** Its frame-callback handler is empty ([`src/main.rs:396-402`](https://github.com/pop-os/cosmic-bg/blob/91d525b/src/main.rs#L396-L402)), and nothing redraws on a timer except slideshow rotation.
- **Supported formats.** The image crate is built with `hdr, jpeg, png, rayon, webp`, plus AVIF (`avif-native`) and JPEG XL (jxl-oxide) ([`Cargo.toml`](https://github.com/pop-os/cosmic-bg/blob/91d525b/Cargo.toml)). GIF is not decoded at all.
- **Animated files show still.** An animated WebP or APNG appears as its first (default) frame.
- **No video or shader source.** The config schema's `Source` is `Path` or `Color` (single or gradient), with `rotation_frequency` for folders ([`config/src/lib.rs:81-100`](https://github.com/pop-os/cosmic-bg/blob/91d525b/config/src/lib.rs#L81-L100)).

**Layering, outputs and lifecycle.** A substitute animated wallpaper would have to be a separate full-screen layer-shell client on every output, sitting above cosmic-bg and below Desktop Cards' bottom-layer cards. It would also need:

- its own decoder and frame scheduling;
- handling for output hot-plug, fractional scale, and pausing when covered or idle;
- GPU and battery budgeting.

That is a new renderer, which the brief excludes. COSMIC has no protocol or setting that would hand frames to cosmic-bg.

**What Desktop Cards does.**

- When the picture selected on **Pop!_OS → Wallpaper** is an animated WebP or APNG, the preview shows **Animated · shown still**.
- Detection reads at most 64 KiB of the file's header, off the UI thread, once per file version.
- Slideshows (folder rotation) remain the supported way to get a changing background. Profile automation can also switch wallpapers by time of day or by display.

**Gap.** Real animation needs upstream cosmic-bg support: a frame loop with damage tracking per output, and an animated or video `Source`. Until then, no control is offered.

## 2. Desktop motion

### COSMIC itself: no motion settings

- **Compositor durations are constants.** In cosmic-comp, window, workspace, tiling and zoom animations use fixed durations:
  - `ANIMATION_DURATION` 200 ms ([`shell/layout/tiling/mod.rs:82`](https://github.com/pop-os/cosmic-comp/blob/4d14528/src/shell/layout/tiling/mod.rs#L82), [`floating/mod.rs:55`](https://github.com/pop-os/cosmic-comp/blob/4d14528/src/shell/layout/floating/mod.rs#L55));
  - `FULLSCREEN_ANIMATION_DURATION` ([`shell/workspace.rs:75`](https://github.com/pop-os/cosmic-comp/blob/4d14528/src/shell/workspace.rs#L75));
  - tab and rescale constants ([`element/stack/tabs.rs:130-131`](https://github.com/pop-os/cosmic-comp/blob/4d14528/src/shell/element/stack/tabs.rs#L130-L131), [`grabs/moving.rs:69`](https://github.com/pop-os/cosmic-comp/blob/4d14528/src/shell/grabs/moving.rs#L69)).
- **No config field.** `CosmicCompConfig` has no animation or motion field ([`cosmic-comp-config/src/lib.rs:76-116`](https://github.com/pop-os/cosmic-comp/blob/4d14528/cosmic-comp-config/src/lib.rs#L76-L116)). Its nearest relatives are `cursor_shake_to_find`, `cursor_hide` and `accessibility_zoom.view_moves`, all cursor and zoom accessibility settings that COSMIC Settings already offers.
- **COSMIC Settings has no reduce-motion switch.** Its Accessibility page offers a screen reader, high contrast, the magnifier, on-screen keyboard and shake-to-find ([`pages/accessibility/mod.rs`](https://github.com/pop-os/cosmic-settings/blob/58aeb65/cosmic-settings/src/pages/accessibility/mod.rs)).
- **libcosmic** `dc11d6a` has no animation preference.
- **The portal publishes no reduced-motion setting.**
  - xdg-desktop-portal-cosmic serves `color-scheme`, `accent-color`, `contrast` and `text-scaling-factor` ([`src/main.rs:196-202`](https://github.com/pop-os/xdg-desktop-portal-cosmic/blob/9d35e63/src/main.rs#L196-L202)).
  - `org.freedesktop.appearance reduced-motion` exists in [xdg-desktop-portal main](https://github.com/flatpak/xdg-desktop-portal/blob/main/data/org.freedesktop.portal.Settings.xml), but not in the installed [1.18.4](https://github.com/flatpak/xdg-desktop-portal/blob/1.18.4/data/org.freedesktop.portal.Settings.xml).
  - A live `Settings.ReadOne org.freedesktop.appearance reduced-motion` returns *NotFound*.
- **Already exposed elsewhere.** The dock and panel auto-hide slide (`autohide.transition_time`, cosmic-panel) is already on Dock & Panel → Advanced (*Slide duration*).

### What applications honour

- **The setting.** `org.gnome.desktop.interface enable-animations` (gsettings-desktop-schemas 46.1, default `true`) is the cross-toolkit "animations off" switch on Linux.
- **GTK.** GTK 4.14.5 maps it to `gtk-enable-animations`, read from GSettings directly or through the Settings portal in a sandbox ([`gdk/wayland/gdkdisplay-wayland.c:1802, 1985`](https://gitlab.gnome.org/GNOME/gtk/-/blob/4.14.5/gdk/wayland/gdkdisplay-wayland.c)). GTK 3 does the same.
- **Firefox** sets `prefers-reduced-motion` from that GTK setting ([`widget/gtk/nsLookAndFeel.cpp`](https://github.com/mozilla-firefox/firefox/blob/main/widget/gtk/nsLookAndFeel.cpp), `mPrefersReducedMotion = !enableAnimations`).
- **Chromium** sets it from the toolkit's animation setting ([`ui/gfx/animation/animation_linux.cc`](https://chromium.googlesource.com/chromium/src/+/main/ui/gfx/animation/animation_linux.cc)).
- **On this session.**
  - COSMIC runs with `DCONF_PROFILE=/usr/share/dconf/profile/cosmic` (`user-db:cosmic`, `user-db:user`), and Desktop Cards inherits it, so `gsettings` reads and writes the same database GTK apps read.
  - The live portal relays the key: `Settings.Read org.gnome.desktop.interface enable-animations` gives `true` (portal routing is `default=cosmic;gtk;`).

**What Desktop Cards does.** **Pop!_OS → Motion** (`--page desktop-motion`) has two controls and one limit line:

- **Animations in apps.** A toggle for that key: on or off, read back after writing.
  - It applies at once, like COSMIC's and GNOME's accessibility switches. It is a single reversible preference with no draft or Apply step.
  - It is not part of profiles, exports or automation, so applying a look never changes someone's motion preference.
  - If `gsettings` or the schema is missing, the row says *Unavailable* with the reason. In preview mode the toggle is shown but locked.
- **Slide the sidebar.** Desktop Cards' own edge sidebar either slides or appears at once (`sidebar_slide` in `config.json`, default on, autosaved). The widgets have no other animations.
- **The limit line.** COSMIC's window, workspace and launcher animations have no setting in this version. A link leads to Dock & Panel for the auto-hide slide speed.

**Limits.**

- COSMIC's own animations, and libcosmic apps such as COSMIC Files and Settings, ignore the GTK key.
- Qt and KDE apps follow their own settings.
- The toggle asks apps to reduce motion. It cannot force an app that ignores the request.

**Gap.** A desktop-wide reduced-motion setting needs:

- cosmic-comp to read a motion preference, such as a `reduce_motion` field or a duration scale;
- libcosmic to honour it;
- xdg-desktop-portal ≥ the release with `reduced-motion`, plus xdg-desktop-portal-cosmic publishing it.

## 3. Session and window restoration

**Supported: starting apps at login.**

- cosmic-session starts XDG autostart entries from `~/.config/autostart` and `$XDG_CONFIG_DIRS/autostart`, respecting `OnlyShowIn` and `NotShowIn` ([`src/main.rs:358-450`](https://github.com/pop-os/cosmic-session/blob/8093b59/src/main.rs#L358-L450)). Where systemd is used, the systemd XDG autostart generator does this instead.
- COSMIC Settings manages these entries on its *Startup applications* page ([`pages/applications/startup_apps.rs`](https://github.com/pop-os/cosmic-settings/blob/58aeb65/cosmic-settings/src/pages/applications/startup_apps.rs); `cosmic-settings startup-apps`).
- This only starts programs. It does not reopen documents, windows, workspaces or positions.

**Not supported: restoring a session or window arrangement.**

- **No session manager.** cosmic-session has no save/restore of running apps, and there is no XSMP manager.
- **Apps can't ask to be restored.** cosmic-comp `4d14528` implements no session-management protocol: no `xdg-session-management` or `xx-session-management` among its protocols ([`src/wayland/protocols/`](https://github.com/pop-os/cosmic-comp/tree/4d14528/src/wayland/protocols)). Apps cannot ask the compositor to put their windows back.
- **The toplevel-management protocol is partial.** `zcosmic_toplevel_manager_v1`, which is privileged and not available to sandboxed clients, lets a tool do several things ([`wayland/protocols/toplevel_management.rs:188-252`](https://github.com/pop-os/cosmic-comp/blob/4d14528/src/wayland/protocols/toplevel_management.rs#L188-L252)):
  - activate, close, maximize, minimize, fullscreen or make sticky an existing window;
  - move it to a workspace.

  It cannot set a window's position or size: `SetRectangle` is only the minimize-animation target. It also cannot launch an app into a given place.
- **Window rules are narrow.** COSMIC's window rules only choose floating or tiling per app ID or title.

**What Desktop Cards does.**

- The Pop!_OS overview's *In System Settings* group links to **Startup applications** ("Apps that open when you log in"), COSMIC's own page.
- Desktop Cards offers no "restore session" control.
- A workspace launcher that starts apps and then moves their windows to workspaces would be a partial window manager: geometry is impossible, and matching new windows to saved ones is guesswork. It would be mislabelled as restoration, so it was not built.

**Gap.** Real restoration needs both of these upstream:

- a session-management protocol in cosmic-comp, with app support;
- a session manager in cosmic-session that records and relaunches apps with restore tokens.
