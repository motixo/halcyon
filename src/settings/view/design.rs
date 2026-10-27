use super::*;
use crate::settings::design::Message as Edit;
impl Settings {
    pub(super) fn design_section(&self) -> Element<'_, Message> {
        let action = |edit| Message::Design(edit);
        let mut rows = vec![detail(
            "Widget design",
            if self.design.managed {
                "Widget identities, appearance and layout use the current storage format."
            } else {
                "Move existing widgets to the current storage format. Their appearance and layout stay the same."
            },
        )];
        if self.design.review {
            rows.push(label("All nine widget identities are retained, including hidden widgets. Notes, accounts and weather location stay in their existing stores. Existing Undo actions remain available.", 13., MUTED));
            rows.push(label("Older Halcyon versions cannot edit the new design. Export legacy settings before switching back.", 13., MUTED));
            rows.push(
                row::with_children(vec![
                    button::text("Cancel")
                        .id("design-cancel".into())
                        .on_press(action(Edit::Cancel))
                        .into(),
                    button::suggested("Migrate widgets")
                        .id("design-migrate".into())
                        .on_press_maybe((!self.native_busy).then_some(action(Edit::Migrate)))
                        .into(),
                ])
                .spacing(8)
                .into(),
            );
        } else if !self.design.managed {
            rows.push(
                button::standard("Review migration…")
                    .id("design-review".into())
                    .on_press(action(Edit::Review))
                    .into(),
            );
        } else {
            rows.push(
                row::with_children(vec![
                    button::text("Export original…")
                        .on_press(action(Edit::Export(false)))
                        .into(),
                    button::text("Export legacy settings…")
                        .on_press(action(Edit::Export(true)))
                        .into(),
                ])
                .spacing(8)
                .wrap()
                .into(),
            );
            rows.push(label("Exports are private backups and may contain local display names or settings. Choose a new file; exports never replace the applied design.", 12., MUTED));
        }
        if let Some(notice) = &self.design.notice {
            rows.push(label(notice, 13., MUTED));
        }
        if self.native_busy {
            rows.push(label("Updating widget storage…", 13., MUTED));
        }
        pages::section(rows)
    }
    pub(super) fn design_recovery_view(&self) -> Element<'_, Message> {
        let mut parts = vec![
            heading(
                "Your widget design needs attention",
                "The saved data was kept. Widget editing is paused.",
            ),
            label(
                self.config
                    .design_problem
                    .as_deref()
                    .unwrap_or("Could not open the widget design"),
                15.,
                FG,
            ),
            label(
                "Restore the saved design file, or open it with a compatible Halcyon version. An older app's changes to the frozen widget settings require review.",
                13.,
                MUTED,
            ),
            row::with_children(vec![
                button::standard("Export original…")
                    .id("design-recovery-export".into())
                    .on_press(Message::Design(Edit::Export(false)))
                    .into(),
                button::text("Check again")
                    .id("design-recheck".into())
                    .on_press(Message::Design(Edit::Recheck))
                    .into(),
            ])
            .spacing(8)
            .wrap()
            .into(),
        ];
        if self.design.source_conflict {
            if self.design.source_review {
                parts.push(pages::section(vec![
                    detail("Keep the current widget design?", "Changes from the older app and its saved history are backed up first. The current design and unrelated app preferences are retained."),
                    row::with_children(vec![
                        button::text("Cancel").on_press(Message::Design(Edit::Cancel)).into(),
                        button::suggested("Keep current design").id("design-keep-source".into()).on_press_maybe((!self.native_busy).then_some(Message::Design(Edit::KeepDesign))).into(),
                    ]).spacing(8).into(),
                ]));
            } else {
                parts.push(
                    button::standard("Review older-app changes…")
                        .id("design-review-source".into())
                        .on_press(Message::Design(Edit::ReviewSource))
                        .into(),
                );
            }
        }
        if self.design.pending {
            parts.push(
                button::suggested("Recover interrupted change")
                    .id("design-recover".into())
                    .on_press_maybe((!self.native_busy).then_some(Message::Design(Edit::Recover)))
                    .into(),
            );
        }
        if let Some(notice) = &self.design.notice {
            parts.push(label(notice, 13., MUTED));
        }
        if self.native_busy {
            parts.push(label("Recovering…", 13., MUTED));
        }
        container(
            scrollable(column::with_children(parts).spacing(18).max_width(680))
                .height(Length::Fill),
        )
        .padding(28)
        .class(cosmic::theme::Container::Primary)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
