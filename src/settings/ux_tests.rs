use super::*;
use crate::native_test::Native;
use cosmic::iced::{Event, Point, mouse};

fn child(name: &str) -> bool {
    if std::env::var("DESKTOP_CARDS_UX_TEST").as_deref() == Ok(name) {
        return false;
    }
    let root = std::env::temp_dir().join(format!("cards-ux-{}-{name}", std::process::id()));
    for dir in ["config", "data", "cache", "state", "desktop"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join("dconf-profile"), "").unwrap();
    std::fs::write(root.join("config.json"), r#"{"setup_dismissed":true,"enabled_widgets":["clock","calendar","weather","devbox"],"future_preference":37}"#).unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("settings::ux_tests::{name}")])
        .env("DESKTOP_CARDS_UX_TEST", name)
        .env("DESKTOP_CARDS_CONFIG", root.join("config.json"))
        .env("DESKTOP_CARDS_DESKTOP_ROOT", root.join("desktop"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("DCONF_PROFILE", root.join("dconf-profile"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    true
}

fn click_at(app: &mut Settings, native: &mut Native, point: Point) {
    for event in [
        mouse::Event::CursorMoved { position: point },
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        for message in native
            .event(app.settings_view(), Event::Mouse(event), point)
            .0
        {
            let _ = app.update(message);
        }
    }
}
fn click(app: &mut Settings, native: &mut Native, id: &str) {
    let bounds = native.bounds(app.settings_view(), id);
    assert!(
        bounds.x >= 0.
            && bounds.y >= 0.
            && bounds.x + bounds.width <= native.size.width
            && bounds.y + bounds.height <= native.size.height,
        "{id}: {bounds:?}"
    );
    click_at(app, native, bounds.center());
}

#[test]
fn review_r2_native_theme_preview_never_falls_back_to_obsolete_home_pixels() {
    if child("review_r2_native_theme_preview_never_falls_back_to_obsolete_home_pixels") {
        return;
    }
    use crate::desktop::{DesktopStore, profiles as records, wallpaper as backgrounds};
    let root = std::path::PathBuf::from(std::env::var_os("DESKTOP_CARDS_DESKTOP_ROOT").unwrap());
    let set = |key: &str, value: &str| {
        let directory = root
            .join("config/cosmic")
            .join(backgrounds::NAMESPACE)
            .join("v1");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(key), value).unwrap();
    };
    std::fs::write(
        Config::path().unwrap(),
        r#"{"setup_dismissed":true,"enabled_widgets":[]}"#,
    )
    .unwrap();
    let bright = [245u8, 243, 207];
    let dark = [10u8, 10, 10];
    let size = Size::new(1024., 720.);
    let count =
        |pixels: &[u8], rgb: [u8; 3]| pixels.chunks_exact(4).filter(|p| p[..3] == rgb).count();
    let capture = |pixels: &[u8], retained: bool, stage: &str| {
        if let Some(directory) = std::env::var_os("DESKTOP_CARDS_LAYOUT_EVIDENCE") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            image::save_buffer(
                directory.join(format!("r2-native-preview-{retained}-{stage}.png")),
                pixels,
                size.width as u32,
                size.height as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    };
    for retained in [false, true] {
        let image = root.join(format!("preview-{retained}.png"));
        image::RgbImage::from_pixel(32, 32, image::Rgb(bright))
            .save(&image)
            .unwrap();
        set(
            "all",
            &ron::to_string(&backgrounds::BgEntry::new(
                "all",
                backgrounds::BgSource::Path(image.clone()),
            ))
            .unwrap(),
        );
        set("same-on-all", if retained { "false" } else { "true" });
        set("backgrounds", if retained { "[\"eDP-1\"]" } else { "[]" });
        std::fs::write(root.join("outputs"), if retained { "eDP-1\n" } else { "" }).unwrap();
        if retained {
            set(
                "output.eDP-1",
                &ron::to_string(&backgrounds::BgEntry::new(
                    "eDP-1",
                    backgrounds::BgSource::Path(image.clone()),
                ))
                .unwrap(),
            );
        }
        let (mut app, _) = Settings::init(Core::default(), Page::Themes);
        app.window = size;
        // Drive jobs explicitly; init tasks are never launched in this fixture.
        let mut editor = profiles::ProfilesEditor::default();
        editor.open();
        editor.open_home();
        for job in editor.take_jobs() {
            editor.update(job.run());
        }
        assert!(
            editor.home_image().is_some(),
            "prime the applied wallpaper cache"
        );
        if retained {
            editor.update(profiles::Message::Select(profiles::Selection::Preset(
                "theme-bare",
            )));
            editor.update(profiles::Message::ThemeColors(true));
            editor.update(profiles::Message::ThemeWallpaper(true));
        } else {
            let mut p = records::themes::record(crate::desktop_theme::Family::Bare);
            p.wallpaper = Some(records::WallpaperLook {
                background: Some(records::BackgroundLook {
                    source: Some(records::SourceLook::Image(image.clone())),
                    ..Default::default()
                }),
                ..Default::default()
            });
            p.widget_style.as_mut().unwrap()["appearance"]["wallpaper_colors"] = true.into();
            let store = DesktopStore::from_env().unwrap();
            let id = records::ProfileLibrary::new(&store).create(&p).unwrap();
            editor.update(profiles::Message::Select(profiles::Selection::Saved(id)));
        }
        for job in editor.take_theme_jobs() {
            editor.update(job.run());
        }
        for job in editor.take_jobs() {
            editor.update(job.run());
        }
        app.profiles = editor;
        let mut native = Native::new(size);
        let pixels = native.render(app.settings_view(), &cosmic::Theme::dark());
        capture(&pixels, retained, "bright");
        assert!(
            count(&pixels, bright) > 1_000,
            "actual preview image must be visible"
        );
        image::RgbImage::from_pixel(33, 32, image::Rgb(dark))
            .save(&image)
            .unwrap();
        assert_eq!(
            app.profiles.update(profiles::Message::Apply),
            profiles::Outcome::None
        );
        assert!(app.profiles.stale());
        app.profiles.update(profiles::Message::Replan);
        for job in app.profiles.take_theme_jobs() {
            app.profiles.update(job.run());
        }
        assert!(app.profiles.backdrop_image().is_none());
        assert!(
            app.profiles.home_image().is_some(),
            "the old Home cache still exists"
        );
        let pixels = native.render(app.settings_view(), &cosmic::Theme::dark());
        capture(&pixels, retained, "loading");
        assert_eq!(
            count(&pixels, bright),
            0,
            "never draw old Home pixels while the new version loads"
        );
        for job in app.profiles.take_jobs() {
            app.profiles.update(job.run());
        }
        let pixels = native.render(app.settings_view(), &cosmic::Theme::dark());
        capture(&pixels, retained, "dark");
        assert_eq!(
            count(&pixels, bright),
            0,
            "old renderer textures must not survive"
        );
        assert!(
            count(&pixels, dark) > 1_000,
            "render the replacement image's actual pixels"
        );
        assert!(app.profiles.can_apply());
    }
}

#[test]
fn dew_point_switch_saves_and_undoes_without_touching_other_preferences() {
    if child("dew_point_switch_saves_and_undoes_without_touching_other_preferences") {
        return;
    }
    let original = std::fs::read(Config::path().unwrap()).unwrap();
    for size in [Size::new(820., 600.), Size::new(1100., 760.)] {
        std::fs::write(Config::path().unwrap(), &original).unwrap();
        let (mut app, _) = Settings::init(Core::default(), Page::Weather);
        let _ = app.update(Message::Resized(size));
        assert!(!app.config.weather.show_dew_point, "Existing users opt in");
        Config::save_weather(serde_json::json!({"future_reading":37})).unwrap();
        let mut native = Native::new(size);
        click(&mut app, &mut native, "weather-dew-point-toggle");
        assert!(app.config.weather.show_dew_point);
        assert!(Config::try_load().unwrap().weather.show_dew_point);
        let stored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(Config::path().unwrap()).unwrap()).unwrap();
        assert_eq!(stored["weather"]["future_reading"], 37);
        assert_eq!(stored["future_preference"], 37);
        for (name, theme) in [
            ("dark", cosmic::Theme::dark()),
            ("light", cosmic::Theme::light()),
        ] {
            native.capture(
                app.settings_view(),
                &theme,
                &format!("dew-point-settings-{name}-{}", size.width),
            );
        }
        let _ = app.update(Message::UndoPreferences);
        assert!(!Config::try_load().unwrap().weather.show_dew_point);
        let restored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(Config::path().unwrap()).unwrap()).unwrap();
        assert_eq!(restored["weather"]["future_reading"], 37);
        let hit = navigation::search("dewpoint")[0];
        assert_eq!(hit.page, Page::Weather);
        assert_eq!(hit.anchor(), "weather-dew-point");
        assert!(!hit.advanced);
    }
}

#[test]
fn native_task_navigation_and_widget_switches_keep_layout_drafts() {
    if child("native_task_navigation_and_widget_switches_keep_layout_drafts") {
        return;
    }
    let original = std::fs::read(Config::path().unwrap()).unwrap();
    for size in [Size::new(820., 600.), Size::new(1024., 720.)] {
        let (mut app, _) = Settings::init(Core::default(), Page::Overview);
        let _ = app.update(Message::Resized(size));
        let mut native = Native::new(size);
        let _ = app.update(Message::Placement(placement::Message::Select(
            "clock".into(),
        )));
        let _ = app.update(Message::Placement(placement::Message::Nudge([1, 0], false)));
        assert!(!app.placement.dirty());
        let draft = serde_json::to_value(app.placement.session_draft()).unwrap();
        let widgets = app
            .nav_model
            .iter()
            .find(|&id| app.nav_model.data::<Page>(id) == Some(&Page::Widgets))
            .unwrap();
        let _ = app.update(Message::NavSelected(widgets));
        assert_eq!(app.page, Page::Widgets);
        assert_eq!(app.nav_model.active_data::<Page>(), Some(&Page::Widgets));
        let clock = native.bounds(app.settings_view(), "widget-open-clock");
        let before = app.config.data_visible(Widget::Clock);
        // The switch has its own hit target, outside the button that opens settings.
        click_at(
            &mut app,
            &mut native,
            Point::new(size.width - 54., clock.center_y()),
        );
        assert_eq!(app.page, Page::Widgets);
        assert_eq!(app.config.data_visible(Widget::Clock), !before);
        let _ = app.update(Message::UndoPreferences);
        click(&mut app, &mut native, "widget-open-weather");
        assert_eq!(app.page, Page::Weather);
        click(&mut app, &mut native, "settings-back");
        assert_eq!(app.page, Page::Widgets);
        // Every Desktop page is its own menu entry, without a Back link.
        click(&mut app, &mut native, "nav-desktop-windows");
        assert_eq!(app.page, Page::DesktopWindows);
        assert!(native.try_bounds(app.view(), "settings-back").is_none());
        let _ = app.update(Message::Page(Page::DesktopWorkspaces));
        assert_eq!(app.page, Page::DesktopWindows, "Workspaces is part of Windows");
        assert_eq!(
            app.nav_model.active_data::<Page>(),
            Some(&Page::DesktopWindows)
        );
        click(&mut app, &mut native, "nav-desktop-dock");
        assert_eq!(app.page, Page::DesktopDock);
        let _ = app.update(Message::Page(Page::DesktopProfiles));
        // Schedules closes the Setups page ("Switch automatically"); a tall
        // window shows it, as this harness does not scroll.
        let tall = Size::new(size.width, 2400.);
        let _ = app.update(Message::Resized(tall));
        let mut tall_native = Native::new(tall);
        click(&mut app, &mut tall_native, "presets-schedules");
        assert_eq!(app.page, Page::DesktopAutomation);
        assert_eq!(
            app.nav_model.active_data::<Page>(),
            Some(&Page::DesktopProfiles)
        );
        click(&mut app, &mut tall_native, "settings-back");
        assert_eq!(app.page, Page::DesktopProfiles);
        let _ = app.update(Message::Resized(size));
        let _ = app.update(Message::Page(Page::Overview));
        assert_eq!(
            serde_json::to_value(app.placement.session_draft()).unwrap(),
            draft
        );
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(Config::path().unwrap()).unwrap()).unwrap();
        assert_eq!(saved["future_preference"], 37);
        assert!(
            saved.get("widget_positions").is_some(),
            "navigation never applies the layout"
        );
        // Restore this fixture between sizes; no live preferences are accessible.
        std::fs::write(Config::path().unwrap(), &original).unwrap();
    }
}

#[test]
fn split_sidebar_keyboard_navigation_keeps_focus_and_drafts() {
    if child("split_sidebar_keyboard_navigation_keeps_focus_and_drafts") {
        return;
    }
    use cosmic::iced::keyboard::{self, key::Named};
    let before = std::fs::read(Config::path().unwrap()).unwrap();
    let (mut app, _) = Settings::init(Core::default(), Page::Overview);
    let _ = app.update(Message::Resized(Size::new(820., 600.)));
    let _ = app.update(Message::Placement(placement::Message::Select(
        "clock".into(),
    )));
    let _ = app.update(Message::Placement(placement::Message::Nudge([1, 0], false)));
    let draft = serde_json::to_value(app.placement.session_draft()).unwrap();
    let mut native = Native::new(Size::new(820., 600.));
    click(&mut app, &mut native, "nav-widgets");
    let key = |name| {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(name),
            modified_key: keyboard::Key::Named(name),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        })
    };
    // Arrows walk the menu in order, across its group headings.
    for (name, page) in [
        (Named::ArrowDown, Page::Overview),
        (Named::ArrowDown, Page::Appearance),
        (Named::ArrowDown, Page::DesktopProfiles),
        (Named::ArrowUp, Page::Appearance),
        (Named::Home, Page::DesktopWallpaper),
        (Named::ArrowUp, Page::DesktopWallpaper),
        (Named::End, Page::General),
        (Named::ArrowDown, Page::General),
    ] {
        let (messages, status) = native.event(app.settings_view(), key(name), Point::ORIGIN);
        assert_eq!(status, cosmic::iced::event::Status::Captured);
        assert!(!messages.is_empty(), "the sidebar keeps keyboard focus");
        for message in messages {
            let _ = app.update(message);
        }
        assert_eq!(app.page, page);
    }
    assert_eq!(
        serde_json::to_value(app.placement.session_draft()).unwrap(),
        draft
    );
    // An arrow in Search must edit the field, never select a sidebar destination.
    click(&mut app, &mut native, "settings-search");
    let (messages, _) = native.event(app.settings_view(), key(Named::ArrowDown), Point::ORIGIN);
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::NavSelected(_)))
    );
    assert_ne!(std::fs::read(Config::path().unwrap()).unwrap(), before);
}

#[test]
fn calendar_account_options_keep_disconnect_deliberate() {
    if child("calendar_account_options_keep_disconnect_deliberate") {
        return;
    }
    let before = std::fs::read(Config::path().unwrap()).unwrap();
    for size in [Size::new(820., 600.), Size::new(1024., 720.)] {
        let (mut app, _) = Settings::init(Core::default(), Page::Calendar);
        let _ = app.update(Message::Resized(size));
        app.calendar.connection.backend = crate::calendar::Backend::Google;
        app.calendar.connection.account = "demo@example.com".into();
        app.calendar.connection.calendar_name = "Work".into();
        app.calendar.loading = false;
        let mut native = Native::new(size);
        assert!(app.help_open.is_none());
        click(&mut app, &mut native, "calendar-account-options");
        assert_eq!(app.help_open, Some("calendar-account"));
        click(&mut app, &mut native, "calendar-disconnect");
        assert!(app.calendar.confirm_disconnect);
        assert!(app.calendar.connection.connected());
        click(&mut app, &mut native, "calendar-keep-connected");
        assert!(!app.calendar.confirm_disconnect);
        assert!(app.calendar.connection.connected());
        let _ = app.update(Message::Page(Page::Widgets));
        assert!(app.help_open.is_none());
    }
    assert_eq!(std::fs::read(Config::path().unwrap()).unwrap(), before);
}

#[test]
fn all_settings_render_in_cosmic_light_dark_and_high_contrast_without_saving() {
    if child("all_settings_render_in_cosmic_light_dark_and_high_contrast_without_saving") {
        return;
    }
    let before = std::fs::read(Config::path().unwrap()).unwrap();
    for size in [Size::new(820., 600.), Size::new(1024., 720.)] {
        for (name, theme) in [
            ("dark", cosmic::Theme::dark()),
            ("light", cosmic::Theme::light()),
            ("contrast", cosmic::Theme::dark_hc()),
        ] {
            let mut native = Native::new(size);
            for page in Page::ALL
                .into_iter()
                .filter(|page| *page != Page::WidgetLayout)
            {
                let (mut app, _) = Settings::init(Core::default(), page);
                let _ = app.update(Message::Resized(size));
                let pixels = native.render(app.settings_view(), &theme);
                if let Some(directory) = std::env::var_os("DESKTOP_CARDS_LAYOUT_EVIDENCE") {
                    let path = std::path::PathBuf::from(directory).join(format!(
                        "cosmic-{name}-{}-{}.png",
                        page.key(),
                        size.width
                    ));
                    image::save_buffer(
                        path,
                        &pixels,
                        size.width as u32,
                        size.height as u32,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
                // The main background follows the rendering theme, even in a headless test.
                let expected: cosmic::iced::Color = theme.cosmic().bg_color().into();
                let index = ((size.height as usize - 2) * size.width as usize + 190) * 4;
                for (got, want) in pixels[index..index + 3]
                    .iter()
                    .zip([expected.r, expected.g, expected.b])
                {
                    assert!(
                        (*got as f32 - want * 255.).abs() < 3.,
                        "{name} {page:?}: background follows COSMIC"
                    );
                }
            }
        }
    }
    assert_eq!(std::fs::read(Config::path().unwrap()).unwrap(), before);
}

#[test]
fn settings_follow_the_system_theme_independently_of_widget_colors() {
    if child("settings_follow_the_system_theme_independently_of_widget_colors") {
        return;
    }
    // libcosmic reads the light and dark themes from COSMIC's installed
    // defaults; without COSMIC (a build server) there is nothing to follow.
    if !std::path::Path::new("/usr/share/cosmic/com.system76.CosmicTheme.Light").exists() {
        eprintln!("COSMIC's theme defaults aren't installed; skipped");
        return;
    }
    let mode = std::path::PathBuf::from(std::env::var_os("XDG_CONFIG_HOME").unwrap())
        .join("cosmic/com.system76.CosmicTheme.Mode/v1/is_dark");
    std::fs::create_dir_all(mode.parent().unwrap()).unwrap();
    let before = std::fs::read(Config::path().unwrap()).unwrap();
    let mut config = Config::try_load().unwrap();
    for is_dark in [false, true] {
        std::fs::write(&mode, is_dark.to_string()).unwrap();
        let expected = controls_theme(&config);
        assert_eq!(expected.theme_type.is_dark(), is_dark);
        assert!(matches!(
            expected.theme_type,
            cosmic::theme::ThemeType::System { .. }
        ));
        for palette in ["lavender", "amber", "teal"] {
            config.theme = palette.into();
            assert_eq!(controls_theme(&config), expected);
        }
    }
    assert_eq!(std::fs::read(Config::path().unwrap()).unwrap(), before);
}

#[test]
fn visual_preview_selection_reorder_and_readability_have_separate_effects() {
    if child("visual_preview_selection_reorder_and_readability_have_separate_effects") {
        return;
    }
    Config::save_changes(json!({"appearance":{"opacity":70}, "location":{"name":"Zagreb","latitude":45.8,"longitude":15.9}})).unwrap();
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    let background = store.user_path(&crate::desktop::wallpaper::shared_key());
    assert!(background.starts_with(std::env::var_os("DESKTOP_CARDS_DESKTOP_ROOT").unwrap()));
    std::fs::create_dir_all(background.parent().unwrap()).unwrap();
    let picture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/wallpapers/teal.jpg");
    std::fs::write(background,format!("(output: \"all\", source: Path({}), filter_by_theme: false, rotation_frequency: 3600, filter_method: Lanczos, scaling_mode: Zoom, sampling_method: Alphanumeric)",serde_json::to_string(&picture).unwrap())).unwrap();
    let before = std::fs::read(Config::path().unwrap()).unwrap();
    for size in [Size::new(820., 600.), Size::new(1024., 720.)] {
        let (mut app, _) = Settings::init(Core::default(), Page::Appearance);
        // Complete the normal background jobs synchronously in this isolated
        // renderer; the init Task is deliberately never launched by tests.
        app.profiles = profiles::ProfilesEditor::default();
        app.profiles.open_home();
        for job in app.profiles.take_jobs() {
            app.profiles.update(job.run());
        }
        assert!(app.profiles.home_image().is_some());
        let _ = app.update(Message::Resized(size));
        let mut native = Native::new(size);
        for widget in [Widget::Weather, Widget::Devbox, Widget::Clock] {
            let _ = app.update(Message::AppearancePreview(widget));
            assert_eq!(app.appearance_preview, widget);
            native.capture(
                app.settings_view(),
                &cosmic::Theme::dark(),
                &format!("polish-style-{}-{}", widget.key(), size.width),
            );
            if size.width >= 990. {
                let stage = native.bounds(app.settings_view(), "widget-preview-stage");
                let pixels = native.render(app.settings_view(), &cosmic::Theme::dark());
                let index = ((stage.center_y() as usize) * size.width as usize
                    + (stage.x as usize - 5))
                    * 4;
                let background: cosmic::iced::Color =
                    cosmic::Theme::dark().cosmic().bg_color().into();
                for (got, want) in
                    pixels[index..index + 3]
                        .iter()
                        .zip([background.r, background.g, background.b])
                {
                    assert!(
                        (*got as f32 - want * 255.).abs() < 3.,
                        "Wallpaper stays inside the preview stage"
                    );
                }
            }
        }
        assert_eq!(
            std::fs::read(Config::path().unwrap()).unwrap(),
            before,
            "Inspecting samples saves nothing"
        );
        // Legacy controls moved below the material chooser into Advanced.
        // The pointer driver's bounds are content coordinates, so use a tall
        // fixture for these legacy controls; preview checks above keep both
        // original viewports. The compact material controls have their own test.
        let tall = Size::new(size.width, 2200.);
        let _ = app.update(Message::Resized(tall));
        native = Native::new(tall);
        app.system_animations = false;
        click(&mut app, &mut native, "settings-advanced");
        click(&mut app, &mut native, "contrast-assist");
        assert!(Config::load().appearance.contrast_assist);
        assert_eq!(Config::load().appearance.opacity, 70);
        click(&mut app, &mut native, "settings-undo");
        assert!(!Config::load().appearance.contrast_assist);
        let _ = app.update(Message::Resized(size));
        native = Native::new(size);
        let _ = app.update(Message::Page(Page::Devbox));
        let previous = app.config.devbox.clone();
        click(&mut app, &mut native, "devbox-details-memory");
        assert!(app.devbox_details.contains(&DevboxSection::Memory));
        assert_eq!(
            app.config.devbox, previous,
            "Details must not toggle the reading"
        );
        click(&mut app, &mut native, "devbox-reorder");
        assert!(app.devbox_reordering);
        native.capture(
            app.settings_view(),
            &cosmic::Theme::dark(),
            &format!("polish-reorder-{}", size.width),
        );
        let original = app.config.devbox.ordered_sections();
        click(&mut app, &mut native, "devbox-move-cpu-1");
        assert_eq!(app.config.devbox.ordered_sections()[1], DevboxSection::Cpu);
        click(&mut app, &mut native, "settings-undo");
        assert_eq!(app.config.devbox.ordered_sections(), original);
        let _ = app.update(Message::Page(Page::Clock));
        assert!(!app.devbox_reordering);
        for page in [
            Page::Clock,
            Page::Projects,
            Page::Music,
            Page::Notes,
            Page::Focus,
            Page::DesktopProfiles,
        ] {
            let _ = app.update(Message::Page(page));
            native.capture(
                app.settings_view(),
                &cosmic::Theme::dark(),
                &format!("polish-{}-{}", page.key(), size.width),
            );
        }
        // The fixture has no accounts or project folders. Both illustrative
        // renderers still work, including the combined calendar's Tomorrow.
        for combined in [false, true] {
            let _ = app.update(Message::Page(Page::Calendar));
            app.config.glance.enabled = combined;
            app.config.glance.show_tomorrow = true;
            app.config.calendar_show_tomorrow = true;
            app.samples.update(&app.config);
            native.capture(
                app.settings_view(),
                &cosmic::Theme::dark(),
                &format!("polish-calendar-{combined}-{}", size.width),
            );
        }
        std::fs::write(Config::path().unwrap(), &before).unwrap();
    }
}

#[test]
fn motion_preference_ignores_stale_reads_and_can_be_undone() {
    if child("motion_preference_ignores_stale_reads_and_can_be_undone") {
        return;
    }
    let (mut app, _) = Settings::init(Core::default(), Page::General);
    let generation = app.motion_generation;
    let _ = app.update(Message::SystemMotion(generation, true));
    assert!(app.system_animations);
    let _ = app.update(Message::ReadSystemMotion);
    let _ = app.update(Message::SystemMotion(generation, true));
    let _ = app.update(Message::SystemMotion(app.motion_generation, false));
    assert!(!app.system_animations);
    let _ = app.update(Message::SystemMotion(generation, true));
    assert!(!app.system_animations);
    let _ = app.update(Message::SettingsAnimations(false));
    assert!(!Config::load().settings_animations);
    let _ = app.update(Message::UndoPreferences);
    assert!(Config::load().settings_animations);
    assert!(
        !app.system_animations,
        "The app toggle never overrides Reduce motion"
    );
}

#[test]
fn review_r1_gallery_keyboard_actions_are_reachable() {
    if child("review_r1_gallery_keyboard_actions_are_reachable") {
        return;
    }
    use cosmic::iced::advanced::{
        clipboard,
        renderer::Headless,
        widget::{
            Operation,
            operation::{Focusable, Outcome as OpOutcome, focusable},
        },
    };
    use cosmic::iced::runtime::user_interface::{Cache, UserInterface};
    use cosmic::iced::{Rectangle, keyboard};
    struct Focused(Option<Rectangle>);
    impl Operation for Focused {
        fn traverse(&mut self, f: &mut dyn FnMut(&mut dyn Operation)) {
            f(self);
        }
        fn focusable(
            &mut self,
            _: Option<&cosmic::widget::Id>,
            bounds: Rectangle,
            s: &mut dyn Focusable,
        ) {
            if s.is_focused() {
                self.0 = Some(bounds);
            }
        }
    }
    let mut renderer =
        cosmic::iced::futures::executor::block_on(<cosmic::Renderer as Headless>::new(
            cosmic::font::default(),
            cosmic::iced::Pixels(14.),
            Some("tiny-skia"),
        ))
        .unwrap();
    let mut missing = Vec::new();
    for size in [Size::new(820., 600.), Size::new(1024., 720.)] {
        for matching in [false, true] {
            let (mut app, _) = Settings::init(Core::default(), Page::Themes);
            let _ = app.update(Message::Resized(size));
            app.profiles
                .update(profiles::Message::Select(profiles::Selection::Preset(
                    "theme-soft",
                )));
            app.profiles
                .update(profiles::Message::ThemeWallpaper(matching));
            for job in app.profiles.take_theme_jobs() {
                app.profiles.update(job.run());
            }
            assert!(
                app.profiles.can_apply(),
                "fixture must have a ready preview: {:?}",
                app.profiles.plan()
            );
            let mut ui =
                UserInterface::build(app.settings_view(), size, Cache::default(), &mut renderer);
            let mut reached = std::collections::BTreeSet::new();
            for _ in 0..100 {
                let mut operation: Box<dyn Operation> = Box::new(focusable::focus_next());
                loop {
                    ui.operate(&renderer, operation.as_mut());
                    match operation.finish() {
                        OpOutcome::Chain(next) => operation = next,
                        _ => break,
                    }
                }
                let mut bounds = Focused(None);
                ui.operate(&renderer, &mut bounds);
                for key in [
                    keyboard::Key::Named(keyboard::key::Named::Enter),
                    keyboard::Key::Character(" ".into()),
                ] {
                    let event = Event::Keyboard(keyboard::Event::KeyPressed {
                        key: key.clone(),
                        modified_key: key,
                        physical_key: keyboard::key::Physical::Unidentified(
                            keyboard::key::NativeCode::Unidentified,
                        ),
                        location: keyboard::Location::Standard,
                        modifiers: keyboard::Modifiers::empty(),
                        text: None,
                        repeat: false,
                    });
                    let mut messages = Vec::new();
                    ui.update(
                        &[event],
                        mouse::Cursor::Available(Point::new(2., 2.)),
                        &mut renderer,
                        &mut clipboard::Null,
                        &mut messages,
                    );
                    for message in messages {
                        let label = format!("{message:?}");
                        if matches!(
                            message,
                            Message::Profiles(profiles::Message::ThemeWallpaper(_))
                        ) && !reached.contains(&label)
                        {
                            let bounds = bounds.0.expect("matching wallpaper has keyboard focus");
                            assert!(
                                bounds.x >= 0.
                                    && bounds.y >= 0.
                                    && bounds.x + bounds.width <= size.width
                                    && bounds.y + bounds.height <= size.height,
                                "primary option must be visible: {bounds:?}"
                            );
                            ui.draw(
                                &mut renderer,
                                &cosmic::Theme::dark(),
                                &cosmic::iced::advanced::renderer::Style::default(),
                                mouse::Cursor::Unavailable,
                            );
                            let pixels = renderer.screenshot(
                                Size::new(size.width as u32, size.height as u32),
                                1.,
                                cosmic::iced::Color::TRANSPARENT,
                            );
                            if let Some(dir) = std::env::var_os("DESKTOP_CARDS_LAYOUT_EVIDENCE") {
                                let dir = std::path::PathBuf::from(dir);
                                std::fs::create_dir_all(&dir).unwrap();
                                image::save_buffer(
                                    dir.join(format!(
                                        "theme-wallpaper-keyboard-{matching}-{}.png",
                                        size.width
                                    )),
                                    &pixels,
                                    size.width as u32,
                                    size.height as u32,
                                    image::ColorType::Rgba8,
                                )
                                .unwrap();
                            }
                        }
                        if reached.insert(label.clone()) {
                            println!("{size:?}: {label}, focused bounds {:?}", bounds.0);
                        }
                    }
                }
            }
            for expected in [
                // Styles are chosen in the browser this row opens.
                "StyleBrowser(true)",
                "Profiles(ThemeColors(true))",
                "Profiles(ThemeColors(false))",
                if matching {
                    "Profiles(ThemeWallpaper(false))"
                } else {
                    "Profiles(ThemeWallpaper(true))"
                },
                "Profiles(Name(Some(CaptureTheme)))",
                "Profiles(ImportTheme)",
                "Profiles(Name(Some(Duplicate)))",
                "Profiles(Export)",
                "Profiles(Apply)",
            ] {
                if !reached.contains(expected) {
                    missing.push(format!("{size:?}: keyboard cannot activate {expected}"));
                }
            }
            drop(ui);
            let _ = app.update(Message::Profiles(profiles::Message::ThemeWallpaper(
                !matching,
            )));
            assert_eq!(
                app.profiles.profile().unwrap().wallpaper.is_some(),
                !matching
            );
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[test]
fn windows_page_ends_with_workspaces_and_the_menu_lists_every_page() {
    if child("windows_page_ends_with_workspaces_and_the_menu_lists_every_page") {
        return;
    }
    let size = Size::new(1020., 1500.);
    let (mut app, _) = Settings::init(Core::default(), Page::DesktopWorkspaces);
    let _ = app.update(Message::Resized(size));
    assert_eq!(app.page, Page::DesktopWindows, "Workspaces opens Windows");
    let mut native = Native::new(size);
    native.capture(app.view(), &cosmic::Theme::dark(), "windows-with-workspaces");
    assert!(native.try_bounds(app.view(), "windows-workspaces").is_some());
    let mut last = 0.;
    for (_, pages) in Page::SIDEBAR {
        for page in pages {
            let entry = native.bounds(app.view(), &format!("nav-{}", page.key()));
            assert!(entry.y > last, "{page:?} follows the previous entry");
            last = entry.y;
        }
    }
}

#[test]
fn color_control_offers_suggestions_and_the_picker_and_applies_a_choice() {
    if child("color_control_offers_suggestions_and_the_picker_and_applies_a_choice") {
        return;
    }
    let size = Size::new(1020., 1300.);
    let (mut app, _) = Settings::init(Core::default(), Page::Themes);
    let _ = app.update(Message::Resized(size));
    let mut native = Native::new(size);
    assert!(
        native.try_bounds(app.settings_view(), "color-panel").is_none(),
        "closed until asked"
    );
    click(&mut app, &mut native, "colors-hex");
    assert!(native.try_bounds(app.settings_view(), "color-panel").is_some());
    native.capture(app.settings_view(), &cosmic::Theme::dark(), "color-control-open");
    click(&mut app, &mut native, "color-quick-2");
    let chosen = crate::settings::color_well::QUICK[2];
    assert_eq!(Config::load().appearance.accent, Some(chosen));
    assert!(
        native.try_bounds(app.settings_view(), "color-panel").is_none(),
        "a choice closes the panel"
    );
    native.capture(app.settings_view(), &cosmic::Theme::dark(), "color-control-chosen");
}

#[test]
fn widget_fonts_list_installed_families_and_save_a_choice() {
    if child("widget_fonts_list_installed_families_and_save_a_choice") {
        return;
    }
    // Advanced now follows the material chooser; keep all font controls in
    // the pointer driver's content-coordinate viewport.
    let size = Size::new(1020., 2200.);
    let (mut app, _) = Settings::init(Core::default(), Page::Appearance);
    let _ = app.update(Message::Resized(size));
    let mut native = Native::new(size);
    assert!(native.try_bounds(app.settings_view(), "font-list-text").is_none());
    app.system_animations = false;
    click(&mut app, &mut native, "settings-advanced");
    click(&mut app, &mut native, "font-text");
    assert!(native.try_bounds(app.settings_view(), "font-list-text").is_some());
    native.capture(app.settings_view(), &cosmic::Theme::dark(), "widget-fonts-open");
    let _ = app.update(Message::FontSearch("zzz-no-such-font".into()));
    assert!(app.widget_font_families().is_empty());
    let _ = app.update(Message::FontSearch(String::new()));
    let _ = app.update(Message::WidgetFont(FontTarget::Text, Some("Noto Sans Mono".into())));
    assert_eq!(Config::load().widget_font.as_deref(), Some("Noto Sans Mono"));
    assert!(app.font_menu.is_none(), "a choice closes the list");
    let _ = app.update(Message::WidgetFont(FontTarget::Clock, Some("Noto Serif".into())));
    assert_eq!(Config::load().clock_font.as_deref(), Some("Noto Serif"));
    native.capture(app.settings_view(), &cosmic::Theme::dark(), "widget-fonts-chosen");
    let _ = app.update(Message::WidgetFont(FontTarget::Text, None));
    assert_eq!(Config::load().widget_font, None, "back to the style's font");
}

#[test]
fn styles_browser_filters_previews_and_returns_to_colors_and_theme() {
    let size = Size::new(1024., 720.);
    let (mut app, _) = Settings::init(Core::default(), Page::Themes);
    let _ = app.update(Message::Resized(size));
    let _ = app.update(Message::StyleBrowser(true));
    assert!(app.style_browser);
    // Each filter shows exactly its styles.
    for filter in StyleFilter::ALL {
        let _ = app.update(Message::StyleFilter(filter));
        let mut native = crate::native_test::Native::new(size);
        for family in crate::desktop_theme::Family::ALL {
            let shown = native
                .try_bounds(app.settings_view(), &format!("preset-{}", family.preset()))
                .is_some();
            assert_eq!(shown, filter.shows(family), "{filter:?} {family:?}");
        }
    }
    assert!(StyleFilter::Light.shows(crate::desktop_theme::Family::Paper));
    assert!(!StyleFilter::Light.shows(crate::desktop_theme::Family::Neon));
    assert!(StyleFilter::SeeThrough.shows(crate::desktop_theme::Family::Frosted));
    // Choosing a tile previews that style; Back keeps the preview; leaving
    // the page closes the browser.
    let _ = app.update(Message::StyleFilter(StyleFilter::All));
    let _ = app.update(Message::Profiles(profiles::Message::Select(
        profiles::Selection::Preset("theme-neon"),
    )));
    assert_eq!(
        *app.profiles.selection(),
        profiles::Selection::Preset("theme-neon")
    );
    let _ = app.update(Message::StyleBrowser(false));
    assert!(!app.style_browser);
    assert_eq!(
        *app.profiles.selection(),
        profiles::Selection::Preset("theme-neon")
    );
    let _ = app.update(Message::StyleBrowser(true));
    let _ = app.update(Message::Page(Page::Widgets));
    let _ = app.update(Message::Page(Page::Themes));
    assert!(!app.style_browser, "the browser is a view of one visit");
}
