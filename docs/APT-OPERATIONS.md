# Local distribution preparation — publication on hold

These tools implement local preparation only. **Do not publish 0.2.1.** No key,
Pages site, repository enrollment, release, tag or deployed schedule was created.
The parent owns genuine signing material, hosting and the eventual publish step.
The native compatibility catalog is still empty, so a real candidate cannot pass
the admission gate. Clean 1.10, complete source closure and physical GPU acceptance
remain unqualified. This document does not override those gates.

## Archive contract

The planned endpoint is `https://profilaz3.github.io/halcyon/apt`; `noble/main`,
`amd64`, signed `Origin: Halcyon` and `Label: Halcyon`. No live availability is
claimed. Only these four exact package names are accepted:

- `cosmic-ext-halcyon`
- `cosmic-ext-halcyon-islands`
- `halcyon-archive-keyring`
- `halcyon-update-source`

`scripts/apt/build_archive.py` consumes final packages without rebuilding or
stripping. It uses apt-ftparchive to index immutable SHA256-named pool objects,
preserves **all** previous versions, source/evidence records and by-hash objects,
and writes deterministic Packages/gzip, AppStream XML/YAML/gzip and icon tarballs.
AppStream comes from the exact final main package, not the mutable checkout.
Release has SHA256, Acquire-By-Hash, explicit Date and seven-day Valid-Until.
Determinism requires identical inputs, explicit epoch, previous generation and
tool versions; signatures are separately produced and are time-dependent.

`archive.json` binds every unsigned file, package identity, predecessor and review.
Its digest is an independent promotion input, not a value to trust because an
unreviewed artifact claims it. Existing Debian-equivalent versions cannot acquire
different bytes (`0.2.0` and `0.2.0-0` count as the same version). Existing output
directories are never overwritten. Files/directories are fsynced before atomic
rename. Local promotion uses a locked compare-and-swap of `current` to a complete
generation. The predecessor must match the archive whose by-hash data was retained.

Retention has no garbage collector yet. Keep all generations and corresponding
source; a future pruning policy must preserve at least the longest still-valid
Release plus the client retry/download window, and retain supported rollback
packages and source. Short-lived CI artifacts are **not** the durable archive.

## Independently reviewed artifact inputs

Keep the existing Group 3 `release-manifest.json`, `approval.json` and full
`native-evidence/` request/result/observations/raw artifact inventory. The existing
release gate must verify the exact main and companion, same-session live main,
native scenarios and reviewed catalog target. A passing string is insufficient.
Keep candidate/verifier execution separate from access to signing credentials.

Extend the release manifest's `artifacts` mapping with the two explicit bootstrap
packages (`kind: enrollment-deb`), actual corresponding source (`corresponding-source`),
retained build recipe(s) (`build-recipe`) and `build-provenance.json`
(`build-provenance`). Every entry retains its exact SHA256. There must be exactly
one current package for each of the four names. Package kind and control name
must agree. Nothing adds a native admission or fills an empty catalog.

The provenance file has this schema (names/digests are real artifact references,
not placeholders to submit for approval):

```json
{
  "format": "halcyon-build-provenance",
  "version": 1,
  "packages": {
    "<final-package.deb>": {
      "sha256": "<final-package-sha256>",
      "source_commit": "<full-40-hex-source-commit>",
      "sources": ["<corresponding-source.tar.xz>"],
      "recipe": "<retained-build-recipe.json>"
    }
  }
}
```

Include all four packages. For main and companion, the reviewer must establish
complete corresponding source/dependency closure and exact reproducible build
inputs; the schema binds the declaration and bytes but cannot prove a compiler's
provenance merely by reading a tarball. Source-only 1.10 observations do not meet
this requirement. Enrollment source and recipe belong in the same reviewed set.
The parent must also inspect the final main metainfo/privacy copy before admission.

After reviewing genuine native evidence and source, an independent reviewer
creates `distribution-review.json`:

```json
{
  "format": "halcyon-distribution-review",
  "version": 1,
  "decision": "approve",
  "review_reference": "<actual-independent-review-reference>",
  "release_manifest_sha256": "<exact-release-manifest-sha256>",
  "native_approval_sha256": "<exact-approval.json-sha256>",
  "native_evidence": {
    "request.json": "<sha256>",
    "result.json": "<sha256>",
    "observations.json": "<sha256>",
    "<every-relative-raw-evidence-path>": "<sha256>"
  }
}
```

Supply its digest separately as `--review-sha256`. Do not auto-approve from a
candidate job. The inventory must exactly equal the evidence tree, including
every raw referenced file. Symlinks/special files are rejected. The staged
`releases/<release-manifest-sha256>/` retains the complete reviewed input tree.
Review evidence for public disclosure before eventually hosting this directory;
native captures must contain no secrets or personal data. If anything needs
redaction, produce a new reviewed evidence set; do not edit after approval.

## Runnable local handoff

Requirements: Python 3.11+, apt-utils/apt-ftparchive, dpkg, GnuPG/gpgv,
appstreamcli and the existing qualifier dependencies. The operator supplies
paths and reviewed digest variables below. Commands do not fetch, enroll, build
app binaries or provision keys. Run from the reviewed product checkout.

Bootstrap preparation, **only after the parent supplies the real public keyring**:

```sh
python3 -B packaging/updates/build-deb.py \
  --public-keyring "$PUBLIC_KEYRING" --version "$ENROLLMENT_VERSION" \
  --epoch "$SOURCE_DATE_EPOCH" --out "$BOOTSTRAP_OUTPUT"
```

Include those exact outputs and source in the review before staging. Initial
generation:

```sh
python3 -B scripts/apt/build_archive.py --artifacts "$REVIEWED_CANDIDATE" \
  --review-sha256 "$REVIEW_SHA256" --epoch "$GENERATION_EPOCH" --out "$UNSIGNED"
sha256sum "$UNSIGNED/archive.json"
```

For **every later** generation, including metadata-only renewal, add
`--previous "$RETAINED_PREVIOUS" --previous-sha256 "$PREVIOUS_SHA256"` and
advance the epoch. Review the staged archive digest, then use a separate trusted
signing environment with the reviewed verifier code and signing-subkey-only home:

```sh
python3 -B scripts/apt/promotion.py sign --archive "$UNSIGNED" \
  --archive-sha256 "$ARCHIVE_SHA256" --review-sha256 "$REVIEW_SHA256" \
  --public-keyring "$PUBLIC_KEYRING" --gnupg-home "$PRIVATE_SIGNING_HOME" \
  --signing-subkey "$SIGNING_SUBKEY_FINGERPRINT" --out "$SIGNED"
python3 -B scripts/apt/promotion.py promote --archive "$SIGNED" \
  --archive-sha256 "$ARCHIVE_SHA256" --review-sha256 "$REVIEW_SHA256" \
  --public-keyring "$PUBLIC_KEYRING" --out "$LOCAL_DEPLOYMENT"
```

The promote command above bootstraps an empty local deployment. For a subsequent
generation add `--previous-sha256 "$PREVIOUS_SHA256"`. It exposes
`$LOCAL_DEPLOYMENT/current/`; it does **not** publish Pages. Both commands re-run
the independent review and native artifact gate against the retained inputs;
signing/verification also checks the selected live, expiring signing subkey and
its certification key. They do not run candidate executables.

`.github/workflows/apt-publish.yml` is deliberately a manually dispatched,
read-only **preparation** workflow, without secrets, write scopes, tag triggers,
schedule, Pages or deployment actions. No workflow has been dispatched. Future
parent publication must use an approved immutable signed directory from this
process, never a tag-triggered rebuild.

The future Pages artifact must put the complete signed tree under `apt/` and
include `.nojekyll`, retain by-hash/pool/source data and deploy as one generation.
Review canonical-index cache headers/ETags, no stale 304 after a generation
change, and fetch immutable URLs without authentication. Verify the real HTTPS
endpoint, fingerprint, expiry, bootstrap install, old-Release/new-index interleave
and exact public file hashes from a disposable clean noble client **before**
announcing or distributing enrollment. Hosting availability and this final
remote acceptance remain parent-owned; no deployment command is executed here.

## Explicit enrollment and removal

The keyring package owns only its public keyring and documentation. The source
package owns a Deb822 source with scoped Signed-By and two conffiles:
`/etc/apt/sources.list.d/halcyon.sources` and
`/etc/apt/preferences.d/halcyon`. There are no app/Islands dependencies that
enroll a user or install the optional companion.

Initial trust must come from independently reviewed release assets and a
published, independently checked public fingerprint. Future end-user enrollment
instructions, after deployment is accepted, use the **two local downloads in one
transaction**:

```sh
sudo apt install ./halcyon-archive-keyring_<version>_all.deb \
                 ./halcyon-update-source_<version>_all.deb
sudo apt update
```

Normal APT/COSMIC Store transactions then update main and an already-installed
Islands package. Neither installation enables Islands nor restarts a session.
Group 3 owns the user-session bootstrap/next-login activation. Manual release
downloads and source installations remain supported. No source build is replaced
by an application-owned updater; OS-owned `/usr` package transactions are distinct.

The first preferences record names the four exact packages and matches the signed
Origin/Label/suite/codename/component at priority 500. General negative records
block other packages from signed Origin Halcyon and the transport hostname,
including a publisher changing its signed Origin. `Pin: origin` means hostname;
`Pin: release o=...` means the signed Origin. APT pins are selection policy, not a
sandbox for authorized packages' maintainer scripts or an administrator deliberately
overriding package selection. No COSMIC holds or downgrade priority is installed.

On `apt remove halcyon-update-source`, postrm moves the source conffile to an
ignored, uniquely named `halcyon.sources.removed.*` file. This preserves even
administrator-edited bytes/symlinks without leaving an active source. The negative
preferences remain ordinary removed-package conffiles until purge. Purging the
source/keyring removes owned active configuration/public trust; the deliberately
saved disabled source remains for administrator recovery and is never overwritten
or restored automatically. Other administrator files and all user settings remain.
The keyring can remain installed alone without enrolling anything.

An administrator-deleted source conffile stays deleted on ordinary upgrades;
`Enabled: no` edits are also preserved under normal dpkg conffile handling.
Re-enrollment after removal is explicit: inspect the saved source and use the OS
package/configuration flow to restore only the intended source, never an update
hook that silently re-enables it. Lintian overrides cover only the two intentional
third-party enrollment files; other packaging errors remain fatal.

## Expiration, renewal and key rotation

Use an offline certification primary with offline encrypted backups/revocation
certificate. Give the publication environment only a restricted signing subkey,
not the certification secret; keep both expiring. Recommended operational windows
are a multi-year primary and a 90-day signing subkey, renewed well before expiry.
No real key generation/import/backup/provisioning happened in this task.

Release validity is seven days. The parent should arrange daily metadata renewal
and a daily independent expiry check after deployment is authorized, alerting a
named maintainer destination at 48 hours remaining. No timer or alert destination
was created here. The runnable local check is:

```sh
python3 -B scripts/apt/check_expiry.py --archive "$SIGNED" \
  --archive-sha256 "$ARCHIVE_SHA256" --public-keyring "$PUBLIC_KEYRING"
```

Exit 0 means current; nonzero means renew, invalid or expired. It checks both
Release and signer/certification-key deadlines. A missed Release renewal is fixed
by a fresh staged generation of the same reviewed packages, with a current date,
retained predecessor and new signature; never disable Check-Valid-Until. A missed
key deadline needs the explicit recovery below if clients cannot authenticate a
keyring update. Do not assume refreshing a signature extends a key's lifetime.

Extending expiration on the same key also requires distributing its updated
public self-signatures in a newer immutable keyring package while clients still
trust the old validity interval. Extending only the publisher's private keyring
does not renew client trust. Perform certification/expiration changes offline;
the publication environment receives only the refreshed signing subkey and
reviewed public material, never the certification secret.

Tested rotation is **keyring overlap**, with a single signature per Release:

1. While old key A is valid, ship a monotonically newer keyring containing public
   keys A and B in an A-signed generation. The source path/Signed-By stays fixed.
2. Leave a reviewed overlap/recovery window for ordinary OS keyring upgrades.
   Verify actual installed public keyring bytes on the canary clients.
3. Switch to B-only signing. Updated clients succeed; A-only clients correctly
   reject it. There is no claim that dual signatures solve offline-client rotation.
4. After the overlap window, ship a newer B-only keyring signed by B. A signatures
   must then fail. Keep independently verifiable recovery bootstrap assets.
5. Clients that missed the bridge explicitly install the reviewed newer keyring
   from authenticated release assets after checking its fingerprint, then retry
   normal APT. Never use trusted=yes, apt-key or unauthenticated update flags.

On compromise, stop signing/publication and use a separately authenticated
recovery channel; a compromised archive key cannot authenticate its own replacement
safely. Metadata/key expiry is a failure, never a reason to bypass verification.

## Local acceptance scope

```sh
python3 -B scripts/apt/test_archive.py -v
PYTHONPATH=scripts/apt python3 -B -m unittest -v test_admission
PYTHONPATH=packaging/updates python3 -B -m unittest -v test_updates
```

The first command re-execs in an unprivileged user namespace, refuses host root,
uses private dpkg/APT state/logs/config/trust, and contacts only its loopback HTTP
fixture. All transient test secrets live under private temporary directories and
are deleted; nothing commits or prints them. Test packages are explicitly synthetic.
Real APT acceptance is package-manager evidence, **not** native COSMIC evidence.
Admission tests mock the native verifier only for byte-binding unit cases; the
unmocked negative gate rejects the fixture's literal passing claim.

See [Updates handoff](UPDATES-HANDOFF.md) for the remaining Rust/UI integration and
native PackageKit/Store acceptance. No Cargo or live desktop operation belongs to
this distribution-preparation task.
