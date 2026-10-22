//! Shared widget editor and wallpaper stage. The card is the desktop renderer;
//! the cached wallpaper is decoded by the existing background worker.
use super::*;

impl Settings {
    pub(super) fn preview_stage<'a>(&self, card: Element<'a, Message>) -> Element<'a, Message> {
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
                .push(container(card).padding(20).width(Length::Fill))
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
        .id(cosmic::widget::Id::new("widget-preview-stage"))
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
        let mut parts = vec![split(label("Preview", 12., FG), label(caption, 11., MUTED))];
        if let Some(header) = header {
            parts.push(header);
        }
        parts.push(self.preview_stage(card));
        let preview = column::with_children(parts).spacing(10);
        if self.window.width >= 990. {
            row::with_children(vec![
                container(controls).width(Length::Fill).into(),
                container(preview).width(360).into(),
            ])
            .spacing(20)
            .into()
        } else {
            column::with_children(vec![
                controls,
                container(preview).max_width(400).width(Length::Fill).into(),
            ])
            .spacing(20)
            .into()
        }
    }
}
