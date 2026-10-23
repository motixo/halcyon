# Packaging

How Halcyon is laid out on disk, for the Debian package and for a per-user
install from source. Both use the same layout under a prefix: `/usr` for the
package, `~/.local` for a user install. The program finds its files relative
to its own executable (`<prefix>/bin/halcyon`), so the same build works in
both places.

## Supported processors

The `amd64` packages run on **64-bit Intel and AMD PCs** (x86-64). The name
does not mean AMD-only. ARM devices require a separate build. The packaged
release targets Pop!_OS 24.04 with COSMIC.

## Identity

| What | Value |
|---|---|
| Display name | Halcyon |
| Summary | Desktop widgets and themes for COSMIC |
| Debian package | `cosmic-ext-halcyon` |
| App ID (Settings window, launcher, metainfo, icon) | `io.github.profilaz3.Halcyon` |
| App ID (desktop widgets process) | `io.github.profilaz3.Halcyon.Cards` |
| Binary | `halcyon` (`halcyon` runs the widgets, `halcyon --settings [page]` opens Settings) |
| Controller | `halcyon-ctl` (start, stop, settings, focus commands, used by shortcuts) |
| User service | `halcyon.service` |
| Homepage | https://github.com/profilaz3/halcyon |
| License | GPL-3.0-or-later (metadata: CC0-1.0) |
| Maintainer | profilaz3 <261673162+profilaz3@users.noreply.github.com> |

## Layout under a prefix

| Path | Contents |
|---|---|
| `bin/halcyon` | The program |
| `bin/halcyon-ctl` | Controller (Python 3) |
| `lib/cosmic-ext-halcyon/` | Helpers run by the program: `calendar-sync.py`, `project-status.py` |
| `share/cosmic-ext-halcyon/` | `google-calendar-setup.html`; optional `google-calendar-client.json` (publisher OAuth client, added at release build time, never committed) |
| `share/backgrounds/halcyon/` | Wallpapers, one JPEG per name: `<name>.jpg` (the seven Halcyon scenes and six palette gradients) |
| `share/applications/io.github.profilaz3.Halcyon.desktop` | Launcher (opens Settings) |
| `share/metainfo/io.github.profilaz3.Halcyon.metainfo.xml` | AppStream metadata |
| `share/icons/hicolor/<size>x<size>/apps/io.github.profilaz3.Halcyon.png` | Icon at 48, 64, 128, 256 and 512 px |
| `share/doc/cosmic-ext-halcyon/` | `copyright`, `changelog.gz`, `THIRD_PARTY.md`, `ASSETS.md`, dependency license texts |

The user service lives at `/usr/lib/systemd/user/halcyon.service` (package)
or `~/.config/systemd/user/halcyon.service` (user install). It is never
enabled by the package; Halcyon's first-run setup enables it for the person
who runs it.

## Per-user data (created by the program, kept on uninstall)

`~/.config/halcyon/`, `~/.local/share/halcyon/`, `~/.local/state/halcyon/`,
`~/.cache/halcyon/`, runtime files in `$XDG_RUNTIME_DIR/halcyon/`. Earlier
versions used `cosmic-desktop-cards` for these; Halcyon copies them over once,
on first start, and leaves the old folders in place.

## Islands add-on

`cosmic-ext-halcyon-islands` is a separate, optional package (see
`packaging/islands/`). It only works with the exact COSMIC panel version it was
built for and is activated per person with `halcyon-islands enable`.
