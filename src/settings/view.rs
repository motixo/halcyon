mod automation;
mod calendar;
mod color_well;
mod colors;
mod desktop;
mod desktop_icons;
pub(super) mod home;
mod layered;
mod materials;
mod motion;
mod navigation;
mod pages;
mod preview;
pub(super) mod profiles;
mod reveal;
mod setup;
mod shortcuts;
mod sidebar;
mod slider_completion;
mod system_appearance;
mod themes;
mod tools;
mod wallpaper;
mod widget_text;
mod window_rules;
mod windows;
mod wizard;
mod workspaces;

use super::*;
use crate::config::Palette;
pub(super) use automation::time_id as automation_time_id;
use cosmic::Apply;
use cosmic::iced::{Alignment, Background, Border, Color};
use cosmic::widget::{
    button, column, container, icon, mouse_area, row, scrollable, slider, text, text_input, toggler,
};
pub(super) use slider_completion::completed_slider;

const FG: Color = Color::from_rgb(0.94, 0.93, 0.97);
const MUTED: Color = Color::from_rgb(0.65, 0.64, 0.71);
const SURFACE: Color = Color::from_rgb(0.13, 0.13, 0.16);
const BASE: Color = Color::from_rgb(0.09, 0.09, 0.115);
const LINE: Color = Color::from_rgb(0.22, 0.22, 0.27);

pub(super) fn text_style(color: Color) -> cosmic::theme::Text {
    if color == FG {
        cosmic::theme::Text::Default
    } else if color == MUTED {
        cosmic::theme::Text::Custom(|theme| cosmic::iced::widget::text::Style {
            color: Some(Color {
                a: if theme.theme_type.is_high_contrast() {
                    1.
                } else {
                    0.75
                },
                ..theme.cosmic().on_bg_color().into()
            }),
            ..Default::default()
        })
    } else if color == cosmic::theme::active().cosmic().accent_text_color().into() {
        cosmic::theme::Text::Custom(|theme| cosmic::iced::widget::text::Style {
            color: Some(theme.cosmic().accent_text_color().into()),
            ..Default::default()
        })
    } else if color == desktop::WARN || color == Color::from_rgb8(245, 190, 107) {
        cosmic::theme::Text::Custom(|theme| cosmic::iced::widget::text::Style {
            color: Some(theme.cosmic().warning_text_color().into()),
            ..Default::default()
        })
    } else if color == desktop::ERROR || color == Color::from_rgb8(245, 149, 138) {
        cosmic::theme::Text::Custom(|theme| cosmic::iced::widget::text::Style {
            color: Some(theme.cosmic().destructive_text_color().into()),
            ..Default::default()
        })
    } else {
        cosmic::theme::Text::Color(color)
    }
}
fn surface_color(theme: &cosmic::Theme, color: Color) -> Color {
    if color == SURFACE {
        theme.current_container().component.base.into()
    } else if color == BASE {
        theme.cosmic().primary(theme.transparent).base.into()
    } else if color == LINE {
        theme.current_container().component.divider.into()
    } else if color == FG {
        theme.cosmic().on_bg_color().into()
    } else if color == cosmic::theme::active().cosmic().accent_text_color().into() {
        theme.cosmic().accent_text_color().into()
    } else if color == desktop::WARN {
        theme.cosmic().warning_text_color().into()
    } else if color == desktop::ERROR {
        theme.cosmic().destructive_text_color().into()
    } else {
        color
    }
}

fn label<'a>(value: impl Into<String>, size: f32, color: Color) -> Element<'a, Message> {
    text::body(value.into())
        .size(size)
        .line_height(1.25)
        .class(text_style(color))
        .into()
}
fn frame<'a>(
    content: Element<'a, Message>,
    color: Color,
    border: Color,
    padding: u16,
) -> Element<'a, Message> {
    container(column::with_children(vec![content]).width(Length::Fill))
        .width(Length::Fill)
        .padding(padding)
        .class(cosmic::theme::Container::custom(move |theme| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(surface_color(theme, color))),
                border: Border {
                    color: surface_color(theme, border),
                    width: if border == LINE && !theme.theme_type.is_high_contrast() {
                        0.
                    } else {
                        1.
                    },
                    radius: theme.cosmic().corner_radii.radius_s.into(),
                },
                ..Default::default()
            }
        }))
        .into()
}
fn detail<'a>(title: impl Into<String>, description: impl Into<String>) -> Element<'a, Message> {
    let description = description.into();
    let mut lines = vec![label(title, 14., FG)];
    if !description.is_empty() {
        lines.push(label(description, 12., MUTED));
    }
    column::with_children(lines)
        .spacing(3)
        .width(Length::Fill)
        .into()
}
fn split<'a>(left: Element<'a, Message>, right: Element<'a, Message>) -> Element<'a, Message> {
    row::with_children(vec![container(left).width(Length::Fill).into(), right])
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
}
fn switch<'a>(
    title: &'static str,
    description: &'static str,
    value: bool,
    message: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    split(
        detail(title, description),
        toggler(value).on_toggle(message).into(),
    )
}
fn heading<'a>(title: &'static str, description: &'static str) -> Element<'a, Message> {
    let mut lines: Vec<Element<'a, Message>> = vec![text::title3(title).into()];
    if !description.is_empty() {
        lines.push(label(description, 12., MUTED));
    }
    column::with_children(lines).spacing(4).into()
}
fn separator() -> Element<'static, Message> {
    container(text::body(""))
        .height(1)
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(|theme| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(surface_color(theme, LINE))),
                ..Default::default()
            }
        }))
        .into()
}
/// A titled native COSMIC list: rows separated by thin dividers, the
/// heading outside the box. Used by the grouped settings pages.
pub(super) fn group<'a>(
    title: Option<&'a str>,
    rows: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut section = cosmic::widget::settings::section();
    if let Some(title) = title {
        section = section.title(title);
    }
    section.extend(rows).into()
}
/// One labelled row of a [`group`]: title (and optional muted description)
/// left, control right.
pub(super) fn group_row<'a>(
    title: impl Into<String>,
    description: impl Into<String>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    split(detail(title, description), control)
}
fn pill<'a>(value: impl Into<String>, color: Color) -> Element<'a, Message> {
    container(label(value, 11., color))
        .padding([4, 9])
        .class(cosmic::theme::Container::custom(move |theme| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(Color {
                    a: 0.12,
                    ..surface_color(theme, color)
                })),
                border: Border {
                    radius: 20.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}
/// One-of-several choice with COSMIC's selection and focus states.
pub(super) fn segmented<'a>(
    options: Vec<(String, bool, Option<Message>)>,
    accent: Color,
) -> Element<'a, Message> {
    let buttons: Vec<Element<'a, Message>> = options
        .into_iter()
        .map(|(title, selected, message)| {
            button::custom(label(title, 12., if selected { FG } else { MUTED }))
                .class(nav_style(selected, accent))
                .selected(selected)
                .padding([6, 12])
                .on_press_maybe(message)
                .into()
        })
        .collect();
    container(row::with_children(buttons).spacing(2).wrap())
        .padding(3)
        .class(cosmic::theme::Container::custom(|theme| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(surface_color(theme, BASE))),
                border: Border {
                    radius: 12.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}
/// Standard COSMIC selection, hover and keyboard focus colors.
fn chip_style(on: bool, accent: Color) -> cosmic::theme::Button {
    nav_style(on, accent)
}
pub(super) fn nav_style(selected: bool, _accent: Color) -> cosmic::theme::Button {
    let style = move |theme: &cosmic::Theme, focused: bool, hovered: bool| {
        let component = &theme.current_container().component;
        cosmic::widget::button::Style {
            background: Some(Background::Color(if hovered {
                component.hover.into()
            } else if selected {
                component.selected_state_color().into()
            } else {
                Color::TRANSPARENT
            })),
            border_radius: theme.cosmic().corner_radii.radius_s.into(),
            outline_width: if focused { 2. } else { 0. },
            outline_color: theme.cosmic().accent.base.into(),
            ..Default::default()
        }
    };
    cosmic::theme::Button::Custom {
        active: Box::new(move |focused, theme| style(theme, focused, false)),
        hovered: Box::new(move |focused, theme| style(theme, focused, true)),
        pressed: Box::new(move |focused, theme| style(theme, focused, true)),
        disabled: Box::new(move |theme| style(theme, false, false)),
    }
}

/// Search context uses the same system accent as the rest of settings.
pub(super) fn area_tint(_area: super::navigation::Area, accent: Color) -> Color {
    accent
}
/// A small filled dot.
pub(super) fn dot<'a>(color: Color, size: f32) -> Element<'a, Message> {
    container(cosmic::iced::widget::space().width(size).height(size))
        .class(cosmic::theme::Container::custom(move |_| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: (size / 2.).into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}

impl Settings {
    /// Optional context stays next to its control, and can be opened with
    /// a pointer or keyboard. Status, errors and setup never go in here.
    fn help(
        &self,
        key: &'static str,
        title: &'static str,
        explanation: &'static str,
    ) -> Element<'_, Message> {
        let open = self.help_open == Some(key);
        let mut parts = vec![
            row::with_children(vec![
                label(title, 14., FG),
                cosmic::widget::tooltip(
                    button::custom(container(label("?", 12., MUTED)).center_x(24).center_y(24))
                        .id(cosmic::widget::Id::new(format!("help-{key}")))
                        .class(chip_style(open, self.settings_accent()))
                        .width(24)
                        .height(24)
                        .padding(0)
                        .on_press(Message::ToggleHelp(key)),
                    container(
                        text::body(if open {
                            "Hide explanation"
                        } else {
                            explanation
                        })
                        .size(12),
                    )
                    .max_width(320),
                    cosmic::widget::tooltip::Position::Top,
                )
                .into(),
            ])
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
        ];
        if open {
            parts.push(
                container(label(explanation, 12., MUTED))
                    .id(cosmic::widget::Id::new(format!("help-text-{key}")))
                    .into(),
            );
        }
        column::with_children(parts)
            .spacing(5)
            .width(Length::Fill)
            .into()
    }
    fn settings_accent(&self) -> Color {
        cosmic::theme::active().cosmic().accent_text_color().into()
    }
    pub(super) fn settings_view(&self) -> Element<'_, Message> {
        if let Some(wizard) = &self.wizard {
            return self.wizard_view(wizard);
        }
        let accent = self.settings_accent();
        let sidebar = self.sidebar(accent);
        let searching = !self.search_query.trim().is_empty();
        let page = match self.page {
            Page::Overview => self.overview_page(),
            Page::WidgetLayout => self.overview_page(),
            Page::PendingChanges => self.pending_changes_page(),
            Page::Appearance => self.appearance_page(),
            Page::Widgets => self.widgets_page(),
            Page::Glance => self.glance_page(),
            Page::Calendar => self.calendar_page(),
            Page::Weather => self.weather_page(),
            Page::Devbox => self.devbox_page(),
            Page::Clock => self.clock_page(),
            Page::Projects => self.projects_page(),
            Page::Music => self.music_page(),
            Page::Notes => self.notes_page(),
            Page::Focus => self.focus_page(),
            Page::General => self.general_page(),
            Page::Desktop => self.desktop_page(),
            Page::DesktopDock => self.desktop_dock_page(),
            // On a short viewport the sample window scrolls with the controls
            // so the basic controls stay visible.
            Page::DesktopAppearance if !self.appearance_pinned() => column::with_children(vec![
                self.system_appearance_header(),
                separator(),
                self.system_appearance_page(),
            ])
            .spacing(14)
            .into(),
            Page::DesktopAppearance => self.system_appearance_page(),
            // Like System appearance: the header scrolls with the content
            // on a short viewport.
            // Wallpaper & colors is one page: the title and large preview
            // scroll with the gallery and color controls.
            Page::DesktopWallpaper => self.wallpaper_page(),
            Page::DesktopIcons => self.desktop_icons_page(),
            Page::DesktopProfiles => self.profiles_page(),
            Page::Themes => self.themes_page(),
            // Windows and Workspaces share one page: windows first.
            Page::DesktopWindows => column::with_children(vec![
                self.windows_page(),
                container(self.workspaces_section(true))
                    .id(cosmic::widget::Id::new("windows-workspaces"))
                    .padding([12, 0, 0, 0])
                    .into(),
            ])
            .spacing(16)
            .into(),
            Page::DesktopWorkspaces => self.workspaces_page(),
            Page::DesktopMotion => self.motion_page(),
            Page::DesktopWindowRules => self.window_rules_page(),
            Page::DesktopShortcuts => self.shortcuts_page(),
            Page::DesktopAutomation => self.automation_page(),
        };
        let pinned = !searching
            && (self.page == Page::DesktopDock
                || (self.page == Page::DesktopAppearance && self.appearance_pinned()));
        // Unpinned, System appearance keeps a fixed strip above its scroll
        // area instead of the top padding: a hex field scrolled across the
        // top edge would otherwise paint into the window header bar.
        // Profiles has a name field in its scrolling content too.
        // Every other page scrolls below a fixed strip, so titles start at
        // the same height everywhere and a scrolled text field's escaped text
        // stays covered.
        let strip = !searching && !pinned;
        let mut page_parts = Vec::new();
        if let Some(hit) = self.search_selected {
            page_parts.push(pill(
                format!(
                    "Found: {}{}",
                    hit.label,
                    if hit.advanced {
                        " · Advanced is open below"
                    } else {
                        ""
                    }
                ),
                accent,
            ));
        }
        page_parts.push(
            container(page)
                .id(cosmic::widget::Id::new("page-start"))
                .width(Length::Fill)
                .into(),
        );
        let page = if searching {
            self.search_results_page()
        } else {
            column::with_children(page_parts).spacing(12).into()
        };
        let scrolling = scrollable(container(page).width(Length::Fill).padding(if pinned {
            [16, 22, 24, 22]
        } else if strip {
            [0, 22, 24, 22]
        } else {
            [12, 22, 20, 22]
        }))
        .id(cosmic::widget::Id::new("settings-content"))
        .width(Length::Fill)
        .height(Length::Fill);
        // Dock & Panel keeps its miniature and selector pinned above the
        // scrolling controls, so the preview stays in view while editing.
        // Fixed parts are drawn over the scroll area (see `layered`).
        let content: Element<'_, Message> = if pinned {
            layered::layered(
                vec![
                    layered::opaque(column::with_children(vec![
                        container(match self.page {
                            Page::DesktopDock => self.desktop_dock_header(),
                            _ => self.system_appearance_header(),
                        })
                        .width(Length::Fill)
                        .padding([14, 22, 12, 22])
                        .into(),
                        separator(),
                    ])),
                    scrolling.into(),
                ],
                1,
            )
        } else if strip {
            layered::layered(
                vec![
                    // Taller than a text field, so its escaped text is covered.
                    layered::opaque(cosmic::iced::widget::space().height(36)),
                    scrolling.into(),
                ],
                1,
            )
        } else {
            scrolling.into()
        };
        let status: Element<'_, Message> = if searching {
            self.simple_status()
        } else if self.page == Page::Overview && self.placement.selected().is_some() {
            let mut inspector = Vec::new();
            if self.error {
                inspector.push(label(&self.status, 12., Color::from_rgb8(245, 149, 138)));
            }
            inspector.push(self.placement_inspector());
            inspector.push(self.simple_status());
            container(column::with_children(inspector).spacing(6))
                .height(if self.error {
                    Length::Shrink
                } else {
                    Length::Fixed(126.)
                })
                .padding([6, 22])
                .width(Length::Fill)
                .into()
        } else if self.page == Page::DesktopWallpaper {
            self.wallpaper_actions()
        } else if matches!(self.page, Page::DesktopProfiles | Page::Themes) {
            self.profiles_actions()
        } else {
            self.simple_status()
        };
        row::with_children(vec![
            sidebar,
            layered::layered(
                vec![
                    layered::opaque(column::with_children(vec![
                        self.navigation_header(),
                        separator(),
                    ])),
                    layered::layered(
                        vec![
                            content,
                            layered::opaque(column::with_children(vec![separator(), status])),
                        ],
                        0,
                    ),
                ],
                1,
            ),
        ])
        .height(Length::Fill)
        .apply(container)
        // Opaque like the title bar (Settings::view), the fixed header and
        // the footer, so they read as one surface.
        .class(cosmic::theme::Container::custom(|theme: &cosmic::Theme| {
            let base: Color = theme.cosmic().background(theme.transparent).base.into();
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(Color { a: 1., ..base })),
                ..Default::default()
            }
        }))
        .into()
    }

    /// The four destinations at the top; App settings anchored at the bottom.
    /// Entries come from the shared nav model so selection, search and Back
    /// keep one source of truth.
    fn sidebar(&self, _accent: Color) -> Element<'_, Message> {
        let entry = |id: cosmic::widget::segmented_button::Entity| -> Element<'_, Message> {
            let active = self.nav_model.is_active(id);
            let page = self.nav_model.data::<Page>(id).copied();
            let mut parts: Vec<Element<'_, Message>> = Vec::new();
            if let Some(icon) = self.nav_model.icon(id) {
                parts.push(icon.clone().into());
            }
            parts.push(
                text::body(self.nav_model.text(id).unwrap_or_default().to_owned())
                    .class(if active {
                        cosmic::theme::Text::Custom(|theme| cosmic::iced::widget::text::Style {
                            color: Some(theme.cosmic().accent_text_color().into()),
                            ..Default::default()
                        })
                    } else {
                        cosmic::theme::Text::Default
                    })
                    .into(),
            );
            button::custom(
                row::with_children(parts)
                    .spacing(12)
                    .align_y(Alignment::Center),
            )
            .id(cosmic::widget::Id::new(format!(
                "nav-{}",
                page.map(Page::key).unwrap_or_default()
            )))
            .class(nav_style(active, self.settings_accent()))
            .selected(active)
            .padding([6, 14])
            .width(Length::Fill)
            .on_press(Message::NavSelected(id))
            .into()
        };
        // One heading per group, then its pages, in menu order.
        let mut groups = Vec::new();
        for (title, pages) in Page::SIDEBAR {
            let mut items: Vec<Element<'_, Message>> = Vec::new();
            if let Some(title) = title {
                items.push(
                    container(label(title.to_uppercase(), 11., MUTED))
                        .padding([0, 14, 2, 14])
                        .into(),
                );
            }
            items.extend(pages.iter().filter_map(|page| {
                self.nav_model
                    .iter()
                    .find(|&id| self.nav_model.data::<Page>(id) == Some(page))
                    .map(entry)
            }));
            groups.push(column::with_children(items).spacing(2).into());
        }
        let content = container(
            scrollable(
                container(column::with_children(groups).spacing(14))
                    .padding([12, 8, 12, 8])
                    .width(Length::Fill),
            )
            .height(Length::Fill),
        )
        .width(184)
        .height(Length::Fill)
        .class(cosmic::theme::Container::Primary)
        .into();
        let entries = self
            .nav_model
            .iter()
            .filter_map(|entity| {
                self.nav_model.data::<Page>(entity).map(|page| {
                    (
                        cosmic::widget::Id::new(format!("nav-{}", page.key())),
                        entity,
                    )
                })
            })
            .collect();
        sidebar::navigation(content, entries)
    }

    /// The System appearance header stays pinned only while it leaves room
    /// for the first controls below it.
    pub(super) fn appearance_pinned(&self) -> bool {
        self.window.height >= 800.
    }

    /// A fixed palette as a round accent swatch; its name is the tooltip.
    fn theme_choice(&self, key: &'static str, title: &'static str) -> Element<'_, Message> {
        let p = Palette::named(key);
        let selected = self.wallpaper.selected_palette(&self.config) == Some(key);
        let swatch = container(cosmic::iced::widget::space().width(24).height(24)).class(
            cosmic::theme::Container::custom(move |theme| cosmic::iced::widget::container::Style {
                background: Some(Background::Color(p.accent)),
                border: Border {
                    radius: 12.into(),
                    color: if selected {
                        theme.cosmic().on_bg_color().into()
                    } else {
                        Color::TRANSPARENT
                    },
                    width: if selected { 2. } else { 0. },
                },
                ..Default::default()
            }),
        );
        cosmic::widget::tooltip(
            button::custom(swatch)
                .padding(4)
                .class(nav_style(selected, p.accent))
                .selected(selected)
                .on_press_maybe((!self.busy).then(|| Message::Theme(key.into()))),
            text::body(title),
            cosmic::widget::tooltip::Position::Top,
        )
        .into()
    }

    fn legacy_widget_style(&self) -> Element<'_, Message> {
        let appearance = &self.appearance_draft;
        let p = self.config.palette();
        // Finish presets; "Rounded" so it never reads as the Soft theme.
        let presets = segmented(
            [
                ("Rounded", 92, 24, 20),
                ("Solid", 100, 16, 30),
                ("Minimal", 80, 12, 8),
            ]
            .into_iter()
            .map(|(name, opacity, radius, border)| {
                (
                    name.to_string(),
                    appearance.opacity == opacity
                        && appearance.corner_radius == radius
                        && appearance.border_opacity == border,
                    Some(Message::AppearancePreset(opacity, radius, border)),
                )
            })
            .collect(),
            p.accent,
        );
        let changed = appearance.opacity != 92
            || appearance.corner_radius != 24
            || appearance.border_opacity != 20;
        let controls = if self.config.materials.widgets().is_some() {
            pages::section(vec![
                if self
                    .config
                    .materials
                    .widgets()
                    .is_some_and(|f| f.kind == crate::materials::Kind::Liquid)
                {
                    label("Liquid uses softly rounded corners", 12., MUTED)
                } else {
                    self.style_slider(
                        "Corners",
                        format!("{} px", appearance.corner_radius),
                        0..=32,
                        appearance.corner_radius,
                        AppearanceControl::Corners,
                    )
                },
                self.style_slider(
                    "Outline",
                    format!("{}%", appearance.border_opacity),
                    0..=50,
                    appearance.border_opacity as u16,
                    AppearanceControl::Border,
                ),
            ])
        } else {
            pages::section(vec![
            split(label("Card finish", 14., FG), button::text("Reset")
                .on_press_maybe(if appearance.family.is_some() {self.config.widget_custom().then_some(Message::ResetThemeSurface(crate::desktop::profiles::Domain::WidgetStyle))} else {changed.then_some(Message::AppearancePreset(92, 24, 20))}).into()),
            presets,
            self.style_slider("Opacity",format!("{}%",appearance.opacity),70..=100,
                appearance.opacity as u16,AppearanceControl::Opacity),
            self.style_slider("Corners",format!("{} px",appearance.corner_radius),0..=32,
                appearance.corner_radius,AppearanceControl::Corners),
            self.style_slider("Outline",format!("{}%",appearance.border_opacity),0..=50,
                appearance.border_opacity as u16,AppearanceControl::Border),
            separator(),
            split(self.help("contrast-assist", "Improve readability",
                "Increases card background opacity to keep text readable over bright or busy wallpapers. Turning this off restores your chosen opacity."),
                container(toggler(appearance.contrast_assist).on_toggle(Message::ContrastAssist))
                    .id(cosmic::widget::Id::new("contrast-assist")).into()),
        ])
        };
        let preview_choice = segmented(
            [Widget::Weather, Widget::Devbox, Widget::Clock]
                .into_iter()
                .map(|widget| {
                    (
                        widget.title().to_owned(),
                        self.appearance_preview == widget,
                        Some(Message::AppearancePreview(widget)),
                    )
                })
                .collect(),
            self.settings_accent(),
        );
        column::with_children(vec![
            heading("Widget style", ""),
            if let Some(f) = appearance.family {
                pages::section(vec![
                    split(
                        detail(
                            format!("{} theme", f.title()),
                            if self.config.widget_custom() {
                                "Widgets are customized"
                            } else {
                                "Widgets match the theme"
                            },
                        ),
                        button::text("Colors & theme")
                            .trailing_icon(icon::from_name("go-next-symbolic").size(14))
                            .on_press(Message::Page(Page::Themes))
                            .into(),
                    ),
                    segmented(
                        vec![
                            (
                                "Theme surface".into(),
                                appearance.surface.is_none(),
                                Some(Message::WidgetSurface(None)),
                            ),
                            (
                                "Filled".into(),
                                appearance.surface == Some(crate::desktop_theme::Surface::Panel),
                                Some(Message::WidgetSurface(Some(
                                    crate::desktop_theme::Surface::Panel,
                                ))),
                            ),
                            (
                                "Transparent".into(),
                                appearance.surface
                                    == Some(crate::desktop_theme::Surface::Transparent),
                                Some(Message::WidgetSurface(Some(
                                    crate::desktop_theme::Surface::Transparent,
                                ))),
                            ),
                        ],
                        self.settings_accent(),
                    ),
                ])
            } else {
                container(cosmic::iced::widget::space()).into()
            },
            self.editor_with_preview_header(
                self.card_preview(),
                controls,
                "Sample data",
                Some(preview_choice),
            ),
            self.widget_text_section(),
        ])
        .spacing(16)
        .width(Length::Fill)
        .into()
    }
    fn style_slider(
        &self,
        title: &'static str,
        value_label: String,
        range: std::ops::RangeInclusive<u16>,
        value: u16,
        control: AppearanceControl,
    ) -> Element<'_, Message> {
        column::with_children(vec![
            split(
                label(title, 12., FG),
                label(value_label, 11., self.settings_accent()),
            ),
            slider(range, value, move |value| {
                Message::AppearanceValue(control.clone(), value)
            })
            .step(1u16)
            .on_release(Message::SaveAppearance)
            .width(Length::Fill)
            .apply(|control| completed_slider(control.into(), Message::SaveAppearance)),
        ])
        .spacing(7)
        .into()
    }
    fn card_preview(&self) -> Element<'static, Message> {
        let mut config = self.config.clone();
        config.appearance = self.appearance_draft;
        if let Some(finish) = self.material_draft {
            config.materials.set(self.material_target, Some(finish));
        }
        config.desktop_palette = Some(self.wallpaper.preview_palette(&self.config));
        config.appearance.color_source = crate::config::ColorSource::Desktop;
        match self.appearance_preview {
            Widget::Devbox => crate::app::settings_devbox_preview(&config),
            Widget::Clock => crate::app::settings_clock_preview(&config, [320., 112.]),
            _ => crate::app::settings_weather_preview(&config),
        }
        .map(|_| Message::Preview)
    }
}
