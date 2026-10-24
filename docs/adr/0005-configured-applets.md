# Discover configured applets; measure panel readiness separately

## Context

Issue #12 reproduces rejection of Workspaces and Caffeine before Islands starts.
The supported-family path reused the exact-qualification applet allowlist and
child counts. Adding names repeats the problem for every other applet, wrapper,
missing optional applet or layout. The pinned native host already discovers and
launches configured desktop entries itself.

## Decision

For ordinary supported-family startup, discover configured desktop IDs using
the native host's XDG search order. Record executable entries and changes without
an applet allowlist. The helper never executes desktop-entry commands. Individual
missing/invalid applets are diagnostics, not host-admission failures. Core session
identity, the checksummed host, its ABI probe and child-owned responding IPC
remain required. The supervisor continues cleaning the entire owned process
group, including applets unknown to Halcyon.

Keep exact historical qualification's fixed inventory separate: it describes
what was measured, rather than what an ordinary user may install. No new
user-facing override or compatibility switch is introduced.

Installation status adds a bounded `applet_issues` list of desktop IDs and known
reason codes, never command lines or filesystem paths. Settings displays these
as applet-specific notices independently of the panel's Ready status. Read-only
status diagnoses current configuration even when the host is absent, and a
missing host is labelled “Using standard panel” rather than “Compatibility update needed”.

Direct executables can be observed as owned running children. Launch wrappers
and sandbox launchers are not assumed to stay alive; absence of their original
executable alone is not a failed applet claim. Configuration and executable
replacement during startup still invalidate that startup observation.

The native parser skips empty/malformed applet commands instead of panicking.
Policy v2 binds the fresh companion ELF and patch hash; historical qualification
rows and resolver provenance remain unchanged and cannot qualify this payload.

## Alternatives and consequences

- Expanding the fixed list keeps rejecting arbitrary valid user layouts.
- Removing all startup checks loses host integrity and lifecycle guarantees.
- Reimplementing applet launching introduces a competing desktop-entry loader.

The helper's inventory is diagnostic and never proof of popup/rendering health.
Tests must cover reported IDs, other custom applets, missing/broken entries,
aliases/wrappers, repeated instances, changes, cleanup and bounded status data.
Real popup checks remain distinct from isolated process tests.
