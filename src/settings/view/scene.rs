//! Shared preview presentation. Collapse is local UI state, never a desktop edit.
use super::*;

impl Settings {
    pub(in crate::settings) fn scene_open(&self) -> bool {
        !self.collapsed_scenes.contains(&self.page)
    }

    pub(in crate::settings) fn scene_frame<'a>(
        &'a self,
        content: Element<'a, Message>,
        caption: &str,
    ) -> Element<'a, Message> {
        let open = self.scene_open();
        let live = caption.starts_with("Live controls");
        let header = split(
            container(label(
                if caption == "Sample data" {
                    "Sample preview"
                } else if caption == "Live controls" {
                    "Live controls · changes your desktop widget"
                } else {
                    caption
                },
                12.,
                MUTED,
            ))
            .width(Length::Fill)
            .into(),
            button::text(match (live, open) {
                (true, true) => "Hide controls",
                (true, false) => "Show controls",
                (false, true) => "Hide preview",
                (false, false) => "Show preview",
            })
            .id(cosmic::widget::Id::new("scene-toggle"))
            .trailing_icon(
                icon::from_name(if open {
                    "go-up-symbolic"
                } else {
                    "go-down-symbolic"
                })
                .size(16),
            )
            .on_press(Message::ToggleScene)
            .into(),
        );
        let mut children = vec![header];
        if open {
            children.push(content);
        }
        container(column::with_children(children).spacing(10))
            .id(cosmic::widget::Id::new("desktop-scene"))
            .width(Length::Fill)
            .into()
    }
}
