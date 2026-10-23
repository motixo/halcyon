# Dock Islands companion

Dock Islands is an optional style on Desktop > Dock & panel. Standard remains the default. It uses the existing Start, Center and End applet sections, including repeated and unknown IDs. No applet or top bar is added automatically.

Together has one exact gap and a movable compact group. Separate has individual positions and a minimum gap. Both layouts are saved. Choose an edge, use Arrange, drag the temporary handles, then Done and Apply. Tab selects handles, arrows move or reorder, Home/End place at the ends, and Escape cancels a gesture first. Settings also has move buttons. Cancel restores the draft from before Arrange. Discard and persistent Undo keep their existing meanings.

Apply native screen-edge or applet-list edits before arranging the live dock. Those edits can recreate native applets, so a companion restart or an external structural edit ends Arrange safely. Unapplied drafts retain their baselines. Dock Islands does not write preferences during pointer motion.

Islands use native theme colors, opacity and frosted-alpha mapping. When the compositor offers background blur and the native theme enables panel frosting, each island has its own rounded blur region. The surface-wide radius stays zero; a coalesced union of logical-pixel rectangles supplies the shape. Input masks share that geometry but keep reveal strips and arrangement handles separate from blur. Transparent gaps pass clicks through. No launcher or companion code changes global theme preferences. COSMIC still reserves a rectangular edge strip according to its normal policy.

The IPC `rounded_blur` capability reports whether all active panel surfaces have a blur object; it is not the current frosting preference or a promise about an unqualified compositor. Without native blur the usual theme-backed background remains available. See [the material integration contract](MATERIALS.md) for Task 2.

## Compatibility and build

The patch targets official cosmic-panel `ca20ce3185b3b4404f429d59d429c40b58a2ddef`. The selector admits the complete COSMIC 1.10 and 1.11 package combinations listed in [the package README](../packaging/islands/README.md#supported-versions). `SUPPORTED_SYSTEMS` plus the bounded `reviewed-source-v1` policy in `panel_launcher.py` are the runtime authority; packaging copies the reviewed combinations and declared policy into the manifest so Halcyon can explain the same decision. The legacy `packages` field retains the primary build combination for older clients.

Distribution/timestamp rebuilds of the same reviewed source triple are eligible when the binary declares runtime probe 1. Eligibility is qualified source compatibility, not full OS certification. Unknown source revisions, unparseable versions, versions below the 1.10 panel/session floor, and mixtures of reviewed triples select stock. Add new source triples only after source review and native behavior validation; retain older triples. A future release number alone never bypasses the gate. Desktop Cards and the companion retain separate libcosmic pins and lockfiles. The shared `dock-islands-core` crate contains only schema, IPC types and logical geometry.

Build in a new directory, using Rust 1.93.0 and the native dependencies listed in upstream's README:

```sh
python3 companion/build.py --archive /path/to/verified-upstream.tar.gz --workdir /path/to/new-panel-source --target-dir /path/to/panel-target --offline
```

Omit `--offline` to fetch missing locked dependencies. Omitting `--archive` downloads the exact official archive and verifies its SHA256 before extracting or patching it. `--prepare-only` verifies and applies the patch without building.

## Review and installation

The release handoff contains both binaries, their hashes, frozen sources, native evidence and `companion-manifest.json`. Installation is separate from ordinary Desktop Cards installation. It never changes native applet lists, the sidecar, notes, accounts, widget layout, system binaries or running processes.

Stage into a fixture home first:

```sh
python3 companion/install.py --root /path/to/fixture-home --binary /path/to/cosmic-panel-islands --manifest /path/to/companion-manifest.json
```

After review, the same installer accepts `--user`. It installs a per-user `~/.local/bin/cosmic-panel` selector and a versioned bundle under `~/.local/share/cosmic-desktop-cards/companion`. The session PATH must put `~/.local/bin` before `/usr/bin`. Selection defaults to stock. `--enable-next-login` records the current boot and login identity. Restarts in this login stay on stock; a later login selects the companion. Run this option from the COSMIC desktop session so its login identity is available. The running session is never restarted by this installer.

Exactly one panel host owns applets. The selector checks for an existing host on the same Wayland display and holds a session lock. It verifies the binary hash and source qualification before starting the companion. New bundles then run the checksummed executable's five-second `--islands-probe`: its dynamic loader checks directly linked native libraries and a fresh named Wayland connection checks required globals. This path creates no surfaces and never consumes the session's inherited notification/Wayland descriptors. The running companion must answer its own IPC within 15 seconds; a silent startup is killed and handled as a failed host. Protocol-error flooding is also supervised. These checks cannot prove that every advertised protocol behaves correctly, hence the separate reviewed-source gate. A component crash records a failure marker and selects stock. In COSMIC's supervised session, it exits so cosmic-session can recreate the panel/notifications socket pair before restarting stock. Without a session notification socket, it starts stock after reaping the failed host. The tested notification descriptor is inherited intact; an already authenticated stream is never reused in a different host.

## Restore

The installer returns an `install-record.json` path and retains every replaced companion file. Restore refuses to overwrite a subsequent outside edit:

```sh
python3 companion/install.py --restore /path/to/install-record.json
```

The installer journals intended replacements before writing. An interrupted install can use the same restore command. Restore checks every managed file and backup, including modes, before changing anything. Restoration affects the next login and does not terminate a running host. Selecting Standard in Desktop Cards immediately returns the active companion to its ordinary native layout. Removing the per-user selector also leaves `/usr/bin/cosmic-panel` as the next-session default. Neither operation deletes the saved island layouts or native preferences.

After a crash, `~/.local/state/desktop-cards-panel/failed` keeps future logins on stock. Remove that marker only when deliberately retrying a reviewed component. The companion sidecar is `~/.config/cosmic/io.github.desktopcards.DockIslands/v1/{Dock,Panel}`; Desktop Cards is its only persistent writer. Unsupported versions are preserved and rendered with the ordinary dock after restart.
