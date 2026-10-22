use super::*;
use crate::calendar::Backend;
use crate::settings::calendar::Message as Account;

impl Settings {
    pub(super) fn calendar_connection(&self) -> Element<'_, Message> {
        let editor = &self.calendar;
        let message = |value| Message::Calendar(value);
        // One header row: service, signed-in account and its state.
        let connected = editor.connection.connected() && !editor.busy && !editor.confirm_disconnect;
        let mut header = vec![
            icon::from_name("x-office-calendar-symbolic")
                .size(28)
                .icon()
                .into(),
            detail(
                "Google Calendar",
                if connected {
                    editor.connection.account.clone()
                } else {
                    String::new()
                },
            ),
        ];
        if connected {
            header.push(if editor.error.is_some() {
                pill("Needs attention", Color::from_rgb8(245, 190, 107))
            } else {
                pill(
                    if editor.connection.backend == Backend::Legacy {
                        "Workspace"
                    } else {
                        "Read-only"
                    },
                    self.settings_accent(),
                )
            });
            header.push(
                button::icon(icon::from_name("view-more-symbolic").size(18))
                    .id(cosmic::widget::Id::new("calendar-account-options"))
                    .tooltip("Account options")
                    .on_press(Message::ToggleHelp("calendar-account"))
                    .into(),
            );
        }
        let mut rows = vec![
            row::with_children(header)
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
        ];
        if let Some(error) = &editor.error {
            rows.push(label(error, 12., Color::from_rgb8(245, 190, 107)));
        }
        if editor.busy {
            rows.push(detail(
                if editor.pending.is_some() {
                    "Finish signing in in your browser"
                } else if editor.connecting {
                    "Preparing Google sign-in…"
                } else {
                    "Updating Calendar settings…"
                },
                if editor.pending.is_some() { "Complete sign-in in a normal browser, such as Firefox or Chrome. You can paste the sign-in link into another browser." } else { "" },
            ));
            let mut actions = vec![];
            if editor.pending.is_some() {
                actions.push(
                    button::standard("Open sign-in page")
                        .on_press(message(Account::OpenBrowser))
                        .into(),
                );
            }
            if editor.pending.is_some() {
                actions.push(
                    button::text("Copy sign-in link")
                        .on_press(message(Account::CopyLink))
                        .into(),
                );
            }
            if editor.connecting {
                actions.push(
                    button::text("Cancel")
                        .on_press(message(Account::Cancel))
                        .into(),
                );
            }
            rows.push(row::with_children(actions).spacing(8).into());
        } else if editor.confirm_disconnect {
            rows.push(detail("Disconnect on this computer?", "This removes the app’s cached events. Your calendars and events in Google stay unchanged."));
            rows.push(
                row::with_children(vec![
                    button::standard("Keep connected")
                        .id(cosmic::widget::Id::new("calendar-keep-connected"))
                        .on_press(message(Account::ConfirmDisconnect(false)))
                        .into(),
                    button::text("Disconnect")
                        .on_press(message(Account::Disconnect))
                        .into(),
                ])
                .spacing(8)
                .into(),
            );
        } else if editor.connection.connected() {
            if editor.error.is_some() {
                rows.push(label("Check connection to reconnect", 12., MUTED));
            }
            rows.push(split(
                label("Calendar", 14., FG),
                if editor.labels.is_empty() {
                    label(editor.connection.calendar_name.clone(), 14., MUTED)
                } else {
                    cosmic::widget::dropdown(&editor.labels, editor.selected, move |index| {
                        message(Account::Select(index))
                    })
                    .width(Length::Fixed(220.))
                    .into()
                },
            ));
            let mut actions = vec![
                button::text("Open calendar ↗")
                    .on_press(Message::OpenCalendar)
                    .into(),
                button::text(if editor.loading {
                    "Checking…"
                } else {
                    "Check connection"
                })
                .on_press_maybe((!editor.loading).then_some(message(Account::Check)))
                .into(),
                button::text("Disconnect")
                    .id(cosmic::widget::Id::new("calendar-disconnect"))
                    .on_press(message(Account::ConfirmDisconnect(true)))
                    .into(),
            ];
            if editor.client_ready {
                actions.push(
                    button::text(if editor.error.is_some() {
                        "Reconnect"
                    } else {
                        "Switch account"
                    })
                    .on_press(message(Account::Connect))
                    .into(),
                );
            }
            if self.help_open == Some("calendar-account") || editor.error.is_some() {
                rows.push(row::with_children(actions).spacing(6).wrap().into());
            }
        } else if editor.connection.cleanup_pending() {
            rows.push(detail(
                "Disconnected on this computer",
                "Your cached events were removed. Finish removing the saved sign-in from the system keyring before connecting again.",
            ));
            rows.push(
                button::suggested("Finish disconnecting")
                    .id(cosmic::widget::Id::new("calendar-finish-disconnect"))
                    .on_press(message(Account::Disconnect))
                    .into(),
            );
        } else if editor.client_ready {
            rows.push(
                button::suggested("Connect Google Calendar")
                    .id(cosmic::widget::Id::new("calendar-connect"))
                    .on_press(message(Account::Connect))
                    .into(),
            );
            rows.push(label(
                "Sign in through your browser. Calendar access is read-only.",
                12.,
                MUTED,
            ));
        } else {
            rows.push(detail("Google sign-in isn’t available in this build yet", "The app publisher needs to configure Google sign-in. All other widgets work without a Google account."));
        }
        group(None, rows)
    }
    pub(super) fn calendar_publisher_setup(&self) -> Element<'_, Message> {
        let mut rows = vec![
            button::text(if self.calendar.publisher_help {
                "Hide Google sign-in setup"
            } else {
                "Google sign-in setup…"
            })
            .on_press(Message::Calendar(Account::PublisherHelp(
                !self.calendar.publisher_help,
            )))
            .into(),
        ];
        if self.calendar.publisher_help {
            rows.push(detail("For the app publisher or a personal build", "Register a Google Desktop app with the Calendar API enabled. Users of a configured release only need the Connect button."));
            rows.push(
                row::with_children(vec![
                    button::standard("Import Google app JSON…")
                        .on_press_maybe(
                            (!self.calendar.busy)
                                .then_some(Message::Calendar(Account::ImportClient)),
                        )
                        .into(),
                    button::text("Setup guide ↗")
                        .on_press(Message::Calendar(Account::OpenGuide))
                        .into(),
                ])
                .spacing(8)
                .wrap()
                .into(),
            );
        }
        column::with_children(rows).spacing(10).into()
    }
}
