use super::*;
use crate::settings::updates::{Check, Message as Update, UpdateChannel};

impl Settings {
    pub(in crate::settings) fn updates_section(&self) -> Element<'_, Message> {
        let mut content = vec![label("Updates", 16., FG)];
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
                        actions.push(action(
                            "Download a release",
                            "updates-releases",
                            Update::Releases,
                        ));
                        actions.push(action(
                            "Installation guide",
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
