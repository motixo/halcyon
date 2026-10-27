# README maintenance

The root README is a stable introduction and installation entry point. Release
notes describe what changed in a release.

## Keep stable

- Keep the title, overview, section order and feature groups across releases.
- Use `/releases/latest` for downloads and `/releases` for release notes. Package
  filenames in the README use `<version>` rather than a hard-coded release.
- Keep one representative screenshot. Replace it only when it no longer
  represents the app or a specific presentation change is requested. Its archived
  path may contain a version; that alone is not a reason to replace it.
- Keep setup instructions in focused guides: Calendar in `docs/CALENDAR.md`,
  installation/updates in `docs/DISTRIBUTION.md`, and privacy in `docs/PRIVACY.md`.
- Keep default usage sharing and optional Islands clearly disclosed.

## During a release

1. Write concise product changes in `docs/releases/<version>.md` and render the
   release notes with the existing template. Put versioned download links there.
2. Check README links, platform requirements, installation steps and feature
   claims against the release. A routine version change needs no README edit.
3. If behavior changes, update only the affected sentence and its detailed guide.
   Add a feature line only for a lasting new capability; put smaller additions
   and bug fixes in release notes.
4. Keep release checks and development progress in their existing documents.
   Do not turn the README into a changelog or a test report.

The README describes released behavior. Planned work belongs in
[`docs/PLANNED_WORK.md`](../docs/PLANNED_WORK.md).
