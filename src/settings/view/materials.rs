//! One material chooser, the same wallpaper and colors in every specimen.
use super::*;
use crate::materials::{Finish, Kind, Target};
impl Settings {
    pub(super) fn appearance_page(&self) -> Element<'_, Message> {
        let target = self.material_target;
        let selected = self
            .material_draft
            .or(self.config.materials.effective(target));
        let chooser = row::with_children(
            Kind::ALL
                .into_iter()
                .map(|kind| {
                    let chosen = selected.is_some_and(|f| f.kind == kind);
                    container(
                        button::custom(
                            column::with_children(vec![
                                self.material_specimen(
                                    selected
                                        .filter(|f| f.kind == kind)
                                        .unwrap_or_else(|| Finish::new(kind)),
                                ),
                                label(kind.title(), 13., FG),
                                if self.material_adjust {
                                    cosmic::widget::space().height(0).into()
                                } else {
                                    label(
                                        match kind {
                                            Kind::Frosted => "Soft & balanced",
                                            Kind::Clear => "Light & luminous",
                                            Kind::Smoked => "Quiet & dense",
                                            Kind::Liquid => "Polished & rounded",
                                        },
                                        10.,
                                        MUTED,
                                    )
                                },
                            ])
                            .spacing(5),
                        )
                        .class(profiles::tile_style(chosen, self.settings_accent()))
                        .selected(chosen)
                        .padding(7)
                        .width(Length::Fill)
                        .on_press(Message::MaterialChoose(Some(kind))),
                    )
                    .id(cosmic::widget::Id::new(format!("material-{}", kind.key())))
                    .width(Length::Fill)
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(8);
        let scopes = segmented(
            Target::ALL
                .into_iter()
                .filter(|t| {
                    self.desktop.islands.installation.installed()
                        || matches!(t, Target::Shared | Target::Widgets)
                })
                .map(|t| {
                    (
                        t.title().to_owned(),
                        target == t,
                        Some(Message::MaterialTarget(t)),
                    )
                })
                .collect(),
            self.settings_accent(),
        );
        let state = if self.config.materials.get(target).is_some() {
            if target == Target::Shared {
                if [
                    self.config.materials.widgets,
                    self.config.materials.dock,
                    self.config.materials.panel,
                ]
                .into_iter()
                .any(|f| f.is_some())
                {
                    "Shared material · some surfaces use their own"
                } else {
                    "Shared by widgets, dock and panel"
                }
            } else {
                "Uses its own material"
            }
        } else if target == Target::Shared {
            "Your existing style is kept"
        } else {
            "Follows the shared material"
        };
        let mut rows = vec![
            split(
                heading("Appearance", ""),
                button::text("Colors & theme")
                    .on_press(Message::Page(Page::Themes))
                    .into(),
            ),
            scopes,
            chooser.into(),
            split(
                label(state, 12., MUTED),
                row::with_children(vec![
                    container(
                        button::text(if self.material_adjust {
                            "Done"
                        } else {
                            "Adjust"
                        })
                        .on_press_maybe(
                            selected.map(|_| Message::MaterialAdjust(!self.material_adjust)),
                        ),
                    )
                    .id(cosmic::widget::Id::new("material-adjust"))
                    .into(),
                    container(
                        button::text(if target == Target::Shared {
                            "Reset"
                        } else {
                            "Use shared"
                        })
                        .on_press_maybe(
                            self.config
                                .materials
                                .get(target)
                                .map(|_| Message::MaterialChoose(None)),
                        ),
                    )
                    .id(cosmic::widget::Id::new("material-reset"))
                    .into(),
                ])
                .spacing(4)
                .into(),
            ),
        ];
        if self.material_adjust
            && let Some(finish) = selected
        {
            rows.push(pages::section(vec![
                self.material_slider("Transparency", format!("{}%", 100 - finish.opacity()), 0..=55, 100 - finish.opacity(), true),
                self.material_slider("Color tint", format!("{}%", finish.tint), 0..=100, finish.tint, false),
                split(label("Liquid widget motion", 12., FG), toggler(self.config.material_motion).on_toggle(Message::MaterialMotion).into()),
                split(self.help("material-readability", "Widget readability", "Adds enough opacity for small text on bright or busy wallpaper. Your transparency setting is kept."),
                    container(toggler(self.config.appearance.contrast_assist).on_toggle(Message::ContrastAssist)).id(cosmic::widget::Id::new("material-readability")).into()),
            ]));
        }
        rows.push(label(if self.material_native {
            "Shared finish · compositor blur when available. Previews show tint and finish."
        } else {
            if self.desktop.islands.installation.installed() { "Widgets use native blur when available. Dock & panel materials need a running capable host." } else { "Widgets use native blur when available." }
        }, 11., MUTED));
        rows.push(self.preview_stage(self.card_preview()));
        rows.push(split(
            label("Real widget preview · sample data", 11., MUTED),
            segmented(
                [Widget::Weather, Widget::Devbox, Widget::Clock]
                    .into_iter()
                    .map(|w| {
                        (
                            w.title().into(),
                            self.appearance_preview == w,
                            Some(Message::AppearancePreview(w)),
                        )
                    })
                    .collect(),
                self.settings_accent(),
            ),
        ));
        rows.push(self.page_settings_advanced(Page::Appearance, vec![self.legacy_widget_style()]));
        column::with_children(rows)
            .spacing(12)
            .width(Length::Fill)
            .into()
    }
    fn material_slider(
        &self,
        title: &'static str,
        value: String,
        range: std::ops::RangeInclusive<u8>,
        current: u8,
        opacity: bool,
    ) -> Element<'_, Message> {
        column::with_children(vec![
            split(
                label(title, 12., FG),
                label(value, 11., self.settings_accent()),
            ),
            container(
                slider(range, current, move |v| {
                    Message::MaterialValue(opacity, if opacity { 100 - v } else { v })
                })
                .on_release(Message::MaterialSave)
                .width(Length::Fill)
                .apply(|v| completed_slider(v.into(), Message::MaterialSave)),
            )
            .id(cosmic::widget::Id::new(if opacity {
                "material-transparency"
            } else {
                "material-tint"
            }))
            .into(),
        ])
        .spacing(5)
        .into()
    }
    fn material_specimen(&self, finish: Finish) -> Element<'_, Message> {
        let mut config = self.config.clone();
        config.materials.shared = Some(finish);
        config.materials.widgets = None;
        let p = config.palette();
        let p = if p.background.a == 0. {
            crate::materials::palette(config.surface_palette(), Some(finish), false)
        } else {
            p
        };
        let bg = crate::materials::background(p.background, Some(finish));
        let surface = move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
            background: Some(bg),
            border: Border {
                color: Color::from_rgba(
                    1.,
                    1.,
                    1.,
                    if finish.kind == Kind::Liquid {
                        0.35
                    } else {
                        0.15
                    },
                ),
                width: 1.,
                radius: 9.into(),
            },
            ..Default::default()
        };
        let bar = |width| {
            container(cosmic::iced::widget::space().width(width).height(5))
                .padding([2, 5])
                .class(cosmic::theme::Container::custom(surface))
        };
        let card = container(
            column::with_children(vec![
                label("09:41", 19., p.foreground),
                label("Wed 21 · 18°", 9., p.foreground),
            ])
            .spacing(3),
        )
        .padding([6, 9])
        .class(cosmic::theme::Container::custom(surface));
        if self.material_adjust {
            let compact = container(label("09:41", 19., p.foreground))
                .padding([6, 9])
                .class(cosmic::theme::Container::custom(surface));
            return self.preview_stage_padded(compact.into(), 6);
        }
        let contents = column::with_children(vec![
            bar(Length::Fill).into(),
            card.into(),
            container(bar(Length::Fixed(28.)))
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .into(),
        ])
        .spacing(5);
        // Every sample uses the same cached wallpaper, never the style's bundled one.
        self.preview_stage_padded(contents.into(), 6)
    }
}
