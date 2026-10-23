# Updates adapter handoff — Rust/UI owner

Group 4 packaging preparation did **not** edit Rust/UI, run Cargo, launch Store,
call the system PackageKit service or change a live desktop. Arrange's Rust owner
implements this remaining work. The Python reference model at
`packaging/updates/status_contract.py` and its tests are executable interface
examples, not installed helpers or claims of UI acceptance.

## Interfaces actually inspected

Read-only inspection on 2026-10-23 found:

- APT `2.8.3`, PackageKit `1.2.8-2ubuntu1.5`, appstream `1.0.2-1build6`.
- Installed COSMIC Store `1.11.0~1792525498~24.04~16b1667`, executable SHA256
  `35d5b8095552e2b8d4db4729d6702b1ad36fbe05ff3c0433b8cb828c73ef0711`.
- `/usr/share/applications/com.system76.CosmicStore.desktop`, SHA256
  `30dfd24161028dab44bd56a5bc1acf5e98764b4d651165c8c95309513d79551e`,
  declares `Exec=cosmic-store %u`, no Updates desktop action.
- Installed `/usr/share/dbus-1/interfaces/org.freedesktop.PackageKit.xml`, SHA256
  `67b74cafb27e8579ecadf1a3d2da0197766aaecc8ed742f66e18f8a11fbc6d22`.
- Installed `org.freedesktop.PackageKit.Transaction.xml`, SHA256
  `316c468e843d8f0d277113118302dd4adca770c3abc3843e4c85bd0211a91421`.

The Store package's version suffix resolves to upstream commit
`16b1667e43c634318a99995c8eb77c23603ae560`. This is a source inspection anchor,
**not** proof that the installed binary was built unmodified from that commit.
Retain the guest's real signed package/source provenance for final qualification.
Relevant inspected upstream files and SHA256:

| File | SHA256 |
| --- | --- |
| [main.rs](https://github.com/pop-os/cosmic-store/blob/16b1667e43c634318a99995c8eb77c23603ae560/src/main.rs) | `22649e39931023b941ff21ca1edeae30a8708c0eda87b60423346e9619a6dcdf` |
| [backend/packagekit.rs](https://github.com/pop-os/cosmic-store/blob/16b1667e43c634318a99995c8eb77c23603ae560/src/backend/packagekit.rs) | `820c46ff0965824aa9bc472a559e17ede9e07c3eb7744092bb761a2ca751f7da` |
| [appstream_cache.rs](https://github.com/pop-os/cosmic-store/blob/16b1667e43c634318a99995c8eb77c23603ae560/src/appstream_cache.rs) | `7af44b747de390b85116db1c412f5b197970816d54452c49054b8af8f6a59c1a` |
| [update.rs](https://github.com/pop-os/cosmic-store/blob/16b1667e43c634318a99995c8eb77c23603ae560/src/update.rs) | `a40b6649c40c144299dbc9a86ab9e40b0bb6cc300d9bd47e381a901e26a9860f` |
| [Cargo.lock](https://github.com/pop-os/cosmic-store/blob/16b1667e43c634318a99995c8eb77c23603ae560/Cargo.lock) | `0ab27320867b3e443e1c41abff28bafa600cf185a723942e1cb207ee28dc23f2` |

Also inspected current upstream `5b9bbeddb02d5a783299fb8c809fca0edb302c89`:
the same relevant launch behavior remains. In pinned `main.rs`, a positional
argument becomes `search_input` at initialization; single-instance
`ActivateAction` likewise becomes search text. **Do not implement
`cosmic-store updates`, `--updates`, an `appstream://updates` URL or an invented
D-Bus Updates action.** None is a verified route in this source.

Verified available launch contract: launch desktop ID
`com.system76.CosmicStore.desktop` without a URI/argument through the normal
desktop application launcher. Label the row action **Open COSMIC Store**, with
concise guidance **Choose Updates**. Check desktop entry availability; if missing,
show a nonblocking unavailable state. Native confirmation that the correct Store
window opens remains outstanding. System76's documented flow is Store → Updates →
Update all, independently consistent with the inspected code:
[System76 software management](https://system76.com/support/managing-apps).

Store's PackageKit backend creates transactions on the system bus, queries
GetUpdates and groups updates without AppStream entries as system packages.
Its AppStream cache scans `swcatalog` and `app-info` XML/YAML directories under
`/usr/share`, `/var/lib`, `/var/cache`. This explains why OS updates and searchable
catalog metadata are separate. Local tests validate/fetch collection XML/YAML and
icons, but do not establish clean-Store search, screenshots or icon rendering.

## Rust adapter to implement

Use asynchronous D-Bus access to the **installed noble interface**, whose case
differs from some newer PackageKit web documentation:

| Operation | Destination/interface/path and signature |
| --- | --- |
| Create transaction | System bus destination/interface `org.freedesktop.PackageKit`, path `/org/freedesktop/PackageKit`, `CreateTransaction() → o` |
| Transaction interface | Same destination, returned object path, `org.freedesktop.PackageKit.Transaction` |
| Hints | `SetHints(as)` before starting; use documented supported hints, no credentials |
| Refresh metadata | `RefreshCache(b force=false)` on its own new transaction |
| Query updates | `GetUpdates(t filter=2)` on a separate new transaction; None filter is `1 << 1`, not zero |
| Installed resolve, if needed | `Resolve(t filter=4, as package_names)`; Installed is `1 << 2` |
| Package signal | `Package(u info, s package_id, s summary)` |
| Failure signal | `ErrorCode(u code, s details)` |
| Completion signal | `Finished(u exit, u runtime)` |
| Cancellation | `Cancel()` with `AllowCancel` property respected |

Use the actual binding enum values for successful exit and error codes, not
guessed numbers. Package IDs are `name;version;architecture;data`. The `data`
field is backend metadata, **not** authenticated Release Origin; never use it as
the enrollment trust check. See the pinned backend's `FilterKind`,
`transaction()`, `updates()` and `refresh_cache(false)` implementation. No runtime
bus calls were made in this task, including discovery that might activate a daemon.

Subscribe to signals before starting the transaction. A method reply is not
completion. An explicit **Check for updates** creates a request token, refreshes
metadata, waits for successful completion, then queries updates in a new
transaction. Bound the whole operation (suggested 60 seconds) and each connection
step. Cancellation/Settings close invalidates the token and drops subscriptions;
ask PackageKit to cancel when allowed. Ignore late signals from obsolete tokens.
Never kill the daemon or keep the UI waiting for a transaction it cannot cancel.
Do not run synchronous commands on the UI thread or periodically poll/network.

If RefreshCache needs authorization, use only normal OS/PolicyKit interaction;
never prompt for, collect or forward a password. Updates themselves are owned by
COSMIC Store. There is no application InstallPackages/UpdatePackages action in
this handoff. Distinguish missing service, offline, timeout, cancellation,
authorization cancellation, trust failure, malformed results and success. On
error, discard the candidate from this request; cached metadata must not be
described as a fresh successful check or “up to date.”

## Installation-channel measurement and row

Identify the **running executable**, not `which halcyon`, another PATH match,
the mere existence of `/usr/bin/halcyon`, a running Islands IPC endpoint or a
package installed alongside a source build.

1. Resolve and hash `/proc/self/exe`, retaining its deleted/replaced status.
   Measure the canonical installed path and compare the actual bytes. Require
   dpkg's installed state, exact package owner, architecture and verified packaged
   executable (including dpkg's installed file checksum record). No package-owned
   match means Unknown; a positively known source/manual build is Source.
2. Treat source builds running outside `/usr/bin/halcyon` independently of any
   packaged installation on the machine. Unknown ownership stays Unknown. A
   deleted/replaced old running executable requires restart/recheck, not silently
   treating the current `/usr` version as the running app.
3. Enrollment is verified-enabled only with the scoped keyring package, owned
   enabled Deb822 source, exact HTTPS endpoint/suite/component/architecture,
   Signed-By path, and exact origin/package preferences contract. Check actual
   current configuration and package ownership, not a remembered Boolean.
   Administrator edits/deleted/Enabled:no sources are preserved. Unknown or
   conflicting configuration is not an APT channel claim.
4. Packaged main + verified-enabled enrollment → Apt. Packaged main with absent
   or disabled source → ManualDeb. Source → build/release information. Unknown →
   installation information without a wrong-install upgrade action.

Implement `UpdateChannel::{Apt, ManualDeb, Source, Unknown}` and a versioned status
with running version, verified installed package version, candidate version,
check state, channel/reason, action and independent pending-Islands-login flag.
Only a verified same-install Apt channel can offer a manual PackageKit check.
Debian version ordering is mandatory (`~rc`, numeric revisions, epochs); do not
use string or SemVer ordering. A returned Islands update never becomes a main
version, and absent Islands never creates an install recommendation.

One App settings Updates row is enough:

- Apt: installed/current version, Check for updates, Open COSMIC Store. Available
  version is separate from the installed/running version.
- ManualDeb: Get latest release and Enable system updates information. The latter
  remains unavailable until the parent has verified the live endpoint, bootstrap
  assets and fingerprint; opening information is not enrollment consent.
- Source: build/release information. Unknown: truthful installation status.
- Verified installed Islands with the existing Group 3 `logout_required` refresh
  status: a separate short next-login notice. No forced logout or live replacement.
  With Islands absent, retain only the existing Add-ons **Get Islands** link.

Use the existing installation/status and migration helpers; no packaging helper
rewrite is required. An actual successful staged update is what warrants “Log out
to activate,” not simply a newer archive candidate. Preserve enabled/disabled,
external-edit and crash-marker behavior.

## Remaining validation and ownership

`test_updates.py` covers source/other-PATH installation, replaced/deleted binaries,
explicit/disabled enrollment, no automatic check, update/no-update ordering,
wrong architecture, offline/missing PackageKit/timeout/auth cancellation, malformed
signals and absent/pending Islands. These are **policy reference tests only**.
The Rust owner must port them into its actual async adapter and exercise request
tokens, timeout/cancel ordering, stale signals, missing Store, real measurements
and row rendering/accessibility. Implement in the existing Settings architecture
with focused tests; parent coordinates Cargo and the final integrated suite.

Parent-owned native trial must use the exact finalized main/companion/source:
genuine old GitHub install → explicit local bootstrap → normal Store/PackageKit
main + already-installed companion upgrade → widgets stopped/user bootstrap →
logout/login → new bundle → unknown-cohort stock fallback → Disable/restore.
Include source/other-PATH cases, absent/disabled Islands, offline/trust failure,
OS authentication cancellation, interrupted update/recovery, removed enrollment
and clean Store discovery separately. Retain real process, input, render and
transaction observations in the Group 3 scenario contract. The Python model and
APT fixture results cannot replace that evidence. No 1.10, complete source closure,
physical GPU or new catalog admission is established by this task.
