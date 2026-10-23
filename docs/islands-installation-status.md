# Islands installation capability, version 1

Halcyon consumes the read-only `halcyon-islands status --json` contract. The same
implementation (`companion/install_status.py`) is also shipped with the main app
at `/usr/lib/cosmic-ext-halcyon/islands-status.py`, so removing the optional CLI
still gives a reliable absence result. Rust uses that stable helper directly.
No probe launches the companion, changes selection, or opens session IPC.

The JSON object has `format: "halcyon-islands-installation"`, `version: 1`,
`installation: "installed" | "absent"`, `provenance: "package" | "manual" | null`,
`enabled: boolean`, and nullable booleans `compatible`, `running`, `healthy`.
Unknown format/version, timeout, invalid data or failed discovery means **pending**
in Rust. Pending exposes neither Islands controls nor the install link.

Package evidence requires dpkg's installed status, the CLI, and a regular,
executable, checksummed protocol-1 binary and reviewed manifest. Manual evidence
requires a completed, unrestored `companion/install.py` record with no package
owner and exact binary, manifest and selector hashes/modes. A removed package's
user bundle is not a manual installation. Files or a socket alone are not proof.
Installation does not imply activation. Compatibility uses the existing selector's
reviewed-source policy; it does not claim runtime ABI or full-session acceptance.
Running and health stay unknown in this read-only response; a capable IPC host is
required separately for live arrangement.

Settings refreshes on open/focus. It uses the shared installation state for
Add-ons and Islands controls; the store revalidates before sidecar writes. Imported
and scheduled looks omit optional Islands settings, including `island_gap`, when
absent, keeping saved sidecars. Applying a look does not enable disabled Islands. The setup wizard and Dock
controls still honor an explicit user choice to enable or disable them.
Only App settings → Add-ons shows `Get Islands ↗` when absent.

Tests use an explicit root. The CLI `--root` path never queries host packages or
IPC; Rust fixtures read only `<fixture>/islands-installation.json`, with the same
versioned JSON shape. Missing fixture evidence means absent. Pending/corrupt
fixture evidence never falls back to the host. The fixture record is not a live
installation attestation API.
