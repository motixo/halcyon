use super::*;

#[test]
#[ignore = "Owned private depth worker subprocess"]
fn worker_entry() {
    super::main();
}

fn fixture() -> (std::path::PathBuf, Config, Request) {
    use crate::widget_style::resolver::{ClockFace, ClockSurface, ClockWeight};
    let root = std::env::temp_dir().join(format!(
        "halcyon-depth-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("ridge.png");
    std::fs::write(&path, PICTURE).unwrap();
    let mut config = Config::default();
    config.appearance.family = Some(crate::desktop_theme::Family::Soft);
    config.appearance.wallpaper_light = false;
    config.clock.depth = true;
    config.clock.face = Some(ClockFace::Digital);
    config.clock.surface = Some(ClockSurface::None);
    config.clock.weight = Some(ClockWeight::Bold);
    config.clock.show_date = false;
    config.clock.scale = 200;
    let time = chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
        .unwrap()
        .and_hms_opt(9, 41, 0)
        .unwrap();
    let request = Request::new(
        &config,
        Frame {
            screen: [1920., 1080.],
            origin: [560., 260.],
            size: [800., 330.],
            scale: 1.,
        },
        BgEntry::new("all", BgSource::Path(path)),
        time,
    )
    .unwrap();
    (root, config, request)
}

#[test]
fn fb14_prepared_identity_rejects_replacement_devices_and_folders() {
    let (root, _, mut request) = fixture();
    assert!(matches_picture(&request.background));
    let BgSource::Path(path) = &request.background.source else {
        unreachable!()
    };
    let mut bytes = PICTURE.to_vec();
    bytes[400] ^= 1;
    std::fs::write(path, bytes).unwrap();
    assert!(!matches_picture(&request.background));
    for path in [
        std::path::PathBuf::from("/dev/zero"),
        root.clone(),
        root.join("missing"),
    ] {
        request.background.source = BgSource::Path(path);
        assert!(!matches_picture(&request.background));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fb14_mask_alignment_is_shared_across_scales_and_letterboxing() {
    let mask = image::GrayImage::from_fn(100, 100, |_, y| {
        image::Luma([if y >= 50 { 255 } else { 0 }])
    });
    for scale in [1., 1.5, 2.] {
        for (mode, screen, origin) in [
            (ScalingMode::Zoom, [200., 100.], [40., 30.]),
            (ScalingMode::Fit([0.; 3]), [200., 100.], [60., 30.]),
        ] {
            let frame = Frame {
                screen,
                origin,
                size: [40., 40.],
                scale,
            };
            let [width, height] = frame.pixels().unwrap();
            let mut p = Pixels {
                width,
                height,
                rgba: [39, 80, 140, 255].repeat((width * height) as usize),
            };
            apply_mask(&mut p, &frame, &mode, &mask, None).unwrap();
            for (i, pixel) in p.rgba.chunks_exact(4).enumerate() {
                assert_eq!(&pixel[..3], &[39, 80, 140]);
                let y = (i as u32 / width) as f32 / scale + origin[1];
                if y < 48. {
                    assert_eq!(pixel[3], 255);
                }
                if y > 52. {
                    assert_eq!(pixel[3], 0);
                }
            }
        }
    }
    let frame = Frame {
        screen: [200., 100.],
        origin: [0., 0.],
        size: [40., 40.],
        scale: 1.,
    };
    let mut p = Pixels {
        width: 40,
        height: 40,
        rgba: [255; 4].repeat(1600),
    };
    apply_mask(&mut p, &frame, &ScalingMode::Fit([0.; 3]), &mask, None).unwrap();
    assert!(
        p.rgba.chunks_exact(4).all(|p| p[3] == 255),
        "letterbox never occludes"
    );
}

#[test]
fn fb14_native_clock_alpha_only_at_three_scales_and_movement() {
    let (root, config, mut request) = fixture();
    let mut cut = 0;
    for scale in [1., 1.5, 2.] {
        request.frame.scale = scale;
        let masked = render(&request).unwrap();
        let mut native = crate::native_test::Native::new(cosmic::iced::Size::new(800., 330.));
        let time =
            chrono::NaiveDateTime::parse_from_str(&request.time, "%Y-%m-%dT%H:%M:%S").unwrap();
        let plain = native.render_scaled(
            crate::app::settings_clock_preview_at(&config, request.frame.size, time),
            &cosmic::Theme::dark(),
            f64::from(scale),
        );
        assert_eq!(plain.len(), masked.rgba.len());
        for (a, b) in plain.chunks_exact(4).zip(masked.rgba.chunks_exact(4)) {
            let rgb = [a[0], a[1], a[2]].map(|v| {
                if a[3] == 0 {
                    0
                } else {
                    ((u32::from(v) * 255 + u32::from(a[3]) / 2) / u32::from(a[3])).min(255) as u8
                }
            });
            assert_eq!(&rgb, &b[..3], "depth keeps straight glyph colors");
            assert!(b[3] <= a[3]);
            cut += usize::from(b[3] < a[3]);
        }
        if let Ok(dir) = std::env::var("HALCYON_DEPTH_EVIDENCE") {
            std::fs::create_dir_all(&dir).unwrap();
            let dir = std::path::Path::new(&dir);
            image::save_buffer(
                dir.join(format!("clock-{scale}.png")),
                &masked.rgba,
                masked.width,
                masked.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
            if scale == 1. {
                use cosmic::iced::widget::{image as native_image, pin, stack};
                let picture = image::load_from_memory(PICTURE).unwrap().to_rgba8();
                let bg = native_image(native_image::Handle::from_rgba(
                    picture.width(),
                    picture.height(),
                    picture.into_raw(),
                ))
                .width(1920.)
                .height(1080.)
                .content_fit(cosmic::iced::ContentFit::Cover);
                let clock = native_image(native_image::Handle::from_rgba(
                    masked.width,
                    masked.height,
                    masked.rgba.clone(),
                ))
                .width(800.)
                .height(330.);
                let view: cosmic::Element<'_, crate::app::Message> = stack([
                    bg.into(),
                    pin(clock)
                        .position(cosmic::iced::Point::new(560., 260.))
                        .into(),
                ])
                .width(1920.)
                .height(1080.)
                .into();
                let mut native =
                    crate::native_test::Native::new(cosmic::iced::Size::new(1920., 1080.));
                let pixels = native.render(view, &cosmic::Theme::dark());
                image::save_buffer(
                    dir.join("ridge-scene.png"),
                    &pixels,
                    1920,
                    1080,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
    assert!(cut > 1000, "the foreground actually occludes clock ink");
    request.frame.origin = [560., 0.];
    request.frame.scale = 1.;
    let clear = render(&request).unwrap();
    request.frame.origin = [560., 650.];
    let hidden = render(&request);
    assert!(clear.rgba.chunks_exact(4).any(|p| p[3] > 0));
    assert!(
        hidden.is_none(),
        "fully covered time must use the plain fallback"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fb14_fallbacks_reject_unsupported_frames_faces_and_surfaces() {
    let (root, mut config, mut request) = fixture();
    request.background.scaling_mode = ScalingMode::Stretch;
    assert!(render(&request).is_none());
    for bad in [[f32::NAN, 10.], [4096., 2048.], [0., 10.]] {
        request.frame.size = bad;
        assert!(request.frame.pixels().is_none());
    }
    config.clock.face = Some(crate::widget_style::resolver::ClockFace::Analog);
    assert!(!eligible(&config));
    config.clock.face = Some(crate::widget_style::resolver::ClockFace::Digital);
    config.clock.surface = Some(crate::widget_style::resolver::ClockSurface::Panel);
    assert!(!eligible(&config));
    config.clock.surface = Some(crate::widget_style::resolver::ClockSurface::None);
    config.appearance.contrast_assist = true;
    assert!(!eligible(&config));
    config.appearance.contrast_assist = false;
    config.glance.enabled = true;
    assert!(!eligible(&config));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fb14_worker_renders_and_cancellation_retires_owned_work() {
    let (root, _, request) = fixture();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let (sender, cancel) = tokio::sync::watch::channel(false);
    let bytes = serde_json::to_vec(&request).unwrap();
    let pixels = runtime
        .block_on(worker::render_depth(bytes.clone(), cancel.clone()))
        .unwrap();
    assert_eq!((pixels.width, pixels.height), (800, 330));
    sender.send_replace(true);
    assert!(
        runtime
            .block_on(worker::render_depth(bytes, cancel))
            .is_none()
    );
    assert!(
        runtime
            .block_on(worker::render_depth(
                vec![0; MAX_INPUT + 1],
                sender.subscribe()
            ))
            .is_none()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fb14_profile_depth_has_its_own_version_boundary() {
    use crate::desktop::profiles;
    let mut profile = profiles::preset("tidal").unwrap().profile;
    assert_eq!(profile.written_version(), 9);
    profile.widget_style.as_mut().unwrap()["clock"]["depth"] = true.into();
    profile.version = 10;
    assert!(profile.validate().is_err());
    profile.version = 11;
    profile.validate().unwrap();
    assert_eq!(profile.written_version(), 11);
    let exported = profiles::export(&profile).unwrap();
    assert!(
        profiles::import(&exported.bytes)
            .unwrap()
            .widget_style
            .unwrap()["clock"]["depth"]
            == true
    );
}

#[test]
fn fb14_date_stays_visible_and_public_render_needs_no_local_picture() {
    let (root, mut config, mut request) = fixture();
    config.clock.show_date = true;
    config.clock.center_vertically = true;
    let now = chrono::NaiveDateTime::parse_from_str(&request.time, "%Y-%m-%dT%H:%M:%S").unwrap();
    request.frame.origin = [560., 220.];
    let public = public_clock(&config, request.frame.clone(), ScalingMode::Zoom, now).unwrap();
    let mut native = crate::native_test::Native::new(cosmic::iced::Size::new(800., 330.));
    let plain = native.render(
        crate::app::settings_clock_preview_at(&config, request.frame.size, now),
        &cosmic::Theme::dark(),
    );
    let rows = crate::app::settings_clock_face_rows(&config, request.frame.size, now);
    let mut visible = 0;
    for (i, (a, b)) in plain
        .chunks_exact(4)
        .zip(public.rgba.chunks_exact(4))
        .enumerate()
    {
        if (i as u32 / public.width) as f32 > rows[1] + 2. {
            assert_eq!(a[3], b[3], "date is never occluded");
            visible += usize::from(b[3] > 0);
        }
    }
    assert!(visible > 50);
    std::fs::remove_dir_all(root).unwrap();
    assert!(public_clock(&config, request.frame.clone(), ScalingMode::Zoom, now).is_some());
    request.style["clock"]["scale"] = 65535.into();
    assert!(
        request.config().is_none(),
        "child validation cannot bypass saved bounds"
    );
}

#[test]
fn fb14_observation_uses_assigned_output_and_shared_fallback() {
    use crate::desktop::store::tests::Fixture;
    let fixture = Fixture::new("fb14-output-source");
    let shared = BgEntry::new("all", BgSource::Path(fixture.0.join("shared.png")));
    let selected = BgEntry::new("DP-2", BgSource::Path(fixture.0.join("selected.png")));
    fixture.user(
        wallpaper::NAMESPACE,
        "all",
        &ron::to_string(&shared).unwrap(),
    );
    fixture.user(wallpaper::NAMESPACE, "same-on-all", "false");
    fixture.user(
        wallpaper::NAMESPACE,
        wallpaper::output_key("DP-2").unwrap().key(),
        &ron::to_string(&selected).unwrap(),
    );
    // Native background selection ignores unlisted retained output files.
    assert_eq!(observe_from(&fixture.store(), "DP-2"), Some(shared.clone()));
    fixture.user(wallpaper::NAMESPACE, "backgrounds", "[\"DP-2\"]");
    assert_eq!(observe_from(&fixture.store(), "DP-2"), Some(selected));
    assert_eq!(observe_from(&fixture.store(), "DP-1"), Some(shared.clone()));
    fixture.user(wallpaper::NAMESPACE, "same-on-all", "true");
    assert_eq!(observe_from(&fixture.store(), "DP-2"), Some(shared));
}
