![Halcyon — Make COSMIC your own.](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/presentation/hero.png)

# Halcyon ${VERSION}

Desktop widgets and coordinated looks for **COSMIC on Pop!_OS 24.04**.

**[Download for amd64](https://github.com/${REPOSITORY}/releases/download/${TAG}/cosmic-ext-halcyon_${VERSION}_amd64.deb)** ·
[Source](https://github.com/${REPOSITORY}/tree/${TAG}) ·
[Report a problem](https://github.com/${REPOSITORY}/issues)

## What's new

${CHANGES}

## Install or update

Download the main package above, then run:

```sh
sudo apt install ./cosmic-ext-halcyon_${VERSION}_amd64.deb
```

Open **Halcyon** from the application launcher. Installing the package alone
does not change your desktop or start the widgets. Existing settings are
preserved when updating.

Updates currently use downloads from GitHub Releases. This package does not
add an APT repository or enable automatic updates.

## Before you start

- **Platform:** Pop!_OS 24.04 with COSMIC, x86-64 (amd64).
- **Google Calendar:** ${CALENDAR}
- **Islands:** optional, distributed separately, and tied to specific COSMIC
  versions. Use only a companion build matching your desktop.
  [Compatibility, activation and removal](https://github.com/${REPOSITORY}/blob/${TAG}/packaging/islands/README.md).
- **Checksums:** download the matching `.sha256` file beside your package
  and check it with `sha256sum -c filename.sha256`.

## Explore Halcyon

![Native weather, system monitor, music and notes widgets with sample data](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/presentation/widgets.png)

[Features and setup](https://github.com/${REPOSITORY}/blob/${TAG}/README.md) ·
[Package details](https://github.com/${REPOSITORY}/blob/${TAG}/packaging/README.md) ·
[License and dependency notices](https://github.com/${REPOSITORY}/blob/${TAG}/THIRD_PARTY.md)

