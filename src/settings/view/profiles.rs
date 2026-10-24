//! Presets page: four preset tiles, the local library, one desktop
//! miniature for the selection with the parts to include, and the fixed
//! footer. The miniature is illustrative; the listed changes are the plan.
//! Its backdrop is the planned wallpaper when the Wallpaper part is
//! included and can be applied (a picture the editor made off the UI
//! thread), otherwise a neutral backdrop.
use super::desktop::{ERROR, WARN, banner, line};
use super::pages::section;
use super::system_appearance::{color, fill};
use super::*;
use crate::desktop::appearance::Preview;
use crate::desktop::profiles::{Preset, ProfilePlan};
use crate::drawer::Widget;
use crate::settings::desktop::Notice;
use crate::settings::profiles::{Message as Edit, NameAction, Selection, name_id};
use cosmic::iced::Point;
use cosmic::iced::widget::{pin, stack};
use cosmic::widget::responsive;

/// Reasons shown before the rest is counted.
const SHORT: usize = 3;

fn edit(message: Edit) -> Message {
    Message::Profiles(message)
}

/// A selectable tile: a quiet outline, or the accent when selected.
pub(super) fn tile_style(selected: bool, _accent: Color) -> cosmic::theme::Button {
    let style = move |theme: &cosmic::Theme, focused: bool, hovered: bool| {
        let component = &theme.current_container().component;
        cosmic::widget::button::Style {
            background: Some(Background::Color(if hovered {
                component.hover.into()
            } else {
                component.base.into()
            })),
            border_color: if selected {
                theme.cosmic().accent.base.into()
            } else {
                component.divider.into()
            },
            border_width: if selected { 2. } else { 1. },
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

/// The bundled wallpaper a starter look shows, by palette name.
fn bundled_name(profile: &crate::desktop::profiles::Profile) -> Option<&str> {
    match profile
        .wallpaper
        .as_ref()?
        .background
        .as_ref()?
        .source
        .as_ref()?
    {
        crate::desktop::profiles::SourceLook::Bundled(name) => Some(name),
        _ => None,
    }
}
/// A starter look's rendered preview (`assets/looks`, made by the look
/// render tool): its wallpaper, panel, dock, a window and its widgets.
pub(super) fn look_preview(id: &str) -> Option<cosmic::widget::image::Handle> {
    look_image(id, false)
}
/// Asset presence is cheap: testing whether a preview exists must not decode
/// an image that the stock-bar miniature will never display.
fn look_bytes(id: &str, wallpaper: bool) -> Option<&'static [u8]> {
    macro_rules! looks {
        ($($id:literal),*) => {
            match (id, wallpaper) {
                $(
                    ($id, false) => Some(&include_bytes!(concat!("../../../assets/looks/", $id, ".jpg"))[..]),
                    ($id, true) => Some(&include_bytes!(concat!("../../../assets/looks/", $id, "-wallpaper.jpg"))[..]),
                )*
                _ => None,
            }
        };
    }
    looks!("halcyon", "calm", "evening", "focus", "paper", "rose")
}
/// Decoded once per look and kind.
pub(super) fn look_rgba(id: &str, wallpaper: bool) -> Option<std::sync::Arc<image::RgbaImage>> {
    use std::sync::{Arc, Mutex};
    static DECODED: Mutex<Vec<((String, bool), Option<Arc<image::RgbaImage>>)>> =
        Mutex::new(Vec::new());
    let mut decoded = DECODED.lock().ok()?;
    if let Some((_, image)) = decoded
        .iter()
        .find(|(key, _)| key.0 == id && key.1 == wallpaper)
    {
        return image.clone();
    }
    let bytes = look_bytes(id, wallpaper);
    let image = bytes
        .and_then(|bytes| image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg).ok())
        .map(|image| Arc::new(image.to_rgba8()));
    decoded.push(((id.to_string(), wallpaper), image.clone()));
    image
}
pub(super) fn look_image(id: &str, wallpaper: bool) -> Option<cosmic::widget::image::Handle> {
    use cosmic::widget::image::Handle;
    static HANDLES: std::sync::Mutex<Vec<((String, bool), Option<Handle>)>> =
        std::sync::Mutex::new(Vec::new());
    let mut handles = HANDLES.lock().ok()?;
    if let Some((_, handle)) = handles
        .iter()
        .find(|(key, _)| key.0 == id && key.1 == wallpaper)
    {
        return handle.clone();
    }
    let handle = look_rgba(id, wallpaper)
        .map(|image| Handle::from_rgba(image.width(), image.height(), image.as_raw().clone()));
    handles.push(((id.to_string(), wallpaper), handle.clone()));
    handle
}
/// The compiled-in thumbnail of a bundled wallpaper, decoded once.
fn bundled_handle(name: &str) -> Option<cosmic::widget::image::Handle> {
    use cosmic::widget::image::Handle;
    static DECODED: std::sync::Mutex<Vec<(String, Option<Handle>)>> =
        std::sync::Mutex::new(Vec::new());
    let mut decoded = DECODED.lock().ok()?;
    if let Some((_, handle)) = decoded.iter().find(|(known, _)| known == name) {
        return handle.clone();
    }
    let handle = crate::desktop::wallpaper::media::bundled_thumbnail(name)
        .map(|pixels| Handle::from_rgba(pixels.width, pixels.height, pixels.rgba));
    decoded.push((name.to_string(), handle.clone()));
    handle
}
fn swatch<'a>(color: Color) -> Element<'a, Message> {
    container(cosmic::iced::widget::space().width(12).height(12))
        .class(cosmic::theme::Container::custom(move |_| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 6.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}

/// Pure preset projection for the chooser. Sparse records retain unmentioned settings.
fn preset_config(config: &Config, preset: &Preset) -> Config {
    fn merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
        if let (Some(base), Some(patch)) = (base.as_object_mut(), patch.as_object()) {
            for (key, value) in patch {
                merge(
                    base.entry(key.clone()).or_insert(serde_json::Value::Null),
                    value,
                );
            }
        } else {
            *base = patch.clone();
        }
    }
    let mut value = serde_json::to_value(config).expect("config serializes");
    for patch in [&preset.profile.widget_style, &preset.profile.widget_layout]
        .into_iter()
        .flatten()
    {
        merge(&mut value, patch);
    }
    let mut result: Config =
        serde_json::from_value(value).expect("built-in preset config is valid");
    result.desktop_palette = config.desktop_palette;
    result
}

/// One reason per line (they are sentences), the first few and a count of
/// the rest, which the details list in full.
fn reasons(lines: &[String]) -> String {
    let mut text = lines[..lines.len().min(SHORT)].join("\n");
    if lines.len() > SHORT {
        text.push_str(&format!("\n+{} more in the details", lines.len() - SHORT));
    }
    text
}

/// One widget card on the miniature: the card palette and style, a sample
/// title and placeholder lines; never the user's own data.
pub(in crate::settings) fn card<'a>(
    widget: Widget,
    size: [f32; 2],
    config: &Config,
    scale: f32,
) -> Element<'a, Message> {
    // At a glance can keep its own background under a widget style.
    let transparent = if widget == Widget::Glance {
        !config.glance_panel()
    } else {
        widget == Widget::Clock || config.transparent_widgets()
    };
    let palette = if transparent {
        config.wallpaper_palette()
    } else {
        config.palette()
    };
    let style = &config.appearance;
    // The style's border width and shadow, scaled to the miniature.
    let edge = {
        let _style = crate::widget_style::scope(config);
        crate::widget_style::edge(Palette {
            background: if transparent {
                Color::TRANSPARENT
            } else {
                palette.background
            },
            ..palette
        })
    };
    let background = if transparent {
        Color::TRANSPARENT
    } else if widget == Widget::Glance && config.appearance.family.is_none() {
        Color {
            a: if style.contrast_assist {
                palette
                    .background
                    .a
                    .max(config.glance.opacity as f32 / 100.)
            } else {
                config.glance.opacity as f32 / 100.
            },
            ..palette.background
        }
    } else {
        palette.background
    };
    let border = Color {
        a: if transparent {
            0.
        } else {
            f32::from(style.border_opacity.min(50)) / 100.
        },
        ..palette.accent
    };
    let radius = (f32::from(style.corner_radius) * scale)
        .max(0.)
        .min((size[1] / 3.).max(0.));
    let (title, detail) = match widget {
        Widget::Clock => ("09:41", "Monday, 19 October"),
        Widget::Glance if config.glance.show_clock => (
            "09:41",
            if config.glance.show_weather {
                "21° · Partly cloudy"
            } else if config.glance.show_calendar {
                "10:00 Design review"
            } else if config.clock.show_date {
                "Monday, 19 October"
            } else {
                ""
            },
        ),
        Widget::Glance if config.glance.show_weather => ("21°", "Partly cloudy"),
        Widget::Glance => ("Today", "10:00 Design review"),
        Widget::Weather => ("21°", "Partly cloudy"),
        Widget::Calendar => ("Today", "10:00  Design review"),
        Widget::Devbox => ("CPU 26%", "RAM 8.2 / 32 GiB"),
        Widget::Projects => ("Atlas", "main · 2 changes"),
        Widget::Music => ("Now playing", "Artist · Album"),
        Widget::Focus => ("25:00", "Focus"),
        Widget::Notes => ("Notes", "Plan the next release"),
    };
    let main_size = if matches!(widget, Widget::Clock | Widget::Glance | Widget::Weather) {
        (52. * scale).clamp(4., 18.)
    } else {
        (34. * scale).clamp(4., 10.)
    };
    let secondary_size = (26. * scale).clamp(3.8, 8.);
    let pad = (18. * scale).clamp(1., 5.);
    let font = config
        .theme_roles()
        .map_or(cosmic::iced::Font::DEFAULT, |r| {
            if widget == Widget::Clock || widget == Widget::Glance {
                r.clock
            } else {
                r.body
            }
        });
    let mut content = vec![
        text::body(title)
            .size(main_size)
            .font(font)
            .class(cosmic::theme::Text::Color(palette.foreground))
            .into(),
    ];
    if size[1] >= main_size * 1.25 + secondary_size * 1.25 + pad * 2. + 2. {
        content.push(line(detail, secondary_size, palette.muted));
    }
    container(column::with_children(content).spacing((6. * scale).max(1.)))
        .width(size[0].max(1.))
        .height(size[1].max(1.))
        .padding(pad)
        .clip(true)
        .class(cosmic::theme::Container::custom(move |_| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    color: border,
                    width: if transparent {
                        1.
                    } else {
                        (edge.0 * scale * 2.).max(1.)
                    },
                    radius: radius.into(),
                },
                shadow: cosmic::iced::Shadow {
                    blur_radius: edge.1.blur_radius * scale,
                    offset: edge.1.offset * scale,
                    ..edge.1
                },
                ..Default::default()
            }
        }))
        .into()
}

/// The selected theme as a small window without words: its surfaces,
/// accent, corners, outline and title dots, with an accent pill and quiet
/// lines where the text would be. Sized for the miniature.
pub(super) fn schematic_window<'a>(preview: Preview, size: [f32; 2]) -> Element<'a, Message> {
    let [width, height] = size;
    let pad = (height / 10.).clamp(4., 8.);
    let gap = (height / 14.).clamp(3., 6.);
    let inset = (height / 16.).clamp(3., 6.);
    let dot = (height / 14.).clamp(4., 8.);
    let bar = (height / 26.).clamp(2., 4.);
    let pill = (height / 9.).clamp(5., 10.);
    let radius_s = preview.radius_s.min(pill / 2.);
    let faint = |tone, alpha: f32| {
        let tone = color(tone);
        Color {
            a: tone.a * alpha,
            ..tone
        }
    };
    let block = |width: f32, height: f32, tone: Color, radius: f32| -> Element<'a, Message> {
        fill(
            cosmic::iced::widget::space()
                .width(width.max(1.))
                .height(height)
                .into(),
            tone,
            Color::TRANSPARENT,
            0.,
            radius,
        )
        .into()
    };
    let title_bar = row::with_children(vec![
        block(
            width * 0.22,
            bar,
            faint(preview.on_background, 0.45),
            bar / 2.,
        ),
        cosmic::iced::widget::space().width(Length::Fill).into(),
        block(dot, dot, faint(preview.on_background, 0.5), dot / 2.),
        block(dot, dot, faint(preview.on_background, 0.5), dot / 2.),
        block(dot, dot, color(preview.accent), dot / 2.),
    ])
    .spacing(dot * 0.6)
    .align_y(Alignment::Center);
    let inner = (width - 2. * pad - gap).max(2.);
    let side = inner * 0.4;
    let main = inner - side;
    let line_in = |panel: f32, fraction: f32| (panel - 2. * inset) * fraction;
    let sidebar = fill(
        column::with_children(vec![
            block(line_in(side, 1.), pill, color(preview.accent), radius_s),
            block(
                line_in(side, 0.65),
                bar,
                faint(preview.on_primary, 0.4),
                bar / 2.,
            ),
        ])
        .spacing(gap)
        .into(),
        color(preview.primary),
        Color::TRANSPARENT,
        0.,
        radius_s,
    )
    .padding(inset)
    .width(side)
    .height(Length::Fill);
    let card = fill(
        column::with_children(vec![
            block(
                line_in(main, 0.8),
                bar,
                faint(preview.on_secondary, 0.6),
                bar / 2.,
            ),
            block(
                line_in(main, 0.55),
                bar,
                faint(preview.on_secondary, 0.35),
                bar / 2.,
            ),
        ])
        .spacing(gap)
        .into(),
        color(preview.secondary),
        Color::TRANSPARENT,
        0.,
        radius_s,
    )
    .padding(inset)
    .width(main)
    .height(Length::Fill);
    fill(
        column::with_children(vec![
            title_bar.into(),
            row::with_children(vec![sidebar.into(), card.into()])
                .spacing(gap)
                .height(Length::Fill)
                .into(),
        ])
        .spacing(gap)
        .into(),
        color(preview.background),
        color(preview.outline),
        (preview.outline_width as f32).clamp(0., 3.),
        preview.radius_m.min(height / 6.),
    )
    .padding(pad)
    .width(width)
    .height(height)
    .clip(true)
    .into()
}

impl Settings {
    /// Miniature height: compact at 820×600 so the tiles and parts stay
    /// close, roomier on taller windows.
    pub(super) fn profile_preview_height(&self) -> f32 {
        (self.window.height * 0.3).clamp(150., 230.)
    }
    /// Fit the tiles to the actual content width, including the sidebar,
    /// padding, scrollbar allowance and gaps: four styles on Colors & theme,
    /// three starter looks on Setups.
    fn tile_width(&self) -> f32 {
        if self.page == Page::Themes {
            ((self.window.width - 184. - 44. - 14. - 24.) / 4.)
                .floor()
                .clamp(132., 240.)
        } else {
            ((self.window.width - 184. - 44. - 14. - 16.) / 3.)
                .floor()
                .clamp(150., 320.)
        }
    }

    /// The selection over the current desktop: the planned wallpaper (or a
    /// neutral backdrop), a sample window in the theme, the visible cards
    /// where they sit, and both bars.
    pub(super) fn profile_preview(&self, height: f32) -> Element<'_, Message> {
        let editor = &self.profiles;
        let Some(preview) = editor.preview() else {
            return label("Reading your desktop…", 12., MUTED);
        };
        let config = preview
            .config
            .clone()
            .unwrap_or_else(|| self.config.clone());
        let accent = config.palette().accent;
        let mut bars = preview.bars.clone();
        if let Some(appearance) = &preview.appearance {
            for bar in &mut bars {
                use crate::desktop::panels::Background as BarBackground;
                let variant = match bar.background().known() {
                    Some(BarBackground::Light) => crate::desktop::appearance::ThemeVariant::Light,
                    Some(BarBackground::Dark) => crate::desktop::appearance::ThemeVariant::Dark,
                    Some(BarBackground::ThemeDefault) => appearance.active_variant(),
                    _ => continue,
                };
                let c = appearance.preview(variant).background;
                bar.set_background(BarBackground::Color([c.red, c.green, c.blue]));
            }
        }
        let layout = crate::layout_state::read(&config);
        let screen_size = layout.screen;
        // An active COSMIC window shows the planned window colors and its
        // outline, which a theme now coordinates too.
        let show_window = true;
        responsive(move |available| {
            let scale = (available.width.max(1.) / screen_size[0].max(1.))
                .min(available.height.max(1.) / screen_size[1].max(1.));
            let screen = [screen_size[0] * scale, screen_size[1] * scale];
            let wallpaper = editor
                .backdrop_image()
                .or_else(|| {
                    // A theme must wait for its analyzed image/version. Home
                    // may still hold an older picture at the very same path.
                    if editor.profile().is_some_and(|p| p.appearance_only) {
                        None
                    } else {
                        editor.home_image()
                    }
                })
                .map(|handle| {
                    cosmic::widget::image(handle.clone())
                        .width(screen[0])
                        .height(screen[1])
                        .content_fit(cosmic::iced::ContentFit::Cover)
                        .border_radius(9.)
                        .into()
                });
            let backdrop = container(text::body(""))
                .width(screen[0])
                .height(screen[1])
                .class(cosmic::theme::Container::custom(|_| {
                    cosmic::iced::widget::container::Style {
                        background: Some(Background::Gradient(
                            cosmic::iced::gradient::Linear::new(2.4)
                                .add_stop(0., Color::from_rgb(0.20, 0.21, 0.24))
                                .add_stop(1., Color::from_rgb(0.12, 0.13, 0.15))
                                .into(),
                        )),
                        border: Border {
                            color: LINE,
                            width: 1.,
                            radius: 9.into(),
                        },
                        ..Default::default()
                    }
                }));
            let mut layers: Vec<Element<'_, Message>> =
                vec![wallpaper.unwrap_or_else(|| backdrop.into())];
            for geometry in &layout.widgets {
                let Some(widget) = Widget::parse(&geometry.key) else {
                    continue;
                };
                if !config.widget_visible(widget) {
                    continue;
                }
                let size = [geometry.size[0] * scale, geometry.size[1] * scale];
                layers.push(
                    pin(card(widget, size, &config, scale))
                        .position(Point::new(
                            geometry.position[0] * scale,
                            geometry.position[1] * scale,
                        ))
                        .into(),
                );
            }
            // Windows sit above the cards, as on the desktop; centered,
            // where cards rarely are.
            if let Some(draft) = preview.appearance.as_ref().filter(|_| show_window) {
                let size = [screen[0] * 0.32, (screen[1] * 0.42).max(64.)];
                layers.push(
                    pin(schematic_window(
                        draft.preview(draft.active_variant()),
                        size,
                    ))
                    .position(Point::new(
                        (screen[0] - size[0]) / 2.,
                        (screen[1] - size[1]) * 0.42,
                    ))
                    .into(),
                );
            }
            layers.extend(super::desktop::preview::bars(
                self.desktop.islands.installation.installed(),
                bars.iter().collect(),
                &editor.installed.applets,
                None,
                screen,
                scale,
                accent,
                None,
            ));
            container(stack(layers).width(screen[0]).height(screen[1]).clip(true))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        })
        .width(Length::Fill)
        .height(height)
        .into()
    }

    /// A starter look's or style's accent, as its tile shows it.
    pub(super) fn preset_color(&self, preset: &Preset) -> Color {
        self.tile_config(preset).palette().accent
    }
    fn tile_config(&self, preset: &Preset) -> Config {
        let mut config = preset_config(&self.config, preset);
        // Family specimens stay legible while the actual selection's backdrop
        // is loading or changing. The larger preview uses the planned desktop.
        if let Some(family) = crate::desktop::profiles::themes::family(&preset.profile) {
            config.appearance.wallpaper_light = family == crate::desktop_theme::Family::Paper;
        }
        config
    }

    /// A miniature desktop: the look's wallpaper (or style colors) with its
    /// widget cards where the layout puts them.
    pub(crate) fn preset_thumbnail<'a>(
        &'a self,
        preset: &'a Preset,
        height: f32,
    ) -> Element<'a, Message> {
        // A starter look shows its rendered desktop, filling the tile.
        let standard_bars = look_bytes(preset.id, false).is_some()
            && !self.desktop.islands.installation.installed();
        if !standard_bars && let Some(handle) = look_preview(preset.id) {
            return container(
                cosmic::widget::image(handle)
                    .width((height * 16. / 9.).floor())
                    .height(height)
                    .content_fit(cosmic::iced::ContentFit::Fill)
                    .border_radius(6.),
            )
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .into();
        }
        let config = self.tile_config(preset);
        let family = crate::desktop::profiles::themes::family(&preset.profile);
        let geometry =
            crate::layout_state::project(self.placement.preview_snapshot().clone(), &config);
        // A starter look shows its own wallpaper; a style shows its colors.
        let wallpaper = bundled_name(&preset.profile)
            .and_then(bundled_handle)
            .or_else(|| {
                family
                    .is_none()
                    .then(|| self.profiles.home_image().cloned())
                    .flatten()
            });
        responsive(move |bounds| {
            let scale = (bounds.width / geometry.screen[0]).min(bounds.height / geometry.screen[1]);
            let screen = [geometry.screen[0] * scale, geometry.screen[1] * scale];
            let background: Element<'_, Message> = match &wallpaper {
                Some(handle) => cosmic::widget::image(handle.clone())
                    .width(screen[0])
                    .height(screen[1])
                    .content_fit(cosmic::iced::ContentFit::Cover)
                    .border_radius(6.)
                    .into(),
                None if family.is_some() => {
                    let (a, b) = family.unwrap().wallpaper();
                    container(cosmic::iced::widget::space())
                        .class(cosmic::theme::Container::custom(move |_| {
                            cosmic::iced::widget::container::Style {
                                background: Some(Background::Gradient(
                                    cosmic::iced::gradient::Linear::new(2.4)
                                        .add_stop(0., Color::from_rgb8(a[0], a[1], a[2]))
                                        .add_stop(1., Color::from_rgb8(b[0], b[1], b[2]))
                                        .into(),
                                )),
                                border: Border {
                                    radius: 6.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        }))
                        .width(screen[0])
                        .height(screen[1])
                        .into()
                }
                None => fill(
                    cosmic::iced::widget::space().into(),
                    Color::from_rgb(0.12, 0.13, 0.15),
                    Color::TRANSPARENT,
                    0.,
                    6.,
                )
                .width(screen[0])
                .height(screen[1])
                .into(),
            };
            let mut layers = vec![background];
            if standard_bars {
                let bar_height = (screen[1] * 0.055).max(8.);
                layers.push(
                    pin(container(fill(
                        cosmic::widget::space().into(),
                        Color::from_rgba(0.08, 0.09, 0.11, 0.92),
                        Color::TRANSPARENT,
                        0.,
                        0.,
                    ))
                    .id(cosmic::widget::Id::new("starter-standard-panel"))
                    .width(screen[0])
                    .height(bar_height))
                    .position(Point::ORIGIN)
                    .into(),
                );
                layers.push(
                    pin(fill(
                        cosmic::widget::space().into(),
                        Color::from_rgba(0.08, 0.09, 0.11, 0.92),
                        Color::TRANSPARENT,
                        0.,
                        3.,
                    )
                    .width(screen[0] * 0.3)
                    .height(bar_height))
                    .position(Point::new(screen[0] * 0.35, screen[1] - bar_height - 3.))
                    .into(),
                );
            }

            for geometry in &geometry.widgets {
                if let Some(widget) =
                    Widget::parse(&geometry.key).filter(|widget| config.widget_visible(*widget))
                {
                    layers.push(
                        pin(card(
                            widget,
                            [geometry.size[0] * scale, geometry.size[1] * scale],
                            &config,
                            scale,
                        ))
                        .position(Point::new(
                            geometry.position[0] * scale,
                            geometry.position[1] * scale,
                        ))
                        .into(),
                    );
                }
            }
            container(stack(layers).width(screen[0]).height(screen[1]).clip(true))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        })
        .height(height)
        .into()
    }

    pub(super) fn preset_tile<'a>(&'a self, preset: &'a Preset) -> Element<'a, Message> {
        let selected = *self.profiles.selection() == Selection::Preset(preset.id);
        let color = self.preset_color(preset);
        let thumbnail = self.preset_thumbnail(
            preset,
            if self.page == Page::Themes {
                76.
            } else {
                ((self.tile_width() - 16.) * 9. / 16.).round()
            },
        );
        let mut parts = vec![
            thumbnail.into(),
            row::with_children(vec![
                container(line(preset.profile.name.clone(), 13., FG))
                    .width(Length::Fill)
                    .into(),
                label(if selected { "✓" } else { "" }, 12., self.settings_accent()),
                swatch(color),
            ])
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
        ];
        if self.page != Page::Themes {
            parts.push(label(preset.description, 11., MUTED));
        }
        button::custom(column::with_children(parts).spacing(6))
            .id(cosmic::widget::Id::new(format!("preset-{}", preset.id)))
            .class(tile_style(selected, self.settings_accent()))
            .selected(selected)
            .padding(8)
            .width(self.tile_width())
            .on_press(edit(Edit::Select(Selection::Preset(preset.id))))
            .into()
    }

    pub(super) fn name_field(&self, save: &'static str) -> Element<'_, Message> {
        let text = self
            .profiles
            .name
            .as_ref()
            .map(|name| name.text.clone())
            .unwrap_or_default();
        row::with_children(vec![
            text_input(
                if self.page == Page::Themes {
                    "Theme name"
                } else {
                    "Setup name"
                },
                text,
            )
            .id(name_id())
            .on_input(|value| edit(Edit::NameInput(value)))
            .on_submit(|_| edit(Edit::NameSubmit))
            .width(Length::Fill)
            .into(),
            button::text("Cancel")
                .on_press(edit(Edit::Name(None)))
                .into(),
            button::suggested(save)
                .on_press(edit(Edit::NameSubmit))
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    pub(super) fn library_section(&self) -> Element<'_, Message> {
        let editor = &self.profiles;
        let accent = self.settings_accent();
        let writable = editor.writable();
        let mut items: Vec<Element<'_, Message>> = vec![split(
            label(
                if self.page == Page::Themes {
                    "Your themes"
                } else {
                    "Your setups"
                },
                15.,
                FG,
            ),
            row::with_children(vec![
                button::text("Import…")
                    .leading_icon(icon::from_name("document-open-symbolic").size(14))
                    .on_press_maybe((writable && !editor.busy()).then_some(edit(
                        if self.page == Page::Themes {
                            Edit::ImportTheme
                        } else {
                            Edit::Import
                        },
                    )))
                    .into(),
                button::text("Save current…")
                    .leading_icon(icon::from_name("list-add-symbolic").size(14))
                    .on_press_maybe(writable.then_some(edit(Edit::Name(Some(
                        if self.page == Page::Themes {
                            NameAction::CaptureTheme
                        } else {
                            NameAction::Capture
                        },
                    )))))
                    .into(),
            ])
            .spacing(6)
            .into(),
        )];
        if editor.name.as_ref().is_some_and(|name| {
            matches!(name.action, NameAction::Capture | NameAction::CaptureTheme)
        }) {
            items.push(self.name_field("Save"));
        }
        match editor.library() {
            Err(error) => items.push(banner(
                format!("Saved setups can’t be read. {error}"),
                ERROR,
                Vec::new(),
            )),
            Ok([]) => items.push(label(
                "Nothing saved yet. Save your current desktop to come back to it.",
                12.,
                MUTED,
            )),
            Ok(entries) => {
                for entry in entries {
                    if entry
                        .profile
                        .as_ref()
                        .is_ok_and(|p| p.appearance_only != (self.page == Page::Themes))
                    {
                        continue;
                    }
                    let selected = *editor.selection() == Selection::Saved(entry.id.clone());
                    let (name, tone) = match &entry.profile {
                        Ok(profile) => (profile.name.clone(), if selected { FG } else { MUTED }),
                        Err(_) => ("Unreadable setup".to_string(), WARN),
                    };
                    let mut parts =
                        vec![container(line(name, 13., tone)).width(Length::Fill).into()];
                    if entry.profile.is_err() {
                        parts.push(label(entry.id.as_str().to_string(), 10., MUTED));
                    }
                    items.push(
                        button::custom(
                            row::with_children(parts)
                                .spacing(10)
                                .align_y(Alignment::Center),
                        )
                        .class(nav_style(selected, accent))
                        .selected(selected)
                        .padding([7, 10])
                        .width(Length::Fill)
                        .on_press(edit(Edit::Select(Selection::Saved(entry.id.clone()))))
                        .into(),
                    );
                }
            }
        }
        section(items)
    }

    /// The selection: miniature, name and actions, parts to include, what
    /// blocks it, missing components and (on demand) every change.
    pub(super) fn selection_section<'a>(&'a self) -> Element<'a, Message> {
        let editor = &self.profiles;
        let accent = self.settings_accent();
        let height = self.profile_preview_height();
        let selection = editor.selection();
        // The miniature with the name and actions beside it.
        let beside = |info: Vec<Element<'a, Message>>| -> Element<'a, Message> {
            row::with_children(vec![
                container(self.profile_preview(height))
                    .width(Length::FillPortion(3))
                    .into(),
                column::with_children(info)
                    .spacing(10)
                    .width(Length::FillPortion(2))
                    .into(),
            ])
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
        };
        let mut items: Vec<Element<'_, Message>> = Vec::new();
        if let Some(error) = editor.load_error() {
            items.push(beside(vec![label("Unreadable setup", 18., WARN)]));
            items.push(banner(
                format!("This setup can’t be read: {error}"),
                ERROR,
                vec![
                    button::text("Delete")
                        .on_press_maybe(editor.writable().then_some(edit(Edit::Delete(false))))
                        .into(),
                ],
            ));
            if editor.confirm_delete {
                items.push(self.delete_confirm("this unreadable setup".into()));
            }
            return section(items);
        }
        let Some(profile) = editor.profile() else {
            items.push(beside(vec![detail(
                "Your current desktop",
                "Choose a look to see it here first.",
            )]));
            return section(items);
        };
        let saved = matches!(selection, Selection::Saved(_));
        let subtitle = editor.description().unwrap_or(if saved {
            "Saved on this computer"
        } else {
            "Built-in"
        });
        let title: Element<'_, Message> = column::with_children(vec![
            label(profile.name.clone(), 18., FG),
            label(subtitle, 12., MUTED),
        ])
        .spacing(3)
        .into();
        let writable = editor.writable();
        let mut actions = vec![
            button::text("Duplicate")
                .leading_icon(icon::from_name("edit-copy-symbolic").size(14))
                .on_press_maybe(writable.then_some(edit(Edit::Name(Some(NameAction::Duplicate)))))
                .into(),
        ];
        if let Selection::Saved(id) = selection {
            actions.push(
                button::text("Rename")
                    .on_press_maybe(
                        writable.then(|| edit(Edit::Name(Some(NameAction::Rename(id.clone()))))),
                    )
                    .into(),
            );
        }
        actions.push(
            button::text("Export…")
                .leading_icon(icon::from_name("document-save-as-symbolic").size(14))
                .on_press_maybe((writable && !editor.busy()).then_some(edit(Edit::Export)))
                .into(),
        );
        if saved && !matches!(selection, Selection::Saved(id) if id.as_str() == "before-themes") {
            actions.push(
                button::text("Delete…")
                    .leading_icon(icon::from_name("user-trash-symbolic").size(14))
                    .on_press_maybe(writable.then_some(edit(Edit::Delete(false))))
                    .into(),
            );
        }
        let mut info = vec![title];
        if let Selection::Preset(id) = selection
            && let Some(hint) = crate::desktop::profiles::preset(id).and_then(|preset| {
                crate::desktop::profiles::missing(&preset, &editor.bundled_dir(), editor.islands())
            })
        {
            info.push(label(hint, 12., WARN));
        }
        if !profile.appearance_only {
            info.push(
                button::suggested("Use this look")
                    .id(cosmic::widget::Id::new("setup-use"))
                    .on_press_maybe(editor.can_apply().then_some(edit(Edit::Apply)))
                    .into(),
            );
        }
        info.push(row::with_children(actions).spacing(4).wrap().into());
        items.push(beside(info));
        if editor.name.as_ref().is_some_and(|name| {
            !matches!(name.action, NameAction::Capture | NameAction::CaptureTheme)
        }) {
            items.push(self.name_field("Save"));
        }
        if editor.confirm_delete {
            items.push(self.delete_confirm(format!("“{}”", profile.name)));
        }
        items.push(separator());
        if !profile.appearance_only {
            items.push(self.parts(profile, accent));
        }
        items.extend(self.plan_states());
        section(items)
    }

    pub(super) fn delete_confirm(&self, name: String) -> Element<'_, Message> {
        let rules = match self.profiles.rules_using {
            0 => String::new(),
            1 => " An automation rule uses it and will stop working.".into(),
            count => format!(" {count} automation rules use it and will stop working."),
        };
        banner(
            format!(
                "Delete {name} from your {}?{rules}",
                if self.page == Page::Themes {
                    "themes"
                } else {
                    "presets"
                }
            ),
            WARN,
            vec![
                button::text("Cancel")
                    .on_press(edit(Edit::Delete(false)))
                    .into(),
                button::destructive("Delete")
                    .on_press(edit(Edit::Delete(true)))
                    .into(),
            ],
        )
    }

    /// One chip per part the profile has; the count is what would change.
    fn parts<'a>(
        &'a self,
        profile: &'a crate::desktop::profiles::Profile,
        accent: Color,
    ) -> Element<'a, Message> {
        let plan = self.profiles.plan();
        let chips: Vec<Element<'_, Message>> = profile
            .domains()
            .into_iter()
            .map(|domain| {
                let on = self.profiles.included(domain);
                let part = plan.and_then(|plan| plan.domains.iter().find(|p| p.domain == domain));
                let text = match part {
                    Some(part) if !part.blocked.is_empty() => {
                        format!("{} · blocked", domain.title())
                    }
                    Some(part) => format!("{} · {}", domain.title(), part.changes.len()),
                    None => domain.title().to_string(),
                };
                let mut chip = row::with_children(vec![
                    icon::from_name(if on {
                        "checkbox-checked-symbolic"
                    } else {
                        "checkbox-symbolic"
                    })
                    .size(14)
                    .icon()
                    .into(),
                    label(text, 12., if on { FG } else { MUTED }),
                ])
                .spacing(6)
                .align_y(Alignment::Center);
                if part.is_some_and(|part| !part.warnings.is_empty()) {
                    chip = chip.push(label("!", 12., WARN));
                }
                button::custom(chip)
                    .class(chip_style(on, accent))
                    .padding([5, 11])
                    .on_press(edit(Edit::Include(domain, !on)))
                    .into()
            })
            .collect();
        let mut items = vec![
            label("Include", 12., MUTED),
            row::with_children(chips).spacing(6).wrap().into(),
        ];
        if profile.local_member().is_some() {
            items.push(label(
                "Your own pictures and display arrangement stay on this computer: Export leaves them out.",
                11.,
                MUTED,
            ));
        }
        column::with_children(items).spacing(8).into()
    }

    /// Blockers, overlapping drafts, a stale preview and missing components
    /// stay visible; every change is listed on demand.
    pub(super) fn plan_states(&self) -> Vec<Element<'_, Message>> {
        let editor = &self.profiles;
        let mut items = Vec::new();
        let Some(plan) = editor.plan() else {
            return Vec::new();
        };
        if editor.stale() {
            items.push(banner(
                "Review the current changes before applying.",
                WARN,
                vec![
                    button::standard("Review changes")
                        .on_press(edit(Edit::Replan))
                        .into(),
                ],
            ));
        }
        let blocked = plan.blocking_reasons();
        if !blocked.is_empty() {
            // A part that can't be applied can be left out to apply the rest.
            let others = plan.domains.iter().any(|part| part.blocked.is_empty());
            let actions = plan
                .domains
                .iter()
                .filter(|part| !part.blocked.is_empty() && others)
                .map(|part| {
                    button::standard(format!("Leave out {}", part.domain.title()))
                        .on_press(edit(Edit::Include(part.domain, false)))
                        .into()
                })
                .collect();
            items.push(banner(reasons(&blocked), WARN, actions));
        }
        let warnings: Vec<String> = plan
            .domains
            .iter()
            .flat_map(|part| part.warnings.iter().cloned())
            .collect();
        // One line for missing components; the rest is in details.
        if let Some(first) = warnings.first() {
            let mut parts = vec![
                container(line(first.clone(), 12., WARN))
                    .width(Length::Fill)
                    .into(),
            ];
            if warnings.len() > 1 {
                parts.push(label(format!("+{} more", warnings.len() - 1), 12., MUTED));
            }
            items.push(
                row::with_children(parts)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }
        if blocked.is_empty() && plan.is_empty() {
            items.push(label("Already matches your desktop.", 12., MUTED));
        }
        if plan.domains.iter().any(|part| {
            !part.changes.is_empty() || !part.warnings.is_empty() || !part.blocked.is_empty()
        }) {
            items.push(
                button::text(if editor.details {
                    "Hide details"
                } else {
                    "Show details"
                })
                .trailing_icon(
                    icon::from_name(if editor.details {
                        "go-up-symbolic"
                    } else {
                        "go-down-symbolic"
                    })
                    .size(14),
                )
                .on_press(edit(Edit::Details(!editor.details)))
                .into(),
            );
        }
        if editor.details {
            items.push(plan_details(plan));
        }
        items
    }

    pub(super) fn profiles_page(&self) -> Element<'_, Message> {
        let editor = &self.profiles;
        let accent = self.settings_accent();
        let mut items: Vec<Element<'_, Message>> = vec![
            split(
                heading("Setups", ""),
                super::windows::store_pill(editor.store(), accent),
            ),
            label(
                "A whole look in one click: wallpaper, colors, widgets and dock. Undo puts everything back.",
                13.,
                MUTED,
            ),
        ];
        if let Some(error) = editor.unavailable() {
            items.push(banner(
                format!("Desktop settings are unavailable. {error}"),
                ERROR,
                Vec::new(),
            ));
            return column::with_children(items).spacing(16).into();
        }
        match editor.recovery() {
            Some(Ok(_)) => items.push(banner(
                "An earlier change was interrupted.",
                WARN,
                vec![
                    button::text("Keep current")
                        .on_press_maybe(editor.writable().then_some(edit(Edit::KeepRecovery)))
                        .into(),
                    button::standard("Restore previous")
                        .on_press_maybe(editor.writable().then_some(edit(Edit::Recover)))
                        .into(),
                ],
            )),
            Some(Err(error)) => items.push(banner(
                error.clone(),
                ERROR,
                vec![
                    button::text("Keep current")
                        .on_press_maybe(editor.writable().then_some(edit(Edit::KeepRecovery)))
                        .into(),
                ],
            )),
            None => {}
        }
        // Someone who has never chosen a look starts here.
        if self.config.appearance.family.is_none() {
            items.push(
                container(
                    column::with_children(vec![
                        label("Welcome! Start with a look", 15., FG),
                        label(
                            "Pick one below and use it as is, or change any part afterwards.",
                            12.,
                            MUTED,
                        ),
                    ])
                    .spacing(3),
                )
                .id(cosmic::widget::Id::new("setups-welcome"))
                .padding([12, 14])
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card)
                .into(),
            );
        }
        if !editor.notes.is_empty() {
            items.push(
                column::with_children(
                    editor
                        .notes
                        .iter()
                        .map(|note| label(note.clone(), 11., MUTED))
                        .collect::<Vec<_>>(),
                )
                .spacing(3)
                .into(),
            );
        }
        let looks: Vec<Element<'_, Message>> = editor
            .presets()
            .iter()
            .filter(|preset| {
                !preset.profile.appearance_only && preset.id != crate::desktop::profiles::CLEAN
            })
            .map(|preset| self.preset_tile(preset))
            .collect();
        items.push(
            column::with_children(vec![
                label("Starter looks", 14., FG),
                row::with_children(looks).spacing(8).wrap().into(),
            ])
            .spacing(8)
            .into(),
        );
        items.push(self.selection_section());
        items.push(self.library_section());
        items.push(self.quick_switches());
        items.push(self.schedules_summary());
        column::with_children(items).spacing(16).into()
    }

    /// One-tap states that are not looks, such as hiding everything.
    fn quick_switches(&self) -> Element<'_, Message> {
        let id = crate::desktop::profiles::CLEAN;
        let selected = *self.profiles.selection() == Selection::Preset(id);
        let rows = self
            .profiles
            .presets()
            .iter()
            .filter(|preset| preset.id == id)
            .map(|preset| {
                split(
                    detail(preset.profile.name.clone(), preset.description),
                    button::text(if selected { "Chosen" } else { "Choose" })
                        .id(cosmic::widget::Id::new(format!("preset-{}", preset.id)))
                        .on_press_maybe(
                            (!selected).then_some(edit(Edit::Select(Selection::Preset(id)))),
                        )
                        .into(),
                )
            })
            .collect();
        column::with_children(vec![label("Quick switches", 14., FG), section(rows)])
            .spacing(8)
            .into()
    }

    /// The schedule rules at a glance; editing stays on its own page.
    fn schedules_summary(&self) -> Element<'_, Message> {
        let h24 = self.config.clock.format_24h;
        let mut rows: Vec<Element<'_, Message>> = Vec::new();
        match self.automation.settings() {
            Ok(settings) if !settings.rules.is_empty() => {
                for rule in settings.rules.iter().take(4) {
                    let name = self.automation.profile_name(&rule.profile, &rule.name);
                    rows.push(split(
                        detail(
                            format!("{} → {name}", rule.when.summary(h24)),
                            if !settings.enabled {
                                "Automatic switching is off"
                            } else if rule.enabled {
                                "On"
                            } else {
                                "Off"
                            },
                        ),
                        cosmic::widget::Space::new().width(0).into(),
                    ));
                }
                if settings.rules.len() > 4 {
                    rows.push(label(
                        format!("{} more", settings.rules.len() - 4),
                        12.,
                        MUTED,
                    ));
                }
            }
            _ => rows.push(label(
                "Switch to a look at a time of day, or when a display connects.",
                12.,
                MUTED,
            )),
        }
        column::with_children(vec![
            split(
                label("Switch automatically", 14., FG),
                button::text("Schedules")
                    .trailing_icon(icon::from_name("go-next-symbolic").size(14))
                    .id(cosmic::widget::Id::new("presets-schedules"))
                    .on_press(Message::Page(Page::DesktopAutomation))
                    .into(),
            ),
            section(rows),
        ])
        .spacing(8)
        .into()
    }

    pub(super) fn profiles_actions(&self) -> Element<'_, Message> {
        let editor = &self.profiles;
        // Repair/reset previews can be based on Current without selecting a
        // preset. Their active profile still needs an explicit confirmation.
        if *editor.selection() == Selection::Current && editor.profile().is_none() {
            return self.simple_status();
        }
        let problem = match &editor.notice {
            Some(Notice::Error(error)) => error.as_str(),
            _ => editor.theme_problem().unwrap_or(if editor.theme_loading() {
                "Finding wallpaper colors…"
            } else {
                "Local preview"
            }),
        };
        container(
            row::with_children(vec![
                container(label(problem, 12., MUTED))
                    .width(Length::Fill)
                    .into(),
                button::text("Cancel").on_press(edit(Edit::Discard)).into(),
                button::suggested(if self.page == Page::Themes {
                    "Use theme"
                } else {
                    "Use setup"
                })
                .id("profiles-confirm".into())
                .on_press_maybe(editor.can_apply().then_some(edit(Edit::Apply)))
                .into(),
            ])
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([8, 22])
        .width(Length::Fill)
        .into()
    }
}

/// Every part: what would change, what to know and what prevents it.
fn plan_details<'a>(plan: &ProfilePlan) -> Element<'a, Message> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for part in &plan.domains {
        let mut lines = vec![label(part.domain.title(), 12., FG)];
        if part.changes.is_empty() && part.blocked.is_empty() {
            lines.push(label("No changes", 11., MUTED));
        } else if !part.changes.is_empty() {
            lines.push(label(part.changes.join(", "), 11., MUTED));
        }
        for warning in &part.warnings {
            lines.push(label(warning.clone(), 11., WARN));
        }
        for reason in &part.blocked {
            lines.push(label(reason.clone(), 11., ERROR));
        }
        for note in &part.notes {
            lines.push(label(note.clone(), 11., MUTED));
        }
        rows.push(column::with_children(lines).spacing(3).into());
    }
    column::with_children(rows).spacing(10).into()
}
