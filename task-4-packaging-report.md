# Task 4 packaging report — local and unpublished

Implemented Group 4 distribution foundations in the product checkout only.
**0.2.1 publication remains ON HOLD.** No push, PR, tag, release, hosting, real key
provisioning, live enrollment, production collector call, credentials, desktop
mutation, infrastructure checkout edit or subagent was used. No Rust/UI files were
edited and no Cargo command was run. The parent owns final native integrated
acceptance, real signing material, Pages and publication.

## Scope and source identity

Task start was `8ddf03b`. Concurrent Arrange work committed `2aae01f` while this
implementation was in progress. The implementation commit's reviewed parent is
`2aae01f0db23e468e1071e3f845286b8c29a2db8`. Required ancestors `8ddf03b`, `b954c7c` and `43bc18d` remain present.
No migration, compatibility, main/Islands builder or release-gate rewrite was
needed. Parent `docs/PLANNED_WORK.md`, pre-existing untracked evidence and all
sibling work were left outside this commit. This report is intentionally in the
product root, not in the parent's evidence sibling.

Read the applicable task brief, system-updates plan, recovered execution amendment,
preflight rulings, current distribution/compatibility documentation and recent
Group 3 fix reports/reviews. No applicable AGENTS.md was found in ancestors or the
product/evidence trees. The latest explicit user publication prohibition governs
the stale brief/amendment language.

Owned implementation paths and exact SHA256 (excluding this self-referential
report; the Git commit binds it):

| Path | SHA256 |
| --- | --- |
| `.github/workflows/apt-publish.yml` | `4f832d18d95615e9a2684eeb45d96a81f07e304e5f60b737ccd4ba35ef8dee69` |
| `docs/DISTRIBUTION.md` | `f7dc5b50d68cbc4da7a8e32b2b763104817834a1f27213705455b594f5b226bc` |
| `docs/APT-OPERATIONS.md` | `b5e0d46aca7e04a622dc39931445fb279034c5e09bdbd3a47698b18e8b33015c` |
| `docs/UPDATES-HANDOFF.md` | `04be5b8a4a81f9dc7da07943fea233e4813a578d87ef5db10f78c66627da037c` |
| `scripts/apt/archive.py` | `04326ed122c7fa6b6b1331cf0c437167663646842d65fe8e36cbf43156995651` |
| `scripts/apt/build_archive.py` | `397619251b697478bb55485aa938f3bf1c42082befda22f5c7fa39c0376bface` |
| `scripts/apt/check_expiry.py` | `a6b70be189f8a58ee368ebb519bf98a5f33102c297839112fb9f60002e50d701` |
| `scripts/apt/promotion.py` | `3968d7b1d6579d6bfff9c8bf813d2f1b26fa7a03856761105f4f169da3aa6934` |
| `scripts/apt/test_admission.py` | `81a9d131eaa5efee685ce9c289e830418a0b34bd16335f10428f3fd12f12d259` |
| `scripts/apt/test_archive.py` | `6227b366d6870a1535ec6876e8cdc69cf93ed9b722b74d36cb2964070339c27f` |
| `packaging/updates/build-deb.py` | `3d9f935f1267928797d319ebd9188164e8e0b82146156ec68ddb3e3bb265dcf8` |
| `packaging/updates/status_contract.py` | `9b3685e013cf4d82aba99afa1b341e9e959891836f52fd0b2ea9225c2dc5a997` |
| `packaging/updates/test_updates.py` | `0f4ee87d148200c185ecea6ee3a110fda47b5322ada52b9dd4d405794eb9714d` |

Unchanged integration authorities:

- `scripts/compatibility/release_gate.py`: `1891cc817a7372eac3146be3811e9a9d7c7dd59ec310221199c7e369c7a5ccfa`.
- `scripts/compatibility/qualify.py`: `a67b1d3805106c30a110019a3db8758880c03411617e47c4b577b6e82386a859`.
- `companion/compatibility/catalog.json`: `2ffc2f01baf8ab6431aea88dd39a99e41fb6b0b32372221168fe509c6b391497`; `cohorts` remains empty.

No final product deb, source release archive, admitted catalog row or genuine
native evidence was fabricated. Tests create synthetic disposable packages only.
The existing promotion regression's synthetic driver is identity-test machinery,
not native qualification. All temporary test private keys were confined to private
temporary directories, never printed/committed, and removed at test completion.

## Implemented boundaries

- Deterministic unsigned archive staging from reviewed exact packages, separate
  signing, SHA256 pool names, Debian-equivalent immutable version collision checks,
  retained previous versions/by-hash/source/evidence, seven-day Release validity,
  package-derived AppStream XML/YAML/icon collections, and complete local atomic
  promotion with a predecessor compare-and-swap. Large package/source copies are
  streamed; arbitrary package members are not extracted for AppStream.
- Explicit keyring/source bootstrap debs; scoped Signed-By; four exact package
  names; signed Origin/Label/suite/component plus transport-host negative policy.
  Removing enrollment disables its source while preserving edited bytes in an
  ignored uniquely named file. Purge removes owned active configuration/trust,
  leaves unrelated administrator files and user settings intact. Deleted sources
  are not silently restored on upgrade. Main/Islands never depend on enrollment.
- Independently pinned distribution review binds the original release manifest,
  entire native-evidence inventory, native approval and per-package corresponding
  source/build-recipe provenance. Signing and promotion re-run the original native
  gate on retained exact main/companion bytes and verify pool identity. This binds
  reviewed source declarations; it does not manufacture missing source closure or
  prove compiler provenance by reading a tarball.
- Live expiring signing-subkey enforcement, matching InRelease/detached signer,
  read-only expiry check, documented daily renewal/48-hour warning, same-key public
  expiration renewal, tested keyring-overlap rotation and explicit offline-client
  recovery. No operational timer, alert destination or real key was provisioned.
- Read-only manually dispatched preparation workflow. No signing credentials,
  publication permissions, scheduled trigger, tag-triggered rebuild or Pages job.
- Non-Rust executable update-status policy handoff with focused tests. This is not
  integrated into the running app and does not claim async D-Bus/UI acceptance.

## Commands and verification

Environment inspected read-only: noble APT **2.8.3**, PackageKit
**1.2.8-2ubuntu1.5**, AppStream **1.0.2-1build6**, GnuPG **2.4.4**.

```sh
python3 -B scripts/apt/test_archive.py -v
PYTHONPATH=scripts/apt:packaging/updates:companion:scripts/compatibility \
  python3 -B -m unittest -v test_admission test_updates test_promotion_pipeline
```

**40 distinct tests passed, zero skips:**

- **20 real APT acceptance tests**; final run after streamed-copy changes passed
  in **49.994 seconds**. Unprivileged user namespace with namespace-root mapped
  to the normal host uid; private APT/dpkg configuration, status, logs, keyrings,
  caches and loopback HTTP only. Includes normal main+installed-companion upgrade,
  absent companion staying absent, deterministic archive/bootstrap, exact
  allowlist and hostile dependencies, wrong Origin, corrupt index/package/signature,
  wrong/missing keys, unsigned/expired/future metadata, expired signing key,
  stale-authenticated-cache preservation, offline failure, Debian version ordering,
  retained version install/no downgrade, client crossing publication, failed
  promotion preserving the prior generation, remove/purge/admin-disabled source,
  preservation of installed apps/user settings, keyring overlap/retirement, expiry
  monitor and AppStream validation. Renewal recovery uses no trust bypass.
- **8 admission tests**: reviewed main/companion/source/raw-evidence mutation,
  provenance requirements, retained promotion revalidation, symlink rejection,
  private/armored-key rejection, unmocked literal-pass rejection and bootstrap lintian.
  Final rerun after the binary-keyring format guard: **8 passed**.
  Native verification is explicitly mocked only for isolated byte-binding cases;
  no test fixture is admitted for release.
- **8 update-status policy tests**: running source/other-PATH/modified/deleted
  binaries, explicit/disabled enrollment, no automatic check, candidate ordering,
  no update, wrong architecture, offline/missing PackageKit/timeout/auth cancel,
  malformed/error signals and absent/pending Islands. Actual async cancellation
  and OS authentication remain native/Rust owner work.
- **4 unchanged existing promotion pipeline regressions**: repackaged companion,
  summary binding, final main/system-update evidence and live executable identity.
  Combined 19-test admission/status/existing-promotion command passed in **9.378
  seconds** before the narrowly scoped streamed-copy change; admission was rerun
  afterward, including the added armored-key rejection case. No unrelated full Rust/application suite was repeated.

Additional checks: Python AST parsing; CLI help for archive/sign/promote;
workflow YAML parsed with read-only contents/actions scopes; owned whitespace
checks; required ancestor commits. AppStream collection validates offline.
Lintian has no unoverridden errors. The two exact apt-sources/preferences lintian
errors are intentionally overridden only in the opt-in enrollment package, with
justification in its override file. Nonstandard directory modes were corrected.

Initial red fixture runs caught second-granularity HTTP 304 ambiguity, dpkg
conffile prompts, wrong failure-string expectations and warning text contaminating
an exported expired test public key; those were corrected without relaxing trust.
An early isolated dpkg invocation attempted its default host log, was denied, and
was corrected to an explicit private log path; host package state/logs were not
changed. Failure assertions remain active. The HTTP fixture now models a
revalidating origin; final real hosting cache/ETag behavior is a remaining gate.

## Runnable deployment and Updates handoff

[APT-OPERATIONS.md](docs/APT-OPERATIONS.md) contains exact artifact schemas and
commands for bootstrap build → review → deterministic staging → independent digest
review → restricted local signing → local atomic promotion, subsequent metadata
renewal, expiry checks, future explicit bootstrap installation, removal and key
recovery. The operator must supply genuine reviewed artifacts, independent hashes,
public keyring and restricted signing-subkey home. No production-ready artifacts
or secrets exist from this task. The future parent Pages step must deploy the
complete signed tree under `apt/`, with `.nojekyll`, retained hashes/pool/source,
and verify unauthenticated public downloads and interleaving in a disposable guest
before enrollment assets are shipped. The workflow remains preparation-only.

[UPDATES-HANDOFF.md](docs/UPDATES-HANDOFF.md) records installed D-Bus XML hashes,
Store executable/desktop-entry identity and pinned upstream source hashes. Actual
source inspected at `16b1667e43c634318a99995c8eb77c23603ae560` and current upstream
`5b9bbeddb02d5a783299fb8c809fca0edb302c89`. A version suffix identifies inspected
source, not binary attestation. No live Store or PackageKit call was made.

Concrete launch finding: `com.system76.CosmicStore.desktop` uses `cosmic-store %u`.
Its positional argument and single-instance ActivateAction are search text. There
is **no verified direct Updates route**. Implement **Open COSMIC Store → choose
Updates**, without inventing `--updates` or a URI. Native launch confirmation is
still required. Search/catalog discovery is separate from system update capability.

Exact remaining Rust/UI work for Arrange owner:

1. Collect verified running-executable/dpkg/enrollment identity; implement
   Apt/ManualDeb/Source/Unknown without upgrading another installation on PATH.
2. Add manual-only async system-bus PackageKit RefreshCache then GetUpdates, fresh
   transaction per operation, signal subscriptions before calls, request-token
   invalidation, bounded timeouts and cancellation. Honor OS-owned authentication;
   never collect passwords or add an executable updater. Port the policy cases
   and add real async/race/UI tests.
3. Wire one Updates row with installed/running/candidate versions, truthful errors,
   verified Store desktop launch and manual/source links. Keep enrollment action
   informational/unavailable until the real endpoint/bootstrap is accepted.
4. Reuse existing verified Islands installation/refresh status for the independent
   next-login notice. Absent Islands retains only the existing Add-ons Get Islands
   link. No forced logout, activation or live panel replacement.

## Unresolved parent gates

The **empty catalog cannot ship**. Clean **1.10**, **complete source closure** and
**physical GPU** acceptance remain unqualified. The parent's final combined code
must be built once into immutable main/companion/bootstrap/source artifacts,
reviewed, then qualified on actual retained native environments. No fixture or
passing string can replace process/render/input evidence.

Remaining integrated native path: genuine old GitHub install → explicit enrollment
→ normal Store/PackageKit main+already-installed companion update → bootstrap with
widgets stopped → logout/login → new bundle → unknown-cohort stock fallback →
Disable/restore, including offline/auth cancellation, interruption recovery,
source/other-binary and absent/disabled users. Store search/install/upgrade/remove
and public hosting/cache/expiry/rotation must be accepted separately. The parent
also needs genuine key ownership/backups/restricted environment, durable archive
retention and an operational expiry alert destination. Final main metainfo/privacy
copy must match the finalized app before admission.

No known failing local test remains. Independent review, Rust/UI integration and
native/public delivery prerequisites remain; none authorizes publication now.
