//! Shared widget editor and wallpaper stage. The card is the desktop renderer;
//! the cached wallpaper is decoded by the existing background worker.
use super::*;

impl Settings {
    pub(super) fn preview_stage<'a>(&self, card: Element<'a, Message>) -> Element<'a, Message> {
        container(self.preview_stage_padded(card, 14))
            .id(cosmic::widget::Id::new("widget-preview-stage"))
            .width(Length::Fill)
            .into()
    }
    pub(super) fn preview_stage_padded<'a>(
        &self,
        card: Element<'a, Message>,
        padding: u16,
    ) -> Element<'a, Message> {
        let background: Element<'a, Message> = match self.profiles.home_image() {
            Some(handle) => cosmic::widget::image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(cosmic::iced::ContentFit::Cover)
                .border_radius(10.)
                .into(),
            None => container(cosmic::iced::widget::space())
                .width(Length::Fill)
                .height(Length::Fill)
                .class(cosmic::theme::Container::custom(|theme| {
                    cosmic::iced::widget::container::Style {
                        background: Some(Background::Gradient(
                            cosmic::iced::gradient::Linear::new(2.4)
                                .add_stop(0., theme.cosmic().primary(theme.transparent).base.into())
                                .add_stop(
                                    1.,
                                    theme.cosmic().background(theme.transparent).base.into(),
                                )
                                .into(),
                        )),
                        border: Border {
                            radius: 10.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }))
                .into(),
        };
        container(
            cosmic::iced::widget::Stack::new()
                .push(container(card).padding(padding).width(Length::Fill))
                .push_under(background)
                // Iced's first stack layer draws directly. Keep the image in a
                // later layer so Cover is clipped by a real renderer layer.
                .push_under(
                    cosmic::iced::widget::space()
                        .width(Length::Fill)
                        .height(Length::Fill),
                )
                .width(Length::Fill)
                .clip(true),
        )
        .width(Length::Fill)
        .into()
    }

    pub(super) fn widget_settings_body<'a>(
        &'a self,
        card: Element<'a, Message>,
        controls: Element<'a, Message>,
    ) -> Element<'a, Message> {
        self.editor_with_preview(card, controls, "Sample data")
    }

    pub(super) fn editor_with_preview<'a>(
        &'a self,
        card: Element<'a, Message>,
        controls: Element<'a, Message>,
        caption: &'static str,
    ) -> Element<'a, Message> {
        self.editor_with_preview_header(card, controls, caption, None)
    }

    pub(super) fn editor_with_preview_header<'a>(
        &'a self,
        card: Element<'a, Message>,
        controls: Element<'a, Message>,
        caption: &'static str,
        header: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let mut parts = Vec::new();
        if let Some(header) = header {
            parts.push(header);
        }
        if self.scene_open() {
            let card = if !caption.starts_with("Live controls") {
                super::sample::fit(card, self.stage_height())
            } else {
                card
            };
            parts.push(self.preview_stage(card));
        }
        let preview = self.scene_frame(column::with_children(parts).spacing(10).into(), caption);
        {
            // Empty widgets need content before a sample. Notes always keeps
            // its editor and Save/Revert ahead of the live checklist.
            let needs_content = self.page == Page::Notes
                || (self.page == Page::Calendar && !self.calendar.connection.connected())
                || (self.page == Page::Weather && self.config.location.is_none())
                || (self.page == Page::Projects && self.config.project_roots.is_empty());
            if needs_content {
                self.inspector_layout(controls, preview, "content")
            } else {
                self.inspector_layout(preview, controls, "widget")
            }
        }
    }
}
