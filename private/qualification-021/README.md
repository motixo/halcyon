# Disposable COSMIC guest infrastructure

Scope: independent test infrastructure on `ci/qualification-probe-021`; no product
changes and no release qualification claim. Standard public Ubuntu 24.04 runner,
one KVM VM per job, 120-minute hard job limit, 100-minute builder limit. No paid
services. `checkout` does not persist its read-only repository credential.

## Provisioning and retained provenance

Run `.github/workflows/qualification-guest-builder.yml`. Build 28 generic amd64 is
downloaded only on the disposable runner. Its detached SHA256SUMS signature is
checked using the fingerprint published at https://apt.pop-os.org/ and the pinned
Pop keyring revision. The ISO must pass that signed checksum before extraction.

This is a **verified-rootfs installation**, not an interactive installer run:
extract the ISO's complete `casper/filesystem.squashfs` to a GPT/ext4 installed
disk; remove the live `casper`/installer package; configure fstab, UEFI
systemd-boot, a disposable local user, display manager and QEMU guest agent.
No host home, desktop configuration, personal account, tokens or signing key is
mounted or copied. The public disposable login is `probe` / `cosmic-probe`.
This password protects no real account or service. VNC/QMP/QGA are private Unix
sockets. The guest has outbound NAT during construction, no listening forwarded
ports. Qualification overlays should use `-nic none` unless the scenario needs
network access.

APT authenticates the complete source indexes against the ISO's distribution
keyrings. The builder upgrades the complete installed set, explicitly includes
COSMIC session, Store and PackageKit, checks dpkg/APT consistency, records all
installed source/binary identities and holds the entire installed cohort. Exact
unchanged base payloads remain in the signed ISO; changed/additional payloads
remain in retained APT archives, alongside original source definitions, binary
and source indexes, keys and installed dpkg status. Source mappings are package
source name/version plus the signed Sources index's archive/hash records.
The image is a distribution rootfs plus documented VM-specific installation
changes, not a rebuilt COSMIC from a tag and not a three-package downgrade.

Artifacts expire after **7 days**. The parent must retain them elsewhere before
expiry. No release assets or Pages are created. `cosmic-evidence-RUN` is small;
`cosmic-retained-RUN` contains the ISO, signed checksum/key, APT closure, disk,
UEFI state/firmware and executable recipes. Do not download the latter on a
low-disk controller. Obtain the artifact IDs and digest through the Actions API.

## Consuming finalized Halcyon / Islands packages later

1. Download the chosen retained artifact on a disposable runner. Verify artifact
   digest and `evidence/retained-hashes.txt` from the associated evidence artifact.
   Verify the ISO signature/checksum and the retained package payload hashes.
   Verify the exact image and firmware digests. Do not run `apt upgrade` against
   live repositories and continue calling the result the same environment.
2. Copy `cosmic-vars.fd` and make an overlay:
   `qemu-img create -f qcow2 -F qcow2 -b /absolute/cosmic-1.11.qcow2 trial.qcow2`.
   Boot with the recorded QEMU machine/version, OVMF bytes, virtio devices,
   software graphics and one VM configuration from `build.sh`; use `-nic none`.
3. Supply **final, already stripped and packaged** `.deb` files plus an immutable
   SHA256 input manifest on read-only ISO media (`xorriso -as mkisofs ...`). Do
   not share a product checkout or home and do not mount credentials. Record
   the package hashes and all extracted installed helper/catalog/manifest/ELF
   hashes. Reject a mismatch before installation. If dependencies are needed,
   supply the exact signed retained `.deb` closure on the same read-only media.
4. Install through the guest evidence/control channel. Retain the transaction
   and independently map running executable bytes through `/proc/PID/exe`;
   newly installed path hashes alone cannot identify an old running session.
   Use `exercise.py` QMP/QGA primitives for input, screen captures and process
   evidence. Keep the same overlay for deliberate upgrades; reset image+vars
   between unrelated scenarios. Save scenario assertions with the input hashes.
5. Require independent rendered bars, working applet/input, notification pixels,
   real logout/login and the product-specific assertions. A zero exit code or
   notify-send success alone is insufficient. Baseline confidence is not product
   qualification. Preserve failures as failures and shut down only owned VM
   processes. Physical GPU, multi-monitor and fractional scaling remain outside
   this single-output software-rendered VM scope.

## Limits

A second offline reconstruction must be demonstrated before claiming that the
retained closure recipe was replay-verified. A retained sealed disk can be used
for reproducible overlay scenarios independently of that further reconstruction.
Exact historical 1.10 is not supplied by this work. Its prior partial evidence
remains partial. Read `guest-builder-report.md` for actual run outcomes; this
README describes the intended reusable interface, not evidence of success.
