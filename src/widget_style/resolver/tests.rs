use super::*;
use crate::{config::Config, desktop_theme::Family};

fn defaults() -> Values {
    Values {
        ink: Color::WHITE,
        muted: Color::from_rgb(0.7, 0.7, 0.7),
        accent: Color::from_rgb(0.2, 0.8, 0.7),
        surface: Surface::Solid(Color::BLACK),
        body: Font::DEFAULT,
        display: Font::DEFAULT,
        radius: 12,
        padding: 18,
        gap: 8,
        motion: true,
        clock_face: ClockFace::Digital,
    }
}

#[test]
fn foundation_precedence_is_per_property_and_reset_exposes_the_actual_source() {
    let mut layers = Layers {
        setup: Patch {
            gap: Some(12),
            radius: Some(16),
            ..Default::default()
        },
        moment: Patch {
            gap: Some(20),
            ..Default::default()
        },
        instance: Patch {
            gap: Some(6),
            ..Default::default()
        },
        moment_instance: Patch {
            gap: Some(4),
            ..Default::default()
        },
    };
    let value = resolve(defaults(), layers, Constraints::default(), |_| true);
    assert_eq!(
        (value.gap.value, value.gap.source),
        (4, Source::MomentInstance)
    );
    assert_eq!(
        (value.radius.value, value.radius.source),
        (16, Source::Setup)
    );
    layers.moment_instance.gap = None;
    let value = resolve(defaults(), layers, Constraints::default(), |_| true);
    assert_eq!((value.gap.value, value.gap.source), (6, Source::Instance));
    layers.instance.gap = None;
    let value = resolve(defaults(), layers, Constraints::default(), |_| true);
    assert_eq!((value.gap.value, value.gap.source), (20, Source::Moment));
}

#[test]
fn foundation_explicit_no_surface_is_distinct_from_inheritance() {
    let mut layers = Layers::default();
    layers.instance.surface = Some(Surface::None);
    let explicit = resolve(defaults(), layers, Constraints::default(), |_| true);
    assert_eq!(explicit.surface.value, Surface::None);
    assert_eq!(explicit.surface.source, Source::Instance);
    layers.instance.surface = None;
    assert_eq!(
        resolve(defaults(), layers, Constraints::default(), |_| true)
            .surface
            .value,
        Surface::Solid(Color::BLACK)
    );
    let inherited: crate::config::ClockPreferences =
        serde_json::from_str(r#"{"surface":null}"#).unwrap();
    let none: crate::config::ClockPreferences =
        serde_json::from_str(r#"{"surface":"none"}"#).unwrap();
    assert_eq!(inherited.surface, None);
    assert_eq!(none.surface, Some(ClockSurface::None));
}

#[test]
fn foundation_fallback_retains_authoring_origin_and_accessibility_constraints() {
    let mut layers = Layers::default();
    layers.instance.display = Some(Font {
        family: cosmic::iced::font::Family::Name("Missing face"),
        ..Font::DEFAULT
    });
    layers.moment_instance.surface = Some(Surface::None);
    layers.moment_instance.motion = Some(true);
    let readable = Palette {
        background: Color::WHITE,
        foreground: Color::BLACK,
        muted: Color::BLACK,
        accent: Color::BLACK,
    };
    let style = resolve(
        defaults(),
        layers,
        Constraints {
            reduce_motion: true,
            readable: Some(readable),
        },
        |_| false,
    );
    assert_eq!(style.display.source, Source::Capability);
    assert_eq!(style.display.requested_source, Source::Instance);
    assert_eq!(style.missing_font, Some("Missing face"));
    assert_eq!(style.surface.value, Surface::Solid(Color::WHITE));
    assert_eq!(style.surface.source, Source::Accessibility);
    assert!(!style.motion.value);
    assert_eq!(
        layers.instance.display.unwrap().family,
        cosmic::iced::font::Family::Name("Missing face")
    );
}

#[test]
fn foundation_legacy_projection_retains_clock_and_weather_appearance() {
    for family in [
        None,
        Some(Family::Soft),
        Some(Family::Paper),
        Some(Family::Bare),
    ] {
        let mut config = Config::default();
        config.appearance.family = family;
        let _scope = crate::widget_style::scope(&config);
        let original = config.wallpaper_palette();
        let clock = crate::widget_style::presentation(&config, original, true);
        assert_eq!(clock.ink.value, original.foreground);
        assert_eq!(clock.muted.value, original.muted);
        assert_eq!(clock.clock_face.value, ClockFace::Digital);
        assert_eq!(clock.surface.value, Surface::None);
        assert_eq!((clock.padding.value, clock.gap.value), (4, 8));
        let original = config.palette();
        let weather = crate::widget_style::presentation(&config, original, false).palette();
        assert_eq!(weather.foreground, original.foreground);
        assert_eq!(weather.muted, original.muted);
        assert_eq!(weather.accent, original.accent);
        assert_eq!(weather.background.a, original.background.a);
    }
}

#[test]
fn foundation_clock_reading_is_stable_within_a_minute() {
    let at = |hour, minute, second| chrono::NaiveTime::from_hms_opt(hour, minute, second).unwrap();
    let midnight = ClockReading::at(at(0, 0, 0), false);
    assert_eq!(
        (&*midnight.time, midnight.period.as_deref()),
        ("12:00", Some("AM"))
    );
    assert_eq!(
        ClockReading::at(at(12, 0, 0), false).period.as_deref(),
        Some("PM")
    );
    let reading = ClockReading::at(at(9, 30, 0), true);
    assert_eq!(reading, ClockReading::at(at(9, 30, 59), true));
    assert_eq!(reading.minute_turn, 0.5);
    assert!((reading.hour_turn - 9.5 / 12.).abs() < 1e-6);
}

#[test]
fn foundation_font_and_surface_overrides_are_lexical() {
    let config = Config::default();
    let _outer = crate::widget_style::scope(&config);
    let before = crate::widget_style::clock_font();
    let mut customized = config.clone();
    customized.clock.face = Some(ClockFace::Analog);
    customized.clock.weight = Some(ClockWeight::Bold);
    customized.clock.italic = Some(true);
    customized.clock.surface = Some(ClockSurface::Panel);
    {
        let _inner = crate::widget_style::scope(&customized);
        let style =
            crate::widget_style::presentation(&customized, customized.wallpaper_palette(), true);
        assert_eq!(style.clock_face.value, ClockFace::Analog);
        assert_eq!(style.display.source, Source::Instance);
        assert_eq!(style.display.value.weight, cosmic::iced::font::Weight::Bold);
        assert_eq!(style.display.value.style, cosmic::iced::font::Style::Italic);
        assert!(matches!(style.surface.value, Surface::Solid(_)));
    }
    assert_eq!(crate::widget_style::clock_font(), before);
}

#[test]
fn foundation_invalid_clock_choices_are_rejected_by_config_and_owned_store() {
    for (field, value) in [
        ("face", serde_json::json!("script")),
        ("weight", serde_json::json!(100000)),
        ("italic", serde_json::json!("yes")),
        ("surface", serde_json::json!({"url":"remote"})),
    ] {
        let mut clock = serde_json::Map::new();
        clock.insert(field.into(), value.clone());
        assert!(serde_json::from_value::<Config>(serde_json::json!({"clock":clock})).is_err());
        let pointer = format!("/clock/{field}");
        assert!(
            !crate::desktop::store::widget::field(&pointer)
                .unwrap()
                .accepts(&value)
        );
    }
}
