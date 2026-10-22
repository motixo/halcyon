# Release presentation

Tagged releases use [.github/RELEASE_TEMPLATE.md](../../.github/RELEASE_TEMPLATE.md).
The release workflow fills in the version, repository, package changelog and
actual Calendar build configuration. Images and documentation links are pinned
to the release tag, so old release pages do not silently change with main.

Before tagging, update `packaging/debian/changelog` with user-visible changes.
Lead with the change people will notice, then mention important fixes. The
template already supplies download, installation, updates and compatibility
information; do not repeat these in the changelog.

If an Islands package is added manually, extend the release notes with its exact
supported COSMIC versions, activation instructions, and corresponding source
archive. Do not claim an Islands binary exists just because Halcyon was built.

For a presentation-only edit to an existing release, preserve its download
assets, tag, compatibility information and known limitations. Use a pinned
commit URL for artwork added after that release's tag. Do not imply newer
source fixes are present in the old binaries.
