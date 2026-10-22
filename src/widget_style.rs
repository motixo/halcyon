//! Shared card geometry and readable ink. Contrast assistance is opt-in and
//! uses a worst-case wallpaper range, so it needs no sampling or background work.
use crate::config::Palette;
use cosmic::iced::{Color, Font};

pub const PADDING: u16 = 18;
pub const TITLE: f32 = 18.;
pub const BODY: f32 = 14.;
pub const SECONDARY: f32 = 12.;
pub const META: f32 = 10.;
/// Font families chosen in Settings › Widget style, over the theme's.
#[derive(Clone, Copy, Default)]
struct Families {
    body: Option<&'static str>,
    clock: Option<&'static str>,
}
thread_local! {
    static CONTEXT: std::cell::Cell<Option<crate::desktop_theme::Roles>> = const { std::cell::Cell::new(None) };
    static FAMILIES: std::cell::Cell<Families> = const { std::cell::Cell::new(Families { body: None, clock: None }) };
}
/// A lexical rendering/measurement context. The guard restores the prior
/// context, so a widget preview cannot recolor or change Settings typography.
pub struct Scope(Option<crate::desktop_theme::Roles>, Families);
impl Drop for Scope {
    fn drop(&mut self) {
        CONTEXT.set(self.0);
        FAMILIES.set(self.1);
    }
}
pub fn scope(config: &crate::config::Config) -> Scope {
    let families = Families {
        body: config.widget_font.as_deref().map(intern),
        clock: config.clock_font.as_deref().map(intern),
    };
    Scope(
        CONTEXT.replace(config.theme_roles()),
        FAMILIES.replace(families),
    )
}
/// One widget's own families over the widget fonts; None keeps the current.
pub struct Fonts(Families);
impl Drop for Fonts {
    fn drop(&mut self) {
        FAMILIES.set(self.0);
    }
}
pub fn fonts(body: Option<&str>, clock: Option<&str>) -> Fonts {
    let current = FAMILIES.get();
    Fonts(FAMILIES.replace(Families {
        body: body.map(intern).or(current.body),
        clock: clock.map(intern).or(if body.is_some() { None } else { current.clock }),
    }))
}
/// The style's border width and the shadow around a card drawn with
/// palette `p` (no shadow without a background).
pub fn edge(p: Palette) -> (f32, cosmic::iced::Shadow) {
    use crate::desktop_theme::Shadow;
    let (width, kind) = CONTEXT
        .get()
        .map_or((1., Shadow::None), |r| (r.border_width, r.shadow));
    let kind = if p.background.a == 0. { Shadow::None } else { kind };
    let shadow = match kind {
        Shadow::None => cosmic::iced::Shadow::default(),
        Shadow::Soft => cosmic::iced::Shadow {
            color: Color::from_rgba(0., 0., 0., 0.38),
            offset: cosmic::iced::Vector::new(0., 4.),
            blur_radius: 10.,
        },
        Shadow::Hard => cosmic::iced::Shadow {
            color: Color { a: 1., ..p.foreground },
            offset: cosmic::iced::Vector::new(3., 3.),
            blur_radius: 0.,
        },
        Shadow::Glow => cosmic::iced::Shadow {
            color: Color { a: 0.6, ..p.accent },
            offset: cosmic::iced::Vector::new(0., 0.),
            blur_radius: 10.,
        },
    };
    (width, shadow)
}
/// A rounded rectangle as plain rectangles, for a compositor blur region:
/// the middle band plus a few stepped strips in each corner.
pub fn rounded_rects(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Vec<cosmic::iced::Rectangle> {
    use cosmic::iced::Rectangle;
    let r = radius.min(width / 2.).min(height / 2.).max(0.);
    let mut rects = vec![Rectangle::new(
        cosmic::iced::Point::new(x, y + r),
        cosmic::iced::Size::new(width, (height - 2. * r).max(0.)),
    )];
    const STEPS: usize = 4;
    for step in 0..STEPS {
        if r < 1. {
            break;
        }
        let band = r / STEPS as f32;
        // Inset at the strip's outer edge, where the curve is narrowest,
        // so no blur shows outside it.
        let dy = r - band * step as f32;
        let inset = r - (r * r - dy * dy).max(0.).sqrt();
        let size = cosmic::iced::Size::new((width - 2. * inset).max(0.), band);
        let top = y + band * step as f32;
        let bottom = y + height - band * (step as f32 + 1.);
        rects.push(Rectangle::new(cosmic::iced::Point::new(x + inset, top), size));
        rects.push(Rectangle::new(cosmic::iced::Point::new(x + inset, bottom), size));
    }
    rects
}
/// Room every widget surface keeps around its card for the style's shadow.
pub fn shadow_margin(config: &crate::config::Config) -> f32 {
    config.theme_roles().map_or(0., |r| r.shadow.margin())
}
/// Font names live for the program: a few chosen families, kept once each.
pub(crate) fn intern(name: &str) -> &'static str {
    static NAMES: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    let mut names = NAMES.lock().unwrap();
    if let Some(known) = names.iter().find(|known| **known == name) {
        return known;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.push(leaked);
    leaked
}
fn with_family(font: Font, family: Option<&'static str>) -> Font {
    family.map_or(font, |name| Font {
        family: cosmic::iced::font::Family::Name(name),
        ..font
    })
}
pub fn font() -> Font {
    let theme = CONTEXT
        .get()
        .map_or(Font::with_name("Fira Sans"), |r| r.body);
    with_family(theme, FAMILIES.get().body)
}
pub fn clock_font() -> Font {
    let theme = CONTEXT.get().map_or(
        Font {
            weight: cosmic::iced::font::Weight::Light,
            ..Font::with_name("Fira Sans")
        },
        |r| r.clock,
    );
    // The clock follows the widget font unless it has its own.
    let families = FAMILIES.get();
    with_family(theme, families.clock.or(families.body))
}
pub fn semantic(color: Color) -> Color {
    CONTEXT.get().map_or(color, |r| {
        // Existing event notices, stale state and errors retain their semantic
        // hue, with ink appropriate to Paper and transparent widgets.
        if color.r > 0.75 && color.g > 0.5 && color.g < 0.82 && color.b < 0.55 {
            r.warning
        } else if color.r > 0.7 && color.g < 0.5 && color.b < 0.6 {
            r.error
        } else {
            color
        }
    })
}

/// A highlight over a transparent card is a translucent ink layer. Mixing
/// against transparent black as if it were an opaque surface hides dark text.
pub fn tint(ink: Color, surface: Color, amount: f32) -> Color {
    if surface.a == 0. {
        Color {
            a: ink.a * amount,
            ..ink
        }
    } else {
        blend(ink, surface, amount)
    }
}

pub fn on_accent(palette: Palette) -> Color {
    if palette.background.a == 0. {
        crate::desktop_theme::ink(palette.foreground, palette.accent)
    } else {
        palette.background
    }
}

fn luminance(c: Color) -> f32 {
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}
fn blend(front: Color, back: Color, alpha: f32) -> Color {
    Color::from_rgb(
        front.r * alpha + back.r * (1. - alpha),
        front.g * alpha + back.g * (1. - alpha),
        front.b * alpha + back.b * (1. - alpha),
    )
}
fn ratio(a: Color, b: Color) -> f32 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn readable_ink(ink: Color, background: Color) -> Color {
    let target = if luminance(background) < 0.179 {
        Color::WHITE
    } else {
        Color::BLACK
    };
    (0..=100)
        .map(|n| blend(target, ink, n as f32 / 100.))
        .find(|candidate| ratio(*candidate, background) >= 4.55)
        .unwrap_or(target)
}
fn worst_ratio(ink: Color, background: Color, alpha: f32) -> f32 {
    let dark = blend(background, Color::BLACK, alpha);
    let light = blend(background, Color::WHITE, alpha);
    let l = luminance(ink);
    if (luminance(dark)..=luminance(light)).contains(&l) {
        1.
    } else {
        ratio(ink, dark).min(ratio(ink, light))
    }
}
pub fn readable(mut p: Palette) -> Palette {
    p.foreground = readable_ink(p.foreground, p.background);
    p.muted = readable_ink(p.muted, p.background);
    p.accent = readable_ink(p.accent, p.background);
    let start = (p.background.a.clamp(0., 1.) * 100.).ceil() as u32;
    p.background.a = (start..=100)
        .map(|n| n as f32 / 100.)
        .find(|&alpha| {
            [p.foreground, p.muted, p.accent]
                .into_iter()
                .all(|ink| worst_ratio(ink, p.background, alpha) >= 4.5)
        })
        .unwrap_or(1.);
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assisted_text_is_readable_over_bright_dark_and_colored_wallpapers() {
        for name in Palette::CHOICES.map(|(name, _)| name) {
            let mut p = Palette::named(name);
            p.background.a = 0.7;
            let p = readable(p);
            for wallpaper in [
                Color::BLACK,
                Color::WHITE,
                Color::from_rgb(1., 0., 0.),
                Color::from_rgb(0., 1., 0.),
                Color::from_rgb(0., 0., 1.),
            ] {
                let background = blend(p.background, wallpaper, p.background.a);
                for ink in [p.foreground, p.muted, p.accent] {
                    assert!(ratio(ink, background) >= 4.5, "{name}");
                }
            }
        }
        let light = Palette {
            background: Color::WHITE,
            foreground: Color::from_rgb(0.1, 0.1, 0.1),
            muted: Color::from_rgb(0.6, 0.6, 0.6),
            accent: Color::from_rgb(0.3, 0.8, 0.8),
        };
        let p = readable(light);
        for ink in [p.foreground, p.muted, p.accent] {
            assert!(ratio(ink, p.background) >= 4.5)
        }
    }
    #[test]
    fn assistance_is_optional_and_does_not_replace_the_saved_opacity() {
        let mut config = crate::config::Config::default();
        config.appearance.opacity = 70;
        let original = config.palette();
        assert!(!config.appearance.contrast_assist);
        config.appearance.contrast_assist = true;
        assert!(config.palette().background.a >= original.background.a);
        assert_eq!(config.appearance.opacity, 70);
        config.appearance.contrast_assist = false;
        assert_eq!(config.palette(), original);
    }
}

#[cfg(test)]
mod edge_tests {
    use super::*;
    #[test]
    fn blur_regions_stay_inside_the_rounded_card() {
        let (x, y, w, h, r) = (12., 12., 300., 200., 22.);
        let rects = rounded_rects(x, y, w, h, r);
        for rect in &rects {
            assert!(rect.x >= x && rect.y >= y, "{rect:?}");
            assert!(rect.x + rect.width <= x + w + 0.01 && rect.y + rect.height <= y + h + 0.01);
            // Every corner of every rectangle is inside the rounded shape.
            for (px, py) in [
                (rect.x, rect.y),
                (rect.x + rect.width, rect.y),
                (rect.x, rect.y + rect.height),
                (rect.x + rect.width, rect.y + rect.height),
            ] {
                let cx = px.clamp(x + r, x + w - r);
                let cy = py.clamp(y + r, y + h - r);
                assert!(
                    (px - cx).hypot(py - cy) <= r + 0.01,
                    "({px},{py}) outside the curve"
                );
            }
        }
        // The middle band covers the full width.
        assert!(rects.iter().any(|rect| rect.width == w && rect.height == h - 2. * r));
        // Square cards are one rectangle.
        assert_eq!(rounded_rects(0., 0., 100., 50., 0.).len(), 1);
    }
}

#[cfg(test)]
mod font_tests {
    use super::*;
    use cosmic::iced::font::Family;

    #[test]
    fn chosen_fonts_override_the_style_and_the_clock_follows_the_text_font() {
        let mut config = crate::config::Config::default();
        let theme = {
            let _scope = scope(&config);
            (font(), clock_font())
        };
        config.widget_font = Some("Inter".into());
        {
            let _scope = scope(&config);
            assert_eq!(font().family, Family::Name("Inter"));
            assert_eq!(clock_font().family, Family::Name("Inter"), "clock follows text");
            assert_eq!(clock_font().weight, theme.1.weight, "the style's weight stays");
        }
        config.clock_font = Some("Noto Serif".into());
        {
            let _scope = scope(&config);
            assert_eq!(font().family, Family::Name("Inter"));
            assert_eq!(clock_font().family, Family::Name("Noto Serif"));
        }
        // Leaving the scope restores whatever was there before.
        assert_eq!(font().family, Font::with_name("Fira Sans").family);
        config.widget_font = None;
        config.clock_font = None;
        let _scope = scope(&config);
        assert_eq!(font(), theme.0);
        assert_eq!(clock_font(), theme.1);
    }
}
