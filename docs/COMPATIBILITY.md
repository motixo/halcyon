# Islands compatibility and immutable qualification

Group 3 implements discovery, classification, package generations and an evidence
interface. **No new native cohort is admitted in the shipped catalog.** The final
new package must pass the full native gate before publication. The empty catalog
blocks new activation/staging; an existing 0.2.0 installation is left untouched.
Previously observed 1.10 source-only and 1.11 development results are retained as
explicit non-admitting observations. Full native 1.10 inputs remain unavailable.

## Trust and identity

`scripts/compatibility/discover.py --source-config <json> --previous <json>
--out <candidate.json>` verifies InRelease with gpgv in a private empty trust
home. The public Pop key is pinned to SHA256 in `pop-noble.json` and fingerprint
`63C46DF0140D738961429F4E204DD8AEC33A7AFF`, independently published at
https://apt.pop-os.org/. Its bytes are from the pinned upstream keyring commit
`356a75e405d00a23889998d2e8f2508a9e67b535`. This key is public verification
material; no signing key is generated or needed.

Discovery verifies signed scope, index SHA256/size, every selected binary and
full source archive SHA256/size. Source archive identities include Debian
patches; a version's abbreviated Git suffix is never binary attestation. HTTPS
is required and downloads, subprocesses and decompression are bounded. Pop's
Release currently has no Valid-Until: the earlier of signed Valid-Until (when
present) and signed Date + **seven days** is the enforced deadline. Future dates,
wrong keys, expired metadata and changed bytes block the candidate. Clock trust
remains the executing machine's responsibility.

The default configuration deliberately discovers the five core package/source
identities without calling them a complete environment. Supply `cohort_lock`, an
exact name→version map from the retained complete image, and all its signed
archives (including the distribution's pinned Ubuntu keys/components). All
Depends/Pre-Depends alternatives, version constraints and virtual providers must
resolve inside that lock using `python3-apt` semantics. Native qualification also
requires an exact guest package inventory, clean dpkg audit and dependency
closure. Session/applets/settings daemon, libraries, graphics, kernel and system
packages belong in the complete image identity, not just panel/comp/session.
Missing archive retention/source inputs explicitly block discovery. Optional
`retain_packages: true` saves verified content-addressed debs beside the result.

Candidate ID is the canonical JSON SHA256 of distribution, suite, architecture,
whole package/source/hash/file inventory and required processes. Index refreshes
with unchanged payloads do not create another candidate. An event ID combines
candidate and stable failure reasons, with a bounded deduplication history.
The six-hour GitHub workflow writes only artifacts/cache, has read-only rights,
and is not deployed until this branch is reviewed and merged. No runtime path
fetches a catalog or package over the network.

## Bundle, catalog and running session

`companion/bundle_contract.py` is the Python policy authority.
`classify_compatibility(installed, running, manifest, catalog)` is pure and returns
`ready`, `logout_required`, `unsupported` or `invalid_bundle` with a stable reason.
Measurements are separate IO adapters. V2 manifests bind every helper, selector,
installer, catalog and ELF by exact digest/mode; the generation is the SHA256 of
the final manifest. The payload hash binds all executable components except the
catalog; the manifest binds the catalog separately. One package can contain
multiple cohorts for that exact payload. If an older ABI requires a different
payload, retain its immutable companion package and evidence; do not attach its
cohort to this package's binary or install COSMIC version holds.

Installed identity checks distribution/suite/architecture, exact package versions
and verified package runtime-file inventories. Running identity measures the
same-user, same-XDG-session compositor, session and settings daemon executable
and mapped-file bytes from procfs against retained cohort inventories. Deleted,
inaccessible, missing or ambiguous processes/mappings mean unknown. It never
uses a newly installed executable's version as evidence for a running process.
A partial cohort is unsupported; installed-new/running-old or unknown running
identity requires another login. These conservative checks can fall back on a
restricted procfs machine; a responsive IPC socket cannot override admission.
Runtime ABI probe, inherited notification isolation, child-owned IPC readiness,
one-host locking/supervision and crash latch remain additional gates.

New launcher execution requires V2 measured admission. Legacy source metadata is
readable for ownership migration and old manual-install status, without expanding
V2 admission. The actual old copied 0.2.0 launcher remains byte-for-byte untouched
until a qualified generation can be successfully staged.

The catalog binds a cohort to an exact payload and preallocated evidence ID,
complete image SHA256 and scope. This avoids a circular hash in which evidence
contains a package digest while that package embeds the evidence digest. A row
is a **reviewed qualification target**, not permission to publish. The detached
final result and explicit approval bind the complete final deb, manifest,
catalog, payload and native observations. No native-session row is invented here.
Catalog authoring and final evidence review remain maintainer-controlled.

## Finalize, qualify, promote

1. Build pinned source with `companion/build.py --revision <full-40-hex>
   --archive-sha256 <64-hex> --archive <file> --workdir <new-dir>
   --target-dir <isolated-target> --offline`. Patch uses `--fuzz=0`.
2. `packaging/islands/build-deb.py --binary <ELF> --manifest <source-manifest>
   --licenses <notices> --version <monotonic-version> --catalog <reviewed-catalog>
   --out <dir>` strips and finalizes **before** qualification. A preliminary build
   may establish payload hash for catalog authoring; only the final complete
   package is submitted for acceptance. Existing version filenames cannot be
   replaced by different bytes. Keep all corresponding source and notices.
3. Supply a `halcyon-native-image` v1 JSON descriptor: `kind:
   complete-login-session`, `sha256` (disk), `recipe_sha256`,
   `retained_inputs_sha256`, `cohort_id`, and a precise `graphics` scope. Retain
   signed ISO/index/package inputs, full package map, firmware/kernel, recipe and
   runner versions independently. Software VM is not physical GPU qualification.
4. Run:
   ```sh
   python3 scripts/compatibility/qualify.py \
     --candidate verified-candidate.json --image retained-image.json \
     --package final-islands.deb --out new-evidence-dir \
     --driver /absolute/path/to/reviewed-native-driver
   ```
   Without `--driver`, this produces the exact immutable request and a blocked
   result, never a simulated pass. The driver is parent-owned infrastructure,
   selected independently of candidate code. It receives `--request <json>
   --out <dir>`, a fresh private HOME, only PATH/locale/TMPDIR from the environment,
   and a 90-minute bound. The request fixes exact package/manifest/payload/catalog,
   image/cohort and mandatory scenarios. Driver outputs `observations.json` and
   relative digest-bound evidence. Packages are not rebuilt inside this step.
5. `native_scenarios.py` requires independently observable login, applet input,
   rendered bars, notifications, one host, logout/crash/fallback, hung and
   responsive-but-absent hosts, descriptor failure, reset/Undo, legacy Repair,
   manual modes, real native Auto, four materials, radii, vertical bars,
   transparent gaps, partial packages, running-old/installed-new, actual 0.2.0
   migration, multi-generation Disable and disabled/new/external users. Each
   scenario needs native=true, result=pass, explicit observed/expected assertions,
   and hashed artifacts of its required kinds. Missing/blocked scenarios fail
   closed. Fractional-scale, multi-monitor and physical GPU are separate claims.
   Unit fixtures validating this schema are explicitly not native evidence.
6. Review the raw observations, images, logs and every failure. Write an explicit
   approval JSON with `decision: approve`, `package_sha256`, `evidence_sha256`
   (result.json) and `review_reference`. Run `qualify.py --package <same-deb>
   --evidence <dir> --approval <file>` to recheck every observation/artifact.
   No automated status or successful build creates this approval.

The existing real infrastructure is the parent's
`ci/qualification-probe-021` retained-image runner. Its documented flow verifies
the retained artifact/ISO/APT/disk/firmware hashes, creates a qcow2 overlay, boots
QEMU with private QMP/QGA sockets and `-nic none`, and injects final debs through
read-only ISO media. Adapt that runner to the above request/result protocol;
never mount a live user home or give the guest publication credentials. The
original reports and scripts remain read-only in the sibling infra worktree.
The recorded 1787-package software-graphics guest and old frozen-package runs
are baseline inputs, not acceptance of this change. Artifact retention currently
expires October 30; the parent must preserve it before running final acceptance.

`release.yml` now verifies reviewed immutable candidate artifacts, rather than
building and automatically publishing fresh bytes on a tag. Group 4 supplies the
restricted publication step after combined acceptance. `release_gate.py` requires
main+companion+corresponding source hashes, exact native approval and a passed
system-update gate. No signing/publishing credentials enter this workflow.

## Generation lifecycle and diagnostics

The package's XDG autostart invokes `refresh.py` independently of Halcyon widgets
or Settings. It runs only in a COSMIC session, from locally installed package
bytes. Root package scripts never touch homes or restart bars. The conffile
respects administrator changes across package upgrades.

`stage_installed_update(home, installed_bundle)` returns `unchanged`, `staged`,
`disabled`, `external_change` or `blocked`, plus a next-login flag. It verifies the
installed package version/root-owned bundle, intended installed cohort, existing
owned enable record and all effective current-file hashes/modes. It shares the
CLI's nonblocking per-user `.islands.lock`, copies a whole generation, fsyncs it,
then journals and atomically replaces selection/selector through `install.py`.
A changed payload under the same version and all downgrades are refused.
Incomplete journal layers are rolled back with the existing guarded replay.
External edits block automatic repair. Orphan staged generations are harmless;
there is no automatic cleanup of saved recovery evidence.

The stable selector verifies a single content-addressed generation before
importing it. It can stage a newer local package but uses its already resolved
generation for this launch. It never swaps a live host. The guarantee is **the
next login after successful staging**, possibly the second login after the
original package upgrade. Crash markers are never cleared. New/disabled/manual
users are never automatically enabled or converted. Unsupported XDG layouts are
blocked, not redirected into another user's state.

Every selection change is a layer in the original checksum/mode-guarded
installation chain. `halcyon-islands disable` unwinds these layers, then the
original 0.2.0 installation. Existing layout, baseline/Undo and crash markers
remain intact. Package removal leaves a verified retained generation usable only
for its known cohort. If the CLI itself was removed, the retained generation's
`install.py --restore <install-record.json>` remains available; restore owned
records newest-first. No removed package's leftover files become manual
installation provenance.

`halcyon-islands status --json` and `diagnostics` expose versioned installation,
compatibility, selected/installed generation and reason separately. Settings
consumes this on its existing background worker and offers Copy diagnostics.
Read-only diagnostics contain no paths, config contents, session IDs, tokens or
exception text. Absent installations still have exactly one Get Islands link;
installed disabled/broken/incompatible states retain management.

## Local regression commands

```sh
python3 -m unittest discover -s scripts/compatibility -p 'test_*.py'
python3 -m unittest discover -s companion -p 'test_*.py'
python3 scripts/test_panel_companion.py
python3 packaging/islands/test_islands.py
```

For actual released-package migration tests, set `HALCYON_RELEASED_020` to the
immutable public `cosmic-ext-halcyon-islands_0.2.0_amd64.deb`, SHA256
`1ec258de00228f02d37fa6201bd4ac4050344594cb452ad6d05ada459ea81c5e`.
`companion.test_refresh.Actual020` extracts and invokes the actual shipped
installer, copies its actual launcher and ELF, migrates, then reverses multiple
generations. With no supplied package those tests explicitly skip; fixture tests
never stand in for them or real-session migration/notification acceptance.
