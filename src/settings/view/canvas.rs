use super::*;
use crate::settings::canvas::Message as Edit;
use crate::widget_style::resolver::{ClockFace, ClockSurface, ClockWeight};

fn action(edit: Edit) -> Message {
    Message::Canvas(edit)
}

impl Settings {
    pub(super) fn canvas_page(&self) -> Element<'_, Message> {
        let mut objects: Vec<Element<'_, Message>> = Widget::ALL
            .into_iter()
            .filter(|w| self.config.widget_visible(*w))
            .map(|w| {
                button::custom(
                    row::with_children(vec![
                        icon::from_name(w.icon()).size(16).icon().into(),
                        text::body(w.title()).into(),
                    ])
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .class(nav_style(
                    self.placement.selected() == Some(w),
                    self.settings_accent(),
                ))
                .id(cosmic::widget::Id::new(format!(
                    "canvas-object-{}",
                    w.key()
                )))
                .on_press(Message::Placement(
                    crate::settings::placement::Message::Select(w.key().into()),
                ))
                .into()
            })
            .collect();
        if objects.is_empty() {
            objects.push(text::body("Turn on a widget in My widgets to place it here.").into());
        }
        let canvas = column::with_children(vec![
            row::with_children(objects).spacing(6).wrap().into(),
            self.placement_editor(self.stage_height()),
        ])
        .spacing(10)
        .width(Length::Fill);
        let inspector = self.canvas_inspector();
        let content = self.inspector_layout(canvas.into(), inspector, "canvas");
        column::with_children(vec![
            heading(
                "Desktop canvas",
                if self.profiles.home_image().is_some() {
                    ""
                } else {
                    "Move, resize and style your widgets. Neutral backdrop until the wallpaper is available."
                },
            ),
            content,
        ])
        .spacing(16)
        .into()
    }
    fn canvas_inspector(&self) -> Element<'_, Message> {
        let config = self.canvas_config();
        let moment = self.canvas.as_ref().and_then(|draft| draft.moment);
        let clock = config.clock_layer(moment);
        let Some(widget) = self.placement.selected() else {
            return group(
                Some("Inspector"),
                vec![text::body("Select a widget on the canvas or in the list above.").into()],
            );
        };
        let mut parts = vec![group(Some("Placement"), vec![self.placement_inspector()])];
        parts.push(
            text::caption(moment.map_or_else(
                || "Editing base style · placement and fonts are shared.".into(),
                |p| {
                    format!(
                        "Editing {} clock style · placement and fonts are shared.",
                        p.title()
                    )
                },
            ))
            .into(),
        );
        if widget == Widget::Clock {
            let choices = |pointer: &'static str,
                           title: &'static str,
                           current: Option<&str>,
                           options: &[(&'static str, &'static str)]| {
                let mut buttons: Vec<Element<'_, Message>> = vec![
                    button::custom(text::body("Inherit"))
                        .class(nav_style(current.is_none(), self.settings_accent()))
                        .id(cosmic::widget::Id::new(format!("canvas-reset-{title}")))
                        .on_press(action(Edit::Reset(pointer)))
                        .into(),
                ];
                buttons.extend(options.iter().map(|(label, value)| {
                    button::custom(text::body(*label))
                        .class(nav_style(current == Some(*value), self.settings_accent()))
                        .id(cosmic::widget::Id::new(format!("canvas-{title}-{value}")))
                        .on_press(action(Edit::Style(pointer, serde_json::json!(value))))
                        .into()
                }));
                column::with_children(vec![
                    split(
                        text::body(title).into(),
                        text::caption(if current.is_some() {
                            if moment.is_some() {
                                "Moment override"
                            } else {
                                "Clock setting"
                            }
                        } else {
                            if moment.is_some() {
                                "Base widget style"
                            } else {
                                "Style default"
                            }
                        })
                        .into(),
                    ),
                    row::with_children(buttons).spacing(5).wrap().into(),
                ])
                .spacing(6)
                .into()
            };
            parts.push(group(
                Some("Clock style"),
                vec![
                    choices(
                        "/clock/face",
                        "Face",
                        clock.face.map(|v| match v {
                            ClockFace::Digital => "digital",
                            ClockFace::Analog => "analog",
                        }),
                        &[("Digital", "digital"), ("Analog", "analog")],
                    ),
                    choices(
                        "/clock/surface",
                        "Surface",
                        clock.surface.map(|v| match v {
                            ClockSurface::None => "none",
                            ClockSurface::Panel => "panel",
                        }),
                        &[("None", "none"), ("Panel", "panel")],
                    ),
                    choices(
                        "/clock/weight",
                        "Weight",
                        clock.weight.map(|v| match v {
                            ClockWeight::Light => "light",
                            ClockWeight::Normal => "normal",
                            ClockWeight::Bold => "bold",
                        }),
                        &[("Light", "light"), ("Normal", "normal"), ("Bold", "bold")],
                    ),
                    self.widget_font_row(
                        FontTarget::Clock,
                        "Clock font",
                        config.clock_font.as_deref(),
                        "Same as widget text",
                    ),
                    text::caption(if config.clock_font.is_some() {
                        "Clock font override · unavailable fonts use the system fallback."
                    } else if config.widget_font.is_some() {
                        "Font inherited from widget text."
                    } else {
                        "Font inherited from the style."
                    })
                    .into(),
                ],
            ));
        }
        if widget == Widget::Clock && clock.surface.is_some() && config.appearance.contrast_assist {
            parts.push(
                text::caption(
                    "Readability keeps a solid surface. Your surface choice is retained.",
                )
                .into(),
            );
        }
        parts.push(group(
            Some("Shared text"),
            vec![
                self.widget_font_row(
                    FontTarget::Text,
                    "Widget font",
                    config.widget_font.as_deref(),
                    "Style default",
                ),
                text::caption("Applies to widgets that inherit this font.").into(),
            ],
        ));
        parts.push(text::caption("Content uses samples here. Edit notes, accounts and live controls in the widget’s content settings.").into());
        column::with_children(parts).spacing(14).into()
    }
    pub(super) fn canvas_actions(&self) -> Element<'_, Message> {
        let id = self.canvas.as_ref().unwrap().id;
        let mut parts = Vec::new();
        if self.error {
            parts.push(label(&self.status, 12., Color::from_rgb8(245, 149, 138)));
        }
        parts.push(
            row::with_children(vec![
                button::text("Undo edit")
                    .id(cosmic::widget::Id::new("canvas-undo"))
                    .on_press_maybe(
                        (self.canvas_can_undo() && !self.native_busy).then_some(action(Edit::Undo)),
                    )
                    .into(),
                container(text::caption(if self.native_busy {
                    "Applying…"
                } else if self.canvas_dirty() {
                    "Previewing"
                } else {
                    "Current desktop"
                }))
                .width(Length::Fill)
                .into(),
                button::standard("Cancel")
                    .id(cosmic::widget::Id::new("canvas-cancel"))
                    .on_press_maybe((!self.native_busy).then_some(action(Edit::Cancel)))
                    .into(),
                button::suggested("Apply")
                    .id(cosmic::widget::Id::new("canvas-apply"))
                    .on_press_maybe(
                        (self.canvas_dirty() && !self.native_busy && !self.placement.dragging())
                            .then_some(action(Edit::Apply(id))),
                    )
                    .into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        );
        container(column::with_children(parts).spacing(6))
            .padding([10, 18])
            .width(Length::Fill)
            .into()
    }
}
