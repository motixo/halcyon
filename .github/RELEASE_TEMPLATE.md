![Halcyon welcome artwork](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/assets/wizard/welcome.jpg)

# Halcyon ${VERSION}

Desktop widgets and themes for **COSMIC on Pop!_OS 24.04**.

**[Download for 64-bit Intel/AMD PCs](https://github.com/${REPOSITORY}/releases/download/${TAG}/cosmic-ext-halcyon_${VERSION}_amd64.deb)** ·
[Source](https://github.com/${REPOSITORY}/tree/${TAG}) ·
[Report a problem](https://github.com/${REPOSITORY}/issues)

## What's new

${CHANGES}

## Appearance and Setups

Choose a complete setup or change individual parts. Review its wallpaper, colors,
widget layout, dock and panel before applying. Islands compositions require the
matching active companion; standard-panel layouts remain available.

Surface finishes use decorative transparency, tint, edges and highlights. They do
not provide optical refraction or native backdrop blur.

## Privacy

Usage sharing is on by default and can be turned off in setup or **App settings →
Privacy**. While enabled, Halcyon attempts a daily activity report with a random
installation ID, app version, approximate first country and first desktop start.
No widget content or hardware details are included.
[Privacy and current delivery status](https://github.com/${REPOSITORY}/blob/${TAG}/docs/PRIVACY.md).

## Install or update

Download the main package above, then run:

```sh
sudo apt install ./cosmic-ext-halcyon_${VERSION}_amd64.deb
```

Open **Halcyon** from the application launcher. Installing the package alone
does not change your desktop or start the widgets. Existing settings are
preserved when updating.

Fresh package installations enable Halcyon's signed system-update source by
default. APT/COSMIC Store manages future updates using your OS settings. Existing
disabled, deleted or modified sources are preserved. Islands remains optional.
[Update settings, disabling the source and older-install enrollment](https://github.com/${REPOSITORY}/blob/${TAG}/docs/DISTRIBUTION.md).

### Updating Islands

For Islands layouts **or Dock/Panel glass materials**, also install the matching
add-on package from this release:

[Companion package](https://github.com/${REPOSITORY}/releases/download/${TAG}/cosmic-ext-halcyon-islands_${VERSION}_amd64.deb) ·
[Full corresponding source](https://github.com/${REPOSITORY}/releases/download/${TAG}/halcyon-${VERSION}-corresponding-source.tar.gz)

```sh
sudo apt install ./cosmic-ext-halcyon-islands_${VERSION}_amd64.deb
halcyon-islands enable
```

Then log out and back in. The `enable` step refreshes your per-user copy;
installing the package alone does not update that copy or restart the panel.
The add-on checks supported COSMIC families and performs bounded native startup
checks. Unsupported families or failed checks select the stock panel; saved layouts are kept.
Widget materials work without the add-on. Bar materials need this updated active
companion even when the bars use Standard layout.

## Before you start

- **Platform:** Pop!_OS 24.04 with COSMIC on 64-bit Intel or AMD PCs.
  `amd64` means x86-64 and supports both manufacturers; this is not an ARM build.
- **Google Calendar:** ${CALENDAR}
- **Islands:** optional and distributed separately. Admission supports bounded
  startup attempts on COSMIC 1.10/1.11. Physical hardware and a clean 1.10 OS
  are not qualified. The compatibility documentation records the tested scope;
  unknown future sources need validation.
  [Compatibility, activation and removal](https://github.com/${REPOSITORY}/blob/${TAG}/packaging/islands/README.md).
- **Checksums:** download the matching `.sha256` file beside your package
  and check it with `sha256sum -c filename.sha256`.

## Explore Halcyon

![Setups in native Halcyon Settings](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/screenshots/030/setups.png)

![Help and diagnostics](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/screenshots/030/help.png)

[Features and setup](https://github.com/${REPOSITORY}/blob/${TAG}/README.md) ·
[Package details](https://github.com/${REPOSITORY}/blob/${TAG}/packaging/README.md) ·
[License and dependency notices](https://github.com/${REPOSITORY}/blob/${TAG}/THIRD_PARTY.md)
