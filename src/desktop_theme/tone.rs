//! Perceptual tones in OKLCH, shared by wallpaper suggestions, card roles and
//! native COSMIC accents. Lightness moves while hue and chroma stay, so a
//! readable color keeps its character instead of washing toward gray.
use cosmic::cosmic_theme::palette::{FromColor, LinSrgb, Oklab, Srgb, convert::FromColorUnclamped};
use cosmic::iced::Color;

/// OKLCH: perceptual lightness 0 to 1, chroma, hue in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
}

/// Below this chroma a color reads as gray; its hue is noise.
pub const NEUTRAL: f32 = 0.012;

pub fn lch(rgb: [u8; 3]) -> Lch {
    let lab = Oklab::from_color(Srgb::new(rgb[0], rgb[1], rgb[2]).into_format::<f32>());
    Lch {
        l: lab.l,
        c: lab.a.hypot(lab.b),
        h: lab.b.atan2(lab.a),
    }
}

/// The color when it is inside sRGB. `FromColor` would clamp each channel,
/// silently shifting hue; the unclamped conversion shows the overflow.
fn inside(lab: Oklab) -> Option<[u8; 3]> {
    let linear = LinSrgb::from_color_unclamped(lab);
    let channels = [linear.red, linear.green, linear.blue];
    channels
        .iter()
        .all(|v| (-0.0005..=1.0005).contains(v))
        .then(|| {
            let rgb: Srgb<f32> = Srgb::from_color_unclamped(lab);
            [rgb.red, rgb.green, rgb.blue].map(|v| (v.clamp(0., 1.) * 255.).round() as u8)
        })
}

/// The sRGB color of `lch`, reducing only chroma when it is out of gamut.
pub fn rgb(lch: Lch) -> [u8; 3] {
    let l = lch.l.clamp(0., 1.);
    let lab = |c: f32| Oklab::new(l, c * lch.h.cos(), c * lch.h.sin());
    if let Some(rgb) = inside(lab(lch.c.max(0.))) {
        return rgb;
    }
    let (mut low, mut high) = (0., lch.c.max(0.));
    for _ in 0..24 {
        let mid = (low + high) / 2.;
        if inside(lab(mid)).is_some() {
            low = mid;
        } else {
            high = mid;
        }
    }
    inside(lab(low)).unwrap_or_else(|| {
        let v = (l * 255.).round() as u8;
        [v, v, v]
    })
}

pub fn color(rgb: [u8; 3]) -> Color {
    Color::from_rgb8(rgb[0], rgb[1], rgb[2])
}

pub fn bytes(c: Color) -> [u8; 3] {
    [c.r, c.g, c.b].map(|v| (v.clamp(0., 1.) * 255.).round() as u8)
}

/// The same hue and chroma at another lightness.
pub fn at(rgb: [u8; 3], l: f32) -> [u8; 3] {
    self::rgb(Lch { l, ..lch(rgb) })
}

/// A wallpaper hue as a lively accent: COSMIC's own dark accents sit near
/// this lightness, with enough chroma to read as a color, not a gray.
pub fn lively(rgb: [u8; 3]) -> [u8; 3] {
    let source = lch(rgb);
    let c = if source.c < NEUTRAL {
        // Grey stays grey; its hue would be noise.
        source.c
    } else {
        (source.c * 1.3).clamp(0.085, 0.17)
    };
    self::rgb(Lch {
        l: 0.76,
        c,
        h: source.h,
    })
}

/// A faint tint (a graphite or slate picture) as a quiet, light steel.
pub fn steel(rgb: [u8; 3]) -> [u8; 3] {
    let source = lch(rgb);
    self::rgb(Lch {
        l: 0.78,
        c: (source.c * 4.).clamp(0.035, 0.055),
        h: source.h,
    })
}

pub fn luminance(c: Color) -> f32 {
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

pub fn contrast(a: Color, b: Color) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Moves only lightness, away from `background`, until `ratio` is reached.
/// Hue and chroma are kept (within gamut), unlike mixing toward white/black.
pub fn readable(rgb: [u8; 3], background: Color, ratio: f32) -> [u8; 3] {
    if contrast(color(rgb), background) >= ratio {
        return rgb;
    }
    let source = lch(rgb);
    let up = luminance(background) <= 0.18;
    let end = if up { 1. } else { 0. };
    let candidate = |t: f32| {
        self::rgb(Lch {
            l: source.l + (end - source.l) * t,
            ..source
        })
    };
    (1..=100)
        .map(|n| candidate(n as f32 / 100.))
        .find(|rgb| contrast(color(*rgb), background) >= ratio)
        .unwrap_or_else(|| if up { [255; 3] } else { [0; 3] })
}

/// Keeps a color whose lightness is already in `band`; otherwise moves it to
/// the nearest edge, keeping hue and chroma.
pub fn within(rgb: [u8; 3], band: std::ops::RangeInclusive<f32>) -> [u8; 3] {
    let source = lch(rgb);
    if band.contains(&source.l) {
        rgb
    } else {
        self::rgb(Lch {
            l: source.l.clamp(*band.start(), *band.end()),
            ..source
        })
    }
}

/// Distance in Oklab, the space the wallpaper analysis clusters in.
pub fn distance(a: [u8; 3], b: [u8; 3]) -> f32 {
    let (a, b) = (lch(a), lch(b));
    let (ax, ay) = (a.c * a.h.cos(), a.c * a.h.sin());
    let (bx, by) = (b.c * b.h.cos(), b.c * b.h.sin());
    ((a.l - b.l).powi(2) + (ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tones_keep_hue_and_reach_contrast_without_graying() {
        let amber = [64, 53, 44];
        let toned = lively(amber);
        let (before, after) = (lch(amber), lch(toned));
        assert!((after.l - 0.76).abs() < 0.02, "{toned:?}");
        assert!(
            after.c >= 0.08,
            "a dark brown becomes a visible amber: {toned:?}"
        );
        assert!((after.h - before.h).abs() < 0.2, "same hue family");
        let gray = lively([128, 128, 128]);
        assert!(
            lch(gray).c < 0.01,
            "neutral pictures stay neutral: {gray:?}"
        );
        for background in [
            Color::from_rgb8(30, 34, 36),
            Color::from_rgb8(247, 243, 233),
        ] {
            let rgb = readable(toned, background, 4.5);
            assert!(contrast(color(rgb), background) >= 4.5);
            assert!(lch(rgb).c >= 0.05, "readable ink keeps its color: {rgb:?}");
        }
        for rgb in [[255, 0, 0], [0, 255, 255], [30, 200, 90], [30, 130, 220]] {
            for l in [0.2, 0.5, 0.76, 0.95] {
                let toned = at(rgb, l);
                assert!((lch(toned).l - l).abs() < 0.02, "{rgb:?} at {l}: {toned:?}");
                let turn = (lch(toned).h - lch(rgb).h).abs();
                assert!(turn < 0.05, "out-of-gamut tones keep their hue: {toned:?}");
            }
        }
        assert_eq!(within([157, 207, 190], 0.7..=0.9), [157, 207, 190]);
        assert!((lch(within([60, 112, 91], 0.72..=0.88)).l - 0.72).abs() < 0.02);
    }
}
