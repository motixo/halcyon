//! One clock renderer for the desktop, Daily overview and Settings preview.
use super::*;
use crate::config::{ClockAlignment, ClockPreferences};
use cosmic::iced::advanced::text::{self as advanced_text, Paragraph};
use cosmic::iced::widget::{pin, stack};
use cosmic::iced::{Point, Size};

fn font() -> Font {
    crate::widget_style::clock_font()
}
fn measure(value: &str, size: f32, font: Font) -> f32 {
    type NativeParagraph = <cosmic::Renderer as advanced_text::Renderer>::Paragraph;
    NativeParagraph::with_text(advanced_text::Text {
        content: value,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: size.into(),
        line_height: advanced_text::LineHeight::Relative(1.05),
        font,
        align_x: advanced_text::Alignment::Left,
        align_y: cosmic::iced::alignment::Vertical::Top,
        shaping: advanced_text::Shaping::Auto,
        wrapping: advanced_text::Wrapping::None,
        ellipsize: advanced_text::Ellipsize::None,
    })
    .min_bounds()
    .width
}

pub(super) fn ink_offset(time: &str, size: f32) -> f32 {
    let bearing = match time.as_bytes().first() {
        Some(b'0' | b'6') => 66.,
        Some(b'1') => 43.,
        Some(b'2') => 33.,
        Some(b'3') => 20.,
        Some(b'4') => 45.,
        Some(b'5') => 34.,
        Some(b'7') => 30.,
        Some(b'8' | b'9') => 53.,
        _ => 0.,
    };
    size * bearing / 1000.
}

/// Fit a full five-character time (and AM/PM) before rendering, so narrow
/// frames never clip or wrap and the font does not jump on each minute tick.
pub(super) fn fit(preferences: &ClockPreferences, width: f32, requested: f32) -> f32 {
    let width = (width - 2.).max(1.);
    let mut needed = measure("00:00", requested, font());
    let gap = if preferences.format_24h { 0. } else { 6. };
    if !preferences.format_24h {
        needed += measure("PM", requested * 0.2, crate::widget_style::font());
    }
    (requested * ((width - gap).max(1.) / needed.max(1.)).min(1.)).max(1.)
}

pub(super) fn face(
    config: &Config,
    time: chrono::NaiveTime,
    width: f32,
    size: f32,
    p: Palette,
) -> Element<'static, Message> {
    let _style = crate::widget_style::scope(config);
    let (time, period) = clock_labels(time, config.clock.format_24h);
    let mut content_width = measure(&time, size, font());
    let offset = if config.appearance.family.is_some() {
        0.
    } else {
        ink_offset(&time, size)
    };
    let mut items = vec![
        text::body(time)
            .size(size)
            .line_height(1.05)
            .font(font())
            .wrapping(advanced_text::Wrapping::None)
            .class(cosmic::theme::Text::Color(p.foreground))
            .into(),
    ];
    if let Some(period) = period {
        content_width += 6. + measure(&period, size * 0.2, crate::widget_style::font());
        items.push(
            container(label(period, size * 0.2, p.muted))
                .padding([0., 0., size * 0.08, 0.])
                .into(),
        );
    }
    let x = match config.clock.alignment {
        ClockAlignment::Left => -offset,
        ClockAlignment::Center => (width - content_width) / 2.,
        ClockAlignment::Right => width - content_width,
    };
    let content = row::with_children(items).spacing(6).align_y(Alignment::End);
    stack(vec![pin(content).position(Point::new(x, 0.)).into()])
        .width(width)
        .height(size * 1.05)
        .into()
}

pub(crate) fn view(
    config: &Config,
    now: chrono::NaiveDateTime,
    size: [f32; 2],
) -> Element<'static, Message> {
    let _style = crate::widget_style::scope(config);
    let p = config.wallpaper_palette();
    let width = (size[0] - 8.).max(1.);
    let height = (size[1] - 8.).max(1.);
    let scale =
        (size[0] / 490.).min(size[1] / 200.).clamp(0.1, 4.) * f32::from(config.clock.scale) / 100.;
    let mut font_size = fit(&config.clock, width, 104. * scale);
    let date = now.format("%A, %B %-d").to_string();
    let gap = if config.clock.show_date { 8. } else { 0. };
    if config.clock.show_date {
        let date_width = measure(&date, 20. * font_size / 104., crate::widget_style::font());
        font_size *= (width / date_width.max(1.)).min(1.);
    }
    let height_per_em = 1.05
        + if config.clock.show_date {
            25. / 104.
        } else {
            0.
        };
    font_size = font_size.min((height - gap).max(1.) / height_per_em);
    let mut content = vec![face(config, now.time(), width, font_size, p)];
    if config.clock.show_date {
        content.push(
            container(label(date, 20. * font_size / 104., p.muted))
                .width(Length::Fill)
                .align_x(config.clock.alignment.layout())
                .into(),
        );
    }
    container(
        column::with_children(content)
            .spacing(gap)
            .width(Length::Fill),
    )
    .width(size[0])
    .height(size[1])
    .padding(4)
    .align_y(if config.clock.center_vertically {
        Alignment::Center
    } else {
        Alignment::Start
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_test::Native;
    fn ink(pixels: &[u8], width: usize) -> [usize; 4] {
        let mut result = [usize::MAX, usize::MAX, 0, 0];
        for (i, pixel) in pixels.chunks_exact(4).enumerate() {
            if pixel[3] > 40 {
                let (x, y) = (i % width, i / width);
                result[0] = result[0].min(x);
                result[1] = result[1].min(y);
                result[2] = result[2].max(x);
                result[3] = result[3].max(y);
            }
        }
        assert!(result[0] < result[2], "Clock must be visible");
        result
    }
    #[test]
    fn resized_clock_fits_and_aligns_the_actual_glyphs() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 21)
            .unwrap()
            .and_hms_opt(23, 58, 0)
            .unwrap();
        let theme = cosmic::Theme::dark();
        let mut config = Config::default();
        for size in [[160., 100.], [300., 120.], [490., 200.], [620., 280.]] {
            let mut native = Native::new(Size::new(size[0], size[1]));
            for format in [true, false] {
                for scale in [50, 100, 200] {
                    config.clock.format_24h = format;
                    config.clock.scale = scale;
                    for alignment in [
                        ClockAlignment::Left,
                        ClockAlignment::Center,
                        ClockAlignment::Right,
                    ] {
                        config.clock.alignment = alignment;
                        config.clock.show_date = false;
                        config.clock.center_vertically = true;
                        let bounds = ink(
                            &native.render(view(&config, now, size), &theme),
                            size[0] as usize,
                        );
                        assert!(
                            bounds[0] >= 2 && bounds[2] < size[0] as usize - 2,
                            "horizontal clipping {size:?} {format} {scale} {alignment:?} {bounds:?}"
                        );
                        assert!(
                            bounds[1] > 2 && bounds[3] < size[1] as usize - 2,
                            "vertical clipping {bounds:?}"
                        );
                        if alignment == ClockAlignment::Center {
                            assert!(
                                ((bounds[0] + bounds[2]) as f32 / 2. - size[0] / 2.).abs() < 7.,
                                "clock is not centered {bounds:?}"
                            );
                        }
                        config.clock.show_date = true;
                        let bounds = ink(
                            &native.render(view(&config, now, size), &theme),
                            size[0] as usize,
                        );
                        assert!(
                            bounds[3] < size[1] as usize - 2,
                            "date must fit: {size:?} {bounds:?}"
                        );
                    }
                }
            }
            config.clock.scale = 130;
            config.clock.alignment = ClockAlignment::Center;
            native.capture(
                view(&config, now, size),
                &theme,
                &format!("clock-centered-{}", size[0]),
            );
        }
        let mut native = Native::new(Size::new(360., 148.));
        config.clock = ClockPreferences::default();
        let small = ink(
            &native.render(view(&config, now, [360., 148.]), &theme),
            360,
        );
        config.clock.scale = 150;
        let large = ink(
            &native.render(view(&config, now, [360., 148.]), &theme),
            360,
        );
        assert!(
            large[2] - large[0] > small[2] - small[0] + 40,
            "Text size changes visible glyphs"
        );
    }
}
