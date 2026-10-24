use super::pages::section;
use super::*;
impl Settings {
    pub(in crate::settings) fn usage_control(&self) -> Element<'_, Message> {
        let mut controls = vec![
            container(
                cosmic::widget::checkbox(self.usage_enabled)
                    .label("Share usage data")
                    .on_toggle_maybe((!self.usage_busy).then_some(Message::ShareUsage)),
            )
            .id(cosmic::widget::Id::new("share-usage-data"))
            .into(),
            label(crate::usage::DISCLOSURE, 12., MUTED),
            label(crate::usage::CHOICE_DISCLOSURE, 12., MUTED),
            button::text("Privacy details")
                .on_press(Message::OpenPrivacy)
                .into(),
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
        section(vec![heading("Privacy", ""), self.usage_control()])
    }
}
