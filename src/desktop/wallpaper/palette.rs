//! Colours from a wallpaper (spec §5): a deterministic median cut in
//! Oklab over a bounded thumbnail, or over a colour background's displayed
//! stops. Reads only; the suggestion is offered, never applied.
use super::media::{Pixels, render_color};
use super::{BgColor, BgGradient};
use cosmic::cosmic_theme::palette::{FromColor, Oklab, Srgb};

/// The thumbnail's pixel count; larger inputs are sampled evenly.
const MAX_PIXELS: usize = 57_600;
const MAX_BOXES: usize = 8;
/// Centroids closer than this (ΔE in Oklab) are one colour.
const MERGE: f32 = 0.05;
const MIN_SHARE: f32 = 0.02;
const MAX_SWATCHES: usize = 5;
const MAX_ACCENTS: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    /// 8-bit sRGB.
    pub rgb: [u8; 3],
    /// Share of the analysed (opaque) pixels, 0–1.
    pub share: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Suggestion {
    /// At most five, by share (largest first) then lightness. Exact picture
    /// colours: polarity and previews use these.
    pub swatches: Vec<Swatch>,
    /// The suggested accent: the first of `accents`, else a lifted swatch.
    pub accent: Option<[u8; 3]>,
    /// Accents to choose from, best first: one per distinct wallpaper hue,
    /// toned like COSMIC's own dark accents, so none is dark or muddy.
    pub accents: Vec<[u8; 3]>,
}

fn oklab(rgb: [u8; 3]) -> [f32; 3] {
    let lab = Oklab::from_color(Srgb::new(rgb[0], rgb[1], rgb[2]).into_format::<f32>());
    [lab.l, lab.a, lab.b]
}
fn srgb(lab: [f32; 3]) -> [u8; 3] {
    let rgb: Srgb<f32> = Srgb::from_color(Oklab::new(lab[0], lab[1], lab[2]));
    [rgb.red, rgb.green, rgb.blue].map(|value| (value.clamp(0., 1.) * 255.).round() as u8)
}

/// Lift only recommendations, preserving the wallpaper hue and chroma.
/// Swatches and explicit user colors retain their original pixels.
pub fn visible_accent(rgb: [u8; 3]) -> [u8; 3] {
    let mut lab = oklab(rgb);
    if !(0.45..=0.85).contains(&lab[0]) {
        lab[0] = lab[0].clamp(0.62, 0.78);
        srgb(lab)
    } else {
        rgb
    }
}

/// Swatches of a picture: opaque pixels (alpha ≥ 128) of a thumbnail.
pub fn from_pixels(pixels: &Pixels) -> Suggestion {
    let total = (pixels.width as usize * pixels.height as usize).min(pixels.rgba.len() / 4);
    let step = total.div_ceil(MAX_PIXELS).max(1);
    let points: Vec<[f32; 3]> = (0..total)
        .step_by(step)
        .map(|index| &pixels.rgba[index * 4..index * 4 + 4])
        .filter(|pixel| pixel[3] >= 128)
        .map(|pixel| oklab([pixel[0], pixel[1], pixel[2]]))
        .collect();
    analyse(&points)
}

/// Swatches of a colour background: its displayed stops, equally weighted.
pub fn from_color(color: &BgColor) -> Suggestion {
    let stops: Vec<[u8; 3]> = match color {
        BgColor::Single(_) => {
            let pixels = render_color(color, 1, 1);
            vec![[pixels.rgba[0], pixels.rgba[1], pixels.rgba[2]]]
        }
        BgColor::Gradient(gradient) => gradient
            .colors
            .iter()
            .map(|stop| {
                let pixels = render_color(
                    &BgColor::Gradient(BgGradient {
                        colors: vec![*stop],
                        radius: 90.,
                    }),
                    1,
                    1,
                );
                [pixels.rgba[0], pixels.rgba[1], pixels.rgba[2]]
            })
            .collect(),
    };
    let points: Vec<[f32; 3]> = stops.into_iter().map(oklab).collect();
    analyse(&points)
}

fn range(points: &[[f32; 3]], members: &[usize], axis: usize) -> f32 {
    let values = members.iter().map(|&index| points[index][axis]);
    let (low, high) = values.fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| {
        (low.min(value), high.max(value))
    });
    high - low
}

fn analyse(points: &[[f32; 3]]) -> Suggestion {
    if points.is_empty() {
        return Suggestion::default();
    }
    // Median cut: split the box with the widest axis at its median, ties
    // broken by value, then pixel index.
    let mut boxes: Vec<Vec<usize>> = vec![(0..points.len()).collect()];
    while boxes.len() < MAX_BOXES {
        let mut widest: Option<(usize, usize, f32)> = None;
        for (index, members) in boxes.iter().enumerate() {
            if members.len() < 2 {
                continue;
            }
            for axis in 0..3 {
                let extent = range(points, members, axis);
                if extent > 0. && widest.is_none_or(|(_, _, best)| extent > best) {
                    widest = Some((index, axis, extent));
                }
            }
        }
        let Some((index, axis, _)) = widest else {
            break;
        };
        let mut members = std::mem::take(&mut boxes[index]);
        members.sort_by(|&a, &b| points[a][axis].total_cmp(&points[b][axis]).then(a.cmp(&b)));
        let upper = members.split_off(members.len() / 2);
        boxes[index] = members;
        boxes.insert(index + 1, upper);
    }
    // Population-weighted centroids, then merge near ones.
    let mut centroids: Vec<([f32; 3], usize)> = boxes
        .iter()
        .filter(|members| !members.is_empty())
        .map(|members| {
            let mut sum = [0f32; 3];
            for &index in members {
                for axis in 0..3 {
                    sum[axis] += points[index][axis];
                }
            }
            (sum.map(|value| value / members.len() as f32), members.len())
        })
        .collect();
    loop {
        let mut closest: Option<(usize, usize, f32)> = None;
        for i in 0..centroids.len() {
            for j in i + 1..centroids.len() {
                let (a, b) = (centroids[i].0, centroids[j].0);
                let distance =
                    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                if distance < MERGE && closest.is_none_or(|(_, _, best)| distance < best) {
                    closest = Some((i, j, distance));
                }
            }
        }
        let Some((i, j, _)) = closest else {
            break;
        };
        let (b, wb) = centroids.remove(j);
        let (a, wa) = centroids[i];
        let total = (wa + wb) as f32;
        centroids[i] = (
            [0, 1, 2].map(|axis| (a[axis] * wa as f32 + b[axis] * wb as f32) / total),
            wa + wb,
        );
    }
    let total = points.len() as f32;
    let mut kept: Vec<([f32; 3], f32)> = centroids
        .into_iter()
        .map(|(lab, count)| (lab, count as f32 / total))
        .filter(|(_, share)| *share >= MIN_SHARE)
        .collect();
    kept.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0[0].total_cmp(&b.0[0])));
    kept.truncate(MAX_SWATCHES);
    let accents = accents(points, &kept);
    Suggestion {
        swatches: kept
            .iter()
            .map(|(lab, share)| Swatch {
                rgb: srgb(*lab),
                share: *share,
            })
            .collect(),
        accent: accents.first().copied(),
        accents,
    }
}

fn chroma(lab: &[f32; 3]) -> f32 {
    lab[1].hypot(lab[2])
}

/// Hue bins of the accent histogram (15 degrees each).
const HUES: usize = 24;
/// Pixels below this chroma carry no usable hue.
const COLOURFUL: f32 = 0.02;
/// A hue needs this share of the analysed pixels to be offered.
const MIN_HUE_SHARE: f32 = 0.004;

/// Distinct lively hues, best first. Median cut follows lightness, so its
/// swatches are mostly one hue's ramp; a chroma-weighted hue histogram also
/// finds a nebula's magenta or a planet's blue rim. The most colourful mass
/// comes first, then the heaviest hue at least 45 degrees from those chosen.
/// A faint tint (graphite, slate) still gives one quiet steel; a truly grey
/// picture gives none.
fn accents(points: &[[f32; 3]], kept: &[([f32; 3], f32)]) -> Vec<[u8; 3]> {
    use crate::desktop_theme::tone;
    // Per bin: chroma-weighted count, count, and sums of a and b.
    let mut bins = [(0f32, 0usize, 0f32, 0f32); HUES];
    for lab in points {
        let c = chroma(lab);
        if c < COLOURFUL || !(0.12..=0.97).contains(&lab[0]) {
            continue;
        }
        let h = lab[2].atan2(lab[1]).rem_euclid(std::f32::consts::TAU);
        let bin = ((h / std::f32::consts::TAU * HUES as f32) as usize).min(HUES - 1);
        let entry = &mut bins[bin];
        entry.0 += c;
        entry.1 += 1;
        entry.2 += lab[1];
        entry.3 += lab[2];
    }
    let total = points.len().max(1) as f32;
    let apart = |a: usize, b: usize| {
        let d = a.abs_diff(b);
        d.min(HUES - d) >= 3
    };
    let mut chosen: Vec<usize> = Vec::new();
    let mut accents: Vec<[u8; 3]> = Vec::new();
    while accents.len() < MAX_ACCENTS {
        let Some(bin) = (0..HUES)
            .filter(|&i| bins[i].1 as f32 / total >= MIN_HUE_SHARE)
            .filter(|&i| chosen.iter().all(|&c| apart(i, c)))
            .max_by(|&a, &b| bins[a].0.total_cmp(&bins[b].0))
        else {
            break;
        };
        chosen.push(bin);
        // The hue and chroma around the bin, not its edge.
        let (mut a, mut b, mut c, mut count) = (0., 0., 0., 0usize);
        for j in [bin + HUES - 1, bin, bin + 1] {
            let entry = bins[j % HUES];
            c += entry.0;
            count += entry.1;
            a += entry.2;
            b += entry.3;
        }
        let mean = c / count as f32;
        // Further hues must be genuinely colourful, not a tinted shadow.
        if !accents.is_empty() && mean < 0.035 {
            continue;
        }
        let rgb = tone::lively(tone::rgb(tone::Lch {
            l: 0.6,
            c: mean,
            h: b.atan2(a),
        }));
        if accents
            .iter()
            .all(|kept| tone::distance(*kept, rgb) >= 0.05)
        {
            accents.push(rgb);
        }
    }
    if accents.is_empty()
        && let Some((lab, _)) = kept
            .iter()
            .filter(|(lab, _)| chroma(lab) >= 0.009)
            .max_by(|a, b| chroma(&a.0).total_cmp(&chroma(&b.0)))
    {
        accents.push(tone::steel(srgb(*lab)));
    }
    accents.truncate(MAX_ACCENTS);
    accents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::wallpaper::media::{self, Preview, THUMB};
    use cosmic::cosmic_theme::palette::{FromColor, Oklab, Srgb};

    fn image(colors: &[([u8; 3], usize)]) -> Pixels {
        let mut rgba = Vec::new();
        for (rgb, count) in colors {
            for _ in 0..*count {
                rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        let width = (rgba.len() / 4) as u32;
        Pixels {
            width,
            height: 1,
            rgba,
        }
    }
    fn lab(rgb: [u8; 3]) -> Oklab {
        Oklab::from_color(Srgb::new(rgb[0], rgb[1], rgb[2]).into_format::<f32>())
    }
    fn distance(a: [u8; 3], b: [u8; 3]) -> f32 {
        let (a, b) = (lab(a), lab(b));
        ((a.l - b.l).powi(2) + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)).sqrt()
    }

    #[test]
    fn bundled_dark_wallpapers_recommend_visible_hue_related_accents() {
        for name in ["lavender", "teal", "ocean", "amber", "rose", "graphite"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("assets/wallpapers/{name}-1080p.png"));
            let Preview::Ready(pixels) = media::thumbnail(&path) else {
                panic!("{name}");
            };
            let suggestion = from_pixels(&pixels);
            let rgb = super::super::automatic::recommended(&suggestion).unwrap();
            assert!((0.45..=0.85).contains(&lab(rgb).l), "{name}: {rgb:?}");
            if name == "lavender" {
                assert!(rgb[2] > rgb[0] && rgb[2] > rgb[1], "{rgb:?}");
            }
        }
        let dark = [29, 29, 44];
        let suggestion = from_pixels(&image(&[(dark, 100)]));
        assert_eq!(
            suggestion.swatches[0].rgb, dark,
            "analysed swatches stay exact"
        );
        let rgb = suggestion.accent.unwrap();
        assert!((0.72..=0.80).contains(&lab(rgb).l), "{rgb:?}");
        assert!(rgb[2] > rgb[0]);
        // The reported amber case: every offered accent is a visible color,
        // never one of the picture's dark browns.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/wallpapers/amber-1080p.png");
        let Preview::Ready(pixels) = media::thumbnail(&path) else {
            panic!("amber");
        };
        let amber = from_pixels(&pixels);
        assert!(!amber.accents.is_empty());
        for accent in &amber.accents {
            let lch = crate::desktop_theme::tone::lch(*accent);
            assert!(lch.l >= 0.72 && lch.c >= 0.08, "{accent:?}");
        }
        assert!(amber.swatches.iter().all(|swatch| lab(swatch.rgb).l < 0.5));
    }

    #[test]
    fn colourful_pictures_offer_their_distinct_hues_and_grey_ones_none() {
        // A dominant blue with a small, vivid orange accent area.
        let picture = image(&[([20, 40, 120], 900), ([240, 140, 30], 100)]);
        let suggestion = from_pixels(&picture);
        assert_eq!(suggestion.accents.len(), 2, "{suggestion:?}");
        let hue = |rgb: [u8; 3]| crate::desktop_theme::tone::lch(rgb).h;
        assert!((hue(suggestion.accents[0]) - hue([20, 40, 120])).abs() < 0.15);
        assert!((hue(suggestion.accents[1]) - hue([240, 140, 30])).abs() < 0.15);
        assert_eq!(suggestion.accent, Some(suggestion.accents[0]));
        let grey = from_pixels(&image(&[([236, 236, 236], 900), ([150, 150, 150], 100)]));
        assert!(grey.accents.is_empty());
        assert_eq!(
            super::super::automatic::recommended_for(
                &grey,
                Some(crate::desktop_theme::Family::Soft)
            ),
            Some(crate::desktop_theme::Family::Soft.accent()),
            "a theme keeps its own color on a grey picture"
        );
        // A faint slate tint gives one quiet steel instead.
        let slate = from_pixels(&image(&[([31, 34, 40], 800), ([62, 66, 73], 200)]));
        assert_eq!(slate.accents.len(), 1);
        let steel = crate::desktop_theme::tone::lch(slate.accents[0]);
        assert!(steel.c < 0.06 && steel.l > 0.74, "{slate:?}");
    }

    #[test]
    fn palette_is_deterministic_and_bounded() {
        let teal = [20, 150, 140];
        let sand = [220, 200, 150];
        let picture = image(&[(teal, 700), (sand, 300)]);
        let first = from_pixels(&picture);
        assert_eq!(first, from_pixels(&picture), "deterministic");
        assert_eq!(first.swatches.len(), 2, "{first:?}");
        let near = |rgb: [u8; 3]| {
            first
                .swatches
                .iter()
                .find(|swatch| distance(swatch.rgb, rgb) <= 0.02)
                .unwrap_or_else(|| panic!("{rgb:?} in {first:?}"))
        };
        assert!((near(teal).share - 0.7).abs() <= 0.02);
        assert!((near(sand).share - 0.3).abs() <= 0.02);
        assert_eq!(first.swatches[0].rgb, near(teal).rgb, "largest share first");
        let accent = first.accent.unwrap();
        assert!(
            distance(accent, crate::desktop_theme::tone::lively(teal)) <= 0.03,
            "{first:?}"
        );

        // Many colours: at most five swatches, all with a visible share.
        let mut colors = Vec::new();
        for index in 0..40u16 {
            let channel = |value: u16| (value % 256) as u8;
            colors.push((
                [
                    channel(index * 6),
                    channel(255 - index * 6),
                    channel(index * 37),
                ],
                25,
            ));
        }
        let many = from_pixels(&image(&colors));
        assert!(many.swatches.len() <= 5);
        assert!(many.swatches.iter().all(|swatch| swatch.share >= 0.02));
        assert!(
            many.swatches
                .windows(2)
                .all(|pair| pair[0].share >= pair[1].share)
        );

        let grey = from_pixels(&image(&[([128, 128, 128], 500), ([60, 60, 60], 500)]));
        assert_eq!(grey.accent, None, "{grey:?}");
        // Transparent pixels are ignored.
        let mut clear = image(&[(teal, 10)]);
        for alpha in clear.rgba.iter_mut().skip(3).step_by(4) {
            *alpha = 0;
        }
        assert_eq!(from_pixels(&clear), Suggestion::default());

        // A large picture is analysed through its bounded thumbnail.
        let fixture = crate::desktop::store::tests::Fixture::new("palette-large");
        let path = fixture.0.join("large.png");
        image::RgbImage::from_pixel(4000, 3000, image::Rgb(teal))
            .save(&path)
            .unwrap();
        let Preview::Ready(thumb) = media::thumbnail(&path) else {
            panic!("decodes");
        };
        assert!(thumb.width <= THUMB.0 && thumb.height <= THUMB.1);
        let analysed = from_pixels(&thumb);
        assert_eq!(analysed.swatches.len(), 1);
        assert!(distance(analysed.swatches[0].rgb, teal) <= 0.02);
    }

    #[test]
    fn colour_suggestions_use_the_displayed_stops() {
        let solid = from_color(&BgColor::Single([0.2, 0.4, 0.6]));
        assert_eq!(solid.swatches.len(), 1);
        assert_eq!(solid.swatches[0].rgb, [51, 102, 153]);
        let accent = solid.accent.unwrap();
        assert!(
            distance(accent, crate::desktop_theme::tone::lively([51, 102, 153])) <= 0.03,
            "{accent:?}"
        );
        // Gradient stops are linear; the swatches are what is displayed.
        let gradient = from_color(&BgColor::Gradient(crate::desktop::wallpaper::BgGradient {
            colors: vec![[0.0331, 0.1329, 0.3185], [0.0331, 0.1329, 0.3185]],
            radius: 90.,
        }));
        assert_eq!(gradient.swatches.len(), 1);
        assert!(distance(gradient.swatches[0].rgb, [51, 102, 153]) <= 0.01);
    }
}
