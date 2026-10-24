# Store listing preparation

Decision recorded 2026-10-24, against Halcyon source `8a74f6c`.

Keep the signed APT channel as the supported installation and update route.
Seek eligibility guidance for the main application from the COSMIC Flatpak
maintainers before committing to a Flatpak port. Keep Islands in the existing
optional APT package for the initial store offering. This is a proposed scope,
not a submitted or accepted listing, and no Flatpak has been built or tested.

## Repository fit and submission ownership

COSMIC's repository hosts applets and other COSMIC Flatpaks unsuitable for
Flathub; its README normally directs applications to Flathub first. Halcyon's
desktop settings and persistent desktop surfaces warrant an eligibility check
before packaging. Acceptance and default repository availability are separate:
a listing is visible only where its repository is enabled.

The COSMIC Flatpak PR checklist permits the contributor's own app source and
manifest, but excludes other LLM-generated contribution content. The linked
Pop!_OS contributor guide also prohibits LLM-generated issues and PR text.
The maintainer must therefore write the eligibility inquiry and eventual PR
description themselves. This internal document is AI-assisted planning; it is
not an upstream issue/PR draft and must not be pasted as one. No upstream message
has been sent.

The human inquiry needs to establish whether a COSMIC-only desktop customization
app with deliberate access to COSMIC preferences is acceptable here, and whether
maintainers require a Flathub eligibility decision first. Identify the existing
public project and intended separation of the main app from Islands. Disclose
the project's AI-assisted development accurately when requested; never certify
that generated material is human-authored.

Flathub is not an automatic alternative: its current inclusion rules restrict
desktop extensions, environment-locked and host-dependent applications, with
some case-by-case exceptions. Its AI policy requires disclosure and excludes
AI-assisted manifests and agent submission interactions. A Flathub submission
would need a human packaging/submission owner following those rules.

## Actual port work identified in source

| Area | Current implementation | Required Flatpak work |
| --- | --- | --- |
| Desktop surfaces | libcosmic Wayland application with separate Settings and widget processes | Verify layer surfaces, input regions and display placement inside the sandbox on a COSMIC session. A normal Settings window opening is insufficient. |
| Startup and lifecycle | `src/settings.rs` invokes `/usr/bin/systemctl`; `packaging/systemd/halcyon.service` starts `/usr/bin/halcyon` | Add a Flatpak-aware lifecycle using the Background portal where supported. Handle denied permission; do not modify a host service or spawn unrestricted host commands. Test settings-to-widget launch and logout/relogin. |
| Native preferences | `DesktopStore::live` in `src/desktop/store.rs` uses `dirs::config_dir()` and XDG data roots | Separate sandbox-owned app state from explicitly permitted host COSMIC preferences/defaults. Flatpak redirects XDG paths, so granting a path alone does not fix the adapter. Preserve backup, Undo and external-change guards. |
| Helpers and data | `src/paths.rs` resolves executable-relative `lib/cosmic-ext-halcyon` and shared assets | Package helpers, Python/Git dependencies, fonts and backgrounds under `/app`; verify path resolution without a host installation. Existing prefix resolution is a useful starting point. |
| Wallpaper and projects | File-based wallpapers and Git scanning | Use selected-file/folder access where possible, with persistence after restart. Verify mounted-volume projects and wallpaper paths remain usable by the host desktop. Avoid blanket home access as a shortcut. |
| Music and metrics | `src/tools/music.rs` uses MPRIS; `src/data.rs` reads `/proc` and `/sys` | Scope MPRIS permissions and test controls. Verify which metrics represent the host accurately; show unavailable metrics honestly instead of silently reporting sandbox values. |
| Calendar | Native sign-in/keyring path plus legacy host-venv fallback in `src/calendar/mod.rs` | Bundle supported native dependencies and verify keyring/browser redirect. Do not rely on the workstation's Workspace Python environment. Current public release sign-in is unconfigured; do not advertise it as ready. |
| Updates and Islands | `src/settings/updates/identity.rs` verifies dpkg ownership; companion discovery calls host Python/dpkg | Add a truthful Flatpak install identity and Store-managed updates. Omit unsupported companion controls and offer the existing external APT instructions. Do not make the Flatpak a wrapper around host execution. |
| Existing installations | Per-user Halcyon config, service and runtime IPC | Define migration/coexistence explicitly so APT and Flatpak copies do not both start widgets or race desktop changes. Do not copy credentials without an intentional migration. |

Flatpak's filesystem, process and bus isolation makes this a product port, not
just a new archive format. A sandbox permission list is not proof of functionality.

## Work order after eligibility guidance

1. Agree supported scope and permissions with the intended repository. Record
   whether full appearance customization can remain in the main app.
2. Build a COSMIC-targeted local prototype for Settings, widget launch and one
   reversible native appearance change. Use an isolated test account/session.
3. Add portal startup, bundled helpers and narrow file/bus access. Test the main
   widget flows and installation coexistence. Keep unsupported controls absent.
4. Finish listing metadata using actual captures from that Flatpak. Refresh
   `packaging/io.github.profilaz3.Halcyon.metainfo.xml`: it still marks 0.2.5 as
   unreleased and has descriptions/captures predating the decorative material
   fallback and unconfigured public Calendar sign-in. Preserve immutable 0.2.5
   release/APT package bytes; changes belong in a new package release.
5. A human maintainer submits the tested package under the destination's current
   contribution rules. Report listing status only after upstream acceptance.

The existing AppStream ID, desktop entry, icons, screenshots and license provide
a starting point. AppStream XML validation and public APT downloads already pass;
neither establishes native Store search visibility or Flatpak readiness.

## Primary references checked

- [COSMIC Flatpak repository](https://github.com/pop-os/cosmic-flatpak)
- [COSMIC Flatpak PR checklist](https://github.com/pop-os/cosmic-flatpak/blob/master/.github/PULL_REQUEST_TEMPLATE.md)
- [Pop!_OS contributor guide](https://github.com/pop-os/pop/blob/master/CONTRIBUTING.md)
- [Flathub requirements and AI policy](https://docs.flathub.org/docs/for-app-authors/requirements)
- [Flatpak sandbox permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html)
- [Background portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Background.html)
- [System76 repository and Store behavior](https://system76.com/support/managing-apps)
