# Distribution and system updates

Halcyon 0.2.5 remains available as immutable [GitHub release downloads](https://github.com/profilaz3/halcyon/releases/tag/v0.2.5).
The signed APT channel is `https://profilaz3.github.io/halcyon/apt`, suite `noble`,
component `main`, architecture `amd64` (Intel and AMD x86-64 PCs).

Enrollment is explicit and optional. After the deployment workflow succeeds and
the bootstrap release is published, use the steps below. A GitHub release or this
repository does not automatically put Halcyon in everyone's COSMIC Store.

## Enable system updates once

Download both bootstrap packages from the separate
[apt-bootstrap-0.2.5-1 release](https://github.com/profilaz3/halcyon/releases/tag/apt-bootstrap-0.2.5-1):

```sh
curl -fLO https://github.com/profilaz3/halcyon/releases/download/apt-bootstrap-0.2.5-1/halcyon-archive-keyring_0.2.5-1_all.deb
curl -fLO https://github.com/profilaz3/halcyon/releases/download/apt-bootstrap-0.2.5-1/halcyon-update-source_0.2.5-1_all.deb
printf '%s\n' \
  '4a0b973025c98b03d4ed901bc499f1af61c001dcdd1f1d7885b4009811ac2262  halcyon-archive-keyring_0.2.5-1_all.deb' \
  'fad29b1760e53d7bf1115829d71f2568f7c208f6176b6480aee3a0c85d2d00af  halcyon-update-source_0.2.5-1_all.deb' \
  | sha256sum -c -
sudo apt install ./halcyon-archive-keyring_0.2.5-1_all.deb ./halcyon-update-source_0.2.5-1_all.deb
sudo apt update
```

Public certification fingerprint: **A1EB 6576 D959 5ECA 5750 9CE1 4EB2 B42D F234 CB16**.
Signing subkey: `610B73343D6F40603553A295A4658CCECBC306AB`.
The independently published checksums above bind the bootstrap downloads to the
reviewed public key. To inspect the included key without importing it:

```sh
dpkg-deb --fsys-tarfile halcyon-archive-keyring_0.2.5-1_all.deb \
  | tar -xOf - ./usr/share/keyrings/halcyon-archive-keyring.gpg \
  | gpg --show-keys --with-subkey-fingerprint
```

The source uses `/usr/share/keyrings/halcyon-archive-keyring.gpg` through scoped
`Signed-By`. It never uses `apt-key` or global trust. Selection preferences allow
only `cosmic-ext-halcyon`, `cosmic-ext-halcyon-islands`, `halcyon-archive-keyring`
and `halcyon-update-source` from this origin. No COSMIC version holds are added.

## Install or update

For a first app installation:

```sh
sudo apt install cosmic-ext-halcyon
```

Existing package installations update normally through APT or **COSMIC Store →
Updates**. The app's Updates row offers an explicit check and opens COSMIC Store;
it does not install software or collect passwords. Unattended installation depends
on the user's OS policy. Store search discovery is separate from system updates;
AppStream metadata and icons are provided, but a native Store search trial has not
been claimed by this deployment work.

Islands remains optional. Enrollment and installing/updating the main app do not
install it. To request it explicitly:

```sh
sudo apt install cosmic-ext-halcyon-islands
```

Installing the companion does not enable it or restart a panel. Already-installed
Islands receives normal package updates. Existing enabled users use the guarded
per-user refresh and next-login activation; disabled users stay disabled. There
is no forced logout or root package script that edits user home directories.

Manual `.deb` downloads remain supported. Source/per-user installs remain distinct
from `/usr` package installations; the app checks its actual running executable
before offering package update status. Enrolling does not replace a source build.

## Stop using this channel

```sh
sudo apt purge halcyon-update-source halcyon-archive-keyring
sudo apt update
```

This leaves Halcyon, Islands and user settings installed. Source removal preserves
its exact bytes as an ignored `halcyon.sources.removed.*` file for administrator
recovery; it is never silently re-enabled. Administrator-disabled/deleted source
conffiles remain disabled on ordinary package upgrades. Unrelated source files and
keys are untouched. The saved disabled file may be removed manually if no longer
needed.

## Release identity, source and maintenance

APT serves the **exact** approved 0.2.5 app/companion bytes. Its existing practical
`cosmic-1.10-1.11-startup-v1` approval and documented limitations remain unchanged;
APT delivery does not turn historical focused evidence into new native/hardware
qualification. No published 0.2.5 package, source or approval is modified.

The original release supplies corresponding source, licenses, provenance and
focused evidence. The separate bootstrap release supplies its own corresponding
source and GPL copyright/license. The archive landing page and retained
`release-assets.json` records link to immutable GitHub release assets and hashes,
so large source/evidence archives do not consume the Pages site budget.

[APT operations](APT-OPERATIONS.md) documents the approved input manifest,
protected signing environment, daily seven-day metadata renewal, expiry checks,
retention, rotation and exact deployment commands.
[Updates integration](UPDATES-HANDOFF.md) records the already-published app contract
and the limited Store discovery claim.
