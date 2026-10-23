![Halcyon welcome artwork](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/assets/wizard/welcome.jpg)

# Halcyon ${VERSION}

Desktop widgets and themes for **COSMIC on Pop!_OS 24.04**.

**[Download for 64-bit Intel/AMD PCs](https://github.com/${REPOSITORY}/releases/download/${TAG}/cosmic-ext-halcyon_${VERSION}_amd64.deb)** ·
[Source](https://github.com/${REPOSITORY}/tree/${TAG}) ·
[Report a problem](https://github.com/${REPOSITORY}/issues)

## What's new

${CHANGES}

## Materials

Appearance offers Frosted, Clear glass, Smoked glass and Liquid, with shared or
per-surface choices, transparency, tint, Widget readability and Undo. Liquid has
optional brief widget edge motion; native bar highlights stay static. Materials
preserve your color choices. They do not provide optical refraction or a blur-strength
slider. The four native comparisons and chooser are in the
[README](https://github.com/${REPOSITORY}/blob/${TAG}/README.md#glass-materials).

## Daily activity reporting

Starting with 0.2.0, Halcyon sends **automatic daily activity reports** while
the desktop widgets run. Reports go to Google and contain a random installation ID, app
version, approximate first country and report format version. Country.is supplies
the country from the public IP; Halcyon discards the IP. No widget content is sent. Reporting is
always on and has no settings toggle, including after upgrading.
[Privacy details](https://github.com/${REPOSITORY}/blob/${TAG}/docs/PRIVACY.md).

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

### Updating Islands

For Islands layouts **or Dock/Panel glass materials**, also install the matching
add-on package from this release:

[Companion package](https://github.com/${REPOSITORY}/releases/download/${TAG}/cosmic-ext-halcyon-islands_${VERSION}_amd64.deb) ·
[Full corresponding source](https://github.com/${REPOSITORY}/releases/download/${TAG}/halcyon-islands-${VERSION}-source.tar.gz)

```sh
sudo apt install ./cosmic-ext-halcyon-islands_${VERSION}_amd64.deb
halcyon-islands enable
```

Then log out and back in. The `enable` step refreshes your per-user copy;
installing the package alone does not update that copy or restart the panel.
The add-on checks reviewed COSMIC source combinations and performs bounded startup
checks. Unknown sources or failed checks select the stock panel; saved layouts are kept.
Widget materials work without the add-on. Bar materials need this updated active
companion even when the bars use Native layout.

## Before you start

- **Platform:** Pop!_OS 24.04 with COSMIC on 64-bit Intel or AMD PCs.
  `amd64` means x86-64 and supports both manufacturers; this is not an ARM build.
- **Google Calendar:** ${CALENDAR}
- **Islands:** optional, distributed separately, and qualified for reviewed COSMIC
  1.10/1.11 source combinations. The 1.10 checks used its compositor with host
  libraries/applets, not a clean 1.10 OS. Unknown future sources need validation.
  [Compatibility, activation and removal](https://github.com/${REPOSITORY}/blob/${TAG}/packaging/islands/README.md).
- **Checksums:** download the matching `.sha256` file beside your package
  and check it with `sha256sum -c filename.sha256`.

## Explore Halcyon

![Halcyon desktop with coordinated widgets and optional Islands](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/screenshots/desktop-purple.png)

<table>
  <tr>
    <td width="50%"><img src="https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/screenshots/desktop-paper.png" alt="Light desktop" width="100%"></td>
    <td width="50%"><img src="https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/screenshots/desktop-mist.png" alt="Transparent widgets" width="100%"></td>
  </tr>
</table>

![Colors and theme settings with automatic wallpaper colors enabled](https://raw.githubusercontent.com/${REPOSITORY}/${TAG}/docs/screenshots/settings-colors.png)

[Features and setup](https://github.com/${REPOSITORY}/blob/${TAG}/README.md) ·
[Package details](https://github.com/${REPOSITORY}/blob/${TAG}/packaging/README.md) ·
[License and dependency notices](https://github.com/${REPOSITORY}/blob/${TAG}/THIRD_PARTY.md)

