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

Replace draft wording with the exact candidate's acceptance before publication.
Include that reviewed text in the release candidate and bind it to the candidate's
manifest/checksums. The publisher must use those same notes; no script here uploads
them. Images and documentation links in the template are tag-pinned.

Preserve published packages, source archives, approvals and old release text.
A new main app does not qualify a companion. Confirm the matching companion,
activation instructions, supported COSMIC inputs and corresponding source archive.

Published release notes: [0.3.0](../releases/0.3.0.md). Current native development gallery:
[provenance and limitations](../screenshots/030/README.md).
