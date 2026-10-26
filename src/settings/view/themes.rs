//! Native Themes: one look for widgets, dock, panel and windows, chosen with
//! a local preview of your desktop, a style, a color and an optional
//! matching wallpaper. The same profile editor owns the draft, conflict
//! gates, file chooser, transaction and Undo on both related pages.
use super::desktop::{ERROR, WARN, banner, line};
use super::profiles::tile_style;
use super::system_appearance::fill;
use super::*;
use crate::desktop::profiles::{Domain, themes};
use crate::desktop::wallpaper::automatic;
use crate::desktop_theme::{Family, tone};
use crate::settings::profiles::{Message as Edit, NameAction, Selection};
use cosmic::iced::widget::stack;

fn edit(message: Edit) -> Message {
    Message::Profiles(message)
}

/// What the page shows: the selected look, otherwise the applied one.
struct Look {
    family: Option<Family>,
    /// Colors follow the wallpaper.
    auto: bool,
    accent: Option<[u8; 3]>,
    matching: bool,
    selected: bool,
}

/// The color the look uses, as the color row names it.
#[derive(Clone, Copy, PartialEq)]
enum Choice {
    Wallpaper,
    Hue(usize),
    Style,
    Custom,
}

impl Settings {
    fn look(&self) -> Look {
        // "Before Themes" and similar saved looks explicitly have no style.
        if let Some(p) = self.profiles.profile().filter(|p| p.appearance_only)
            && p.widget_style
                .as_ref()
                .and_then(|s| s.pointer("/appearance/family"))
                .is_some_and(serde_json::Value::is_null)
        {
            return Look {
                family: None,
                auto: false,
                accent: None,
                matching: false,
                selected: true,
            };
        }
        if let Some(p) = self.profiles.profile().filter(|p| p.appearance_only)
            && let Some(family) = themes::family(p).or(self.config.appearance.family)
        {
            // A surface reset makes no color choice of its own: it previews
            // the applied family's colors for that one surface.
            if themes::family(p).is_none() || self.profiles.resetting() {
                let a = &self.config.appearance;
                return Look {
                    family: Some(family),
                    auto: a.wallpaper_colors,
                    accent: a.accent,
                    matching: false,
                    selected: true,
                };
            }
            let get = |pointer: &str| p.widget_style.as_ref().and_then(|s| s.pointer(pointer));
            return Look {
                family: Some(family),
                auto: get("/appearance/wallpaper_colors").and_then(serde_json::Value::as_bool)
                    == Some(true),
                accent: get("/appearance/accent")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .flatten(),
                matching: p.wallpaper.is_some(),
                selected: true,
            };
        }
        let a = &self.config.appearance;
        // Before the first theme, specimens show the colors choosing one
        // would start with (see `ProfilesEditor::first_theme_colors`); after
        // it, choosing another style keeps this color.
        let first = a.family.is_none()
            && (a.wallpaper_colors
                || (a.accent.is_none()
                    && self
                        .profiles
                        .suggestion()
                        .is_some_and(|s| !s.swatches.is_empty())));
        Look {
            family: a.family,
            auto: (a.family.is_some() && a.wallpaper_colors) || first,
            accent: a.accent,
            matching: false,
            selected: false,
        }
    }
    /// The wallpaper's lively hues, besides the one "From wallpaper" uses.
    fn wallpaper_hues(&self) -> Vec<[u8; 3]> {
        self.profiles
            .suggestion()
            .map(|s| s.accents.iter().skip(1).take(3).copied().collect())
            .unwrap_or_default()
    }
    fn choice(&self, look: &Look) -> Choice {
        if look.auto {
            return Choice::Wallpaper;
        }
        let choice = self
            .profiles
            .profile()
            .and_then(themes::LookColor::of)
            .unwrap_or_else(|| themes::LookColor::applied(&self.config.appearance));
        match choice {
            themes::LookColor::Wallpaper => Choice::Wallpaper,
            themes::LookColor::Style => Choice::Style,
            themes::LookColor::Fixed(rgb) => self
                .wallpaper_hues()
                .iter()
                .position(|hue| *hue == rgb)
                .map_or(Choice::Custom, Choice::Hue),
        }
    }
    /// The seed color a look would use for `family`. A style's own color is
    /// each style's own, as choosing that style gives it.
    fn look_accent(&self, look: &Look, family: Family) -> Option<[u8; 3]> {
        if look.auto {
            self.profiles
                .suggestion()
                .and_then(|s| automatic::recommended_for(s, Some(family)))
        } else {
            let policy = self
                .profiles
                .profile()
                .and_then(themes::LookColor::of)
                .unwrap_or_else(|| themes::LookColor::applied(&self.config.appearance));
            match policy {
                themes::LookColor::Fixed(rgb) => Some(rgb),
                _ => None,
            }
        }
    }

    pub(super) fn themes_page(&self) -> Element<'_, Message> {
        if self.style_browser {
            return self.styles_page();
        }
        let editor = &self.profiles;
        let look = self.look();
        let mut rows = vec![split(
            heading("Theme & colors", "Make your desktop feel like home."),
            button::standard("Choose theme…")
                .id(cosmic::widget::Id::new("theme-browse-styles"))
                .on_press(Message::StyleBrowser(true))
                .into(),
        )];
        if let Some(error) = editor.unavailable() {
            rows.push(banner(error.to_string(), ERROR, vec![]));
            return column::with_children(rows).spacing(16).into();
        }
        if editor.recovery().is_some() {
            rows.push(banner(
                "An earlier desktop change was interrupted.",
                WARN,
                vec![
                    button::text("Keep current")
                        .on_press(edit(Edit::KeepRecovery))
                        .into(),
                    button::standard("Restore previous")
                        .on_press(edit(Edit::Recover))
                        .into(),
                ],
            ));
        }
        if self.config.appearance.family.is_some()
            && self.config.appearance.native_mode_palettes == 0
        {
            rows.push(banner(
                "Native light and dark matching has not been applied. Review a repair to let the bars follow Mode.",
                WARN,
                vec![button::standard("Repair theme matching")
                    .id("theme-matching-repair".into())
                    .on_press(edit(Edit::RepairThemeMatching)).into()],
            ));
        }
        let mut controls = Vec::new();
        controls.push(pages::section(vec![group_row(
            "Color applies to",
            if look.selected {
                "Part of this local theme preview"
            } else {
                "Shared color coordination"
            },
            self.colors_scope(),
        )]));
        if look.selected && look.family.is_some() {
            controls.push(self.theme_colors(&look));
        } else {
            controls.push(self.color_choices());
        }
        controls.push(pages::section(vec![
            split(
                detail(
                    "Desktop mode",
                    "Applies directly, independently of the theme preview",
                ),
                segmented(
                    crate::desktop::appearance::SystemMode::ALL
                        .into_iter()
                        .map(|mode| {
                            (
                                mode.title().into(),
                                self.system_appearance
                                    .draft()
                                    .is_some_and(|d| d.mode().known() == Some(&mode)),
                                Some(Message::SystemAppearance(
                                    crate::settings::system_appearance::Message::Mode(mode),
                                )),
                            )
                        })
                        .collect(),
                    self.settings_accent(),
                ),
            ),
            separator(),
            // The one detailed editor this page opens.
            button::custom(
                row::with_children(vec![
                    detail(
                        "Fonts & icons",
                        "Typography, detailed mode colors and window appearance",
                    ),
                    icon::from_name("go-next-symbolic").size(16).icon().into(),
                ])
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .id(cosmic::widget::Id::new("themes-fonts-icons"))
            .class(nav_style(false, self.settings_accent()))
            .padding([4, 0])
            .width(Length::Fill)
            .on_press(Message::Page(Page::DesktopAppearance))
            .into(),
        ]));
        if self.window.width >= 1120. && self.scene_open() {
            rows.push(
                row::with_children(vec![
                    container(self.theme_stage(&look))
                        .width(Length::FillPortion(1))
                        .into(),
                    container(column::with_children(controls).spacing(16))
                        .width(Length::FillPortion(1))
                        .into(),
                ])
                .spacing(24)
                .align_y(Alignment::Start)
                .into(),
            );
        } else {
            rows.push(self.theme_stage(&look));
            rows.extend(controls);
        }
        if let Some(p) = editor.profile().filter(|p| !p.appearance_only) {
            rows.push(banner(
                format!("“{}” is a desktop setup. Review it in Setups.", p.name),
                WARN,
                vec![
                    button::text("Open Setups")
                        .on_press(Message::Page(Page::DesktopProfiles))
                        .into(),
                ],
            ));
        }
        if look.family == Some(Family::Paper)
            && look.selected
            && editor.installed.fonts.family(false, "Noto Serif").is_none()
        {
            rows.push(label(
                "Noto Serif isn’t installed, so the clock uses the system font.",
                12.,
                MUTED,
            ));
        }
        for note in &editor.notes {
            rows.push(label(note.clone(), 12., MUTED));
        }
        if let Some(error) = editor.theme_problem() {
            rows.push(banner(
                error,
                ERROR,
                vec![
                    button::text("Use the theme’s color")
                        .on_press(edit(Edit::ThemeColors(false)))
                        .into(),
                ],
            ));
        }
        if look.selected {
            let mut states = Vec::new();
            if editor.plan().is_some_and(|plan| !plan.is_empty()) {
                states.push(label(
                    "Changes how widgets, dock, panel and windows look. Your layout, content and applets stay as they are.",
                    12.,
                    MUTED,
                ));
            }
            states.extend(self.plan_states());
            rows.push(column::with_children(states).spacing(8).into());
        }
        if let Some(family) = self.config.appearance.family {
            rows.push(self.theme_desktop_status(family));
        }
        rows.push(self.library_section());
        if self.config.appearance.family.is_none() {
            rows.push(label(
                "Your current look is saved as “Before Themes” when you first use a theme.",
                12.,
                MUTED,
            ));
        }
        column::with_children(rows).spacing(16).into()
    }

    /// Only the scene is collapsible; the candidate identity and controls stay
    /// visible. The existing profile preview has no live desktop side effects.
    fn theme_stage(&self, look: &Look) -> Element<'_, Message> {
        // Size the scene for its column, keeping the whole display visible
        // inside a short window. Wide windows put routine controls beside it.
        let screen = self.placement.preview_snapshot().screen;
        let ratio = (screen[1] / screen[0].max(1.)).clamp(0.4, 0.8);
        let width = (self.window.width - 184. - 44.).max(320.);
        let width = if self.window.width >= 1120. {
            (width - 24.) * 0.5
        } else {
            width
        };
        let height = (width * ratio).min((self.window.height - 250.).max(180.));
        let scene = if self.scene_open() {
            self.profile_preview(height)
        } else {
            cosmic::widget::space().height(0).into()
        };
        let caption = if let Some(profile) = self.profiles.profile() {
            if look.selected {
                format!("Previewing {} · local until Apply theme", profile.name)
            } else {
                format!(
                    "Previewing setup {} · review included changes in Setups",
                    profile.name
                )
            }
        } else {
            format!(
                "Current appearance · {}",
                look.family.map_or("Your desktop", Family::title)
            )
        };
        let mut parts = vec![self.scene_frame(scene, &caption)];
        // A selected candidate keeps its identity and library actions. The
        // scene caption names the current family when there is no candidate.
        if self.profiles.profile().is_some() {
            parts.push(pages::section(vec![self.theme_caption(look)]));
        }
        column::with_children(parts).spacing(12).into()
    }

    /// Every style as a whole small desktop, filtered by kind; choosing one
    /// previews it, and the footer uses it.
    pub(super) fn styles_page(&self) -> Element<'_, Message> {
        let look = self.look();
        let accent = self.settings_accent();
        let content = (self.window.width - 184. - 44. - 14.).max(320.);
        let columns = if content >= 620. { 3 } else { 2 };
        let tile = ((content - 12. * (columns - 1) as f32) / columns as f32).floor();
        let chips = row::with_children(
            crate::settings::StyleFilter::ALL
                .into_iter()
                .map(|filter| {
                    let on = self.style_filter == filter;
                    button::text(filter.title())
                        .id(cosmic::widget::Id::new(format!(
                            "styles-filter-{}",
                            filter.title().to_lowercase()
                        )))
                        .class(chip_style(on, accent))
                        .on_press_maybe((!on).then_some(Message::StyleFilter(filter)))
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(6);
        let tiles: Vec<Element<'_, Message>> = Family::ALL
            .into_iter()
            .filter(|family| self.style_filter.shows(*family))
            .map(|family| self.style_tile(family, &look, tile))
            .collect();
        let mut rows = vec![
            button::text("Theme & colors")
                .leading_icon(icon::from_name("go-previous-symbolic").size(14))
                .id(cosmic::widget::Id::new("styles-back"))
                .on_press(Message::StyleBrowser(false))
                .into(),
            heading(
                "Choose a theme",
                "Browse locally. Apply theme confirms your choice; Cancel keeps your desktop.",
            ),
            label(
                "How widgets, the dock and the panel look. Your color, layout and apps stay the same.",
                13.,
                MUTED,
            ),
            chips.into(),
            row::with_children(tiles).spacing(12).wrap().into(),
            pages::section(vec![split(
                detail(
                    "Style the dock and panel too",
                    "They take the style's color and shape",
                ),
                container(toggler(self.config.appearance.style_bars).on_toggle(Message::StyleBars))
                    .id(cosmic::widget::Id::new("theme-style-bars"))
                    .into(),
            )]),
        ];
        if look.selected {
            rows.extend(self.plan_states());
        }
        column::with_children(rows).spacing(14).into()
    }

    /// The look's name and what can be done with it.
    fn theme_caption(&self, look: &Look) -> Element<'_, Message> {
        let editor = &self.profiles;
        let selection = editor.selection();
        let (title, subtitle) = match (editor.profile(), look.family) {
            (Some(p), family) if p.appearance_only => (
                p.name.clone(),
                match (selection, family) {
                    (Selection::Saved(id), _) if id.as_str() == themes::BEFORE_THEMES => {
                        "Your look before the first theme".to_string()
                    }
                    (Selection::Saved(_), _) => "Saved on this computer".to_string(),
                    (_, Some(family)) => family.description().to_string(),
                    (_, None) => String::new(),
                },
            ),
            (None, Some(family)) => (
                format!("{} · on your desktop", family.title()),
                "Your applied theme".into(),
            ),
            _ => (
                "Your desktop".into(),
                "Choose a theme to preview it here".into(),
            ),
        };
        let mut actions: Vec<Element<'_, Message>> = Vec::new();
        if editor.profile().is_some_and(|p| p.appearance_only) {
            let writable = editor.writable();
            let action = |name: &'static str, tip: &'static str, message: Option<Message>| {
                button::icon(icon::from_name(name).size(16))
                    .tooltip(tip)
                    .on_press_maybe(message)
                    .into()
            };
            actions.push(action(
                "edit-copy-symbolic",
                "Save a copy",
                writable.then_some(edit(Edit::Name(Some(NameAction::Duplicate)))),
            ));
            if let Selection::Saved(id) = selection {
                actions.push(action(
                    "document-edit-symbolic",
                    "Rename",
                    writable.then(|| edit(Edit::Name(Some(NameAction::Rename(id.clone()))))),
                ));
            }
            actions.push(action(
                "document-save-as-symbolic",
                "Export…",
                (writable && !editor.busy()).then_some(edit(Edit::Export)),
            ));
            if matches!(selection, Selection::Saved(id) if id.as_str() != themes::BEFORE_THEMES) {
                actions.push(action(
                    "user-trash-symbolic",
                    "Delete…",
                    writable.then_some(edit(Edit::Delete(false))),
                ));
            }
        }
        let mut items = vec![
            row::with_children(vec![
                column::with_children(vec![label(title, 15., FG), label(subtitle, 12., MUTED)])
                    .spacing(2)
                    .width(Length::Fill)
                    .into(),
                row::with_children(actions).spacing(2).into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        ];
        if editor.name.as_ref().is_some_and(|name| {
            !matches!(name.action, NameAction::Capture | NameAction::CaptureTheme)
        }) {
            items.push(self.name_field("Save"));
        }
        if editor.confirm_delete
            && let Some(p) = editor.profile()
        {
            items.push(self.delete_confirm(format!("“{}”", p.name)));
        }
        column::with_children(items).spacing(10).into()
    }

    /// The configuration a style would draw with: the style's geometry and
    /// fonts (not the user's overrides) in the look's color.
    fn style_config(&self, family: Family, look: &Look) -> Config {
        let mut config = self.config.clone();
        let a = &mut config.appearance;
        a.family = Some(family);
        (a.opacity, a.corner_radius, a.border_opacity) = family.geometry();
        a.surface = None;
        a.contrast_assist = false;
        a.accent = self.look_accent(look, family);
        a.wallpaper_light = family.light();
        config.widget_font = None;
        config.clock_font = None;
        config
    }

    /// A style as a small desktop `width` wide, on its own wallpaper: the
    /// panel with workspaces, clock and tray; a card with the time, date and
    /// an event and a second card, in the style's fonts, borders and
    /// shadows; and the dock. The dock and panel use the style's color.
    fn style_specimen(&self, family: Family, look: &Look, width: f32) -> Element<'_, Message> {
        let config = self.style_config(family, look);
        let spec = family.spec();
        let roles = config.theme_roles().expect("a family is set");
        let card = roles.card;
        let (border_width, shadow) = {
            let _style = crate::widget_style::scope(&config);
            crate::widget_style::edge(card)
        };
        let height = (width * 0.62).round();
        let s = height / 160.;
        let border = Color {
            a: if roles.transparent {
                0.
            } else {
                f32::from(spec.geometry.2.min(50)) / 100.
            },
            ..card.accent
        };
        let radius = f32::from(spec.geometry.1.min(32)) * s * 0.6;
        let card_style = move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
            background: Some(Background::Color(card.background)),
            border: Border {
                color: border,
                width: if roles.transparent { 0. } else { border_width },
                radius: radius.into(),
            },
            shadow: cosmic::iced::Shadow {
                blur_radius: shadow.blur_radius * s * 0.8,
                offset: shadow.offset * s * 0.8,
                ..shadow
            },
            ..Default::default()
        };
        let text = |value: &'static str, size: f32, font, color| -> Element<'static, Message> {
            text::body(value)
                .size((size * s).round().max(6.))
                .font(font)
                .wrapping(cosmic::iced::advanced::text::Wrapping::None)
                .class(cosmic::theme::Text::Color(color))
                .into()
        };
        let block = |w: f32, h: f32, color: Color, r: f32| -> Element<'static, Message> {
            fill(
                cosmic::iced::widget::space()
                    .width(w * s)
                    .height(h * s)
                    .into(),
                color,
                Color::TRANSPARENT,
                0.,
                r * s,
            )
            .into()
        };
        // The bars in the style's own color (see `themes::record`).
        let bar = spec.bar;
        let tone = {
            let [r, g, b] = family.surface(family.accent());
            Color::from_rgba8(r, g, b, bar.opacity)
        };
        let bar_radius = (bar.radius as f32 * s * 0.5).min(8. * s);
        let ink = card.foreground;
        let bar_style = move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
            background: Some(Background::Color(tone)),
            border: Border {
                radius: bar_radius.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let dot_radius = if bar.radius == 0 { 0. } else { 3. };
        let panel = container(
            row::with_children(vec![
                row::with_children(vec![
                    block(6., 6., card.accent, dot_radius),
                    block(6., 6., Color { a: 0.45, ..ink }, dot_radius),
                ])
                .spacing(4. * s)
                .width(Length::Fill)
                .into(),
                text("09:41", 9., roles.body, ink),
                container(
                    row::with_children(vec![
                        block(7., 7., Color { a: 0.7, ..ink }, dot_radius * 0.6),
                        block(7., 7., Color { a: 0.7, ..ink }, dot_radius * 0.6),
                    ])
                    .spacing(4. * s),
                )
                .width(Length::Fill)
                .align_x(Alignment::End)
                .into(),
            ])
            .align_y(Alignment::Center),
        )
        .padding([2. * s, 8. * s])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(bar_style));
        let event = row::with_children(vec![
            block(3., 12., card.accent, 1.5),
            text("Standup", 10., roles.body, card.foreground),
        ])
        .spacing(5. * s)
        .align_y(Alignment::Center);
        let main = container(
            column::with_children(vec![
                text("09:41", 30., roles.clock, card.foreground),
                text("Wed 21 · 18°", 10., roles.body, card.muted),
                event.into(),
            ])
            .spacing(4. * s),
        )
        .padding([9. * s, 11. * s])
        .width(118. * s)
        .class(cosmic::theme::Container::custom(card_style));
        let side = container(
            column::with_children(vec![
                block(52., 4., card.accent, 2.),
                block(
                    38.,
                    4.,
                    Color {
                        a: 0.6,
                        ..card.muted
                    },
                    2.,
                ),
                block(
                    46.,
                    4.,
                    Color {
                        a: 0.6,
                        ..card.muted
                    },
                    2.,
                ),
            ])
            .spacing(6. * s),
        )
        .padding([10. * s, 10. * s])
        .width(76. * s)
        .height(52. * s)
        .class(cosmic::theme::Container::custom(card_style));
        let icon_radius = if bar.radius == 0 { 0. } else { 3.5 };
        let dock = container(
            row::with_children(vec![
                block(11., 11., card.accent, icon_radius),
                block(11., 11., Color { a: 0.8, ..ink }, icon_radius),
                block(11., 11., Color { a: 0.55, ..ink }, icon_radius),
                block(11., 11., Color { a: 0.55, ..ink }, icon_radius),
            ])
            .spacing(5. * s),
        )
        .padding([4. * s, 7. * s])
        .class(cosmic::theme::Container::custom(bar_style));
        let (top, bottom) = spec.wallpaper;
        let stage = container(cosmic::iced::widget::space())
            .width(width)
            .height(height)
            .class(cosmic::theme::Container::custom(move |_| {
                cosmic::iced::widget::container::Style {
                    background: Some(Background::Gradient(
                        cosmic::iced::gradient::Linear::new(2.4)
                            .add_stop(0., Color::from_rgb8(top[0], top[1], top[2]))
                            .add_stop(1., Color::from_rgb8(bottom[0], bottom[1], bottom[2]))
                            .into(),
                    )),
                    border: Border {
                        radius: 7.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }));
        let floating = if bar.floating { 5. * s } else { 0. };
        stack(vec![
            stage.into(),
            container(panel)
                .padding([floating, floating])
                .width(width)
                .into(),
            container({
                // A small specimen keeps just the main card.
                let mut cards: Vec<Element<'_, Message>> = vec![main.into()];
                if width >= 180. {
                    cards.push(
                        container(side)
                            .width(Length::Fill)
                            .align_x(Alignment::End)
                            .into(),
                    );
                }
                row::with_children(cards).align_y(Alignment::Start)
            })
            .padding([24. * s, 12. * s, 0., 12. * s])
            .width(width)
            .height(height)
            .into(),
            container(dock)
                .padding([0., 0., 5. * s + floating * 0.4, 0.])
                .width(width)
                .height(height)
                .align_x(Alignment::Center)
                .align_y(Alignment::End)
                .into(),
        ])
        .width(width)
        .height(height)
        .clip(true)
        .into()
    }

    /// A style to choose: its small desktop, name and description.
    fn style_tile(&self, family: Family, look: &Look, width: f32) -> Element<'_, Message> {
        let editor = &self.profiles;
        let selected = *editor.selection() == Selection::Preset(family.preset())
            || (look.selected
                && look.family == Some(family)
                && matches!(editor.selection(), Selection::Preset(_)));
        let applied = self.config.appearance.family == Some(family);
        let marker: Element<'_, Message> = if selected {
            label("Previewing", 11., self.settings_accent())
        } else if applied {
            label("Current", 11., MUTED)
        } else {
            cosmic::iced::widget::space().into()
        };
        container(
            button::custom(
                column::with_children(vec![
                    self.style_specimen(family, look, width - 16.),
                    row::with_children(vec![
                        container(line(family.title(), 14., FG))
                            .width(Length::Fill)
                            .into(),
                        marker,
                    ])
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .into(),
                    line(family.description(), 12., MUTED),
                ])
                .spacing(6),
            )
            .class(tile_style(selected, self.settings_accent()))
            .selected(selected)
            .padding(8)
            .width(Length::Fill)
            .on_press(edit(Edit::Select(Selection::Preset(family.preset())))),
        )
        .id(cosmic::widget::Id::new(format!(
            "preset-{}",
            family.preset()
        )))
        .width(width)
        .into()
    }

    /// One color choice: from the wallpaper (it follows changes), one of
    /// its other hues, the style's own color, or a custom one; then the
    /// optional matching wallpaper.
    fn theme_colors(&self, look: &Look) -> Element<'_, Message> {
        let family = look.family.expect("a look is shown");
        let accent = self.settings_accent();
        let choice = self.choice(look);
        let chip = |content: Element<'static, Message>,
                    selected: bool,
                    message: Message,
                    id: String|
         -> Element<'static, Message> {
            container(
                button::custom(content)
                    .class(tile_style(selected, accent))
                    .selected(selected)
                    .padding([5, 10])
                    .on_press(message),
            )
            .id(cosmic::widget::Id::new(id))
            .into()
        };
        let named = |rgb: Option<[u8; 3]>, title: String| -> Element<'static, Message> {
            let mut parts: Vec<Element<'static, Message>> = Vec::new();
            if let Some(rgb) = rgb {
                parts.push(dot(tone::color(rgb), 16.));
            }
            parts.push(label(title, 13., FG));
            row::with_children(parts)
                .spacing(7)
                .align_y(Alignment::Center)
                .into()
        };
        let auto_rgb = self
            .profiles
            .suggestion()
            .and_then(|s| automatic::recommended_for(s, Some(family)));
        let mut chips = vec![chip(
            named(auto_rgb, "From wallpaper".into()),
            choice == Choice::Wallpaper,
            edit(Edit::ThemeColors(true)),
            "theme-color-wallpaper".into(),
        )];
        for (index, hue) in self.wallpaper_hues().into_iter().enumerate() {
            let [r, g, b] = hue;
            chips.push(
                cosmic::widget::tooltip(
                    chip(
                        dot(tone::color(hue), 16.),
                        choice == Choice::Hue(index),
                        edit(Edit::ThemeAccent(hue)),
                        format!("theme-color-hue-{index}"),
                    ),
                    text::body(format!(
                        "Another color from your wallpaper · #{r:02X}{g:02X}{b:02X}"
                    ))
                    .size(12),
                    cosmic::widget::tooltip::Position::Top,
                )
                .into(),
            );
        }
        chips.push(chip(
            named(Some(family.accent()), family.title().into()),
            choice == Choice::Style,
            edit(Edit::ThemeColors(false)),
            "theme-color-style".into(),
        ));
        let custom_rgb = (choice == Choice::Custom).then_some(look.accent).flatten();
        chips.push(chip(
            named(custom_rgb, "Custom".into()),
            choice == Choice::Custom || self.profiles.custom.is_some(),
            edit(Edit::ThemeCustom(Some(
                custom_rgb
                    .or(look.accent)
                    .map(|[r, g, b]| format!("#{r:02X}{g:02X}{b:02X}"))
                    .unwrap_or_default(),
            ))),
            "theme-color-custom".into(),
        ));
        let neutral = self
            .profiles
            .suggestion()
            .is_some_and(|s| s.accents.is_empty() && !s.swatches.is_empty());
        let description = match choice {
            Choice::Wallpaper if neutral => format!(
                "This wallpaper has no strong color, so {}’s own is used until it changes",
                family.title()
            ),
            Choice::Wallpaper if auto_rgb.is_some() => {
                "Follows your wallpaper, including when it changes".to_string()
            }
            Choice::Wallpaper => self
                .profiles
                .theme_problem()
                .or(self.profiles.colors_unavailable())
                .map_or_else(
                    || "Finding your wallpaper’s colors…".to_string(),
                    str::to_string,
                ),
            Choice::Hue(_) => "A color from your wallpaper. It stays as it is".to_string(),
            Choice::Style => format!("{}’s own color", family.title()),
            Choice::Custom => "Your color".to_string(),
        };
        let mut rows = vec![
            column::with_children(vec![
                row::with_children(chips)
                    .spacing(4)
                    .wrap()
                    .vertical_spacing(4)
                    .into(),
                label(description, 12., MUTED),
            ])
            .spacing(6)
            .into(),
        ];
        if let Some(text) = &self.profiles.custom {
            let target = crate::settings::color_well::Target::ThemeCustom;
            rows.push(
                row::with_children(vec![
                    self.color_button(
                        "theme-color-hex",
                        target,
                        crate::settings::wallpaper::parse_rgb(text),
                        true,
                    ),
                    button::text("Done")
                        .on_press(edit(Edit::ThemeCustom(None)))
                        .into(),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            );
            if let Some(panel) = self.color_panel(target, &[]) {
                rows.push(panel);
            }
        }
        rows.push(group_row(
            "Wallpaper",
            "",
            container(segmented(
                vec![
                    (
                        "Yours".into(),
                        !look.matching,
                        Some(edit(Edit::ThemeWallpaper(false))),
                    ),
                    (
                        format!("{} gradient", family.title()),
                        look.matching,
                        Some(edit(Edit::ThemeWallpaper(true))),
                    ),
                ],
                accent,
            ))
            .id(cosmic::widget::Id::new("theme-matching-wallpaper"))
            .into(),
        ));
        group(Some("Preview color source"), rows)
    }

    /// Each themed surface after Apply: still the theme, or customized with
    /// a way back. Windows are the COSMIC accent and active outline.
    fn theme_desktop_status(&self, family: Family) -> Element<'_, Message> {
        let editor = &self.profiles;
        let mut rows = vec![label("On your desktop", 14., FG)];
        for (domain, title, customize) in [
            (
                Domain::WidgetStyle,
                "Widgets",
                Message::Page(Page::Appearance),
            ),
            (
                Domain::Dock,
                "Dock",
                Message::OpenBar(crate::desktop::panels::PanelKind::Dock),
            ),
            (
                Domain::Panel,
                "Panel",
                Message::OpenBar(crate::desktop::panels::PanelKind::Panel),
            ),
            (
                Domain::Appearance,
                "Windows",
                Message::Page(Page::DesktopAppearance),
            ),
        ] {
            let custom = if domain == Domain::WidgetStyle {
                self.config.widget_custom()
            } else {
                editor.custom_surfaces.contains(&domain)
            };
            let mut actions = vec![button::text("Customize").on_press(customize).into()];
            if custom {
                actions.push(
                    button::text("Reset")
                        .on_press(Message::ResetThemeSurface(domain))
                        .into(),
                );
            }
            rows.push(split(
                row::with_children(vec![
                    container(label(title, 13., FG)).width(84).into(),
                    label(
                        if custom {
                            "Customized".to_string()
                        } else {
                            format!("Matches {}", family.title())
                        },
                        12.,
                        if custom {
                            self.settings_accent()
                        } else {
                            MUTED
                        },
                    ),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
                row::with_children(actions).spacing(4).into(),
            ));
        }
        pages::section(rows)
    }
}
