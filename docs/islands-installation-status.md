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

Package presence requires dpkg's installed status and the regular package CLI.
An unsupported payload protocol/source sets compatibility false; a missing or
altered payload/manifest sets health false, while installed/package provenance
remains available for management. Manual evidence requires a completed, unrestored `companion/install.py` record with no package
owner and exact binary, manifest and selector hashes/modes. A removed package's
user bundle is not a manual installation. Files or a socket alone are not proof.
Installation does not imply activation. Compatibility uses the existing selector's
reviewed-source policy; it does not claim runtime ABI or full-session acceptance.
Running and runtime health stay unknown in this read-only response; payload
damage can establish unhealthy status. A capable IPC host is required separately
for live arrangement.

Settings refreshes on open/focus. It uses the shared installation state for
Add-ons and Islands controls. Installation/runtime probes and native add-on
edits run on blocking workers; the event handler consumes structured results.
Read-only UI planning uses that cached capability. The store independently
revalidates on the worker under transaction locks before sidecar writes, and
refuses a write if a UI handler bypasses the worker. Imported
and scheduled looks omit optional Islands settings, including `island_gap`, when
absent, keeping saved sidecars. Applying a look does not enable disabled Islands. The setup wizard and Dock
controls still honor an explicit user choice to enable or disable them.
Only App settings → Add-ons shows `Get Islands ↗` when absent.

Tests use an explicit root. The CLI `--root` path never queries host packages or
IPC; Rust fixtures read only `<fixture>/islands-installation.json`, with the same
versioned JSON shape. Missing fixture evidence means absent. Pending/corrupt
fixture evidence never falls back to the host. The fixture record is not a live
installation attestation API.
