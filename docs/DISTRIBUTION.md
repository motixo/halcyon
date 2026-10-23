# Distribution and updates

Status: v0.1.1 ships as GitHub release packages. The signed APT repository,
update UI, and automatic Islands rebuilds below are the next milestone, not
services that are already running.

## Current releases

Publish two separate amd64 packages for Pop!_OS 24.04 on 64-bit Intel and AMD
PCs. `amd64` is the Debian name for x86-64, not an AMD-only package:

- `cosmic-ext-halcyon`: the app, helpers, assets and user service. Installing it
  does not activate the service or change the desktop.
- `cosmic-ext-halcyon-islands`: optional panel extension. Installing it does
  not activate or restart a panel. Runtime checks select the stock panel on
  incompatibility. Its APT dependencies never pin COSMIC to an old version.

People upgrade by downloading the newer package from GitHub and opening it,
or with `sudo apt install ./package.deb`. This preserves user data. There is
no automatic update feed in v0.1.1. For Islands, run `halcyon-islands enable`
after upgrading, then log out and in; currently the enabled user bundle is a
copy, so installing the new package alone does not refresh that copy.

Release assets: both packages and their SHA256 files, corresponding Islands
source and dependency notices. GitHub's generated source archive covers
Halcyon itself, including the pinned companion patch and build instructions.
The release notes state the supported COSMIC package combination and whether
Google Calendar sign-in is configured. Missing OAuth configuration must never
be described as working sign-in. Accounts and user tokens never enter a build.

A release is gated by Rust, Python, companion-core and packaging tests,
AppStream and desktop-file validation, and lintian errors. Test failures must
not be hidden by `continue-on-error`. A tag identifies immutable source.
Do not overwrite an existing version's packages: publish a newer version.

## Planned: one system update channel

Use a signed APT archive on GitHub Pages, proposed URL
`https://profilaz3.github.io/halcyon/apt`, suite `noble`, component `main`,
architecture `amd64`. Enable Pages and validate the real URL before shipping
an enrollment package or source entry. Pages hosting is not a store listing.

APT and PackageKit own installation, authentication and package transactions.
Halcyon must not download and replace its executable, run a shell installer,
or ask for an administrator password itself. Updates appear with other system
updates after the user enrolls in our source; unattended installation depends
on their system policy and must not be promised universally.

### Archive and trust

1. Produce `.deb` assets from a reviewed source commit. Use `apt-ftparchive`
   to generate `Packages`, compressed indexes and `Release`, with SHA256
   hashes, a stable Origin/Label and Suite/Codename. Sign `InRelease` and
   `Release.gpg`. Retain prior versions for rollback.
2. Create a dedicated archive signing key, with an offline backup and a
   documented expiration/rotation procedure. Store the publishing subkey in
   a restricted GitHub Actions environment. Signing jobs run only for trusted
   release inputs, never pull requests. Do not print key material or put it in
   source, artifacts, caches or chat.
3. Ship the public key as `/usr/share/keyrings/halcyon-archive-keyring.gpg` in
   an owned keyring package. Use a Deb822 `.sources` entry with `Signed-By`
   pointing to that key. Never use `apt-key`, `trusted=yes`, or a global key.
4. Provide an explicit one-time "Enable Halcyon updates" enrollment package
   that owns the keyring and source configuration. Show the update source in
   installation instructions. Existing GitHub installations can install this
   package without reinstalling their data. Do not silently add a source from
   the application or publish an entry pointing to an unsigned/absent archive.
5. Restrict this source to the Halcyon package family with APT preferences:
   unrelated packages from the new origin have negative priority, approved
   Halcyon packages normal priority. Keep source/keyring ownership separate
   from the app so removing an app never deletes another administrator's files.
6. Publish a complete validated archive as one Pages deployment. Refresh the
   signed metadata before expiration even when no binary changes. A failed
   build/sign/deploy keeps the previous working archive intact.

Acceptance in a disposable Pop!_OS VM: enroll, `apt update`, install v0.1.0,
upgrade to the candidate through PackageKit, uninstall/reinstall while retaining
preferences, reject a corrupted index and a wrong signature, reject expired
metadata, and rotate keys using overlapping old/new signatures. Also verify
that unrelated packages cannot be selected from this repository.

### COSMIC Store discovery

The Store has native-package and Flatpak backends. A GitHub release or a
third-party APT URL is not automatically included on everybody's machine.
Ship the existing `io.github.profilaz3.Halcyon` metainfo and icons, plus an
AppStream collection in the archive's `dep11` metadata tied to
`cosmic-ext-halcyon`. Verify search, screenshots, install, upgrade and removal
in a clean COSMIC Store with only our source added. Treat search visibility
as a tested deliverable, not an assumption about installed metainfo.

### In-app update entry

Add one Updates row in App settings showing version and installation channel.

- For APT: query PackageKit for this package's installed/candidate versions;
  offer "Open Software Updates". Use a verified Store route or the normal
  PackageKit transaction with its standard authentication UI. Do not guess a
  command-line route or introduce password handling.
- For a manual GitHub package with no source: offer "Get latest release" and
  "Enable system updates" with clear destination descriptions.
- For a source/per-user install: identify it, link release/build instructions,
  and avoid upgrading a different `/usr` installation underneath it.
- For future Flatpak: open the Store's Flatpak update flow. No host-package
  commands from inside the sandbox.

An unavailable network, missing PackageKit, no newer version, canceled
authentication and failed installation each leave the current app intact.
Update notices never block Settings. Start with explicit checks; any later
background check is opt-in and documented in the privacy section.

### Islands updates and per-user activation

The current launcher validates three exact packages: `cosmic-panel`,
`cosmic-comp`, and `cosmic-session`. Monitor all three, not just the panel.
A package filename's commit suffix is a hint, not a verified source identity.

A scheduled GitHub Actions job should read the official signed Pop!_OS archive,
verify its signing chain and package-index hashes, compare the supported trio,
and resolve the full source revision from trusted package metadata. Open a
compatibility PR when it changes. Prepare the upstream tree with pinned
hashes, apply our patch without fuzz, update the core/manifest/selector and
source archive, then build and run core, packaging and native session tests.
Use a monotonically increasing Debian add-on version, e.g. `0.1.1+islands.2`,
so a new compatibility build upgrades without requiring a new app release.

A patch applying and unit tests passing do not prove compositor compatibility.
For changed compositor/session packages, require a disposable native COSMIC
trial before publishing. Until this gate can be automated reliably, publish
only after the maintainer approves the candidate; open an issue/PR on failure
instead of publishing a guessed build. Never block the official COSMIC update.

To eliminate the per-user copy gap, add a guarded user-session refresh:

- Read the already-installed `/usr` package, not a network download. Verify
  its manifest, checksum, package combination and ownership record.
- If this user previously enabled Islands and all owned files are unchanged,
  stage the new bundle atomically and preserve selection/layout. Do not
  enable it for new users or change disabled state. Leave external edits alone.
- Keep old files and a recovery journal. A failed refresh leaves stock selected.
- Activation defaults to next login. A future "Restart panel" action must
  use a proven session restart path preserving notification descriptors and
  ensure exactly one host; never kill an arbitrary process from an update hook.
- Package scripts must not edit users' homes, enable services, or restart a
  desktop. Each logged-in user refreshes independently in their own session.

Test enabled/disabled users, two users sharing one package, external edits,
interrupted copy, downgrade, removed add-on, failed-marker retention, running
old compositor after a package upgrade, and loss of the notification socket.
Show the standard-panel fallback and keep the user's saved Islands layout.
Do not promise a fixed time until a compatible build is available.

## Flatpak and wider discovery

A sandboxed Halcyon build needs a separate capability audit: COSMIC settings
access, layer-shell desktop windows, session startup, helper execution,
calendar secret storage, and local project access. Use portals where possible;
do not turn a Flatpak into an unrestricted host installer to hide missing
capabilities. Submit metainfo and permissions to the chosen repository review.

The native Islands replacement remains a separately installed host package.
COSMIC Store can surface native packages once its source/metadata is available;
"Islands cannot be in the Store" would be too broad. What cannot be bundled
as an ordinary sandboxed app is the host-panel replacement itself.

## References

- [COSMIC Store source and backends](https://github.com/pop-os/cosmic-store)
- [Debian third-party repository guidance](https://wiki.debian.org/DebianRepository/UseThirdParty)
- [APT signature verification](https://manpages.debian.org/stable/apt/apt-secure.8.en.html)
- [APT source configuration](https://manpages.debian.org/stable/apt/sources.list.5.en.html)
- [AppStream collection metadata](https://www.freedesktop.org/software/appstream/docs/chap-CollectionData.html)
