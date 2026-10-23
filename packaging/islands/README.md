# Optional Islands companion

The package contains a panel payload, a versioned local compatibility catalog,
read-only diagnostics and a user-session bootstrap. Installation never enables
Islands or restarts a bar. No package script writes a user home.

**The group 3 candidate has no newly admitted native cohort.** It is a development
artifact for review, not a qualified public update. Existing 0.2.0 assets remain
immutable. Full native 1.10 remains unavailable; old compositor-only tests used
newer libraries/applets. Existing complete 1.11 guest observations used old
released/frozen packages, not this finalized generation package.

For a qualified distributed package, run as your desktop user:

```sh
halcyon-islands status --json
halcyon-islands diagnostics
halcyon-islands enable
halcyon-islands disable
```

Enable is explicit and waits for a later login. Unknown COSMIC cohorts use the
standard panel while retaining your layout. Installed-but-disabled, incompatible
and damaged installations remain manageable. Diagnostics omit private paths,
configuration content and credentials.

The package-owned XDG autostart stages local installed updates only for verified,
previously enabled owned installations. It runs even with Halcyon widgets stopped.
Activation is promised on the **next login after successful staging**; the first
upgrade from 0.2.0 may therefore need a second login. There is no network fetch or
forced live swap. The selector resolves one immutable verified generation, and
Disable unwinds the original restore chain across updates.

A failure marker at `~/.local/state/desktop-cards-panel/failed` keeps the standard
panel selected. Updates and Disable do not clear it. Diagnose the native failure
before an intentional retry; automatic staging is not consent to retry a crash.
External edits and unsupported XDG layouts block automatic migration rather than
being overwritten. Keep the installer records if manually managing a source
installation. After package removal, retained generations include `install.py`
for guarded newest-first restoration of those records.

See [the compatibility contract](../../docs/COMPATIBILITY.md) for signed archive
trust, native qualification, exact artifact identity, generation recovery and
reproducible regression commands. Normal signed APT delivery and Updates UI are
the following group 4 work; this package does not enroll an APT source.
