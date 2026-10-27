use super::*;
use crate::native_test::Native;
use crate::widget_style::resolver::{ClockSurface, ClockWeight};

#[test]
fn fb06_clock_frame_is_independent_of_later_style_and_config_changes() {
    for family in [
        crate::desktop_theme::Family::Soft,
        crate::desktop_theme::Family::Paper,
    ] {
        for face in [ClockFace::Digital, ClockFace::Analog] {
            let mut config = Config::default();
            config.appearance.family = Some(family);
            config.clock.face = Some(face);
            let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 26)
                .unwrap()
                .and_hms_opt(9, 41, 0)
                .unwrap();
            let p = Presentation::new(&config);
            for size in [p.metrics().minimum, p.metrics().natural, [180., 340.]] {
                let geometry = p.frame(now, size).geometry;
                assert!(geometry.face_size > 0. && geometry.face_size.is_finite());
                let mut native = Native::new(Size::new(size[0], size[1]));
                let before = native.render(p.frame(now, size).view(), &cosmic::Theme::dark());
                let mut other = config.clone();
                other.appearance.family = Some(crate::desktop_theme::Family::Neon);
                other.clock.weight = Some(ClockWeight::Bold);
                other.widget_font = Some("DejaVu Sans Mono".into());
                let _scope = crate::widget_style::scope(&other);
                assert_eq!(geometry, p.frame(now, size).geometry);
                assert_eq!(
                    before,
                    native.render(p.frame(now, size).view(), &cosmic::Theme::dark())
                );
            }
        }
    }
}

#[test]
fn fb06_grouped_clock_measures_and_draws_with_its_selected_font() {
    crate::desktop_theme::load_installed_fonts();
    let mut config = Config::default();
    config.clock_font = Some("DejaVu Sans".into());
    config.glance.clock_font = Some("DejaVu Sans Mono".into());
    let grouped = Presentation::grouped(&config, config.palette());
    assert_eq!(
        grouped.style.display.value.family,
        supported_font(Font {
            family: cosmic::iced::font::Family::Name("DejaVu Sans Mono"),
            ..Font::DEFAULT
        })
        .family
    );
    assert_eq!(
        Presentation::new(&config).style.display.value.family,
        supported_font(Font {
            family: cosmic::iced::font::Family::Name("DejaVu Sans"),
            ..Font::DEFAULT
        })
        .family
    );
    let size = grouped.fit(200., 104.);
    assert!(size > 0. && size <= 104.);
    assert_eq!(grouped.style.ink.value, config.palette().foreground);
    config.clock.face = Some(ClockFace::Analog);
    let analog = Presentation::grouped(&config, config.palette());
    assert_eq!(
        analog.fit(200., 104.),
        104.,
        "analog size must not fit a hidden digital string"
    );
    assert!(analog.fit(50., 104.) * 1.05 <= 50.01);
}

#[test]
fn fb06_clock_rejects_nonfinite_viewport_before_native_measurement() {
    let p = Presentation::new(&Config::default());
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 26)
        .unwrap()
        .and_hms_opt(9, 41, 0)
        .unwrap();
    let frame = p.frame(now, [f32::NAN, f32::INFINITY]);
    assert_eq!(frame.geometry.viewport, p.metrics().natural);
    assert!(frame.geometry.face_size.is_finite());
}

#[test]
fn fb06_clock_measurement_is_logical_at_native_output_scales() {
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 26)
        .unwrap()
        .and_hms_opt(9, 41, 0)
        .unwrap();
    for face in [ClockFace::Digital, ClockFace::Analog] {
        let mut config = Config::default();
        config.clock.face = Some(face);
        config.clock.alignment = ClockAlignment::Center;
        let p = Presentation::new(&config);
        let size = p.metrics().minimum;
        let geometry = p.frame(now, size).geometry;
        let mut native = Native::new(Size::new(size[0], size[1]));
        for scale in [1., 1.5, 2.] {
            let pixels =
                native.render_scaled(p.frame(now, size).view(), &cosmic::Theme::dark(), scale);
            assert_eq!(
                pixels.len(),
                (f64::from(size[0]) * f64::from(size[1]) * scale * scale * 4.) as usize
            );
            let width = (f64::from(size[0]) * scale) as usize;
            let ink = super::tests::ink(&pixels, width);
            assert!(ink[0] > 0 && ink[2] < width - 1);
            assert!(ink[3] < (f64::from(size[1]) * scale) as usize - 1);
            assert_eq!(geometry, p.frame(now, size).geometry);
        }
    }
}

#[test]
fn foundation_clock_faces_fit_minimum_portrait_and_wide_frames() {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
        .unwrap()
        .and_hms_opt(11, 58, 0)
        .unwrap();
    for face in [ClockFace::Digital, ClockFace::Analog] {
        for size in [[160., 100.], [180., 340.], [490., 200.], [820., 240.]] {
            let mut native = Native::new(Size::new(size[0], size[1]));
            for alignment in [
                ClockAlignment::Left,
                ClockAlignment::Center,
                ClockAlignment::Right,
            ] {
                let mut config = Config::default();
                config.clock.face = Some(face);
                config.clock.weight = Some(ClockWeight::Bold);
                config.clock.alignment = alignment;
                config.clock.italic = Some(true);
                config.clock.surface = Some(ClockSurface::None);
                let pixels = native.render(view(&config, date, size), &cosmic::Theme::dark());
                let bounds = super::tests::ink(&pixels, size[0] as usize);
                assert!(
                    bounds[0] >= 1 && bounds[2] < size[0] as usize - 1,
                    "clipped horizontally: {face:?} {size:?} {alignment:?} {bounds:?}"
                );
                assert!(
                    bounds[1] >= 1 && bounds[3] < size[1] as usize - 1,
                    "clipped vertically: {face:?} {size:?} {alignment:?} {bounds:?}"
                );
            }
        }
    }
}

#[test]
fn foundation_panel_uses_surface_ink_and_readability_overrides_no_surface() {
    let mut config = Config::default();
    config.appearance.family = Some(crate::desktop_theme::Family::Paper);
    config.clock.surface = Some(ClockSurface::Panel);
    let _scope = crate::widget_style::scope(&config);
    let style = crate::widget_style::presentation(&config, config.wallpaper_palette(), true);
    let p = style.palette();
    assert!(crate::desktop_theme::contrast(p.foreground, p.background) >= 4.5);
    assert!(crate::desktop_theme::contrast(p.muted, p.background) >= 4.5);
    config.appearance.contrast_assist = true;
    config.clock.surface = Some(ClockSurface::None);
    let style = crate::widget_style::presentation(&config, config.wallpaper_palette(), true);
    assert!(matches!(style.surface.value, Surface::Solid(_)));
    assert_eq!(
        config.clock.surface,
        Some(ClockSurface::None),
        "saved intent must not change"
    );
}
