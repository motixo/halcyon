use super::*;
use crate::settings::updates::{Check, Message as Update, Setup, UpdateChannel};

impl Settings {
    pub(in crate::settings) fn updates_section(&self) -> Element<'_, Message> {
        let mut content = vec![label("Updates", 16., FG)];
        match &self.updates.setup {
            Setup::Preparing => {
                return pages::section(vec![
                    label("Preparing system updates…", 16., FG),
                    label(
                        "Downloading and verifying Halcyon’s package source and public key.",
                        14.,
                        MUTED,
                    ),
                    button::standard("Cancel")
                        .on_press(Message::Updates(Update::Cancel))
                        .into(),
                ]);
            }
            Setup::Installing => {
                return pages::section(vec![
                    label(
                        match self.updates.install_phase {
                            crate::settings::updates::InstallPhase::Inspecting => {
                                "Rechecking this installation…"
                            }
                            crate::settings::updates::InstallPhase::Authorizing => {
                                "Authorize in the system dialog"
                            }
                            crate::settings::updates::InstallPhase::Installing => {
                                "Installing the update source…"
                            }
                            crate::settings::updates::InstallPhase::Verifying => {
                                "Verifying system updates…"
                            }
                        },
                        16.,
                        FG,
                    ),
                    label(
                        "Your system handles authorization and installation. Halcyon will verify the result when it finishes.",
                        14.,
                        MUTED,
                    ),
                    label(
                        "Installation may continue after you close this window. Use the system authorization dialog to cancel before it starts.",
                        12.,
                        MUTED,
                    ),
                ]);
            }
            Setup::Ready => {
                return pages::section(vec![
                    label("Enable system updates", 16., FG),
                    label(
                        "Add Halcyon’s verified package source and public key. Future updates will appear in your system’s package manager.",
                        14.,
                        MUTED,
                    ),
                    label("This does not install or enable Islands.", 12., MUTED),
                    button::suggested("Continue with system installer")
                        .id(cosmic::widget::Id::new("updates-install"))
                        .on_press(Message::Updates(Update::Install))
                        .into(),
                    button::text(if self.updates.copied {
                        "Instructions copied"
                    } else {
                        "Copy terminal instructions"
                    })
                    .on_press(Message::Updates(Update::CopyInstructions))
                    .into(),
                    button::text("Recheck installation")
                        .on_press(Message::Updates(Update::Inspect))
                        .into(),
                ]);
            }
            Setup::Failed(error) => {
                content.push(label("System updates are not confirmed", 16., FG));
                content.push(label(error.label(), 14., MUTED));
                if let Some(files) = &self.updates.prepared {
                    content.push(label("You can install the verified files in Terminal, then recheck. The system package manager may ask for your password.", 14., MUTED));
                    content.push(label(files.instructions(), 12., MUTED));
                    content.push(
                        button::standard(if self.updates.copied {
                            "Instructions copied"
                        } else {
                            "Copy terminal instructions"
                        })
                        .on_press(Message::Updates(Update::CopyInstructions))
                        .into(),
                    );
                }
                content.push(
                    button::suggested("Recheck installation")
                        .on_press(Message::Updates(Update::Inspect))
                        .into(),
                );
                return pages::section(content);
            }
            Setup::Enrolled => content.push(label("System updates enabled", 14., FG)),
            Setup::Idle => {}
        }
        let mut actions = Vec::new();
        let action =
            |title: &'static str, id: &'static str, message: Update| -> Element<'_, Message> {
                button::standard(title)
                    .id(cosmic::widget::Id::new(id))
                    .on_press(Message::Updates(message))
                    .into()
            };
        match &self.updates.identity {
            None => content.push(label("Checking this installation…", 14., MUTED)),
            Some(identity) => {
                let version = match &identity.installed_version {
                    Some(installed) if identity.running_version != *installed => format!(
                        "Running {} · Installed {}",
                        identity.running_version, installed
                    ),
                    Some(installed) => format!("Halcyon {installed}"),
                    _ => format!("Halcyon {}", identity.running_version),
                };
                content.push(label(version, 14., FG));
                let status = match &self.updates.check {
                    Check::NotChecked => identity.reason.to_string(),
                    Check::Checking => "Checking system updates…".into(),
                    Check::Current => "No newer Halcyon package found.".into(),
                    Check::Available(version) => format!("Halcyon {version} is available."),
                    Check::Failed(error) => error.label().into(),
                };
                content.push(label(status, 14., MUTED));
                match identity.channel {
                    UpdateChannel::Apt if identity.can_check() => {
                        if self.updates.check == Check::Checking {
                            actions.push(action("Cancel check", "updates-cancel", Update::Cancel));
                        } else {
                            let check = if matches!(self.updates.check, Check::Available(_)) {
                                button::standard("Check for updates")
                            } else {
                                button::suggested("Check for updates")
                            };
                            actions.push(
                                check
                                    .id(cosmic::widget::Id::new("updates-check"))
                                    .on_press(Message::Updates(Update::Check))
                                    .into(),
                            );
                        }
                        if identity.store.is_some() {
                            actions.push(
                                (if matches!(self.updates.check, Check::Available(_)) {
                                    button::suggested("Update in COSMIC Store")
                                } else {
                                    button::standard("Open COSMIC Store")
                                })
                                .id(cosmic::widget::Id::new("updates-store"))
                                .on_press_maybe(
                                    (!self.updates.store_busy)
                                        .then_some(Message::Updates(Update::OpenStore)),
                                )
                                .into(),
                            );
                            content.push(label("Choose Updates in COSMIC Store.", 12., MUTED));
                        } else {
                            content.push(label(
                                "COSMIC Store is unavailable. Use your system package manager.",
                                12.,
                                MUTED,
                            ));
                        }
                    }
                    UpdateChannel::ManualDeb => {
                        if identity.can_enroll() {
                            actions.push(
                                button::suggested("Enable system updates…")
                                    .id(cosmic::widget::Id::new("updates-enroll"))
                                    .on_press(Message::Updates(Update::Prepare))
                                    .into(),
                            );
                            content.push(label("Add Halcyon’s package source. Your system will manage future updates.", 12., MUTED));
                        }
                        actions.push(action(
                            "Download a release",
                            "updates-releases",
                            Update::Releases,
                        ));
                        actions.push(action(
                            "Setup instructions",
                            "updates-information",
                            Update::Information,
                        ));
                    }
                    UpdateChannel::Source => {
                        actions.push(action(
                            "Release information",
                            "updates-releases",
                            Update::Releases,
                        ));
                    }
                    _ => actions.push(action(
                        "Installation information",
                        "updates-information",
                        Update::Information,
                    )),
                }
            }
        }
        if let Some(at) = self.updates.last_checked {
            content.push(label(
                format!("Last successful check: {}", at.format("%e %b, %H:%M")),
                12.,
                MUTED,
            ));
        }
        if self.updates.notes_url().is_some() {
            content.push(
                button::text("Release notes ↗")
                    .on_press(Message::Updates(Update::Notes))
                    .into(),
            );
        }
        if let Some(error) = &self.updates.store_error {
            content.push(label(error, 12., MUTED));
        }
        if !actions.is_empty() {
            content.push(row::with_children(actions).spacing(8).wrap().into());
        }
        if crate::settings::updates::pending_islands(&self.desktop.islands.installation) {
            content.push(label(
                "Islands update ready · log out and back in to activate.",
                12.,
                MUTED,
            ));
        }
        pages::section(content)
    }
}
