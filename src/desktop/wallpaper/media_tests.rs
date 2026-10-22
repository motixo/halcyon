use super::*;
use crate::desktop::store::tests::Fixture;
use crate::desktop::wallpaper::BgGradient;
use std::fs;
use std::time::{Duration, Instant};

fn png(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba(pixel));
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}
fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 40]));
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    bytes
}
fn none(preview: Preview) -> &'static str {
    match preview {
        Preview::None(reason) => reason,
        Preview::Ready(pixels) => panic!("decoded {}×{}", pixels.width, pixels.height),
    }
}

/// Files over the size or dimension bounds are refused before decoding;
/// types cosmic-bg accepts but this app does not decode say so.
#[test]
fn oversized_or_unsupported_files_are_not_decoded() {
    let fixture = Fixture::new("media-oversized");
    let big = fixture.0.join("big.png");
    let file = fs::File::create(&big).unwrap();
    file.set_len(65 << 20).unwrap(); // sparse
    let started = Instant::now();
    assert_eq!(none(thumbnail(&big)), TOO_LARGE);
    // A valid header that claims 20000×20000 (the rest is never read).
    let mut claim = png(1, 1, [0, 0, 0, 255]);
    claim[16..20].copy_from_slice(&20_000u32.to_be_bytes());
    claim[20..24].copy_from_slice(&20_000u32.to_be_bytes());
    let huge = fixture.0.join("huge.png");
    fs::write(&huge, &claim).unwrap();
    assert_eq!(none(thumbnail(&huge)), NO_PREVIEW);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "refused quickly"
    );
    let webp = fixture.0.join("photo.webp");
    fs::write(&webp, b"RIFF\0\0\0\0WEBPVP8 ").unwrap();
    assert_eq!(none(thumbnail(&webp)), "No preview");
    assert_eq!(none(thumbnail(&fixture.0.join("gone.png"))), MISSING);
    // A picture renamed to .png but holding another format.
    let disguised = fixture.0.join("disguised.png");
    fs::write(&disguised, b"RIFF\0\0\0\0WEBPVP8 ").unwrap();
    assert_eq!(none(thumbnail(&disguised)), NO_PREVIEW);
}

#[test]
fn corrupt_image_yields_no_preview_without_panic() {
    let fixture = Fixture::new("media-corrupt");
    let whole = png(64, 36, [10, 20, 30, 255]);
    let path = fixture.0.join("cut.png");
    fs::write(&path, &whole[..whole.len() / 2]).unwrap();
    assert_eq!(none(thumbnail(&path)), NO_PREVIEW);
    let whole = jpeg(64, 36);
    let path = fixture.0.join("cut.jpg");
    fs::write(&path, &whole[..whole.len() / 3]).unwrap();
    // A truncated JPEG may still decode its first rows; it must not panic
    // and anything returned stays inside the thumbnail bounds.
    if let Preview::Ready(pixels) = thumbnail(&path) {
        assert!(pixels.width <= THUMB.0 && pixels.height <= THUMB.1);
    }
    let path = fixture.0.join("garbage.jpeg");
    fs::write(&path, [0xFF, 0xD8, 0xFF, 0x00, 0x13, 0x37]).unwrap();
    assert_eq!(none(thumbnail(&path)), NO_PREVIEW);
    // A whole picture decodes to its thumbnail.
    let path = fixture.0.join("ok.jpg");
    fs::write(&path, jpeg(64, 36)).unwrap();
    let Preview::Ready(pixels) = thumbnail(&path) else {
        panic!("a valid JPEG decodes");
    };
    assert_eq!((pixels.width, pixels.height), (64, 36));
    assert_eq!(pixels.rgba.len(), 64 * 36 * 4);
}

/// A large picture goes through `thumbnail` within `THUMB`, keeping its
/// aspect ratio.
#[test]
fn large_picture_is_reduced_to_the_thumbnail_bounds() {
    let fixture = Fixture::new("media-large");
    let path = fixture.0.join("large.png");
    let image = image::GrayImage::from_pixel(4000, 3000, image::Luma([90]));
    image.save(&path).unwrap();
    let Preview::Ready(pixels) = thumbnail(&path) else {
        panic!("decodes");
    };
    assert!(pixels.width <= THUMB.0 && pixels.height <= THUMB.1);
    assert_eq!((pixels.width, pixels.height), (240, 180));
}

#[test]
fn folder_scan_is_bounded_and_counts_non_images() {
    let fixture = Fixture::new("media-scan");
    let folder = fixture.0.join("slides");
    fs::create_dir_all(folder.join("sub")).unwrap();
    for name in ["c.png", "a.png", "sub/b.PNG"] {
        fs::write(folder.join(name), "png").unwrap();
    }
    for name in ["notes.txt", "sub/readme.txt"] {
        fs::write(folder.join(name), "text").unwrap();
    }
    // A symbolic link back to the folder itself ends.
    std::os::unix::fs::symlink(&folder, folder.join("sub/loop")).unwrap();
    let scan = scan_folder(&folder).unwrap();
    assert_eq!((scan.images, scan.others, scan.truncated), (3, 2, false));
    assert_eq!(
        scan.first,
        vec![
            folder.join("a.png"),
            folder.join("c.png"),
            folder.join("sub/b.PNG")
        ],
        "runtime (path) order"
    );
    // At the entry bound the scan says it is partial (the walk bound
    // itself is proven in the walk tests).
    let limits = Limits {
        depth: 8,
        entries: 4,
        dirs: 256,
    };
    let scan = scan_folder_with(&folder, limits).unwrap();
    assert!(scan.truncated);
    assert!(scan.images + scan.others <= 4, "{scan:?}");
    assert_eq!(
        scan_folder(&fixture.0.join("gone")).map(|_| ()).is_err(),
        true
    );
}

#[test]
fn sources_are_checked_without_writing() {
    let fixture = Fixture::new("media-check");
    let empty = fixture.0.join("empty");
    fs::create_dir_all(&empty).unwrap();
    fs::write(fixture.0.join("a.png"), "png").unwrap();
    let before = fixture.tree();
    let check = |path: PathBuf| check_source(&BgSource::Path(path));
    assert_eq!(check(fixture.0.join("a.png")), SourceCheck::Ready);
    assert_eq!(check(fixture.0.join("gone.png")), SourceCheck::Missing);
    assert_eq!(check(empty.clone()), SourceCheck::Empty);
    use std::os::unix::ffi::OsStrExt;
    let not_text = fixture.0.join(std::ffi::OsStr::from_bytes(b"bad-\xff.png"));
    assert_eq!(check(not_text), SourceCheck::NotText);
    assert_eq!(
        check_source(&BgSource::Color(BgColor::Single([0.1, 0.2, 0.3]))),
        SourceCheck::Ready
    );
    assert_eq!(fixture.tree(), before);
}

fn pixel(pixels: &Pixels, x: u32, y: u32) -> [u8; 3] {
    let at = ((y * pixels.width + x) * 4) as usize;
    [pixels.rgba[at], pixels.rgba[at + 1], pixels.rgba[at + 2]]
}
fn encoded(linear: [f32; 3]) -> [u8; 3] {
    linear.map(|value| (linear_to_srgb(value) * 255.).round() as u8)
}

/// cosmic-bg `colored.rs`: stops blend in linear light; 0 = bottom→top,
/// 90 = left→right, 180 = top→bottom, 270 = right→left. Solid colours are
/// sRGB as stored.
#[test]
fn gradient_preview_matches_cosmic_bg_formula() {
    let first = [0.8, 0.1, 0.05];
    let last = [0.02, 0.3, 0.9];
    let (w, h) = (400, 400);
    let gradient = |radius: f32| {
        render_color(
            &BgColor::Gradient(BgGradient {
                colors: vec![first, last],
                radius,
            }),
            w,
            h,
        )
    };
    let (start, end) = (encoded(first), encoded(last));
    let near = |got: [u8; 3], want: [u8; 3]| {
        got.iter()
            .zip(want)
            .all(|(got, want)| got.abs_diff(want) <= 3)
    };
    // (pixel at the first stop's edge, pixel at the last stop's edge); the
    // top-left pixel is exactly at position 0 or 1 in every direction.
    let cases = [
        (0., (0, h - 1), (0, 0), end),
        (90., (0, 0), (w - 1, 0), start),
        (180., (0, 0), (0, h - 1), start),
        (270., (w - 1, 0), (0, 0), end),
    ];
    for (radius, (sx, sy), (ex, ey), corner) in cases {
        let pixels = gradient(radius);
        assert_eq!((pixels.width, pixels.height), (w, h));
        assert!(near(pixel(&pixels, sx, sy), start), "{radius} first stop");
        assert!(near(pixel(&pixels, ex, ey), end), "{radius} last stop");
        assert_eq!(pixel(&pixels, 0, 0), corner, "{radius} exact edge");
    }
    // The midpoint blends in linear light, not in sRGB.
    let pixels = gradient(90.);
    let mid = encoded([
        (first[0] + last[0]) / 2.,
        (first[1] + last[1]) / 2.,
        (first[2] + last[2]) / 2.,
    ]);
    let got = pixel(&pixels, w / 2, 0);
    assert!(
        got.iter()
            .zip(mid)
            .all(|(got, want)| got.abs_diff(want) <= 4),
        "{got:?} vs {mid:?}"
    );
    let solid = render_color(&BgColor::Single([0.2, 0.4, 0.6]), 8, 4);
    assert_eq!(pixel(&solid, 3, 2), [51, 102, 153]);
}

#[test]
fn fit_preview_letterboxes_fills_and_stretches() {
    let source = Pixels {
        width: 4,
        height: 4,
        rgba: [200, 0, 0, 255].repeat(16),
    };
    let fit = fit_preview(&source, &ScalingMode::Fit([0., 0., 1.]), 16, 8);
    assert_eq!((fit.width, fit.height), (16, 8));
    assert_eq!(pixel(&fit, 0, 4), [0, 0, 255], "letterbox colour");
    assert_eq!(pixel(&fit, 8, 4), [200, 0, 0]);
    let zoom = fit_preview(&source, &ScalingMode::Zoom, 16, 8);
    assert_eq!(pixel(&zoom, 0, 4), [200, 0, 0], "fill crops, no bars");
    let stretch = fit_preview(&source, &ScalingMode::Stretch, 16, 8);
    assert_eq!(pixel(&stretch, 0, 0), [200, 0, 0]);
}

#[test]
fn slideshow_state_is_read_only_from_the_fixture() {
    let fixture = Fixture::new("media-state");
    let store = fixture.store();
    assert_eq!(current_slideshow_image(&store, "DP-1"), None);
    let state = fixture
        .0
        .join("cosmic-state/com.system76.CosmicBackground/v1/wallpapers");
    fs::create_dir_all(state.parent().unwrap()).unwrap();
    fs::write(
        &state,
        "[(\"HDMI-1\", Color(Single((0.1, 0.2, 0.3)))), (\"DP-1\", Path(\"/slides/b.png\"))]",
    )
    .unwrap();
    let before = fixture.tree();
    assert_eq!(
        current_slideshow_image(&store, "DP-1"),
        Some(PathBuf::from("/slides/b.png"))
    );
    assert_eq!(current_slideshow_image(&store, "HDMI-1"), None);
    fs::write(&state, "not ron").unwrap();
    assert_eq!(current_slideshow_image(&store, "DP-1"), None);
    let mut after = fixture.tree();
    after.insert(state.clone(), before.get(&state).cloned().flatten());
    assert_eq!(after, before);
}

#[test]
fn system_wallpapers_lists_one_level_and_stays_bounded() {
    let fixture = Fixture::new("media-system");
    let dir = fixture.0.join("backgrounds");
    fs::create_dir_all(dir.join("cosmic/deeper")).unwrap();
    fs::write(dir.join("top.jpg"), "jpg").unwrap();
    fs::write(dir.join("cosmic/a.png"), "png").unwrap();
    fs::write(dir.join("cosmic/notes.txt"), "text").unwrap();
    fs::write(dir.join("cosmic/deeper/too-deep.png"), "png").unwrap();
    for index in 0..230 {
        fs::write(dir.join(format!("many-{index:03}.png")), "png").unwrap();
    }
    let found = system_wallpapers(&[dir.clone(), fixture.0.join("missing")]);
    assert_eq!(found.len(), 200);
    assert!(found.contains(&dir.join("cosmic/a.png")));
    assert!(!found.iter().any(|path| path.ends_with("too-deep.png")));
    assert!(!found.iter().any(|path| path.ends_with("notes.txt")));
}

/// A minimal chunk list: (type, payload) after a PNG signature.
fn png_chunks(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in chunks {
        bytes.extend((data.len() as u32).to_be_bytes());
        bytes.extend(*kind);
        bytes.extend(*data);
        bytes.extend([0; 4]);
    }
    bytes
}
fn webp(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut body = b"WEBP".to_vec();
    for (kind, data) in chunks {
        body.extend(*kind);
        body.extend((data.len() as u32).to_le_bytes());
        body.extend(*data);
        if data.len() % 2 == 1 {
            body.push(0);
        }
    }
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((body.len() as u32).to_le_bytes());
    bytes.extend(body);
    bytes
}

#[test]
fn animated_pictures_are_told_from_still_ones_by_their_headers() {
    let fixture = Fixture::new("wallpaper-motion");
    let write = |name: &str, bytes: &[u8]| {
        let path = fixture.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    };
    // A real still PNG; an APNG (`acTL` before the image data, here after
    // a text chunk with an odd length); `acTL` after IDAT doesn't count.
    assert!(!animated(&write("still.png", &png(4, 4, [1, 2, 3, 255]))));
    let ihdr: &[u8] = &[0; 13];
    assert!(animated(&write(
        "moving.png",
        &png_chunks(&[
            (b"IHDR", ihdr),
            (b"tEXt", b"odd"),
            (b"acTL", &[0; 8]),
            (b"IDAT", &[0])
        ]),
    )));
    assert!(!animated(&write(
        "late.png",
        &png_chunks(&[(b"IHDR", ihdr), (b"IDAT", &[0]), (b"acTL", &[0; 8])]),
    )));
    // WebP: the VP8X animation flag or an ANIM chunk; lossy/lossless stills.
    assert!(animated(&write(
        "flag.webp",
        &webp(&[(b"VP8X", &[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0])])
    )));
    assert!(animated(&write(
        "anim.webp",
        &webp(&[
            (b"VP8X", &[0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"ICCP", b"abc"),
            (b"ANIM", &[0; 6])
        ]),
    )));
    assert!(!animated(&write(
        "alpha.webp",
        &webp(&[
            (b"VP8X", &[0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"VP8 ", &[0; 10])
        ]),
    )));
    assert!(!animated(&write(
        "lossless.webp",
        &webp(&[(b"VP8L", &[0; 5])])
    )));
    // Truncated, lying lengths, other types and missing files: still.
    let mut cut = png_chunks(&[(b"IHDR", ihdr)]);
    cut.extend(u32::MAX.to_be_bytes());
    cut.extend(b"tEXt");
    assert!(!animated(&write("cut.png", &cut)));
    assert!(!animated(&write("photo.jpg", &jpeg(4, 4))));
    assert!(!animated(&fixture.0.join("absent.webp")));
}
