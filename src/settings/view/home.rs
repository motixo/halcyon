//! Layout edits widgets on their wallpaper. Decorative windows and shell bars
//! belong to their own settings previews, not the widget placement canvas.
//! Completed gestures keep the existing direct-apply and session Undo path.
use super::*;

/// A part of the picture you can click: outlined on hover or keyboard focus.
/// Keep the focus ring inside the hit area so the picture's clip retains it.
pub(in crate::settings) fn hotspot(accent: Color, radius: f32) -> cosmic::theme::Button {
    let style = move |focused: bool, hovered: bool| cosmic::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border_color: if focused || hovered {
            accent
        } else {
            Color::TRANSPARENT
        },
        border_width: if focused {
            3.
        } else if hovered {
            2.
        } else {
            0.
        },
        border_radius: radius.into(),
        ..Default::default()
    };
    cosmic::theme::Button::Custom {
        active: Box::new(move |focused, _| style(focused, false)),
        hovered: Box::new(move |focused, _| style(focused, true)),
        pressed: Box::new(move |focused, _| style(focused, true)),
        disabled: Box::new(move |_| style(false, false)),
    }
}

pub(in crate::settings) fn widget_page(widget: Widget) -> Page {
    Page::parse(widget.key())
}

impl Settings {
    pub(super) fn overview_page(&self) -> Element<'_, Message> {
        if self.canvas.is_some() {
            return self.canvas_page();
        }
        if self.setup_requested {
            return column::with_children(vec![
                button::text("Back to Layout and displays")
                    .leading_icon(icon::from_name("go-previous-symbolic").size(14))
                    .id(cosmic::widget::Id::new("setup-back"))
                    .on_press(Message::SetupOpen(false))
                    .into(),
                self.setup_card(),
            ])
            .spacing(12)
            .into();
        }
        // Leave room for the selected widget's inspector and keyboard hint,
        // including native window decoration, without resizing mid-gesture.
        let height = (self.window.height - 440.).clamp(120., 420.);
        let mut parts = vec![heading(
            "Layout and displays",
            "Select a widget to move, resize or open its content settings.",
        )];
        if self.needs_setup() {
            parts.push(self.setup_prompt());
        }
        parts.push(
            button::suggested("Edit desktop…")
                .id(cosmic::widget::Id::new("canvas-start"))
                .on_press(Message::Canvas(crate::settings::canvas::Message::Start))
                .into(),
        );
        parts.push(self.placement_editor(height));
        // Use the same selection message as the canvas. Keep every visible
        // widget reachable even when cards overlap or a display is disconnected.
        parts.push(group(
            Some("Select a widget"),
            vec![
                row::with_children(
                    Widget::ALL
                        .into_iter()
                        .filter(|widget| self.config.widget_visible(*widget))
                        .map(|widget| {
                            let selected = self.placement.selected() == Some(widget);
                            button::custom(
                                row::with_children(vec![
                                    icon::from_name(widget.icon()).size(16).icon().into(),
                                    text::body(widget.title()).into(),
                                ])
                                .spacing(8)
                                .align_y(Alignment::Center),
                            )
                            .id(cosmic::widget::Id::new(format!(
                                "layout-select-{}",
                                widget.key()
                            )))
                            .class(nav_style(selected, self.settings_accent()))
                            .selected(selected)
                            .on_press(Message::Placement(
                                crate::settings::placement::Message::Select(widget.key().into()),
                            ))
                            .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(6)
                .wrap()
                .into(),
            ],
        ));
        // Manual placement makes automatic column ordering irrelevant.
        if !self.config.has_custom_positions() && !self.placement.dirty() {
            parts.push(self.home_column_order());
        }
        column::with_children(parts).spacing(12).into()
    }

    fn home_column_order(&self) -> Element<'_, Message> {
        let open = self.advanced_open(Page::WidgetLayout);
        let mut items = vec![
            button::text(if open {
                "Hide automatic order"
            } else {
                "Automatic column order"
            })
            .trailing_icon(
                icon::from_name(if open {
                    "go-up-symbolic"
                } else {
                    "go-down-symbolic"
                })
                .size(14),
            )
            .on_press(Message::ToggleAdvanced(Page::WidgetLayout))
            .into(),
        ];
        if open {
            items.push(self.widget_order_controls());
        }
        column::with_children(items).spacing(8).into()
    }

    /// The real wallpaper behind the shared placement editor. The backdrop
    /// deliberately has no actions: clicking empty space clears selection.
    pub(in crate::settings) fn home_backdrop(&self, size: [f32; 2]) -> Vec<Element<'_, Message>> {
        // A missing image must not substitute Settings' own light/dark theme
        // for the desktop: transparent white clock text still needs dark ink
        // contrast. This neutral fallback is labelled by the canvas editor.
        let fallback =
            if crate::desktop_theme::luminance(self.canvas_config().wallpaper_palette().foreground)
                > 0.18
            {
                Color::BLACK
            } else {
                Color::WHITE
            };
        let background: Element<'_, Message> = match self.profiles.home_image() {
            Some(handle) => cosmic::widget::image(handle.clone())
                .width(size[0])
                .height(size[1])
                .content_fit(cosmic::iced::ContentFit::Cover)
                .border_radius(10.)
                .into(),
            None => container(text::body(""))
                .width(size[0])
                .height(size[1])
                .class(cosmic::theme::Container::custom(move |_| {
                    cosmic::iced::widget::container::Style {
                        background: Some(Background::Color(fallback)),
                        border: Border {
                            radius: 10.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }))
                .into(),
        };
        vec![background]
    }
}
