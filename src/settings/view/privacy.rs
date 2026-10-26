use super::pages::section;
use super::*;
impl Settings {
    pub(in crate::settings) fn usage_control(&self) -> Element<'_, Message> {
        let mut controls = vec![
            row::with_children(vec![
                container(
                    cosmic::widget::checkbox(self.usage_enabled)
                        .label("Share usage data")
                        .on_toggle_maybe((!self.usage_busy).then_some(Message::ShareUsage)),
                )
                .id(cosmic::widget::Id::new("share-usage-data"))
                .into(),
                button::text("Privacy")
                    .on_press(Message::OpenPrivacy)
                    .into(),
            ])
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            label(
                "Reports app version, first start, country and daily activity. Never widget content.",
                12.,
                MUTED,
            ),
            label(
                "Turning this off stops activity reporting and queues a final preference update.",
                12.,
                MUTED,
            ),
        ];
        if self.usage_busy {
            controls.push(label("Saving choice…", 12., MUTED));
        }
        if let Some(error) = &self.usage_error {
            controls.push(label(format!("Choice could not be saved: {error}. Try again; check the saved setting before closing."), 12., Color::from_rgb(1., 0.55, 0.5)));
        }
        column::with_children(controls).spacing(8).into()
    }
    pub(in crate::settings) fn privacy_section(&self) -> Element<'_, Message> {
        section(vec![
            detail(
                "Usage reporting",
                "Your choice is independent of themes and saved setups.",
            ),
            self.usage_control(),
        ])
    }
}
