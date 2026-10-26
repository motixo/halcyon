# GitHub free usage and local storage plan

Use GitHub's free public-repository services for source, documentation, normal
CI and published software. Keep bulky development environments and raw evidence
locally. Do not enable paid runner sizes, expanded cache capacity, LFS overages
or paid development environments to support Halcyon.

Rules checked against official GitHub documentation on 2026-10-25. Halcyon is
public and its four current workflows use standard `ubuntu-24.04` runners. The
account billing report is unavailable to the current credential, so storage
inventory is not a statement of charges or the remaining account allowance.

## Free services and limits

| Service | Published rule | Halcyon policy |
| --- | --- | --- |
| Public source and ordinary image files | Public repositories are available on Free. Regular Git warns above 50 MiB per file and blocks above 100 MiB. Repository health limits still apply. | Keep source, selected screenshots, icons and compressed shipped artwork in ordinary Git. Avoid large, frequently regenerated binary files. |
| Standard public Actions | Standard hosted runners in public repositories are free. The Free plan's 2,000 monthly minutes are the private-repository allowance. | Keep pull-request checks, final verification and automated maintenance on standard Ubuntu runners. |
| Actions artifacts | GitHub's public Actions exemption and private-repository quotas must be distinguished. The billing table lists 500 MB on Free, shared with private Packages; retained bytes alone do not establish billable usage. Artifacts are temporary workflow output, not permanent archives. | Keep only small, short-lived reports and job-to-job transport. Move large environments and raw evidence locally regardless of their public billing exemption. Do not assume the 52.8 GiB inventory is a charge. |
| Actions caches | 10 GB per repository is included; increasing the configured limit can incur charges. Old entries are evicted under the normal limit. | Keep the default limit, save useful main-branch Rust caches, and remove obsolete branch/version caches. Never use caches as the only evidence copy. |
| GitHub Releases | Up to 1,000 assets per release; each file under 2 GiB; no documented total release-size or download-bandwidth limit. | Keep published installers, corresponding source, checksums, approval/provenance and focused evidence. Releases distribute software; do not use them as a general VM backup service. |
| GitHub Pages | Free for public repositories; published site at most 1 GB, soft bandwidth limit 100 GB/month. | Keep the signed APT site and its 850,000,000-byte pre-deployment guard. Keep daily renewal independent of a laptop being awake. |
| Public GitHub Packages | Public packages are free. Container registry storage/bandwidth is currently free, with advance notice required for changes. | No migration is needed for the existing Debian release/APT distribution. Do not introduce a registry simply to store arbitrary archives. |
| Git LFS | On Free/Pro, 10 GiB storage and 10 GiB monthly bandwidth; applies to public and private repositories. New versions add whole-file storage. | Do not introduce LFS for screenshots, generated references, guest disks or intermediate builds. Store large working assets locally. |
| Larger runners and Codespaces | Larger runners are paid even for public repositories. Codespaces has a finite allowance and paid overages. | Use the local workstation for heavy native builds and graphical environments; do not provision either service. |

Sources: [Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions),
[public Actions exemption](https://docs.github.com/en/actions/concepts/billing-and-usage),
[large files](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github),
[release limits](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases),
[Pages limits](https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits),
[Packages billing](https://docs.github.com/en/billing/concepts/product-billing/github-packages),
[LFS billing](https://docs.github.com/en/billing/concepts/product-billing/git-lfs), and
[included product usage](https://docs.github.com/en/billing/reference/product-usage-included).

## Inventory before cleanup

The API inventory and cleanup receipts are in
[`../.task-github-storage-evidence/`](../.task-github-storage-evidence/).

| Item | Observed size | Interpretation |
| --- | --- | --- |
| Tracked images in the current worktree | 96 files, 35.44 MiB | Ordinary Git files; no root LFS attributes are configured. This is current content, not total image history. |
| Actions artifacts | 120 unexpired artifacts, 52.758 GiB | Four guest/input archives account for most storage. |
| Actions caches | 11 entries, 9.873 GiB | Near the included cache capacity; not the same allowance as artifacts. |
| Published releases | 5 releases, 33 assets, 772.7 MiB | Preserve all published assets and their stable download URLs. |
| Workstation free disk | About 25 GiB | Insufficient to blindly download another 52.8 GiB; verify existing copies and migrate incrementally. |

The three generated Settings reference images remain local. They are not the
source of the large GitHub artifact total. Keep only final project-relevant
references in documentation; keep unused variants and raw captures locally.

## Workflow placement and retention

1. **CI stays public on GitHub.** Extract a shared local-check entry point so the
   same Rust, Python, JavaScript and package checks can run locally. Continue
   public PR checks. Add branch-scoped cancellation for superseded CI runs; do
   not cancel APT signing/deployment transactions this way. Save caches from
   trusted main builds and retain the useful current dependency generations.
2. **Stop retaining routine development packages indefinitely.** Build and inspect
   them in CI, but publish a download only when explicitly requested. Retain such
   downloads for 1–3 days and failed-layout screenshots for 3 days. Print routine
   summaries in logs/job summaries, which do not count as artifact storage.
3. **COSMIC discovery stays online.** Its four daily checks need upstream metadata
   and currently emit tiny JSON files. Retain 14 days and preserve the latest
   rolling discovery state; optimizing these few kilobytes is not a priority.
4. **APT renewal stays daily on GitHub.** The signed metadata expires after seven
   days. Preserve the prepare/sign/deploy separation, approval checks and secret
   isolation. Keep prepared transport for 3 days and Pages transport only as long
   as deployment/retry needs it. Durable inputs remain pinned release assets.
5. **Release verification stays online when required.** Keep reviewed candidate
   bytes available until promotion completes. After immutable release assets and
   hashes are verified, temporary candidate/verified artifacts can expire. Never
   delete a pending candidate just because its name contains “candidate”.
6. **Guest construction and interactive development move local.** Keep qcow2
   disks, ISOs, package closures, long recordings, renderer experiments and raw
   screenshots outside Git. Local checks can be offline after dependencies are
   cached; upstream discovery and publishing still need a connection.

The CI changes for three-day development-package/failure-evidence retention and
branch-scoped cancellation are prepared locally and validated as YAML; they are
not yet deployed. The shared local-check entry point and on-request-only package
uploads remain planned. Preserve the unrelated review fixes already in CI.
The obsolete one-off experiment workflows are disabled remotely; the four
workflows on current main remain enabled. No personal-machine runner should
automatically execute public pull requests. Local checks run against explicitly
selected code.

## Cleanup procedure

First inventory exact artifact/cache IDs, sizes, digests and workflow runs. Check
for active consumers. Preserve published releases, the live APT site, current
release candidates and all unique evidence until there is a verified replacement.

For an artifact with an existing local ZIP, compare its full SHA256 and byte size
with the GitHub artifact metadata before removing the remote copy. Record the
original run/artifact ID and recovery path. A local copy preserves the bytes but
does not restore the original Actions artifact ID or URL; future work must use
the recorded recovery path. For missing copies, download to a staging file,
verify and rename atomically before deletion. Keep at least 5 GiB free locally.

Caches are reproducible accelerators: obsolete entries can be deleted without
copying them, while retaining current main caches and discovery state. Do not
delete workflow runs to clean artifacts; their logs and results remain useful.
Do not rewrite Git history or remove shipped images for this cleanup.

The first eligible artifact is `12506756949`, the retained COSMIC environment,
whose existing local ZIP is about 14.03 GiB. Six obsolete Rust caches total
about 7.35 GiB. The other two retained COSMIC archives and prepared guest require
separate backup/deduplication checks; similar names and sizes are not proof of
identical content. Their unique bytes remain protected until that check passes.

After the user requested removal of obsolete experiments, the guest-builder
handoff established that the first build and failed replay snapshots were
superseded by the final retained guest. Those two obsolete disks can be discarded
without keeping full duplicate historical environments; retain their small
evidence and the final guest. The later prepared 1,783-package guest is a reusable
COSMIC test input rather than a Halcyon release binary. Archive that guest and
the remaining historical evidence locally with exact SHA256 verification before
removing their remote copies. Keep the two latest CI packages, current APT
transport and recent discovery reports on GitHub.

After deletion, re-query inventories and verify published release asset IDs,
sizes and digests are unchanged. Record actual removed bytes and any permission
failures. Reclaimed storage is not a refund: where usage is billable, deleting
artifacts stops future accumulation but does not erase already accrued storage
or reset consumed minutes. Account-wide billing and hard-stop budgets require
access to the billing settings; do not change payment methods or enable overages.

### Current cleanup result

The 14.03 GiB local ZIP passed full byte-size and SHA256 verification against
GitHub's artifact metadata. Initial deletion attempts used the unrelated active
CLI account and were refused. The already-stored repository-owner login was
then selected only for the cleanup process, leaving the global login unchanged.

**Removed and verified across both rounds: 60.01 GiB.** Actions artifacts fell
from 120 / 52.758 GiB to **seven / 104.90 MiB**. Caches fell from 11 / 9.873 GiB
to **five / 2.52 GiB**. The two newest main Rust caches and all discovery-state
caches remain. All 33 assets across five published releases retain their IDs,
sizes and digests. No Git history, images, live deployment or published download
was removed.

The first round removed the exact locally retained 14.03 GiB guest ZIP and six
obsolete caches. The second round removed 112 historical artifacts: two
superseded guest snapshots were discarded based on the final handoff; the other
110 artifacts were archived locally and verified against GitHub's SHA256 and
byte counts. Their backup receipts record member names and original artifact
IDs. The prepared stock COSMIC guest can support future native qualification;
retaining it does not qualify any new 0.3.0 payload.

The new backups occupy **10.55 GiB** under
`/home/workstation/archives/halcyon/actions-20261025`, with about **14.02 GiB**
remaining free on the workstation after transfer. The earlier final guest ZIP
remains at the recovery path recorded in `cleanup-plan.json`.

Nine obsolete experiment workflows are disabled remotely. CI, COSMIC discovery,
reviewed release verification and APT renewal remain enabled. Three-day CI
artifact retention and superseded-run cancellation are prepared locally; their
YAML checks pass, but these changes are not pushed yet.

Exact results are in `cleanup-execution.json`, `remaining-cleanup-result.json`,
`remaining-backups.json`, `workflow-cleanup-result.json` and
`final-verification.json` in the evidence directory. The owner credential still
does not expose the billing report; reclaimed bytes do not establish a charge
or refund. Future cleanup must start from a fresh inventory and verify required
backups before deletion, rather than reusing these historical candidate lists.

## Separate maintenance issue

The scheduled APT run on 2026-10-25 failed during preparation with
`unsupported_startup_policy`; signing and deployment were skipped. This is a
policy verification failure, not evidence that minutes ran out. Preserve the
existing locally reviewed policy-dispatch correction and validate it against
the pinned published artifacts before a deployment. Storage cleanup must not
bypass the release gate or turn off renewal.
