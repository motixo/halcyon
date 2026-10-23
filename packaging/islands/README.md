# Islands optional add-on

Islands is a patched COSMIC panel that splits the panel into floating islands.
It is shipped separately as `cosmic-ext-halcyon-islands`. Installing Halcyon does
not install or activate Islands. Installing this add-on only places files under
`/usr`; it does not change your session. Activation is per user at the next login.
Choose the Islands style in Halcyon's Dock & panel settings after activation.

The 0.2.0 companion also renders Halcyon's four glass materials on Dock and Panel,
with either Native or Islands layout. Widget materials work in the main app alone.
Bar materials require this updated companion to be active; installing the deb
without refreshing your per-user copy and logging in again is not sufficient.
Native theme colors and opacity remain in charge. Material frosting stays local
to rounded bar regions without changing the global frosting preference. Highlights
on bars are static; the optional Liquid edge motion belongs to widgets.

The `amd64` add-on supports 64-bit Intel and AMD PCs (x86-64), just like the
main Halcyon package.

## Supported versions

The selector in `companion/panel_launcher.py` checks these versions at runtime.
APT dependencies are unversioned so the add-on never holds back COSMIC updates.
The selector admits the following package combinations. Mixing versions between
columns is not supported. Validation scope and limitations are described below.

| Package | COSMIC 1.11 | COSMIC 1.10 |
| --- | --- | --- |
| cosmic-panel | `1.11.0~1792526322~24.04~ca20ce3` | `1.10.0~1791315703~24.04~3ce6ce1` |
| cosmic-comp | `0.1~1792508925~24.04~8cb1802` | `0.1~1791979042~24.04~742b800` |
| cosmic-session | `1.11.0~1792525615~24.04~4b0a0a2` | `1.10.0~1791314577~24.04~8093b59` |

The reviewed cosmic-panel revision is
`ca20ce3185b3b4404f429d59d429c40b58a2ddef`, with companion protocol 1.
New bundles declaring `runtime_probe: 1` also admit timestamp/distribution
rebuilds of either complete source triple (`reviewed-source-v1`). This means the
package's final `~<revision>` fields must match one column, with panel/session
versions at least 1.10. The supported package syntax is
`<numeric-version>~<timestamp>~<numeric-distro>~<source-revision>`; other packaging
schemes remain unqualified. Source labels are evidence supplied by installed
packages, not binary attestation. Distributions with downstream changes still
need native validation. Unknown revisions and mixed triples select stock.

At startup the new companion is executed in a bounded preflight, so native
library loader failures and missing required compositor globals select stock.
It creates no surfaces and never uses the session's notification stream. After
launch, the host must answer IPC within 15 seconds. A failure latches stock through
the existing crash marker; preflight refusal selects stock without a crash marker.
An advertised protocol does not prove behavioral compatibility. This is why
unknown future source revisions are not automatically admitted. Status uses the selector's installed-package, manifest and binary
checksum checks; it does not identify the versions of already running processes.
After a COSMIC upgrade, log out and in before relying on the new package versions.
The release label alone (for example, 1.11) is not a compatibility guarantee:
the compositor, panel and session must form a reviewed source combination. `status` shows
installed and supported versions when they differ. Do not downgrade COSMIC to
match this add-on.

COSMIC 1.10 and newer is the compatibility target, not an unrestricted version
range. Future releases must be validated before admission; the selector retains
older supported combinations when adding a new one. The same companion binary
has been exercised on the listed 1.10 and 1.11 compositor revisions in isolated
native sessions. These checks do not cover every graphics driver, distribution
build or monitor setup.
The 1.10 checks used its source-built compositor with the host's 1.11-era
libraries and applets. They did not qualify a complete clean 1.10 OS or login
session. Hardware GPU performance, fractional scaling and multi-monitor/hotplug
behavior remain outside this qualification. No unconditional future compatibility
is claimed.

## Install and activate

Install Halcyon first, then replace `<v>` with the add-on release version:

```sh
sudo apt install ./cosmic-ext-halcyon-islands_<v>_amd64.deb
halcyon-islands status
halcyon-islands enable
```

Run `enable` as your desktop user from a COSMIC session. Your session PATH must
put `~/.local/bin` before `/usr/bin`, and `XDG_DATA_HOME` must be unset or
`~/.local/share`. The installer checks PATH and login identity. Log out and back
in to start Islands. No running panel is stopped or restarted by these commands.
Restarts in the login where you enabled Islands still select stock.
`enable` checks installed versions before copying any companion files. Unsupported
or unreadable versions leave activation unchanged and exit with an explanation.

`halcyon-islands status` reports whether the add-on is present, whether the
selector compatibility checks pass, and whether Islands is enabled for you.
Exit status is 0 when installed, compatible and enabled (including waiting for
the next login); 1 means disabled, unavailable, incompatible, blocked by a failure
marker, or an operation failed; 2 means invalid command usage. `--help` exits 0.
Enable and disable exit 0 when their requested changes succeed.

A previous companion installation from source must first be restored using its
original `install-record.json`; the command refuses to take over another panel
selector. Repeated `enable` calls are harmless. After installing an updated add-on,
run `enable` again to copy it into your per-user bundle for the next login.

## Turn off and remove

For each user who enabled Islands:

```sh
halcyon-islands disable
```

Log out and back in to use the stock panel. Once all users have disabled it:

```sh
sudo apt remove cosmic-ext-halcyon-islands
```

Disable restores this command's installation records in reverse order, including
across add-on upgrades. Backups are kept. External edits or corrupt backups cause
a clear failure instead of being overwritten. An interrupted enable can also be
undone with `disable`. If an individual restore needs manual recovery, use:

```sh
python3 /usr/lib/cosmic-ext-halcyon-islands/install.py --restore /path/to/install-record.json
```

Package removal alone does not delete per-user data or the copied selector and
binary. If removed before disabling, reinstall the add-on and run `disable`.
Saved island layouts and native preferences are preserved.

The existing paths remain `~/.local/share/cosmic-desktop-cards/companion`,
`~/.local/share/cosmic-desktop-cards/backups/panel-*/install-record.json`, and
`~/.local/state/desktop-cards-panel`. A crash leaves a `failed` marker in that
state directory (or `$XDG_STATE_HOME/desktop-cards-panel`). It keeps stock selected;
`enable` refuses activation while the marker exists. Remove only that marker when
deliberately retrying a reviewed component after investigating the failure.
Enable never silently clears it. IPC and socket names are unchanged.

`disable` restores the panel launcher, not COSMIC appearance settings. If the
standard panel also fails, keep its error log and inspect the implicated native
setting. Disabling Islands does not repair a malformed or problematic native
corner-radius value, and it does not erase your panel configuration.

## Build a release

Use Rust 1.93.0 and the native dependencies in cosmic-panel's upstream README.
Build separately from Halcyon, in new source and target directories:

```sh
python3 companion/build.py --archive /path/to/verified-upstream.tar.gz \
  --workdir /path/to/new-panel-source --target-dir /path/to/panel-target --offline
cp /path/to/panel-target/release/cosmic-panel /path/to/cosmic-panel-islands
```

Omit `--offline` to fetch locked dependencies. Omit `--archive` to download the
pinned archive with SHA256 verification. `companion/build.py` builds the upstream
binary but does not currently emit the renamed binary or manifest. After reviewing
the build, create `companion-manifest.json` alongside `cosmic-panel-islands`:

```sh
python3 - /path/to/cosmic-panel-islands /path/to/companion-manifest.json <<'PY'
import hashlib, json, pathlib, runpy, sys
binary, manifest = map(pathlib.Path, sys.argv[1:])
revision = runpy.run_path('companion/build.py')['REVISION']
manifest.write_text(json.dumps({
    'protocol': 1,
    'runtime_probe': 1,  # only for binaries with --islands-probe
    'source_policy': 'reviewed-source-v1',
    'upstream': revision,
    'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
}, indent=2) + '\n')
PY
python3 packaging/islands/build-deb.py \
  --binary /path/to/cosmic-panel-islands \
  --licenses /path/to/ISLANDS_THIRD_PARTY_LICENSES.txt \
  --manifest /path/to/companion-manifest.json --version 0.2.0 --out dist
```

The builder needs Python 3 and dpkg tools, never invokes Cargo, and validates the
binary and manifest with the selector. It outputs
`cosmic-ext-halcyon-islands_<v>_amd64.deb` and a sibling `.deb.sha256` file.
Generate the companion's notices from its reconstructed source, using the
same cargo-about version as the release workflow:

```sh
cat packaging/licenses/about.toml packaging/islands/licenses.toml > /tmp/islands-about.toml
cargo about generate --locked --fail \
  --manifest-path /path/to/new-panel-source/Cargo.toml \
  --config /tmp/islands-about.toml packaging/licenses/about.hbs \
  --output-file /path/to/ISLANDS_THIRD_PARTY_LICENSES.txt
cargo metadata --locked --format-version 1 --filter-platform x86_64-unknown-linux-gnu \
  --manifest-path /path/to/new-panel-source/Cargo.toml > /tmp/islands-metadata.json
python3 packaging/collect-licenses.py --metadata /tmp/islands-metadata.json \
  --cargo-home "$HOME/.cargo" --allow-missing --out /tmp/islands-source-notices.txt
cat /tmp/islands-source-notices.txt >> /path/to/ISLANDS_THIRD_PARTY_LICENSES.txt
```

The native package builder requires this report and includes it in the package.
Review collector warnings: cargo-about supplies standard license text where
upstream crates omit standalone notice files.

Set `SOURCE_DATE_EPOCH` for reproducible package timestamps. The binary must be
an amd64 release build compatible with the target system's native libraries;
the fake-binary tests do not establish native runtime compatibility.

The package includes the patched panel under GPL-3.0-only, credited to System76,
and the Python helpers. `dock-islands-core` is linked into the binary and is not
a runtime file. The release handoff must include corresponding frozen sources,
patches, lockfiles, dependency notices, binary hashes and native validation
as described in `companion/README.md`.

The main `cosmic-ext-halcyon` package must be available to satisfy the dependency.
It must not activate Islands or depend on this add-on. No main package files,
services or activation hooks belong in the add-on.

## Isolated verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest companion.test_compatibility
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_panel_companion.py
PYTHONDONTWRITEBYTECODE=1 python3 packaging/islands/test_islands.py
```

Tests build a fake-binary package in a temporary directory, inspect it using
`dpkg-deb -c/-I`, and run lintian. They extract the command and exercise it with
fake package queries, fixture HOME/XDG directories and the installer's `--root`
mode. They never launch a real panel or install a Debian package. To retain the
fixture deb and lintian log for review, pass `--artifacts /path/to/temporary-dir`.
The command's hidden `--root /path/to/fixture-home` option is only for fixture
verification; normal activation always uses the installer's `--user` mode.
