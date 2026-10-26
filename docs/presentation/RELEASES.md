# Release presentation

The release workflow verifies and promotes an **already reviewed candidate**.
It does not render `.github/RELEASE_TEMPLATE.md`, rebuild packages or automatically
publish on tag push. Editing the template does not update an existing release.

Prepare reviewable notes locally with:

```sh
python3 scripts/render-release-notes.py --version 0.3.0 \
  --changes docs/releases/0.3.0.md \
  --calendar 'Google sign-in needs your OAuth client; live provider acceptance remains pending.' \
  --output /tmp/halcyon-0.3.0-release-notes.md
```

Keep release notes factual and brief. Lead with user-visible changes, including
renderer changes; keep screenshots and detailed explanations in the README.
Replace draft wording with the exact candidate's acceptance before publication.
Include that reviewed text in the release candidate and bind it to the candidate's
manifest/checksums. The publisher must use those same notes; no script here uploads
them. Images and documentation links in the template are tag-pinned.

After publication, editorial corrections may update the GitHub release page and
repository documentation. Preserve the original checksum-bound `RELEASE-NOTES.md`
asset, packages, source archives, approvals and tag; editing the release page does
not change those approved files.
A new main app does not qualify a companion. Confirm the matching companion,
activation instructions, supported COSMIC inputs and corresponding source archive.

Published release notes: [0.3.0](../releases/0.3.0.md). Current native development gallery:
[provenance and limitations](../screenshots/030/README.md).
