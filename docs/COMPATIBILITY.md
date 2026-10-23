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
wrong keys, expired metadata and changed bytes block the candidate. A static base
pocket with Suite=Codename and no Valid-Until can instead use an explicitly
reviewed `immutable_release_sha256` pin of the complete signed InRelease; a
publisher Valid-Until is never overridden. Rolling pockets retain the seven-day
policy. Clock trust
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

Each archive declares `pocket` (the `dists/` path), expected signed `suite`,
`codename`, `origin`, components and pinned public key. These are distinct from
installed `distribution`/`suite`/`architecture`: Ubuntu noble-updates and
noble-security both have signed Codename noble. Binary and source rows retain
all signed index provenance, including collisions. Conflicting binary bytes or
control dependencies block selection unless `package_sha256[name]` explicitly
pins one binary identity; conflicting source archives independently block it.
Archive iteration order cannot silently overwrite either identity.

`--retained-inputs inputs.json` reads only a URL→`{path, sha256}` blob manifest;
missing entries are never fetched. The signed Release still authenticates every
index. `resolve_metadata_only: true` resolves exact locks without downloading
package/source blobs, reports all missing blobs with URL/hash/size, and always
blocks qualification. A normal run also reports missing retained blobs explicitly.
The result's `missing_inputs`, `collisions`, `indexes` and per-package/source
`provenance` retain the authentication and reconstruction trail. Missing binary
bytes mean no `files` inventory and cannot form an admissible catalog row.

Candidate ID is the canonical JSON SHA256 of distribution, suite, architecture,
whole package/source/hash/file inventory and declared processes, excluding proof
locations (`url`/`provenance`). Refreshing a signed index without changing payload
bytes does not create a new cohort. An event ID combines candidate and stable
failure reasons, with a bounded deduplication history.
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
and verified package runtime-file inventories, including resources. Metapackages
may have an empty file inventory; their version and authenticated deb identity
remain in the installed lock. This is separate from running identity.

Each native target declares `runtime` v1 and evidence-bound
`runtime_compatibility` bindings. The runtime profile contains:

- `processes`: the complete declared required process inventory (same names and
  paths as the cohort's `processes`). Each entry has `executable`, ELF `sha256`,
  exact `count` (1–64) and an allowlist `mapped` of executable mapping paths and
  digests. Include relevant applets/services, not just the three mandatory core
  processes. Paths must match procfs's resolved executable/mapping names.
- `constraints`: explicitly qualified measurable constraints. Supported fields
  are `kernel_release` from `/proc/sys/kernel/osrelease`,
  `kernel_version_sha256` of `/proc/version`, and `module_versions` mapping loaded
  module names to sysfs `version`/`srcversion` strings. These strings are not
  kernel/module/firmware byte attestations. Omitted constraints make no claim.
- `unmeasured`: the explicit list `previously-read-resources`, `kernel-bytes`,
  `driver-firmware`, `anonymous-code`. Native qualification must assess whether
  those limits are appropriate for the target; the classifier never labels them
  observed package bytes.

A profile ID is its canonical JSON SHA256. Each accepted binding has `id`,
`evidence_id` equal to the target's evidence ID, and a nonempty `reason` recording
its qualified equivalence/scope. Profiles referenced by bindings must exist in
this catalog. Unchanged-code package rebuilds may deliberately share a qualified
profile. Resource-only changes need explicit scoped qualification; package
versions or a unique catalog row never establish running identity.

`running_identity` returns `format: halcyon-runtime-measurement`, `version: 1`,
`verified` and **all** matching `profiles`, never a full installed `cohort`.
The pure classifier requires an explicit target binding to a measured profile.
It measures every declared process in the same uid/XDG session. Executable bytes
come from the live `/proc/PID/exe` inode, even after package replacement. Mappings
come from map_files or an open file with the exact procfs device/inode identity;
changed/deleted inaccessible libraries fail closed. Two bounded snapshot attempts
check process starttime, executable and mappings; no process/lock waits occur.
Unrelated exited/zombie processes are ignored; unreadable, disappearing,
duplicated or missing required processes remain unknown (Linux truncated comm
is handled). Multiple matching profiles are safe and do not create ambiguity
about an unmeasured installed lock. Unknown or incompatible running code requires
another login. A responsive IPC socket cannot override admission.

Native drivers can collect the supported read-only measurement with
`python3 /usr/lib/cosmic-ext-halcyon-islands/runtime_identity.py --catalog
/usr/lib/cosmic-ext-halcyon-islands/catalog.json` in the actual target session.
Use its `runtime` object in raw observations; capture the process inventory and
constraint evidence independently. This command never installs, launches or stages.
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
     --package final-islands.deb --main-package final-main.deb --out new-evidence-dir \
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
   and hashed artifacts of its required kinds. Supplying `--main-package` adds
   `updates-discovery`, `updates-authorization-cancel`, `updates-apply-relogin`
   and `updates-failure-recovery`; exact evidence kinds are in SYSTEM_UPDATES.
   The final release requires these integrated scenarios and exact main bytes.
   Missing/blocked scenarios fail
   closed. Fractional-scale, multi-monitor and physical GPU are separate claims.
   Unit fixtures validating this schema are explicitly not native evidence.
6. Review the raw observations, images, logs and every failure. Write an explicit
   approval JSON with `decision: approve`, `package_sha256`, `evidence_sha256`
   (result.json), `inputs` identical to the frozen request, and `review_reference`.
   Run `qualify.py --package <same-deb> --main-package <same-main-deb>
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
main+companion+corresponding source hashes and exact integrated native approval.
A `system_updates_acceptance: passed` string grants nothing. `inputs` is a derived
identity envelope repeated exactly in request, raw observations, result and
approval: package/manifest/payload/catalog, candidate ID and full candidate hash,
image ID and full image descriptor hash, graphics scope, evidence ID, runtime
contract hash, final main package hash and extracted main identity hash. Summary fields are rederived from the
request; promotion also directly verifies final extracted bundle/catalog and
complete final package bytes. Raw observations must include `runtime_measurement`
matching the target profile, actual guest package inventory and all scenarios.
For an integrated run, request `main` is extracted from the actual final
`cosmic-ext-halcyon` deb: package digest/version, every regular `/usr`/`/lib` file
hash, `/usr/bin/halcyon` path and executable digest. Raw `main.installed` must
contain that measured installed identity. Raw `main.running` must contain
`executable`, the live inode's `sha256`, `same_session: true`, and `evidence`
references to hashed `processes` artifacts proving that observation. The driver
must read `/proc/PID/exe` and verify the actual session, not copy an installed
filename's digest. The release gate re-extracts the exact final main deb too.
The synthetic pipeline tests echo fixture identities only inside temporary test
runs; they are neither a native driver nor reusable qualification evidence.
The detached approval hashes result.json, which hashes observations.json and the
canonical request. No catalog digest cycle or post-test repack is permitted. No signing/publishing credentials enter this workflow.

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
Selector bytes and mode come from the verified immutable generation snapshot and
are digest-checked before the transaction; later package replacement cannot mix
a new selector with the selected old generation.
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
blocked, not redirected into another user's state. Production Enable checks that
`~/.local/bin` precedes `/usr/bin` in the session PATH before creating a lock or
journal. A failed prerequisite explains the required PATH; no shell file is edited.

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
