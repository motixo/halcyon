//! First-use and Themes journeys rendered by the production Settings view in
//! the headless native renderer. Every run uses a private fixture: the user's
//! preferences, desktop and accounts are never read or written.
use super::*;
use crate::native_test::Native;

/// Runs `name` again in a child process with private XDG, config and desktop
/// roots, like the other native Settings regressions.
fn child(name: &str, config: &str) -> bool {
    if std::env::var("DESKTOP_CARDS_UX_TEST").as_deref() == Ok(name) {
        return false;
    }
    let root = std::env::temp_dir().join(format!("cards-journey-{}-{name}", std::process::id()));
    for dir in ["config", "data", "cache", "state", "desktop"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join("dconf-profile"), "").unwrap();
    std::fs::write(root.join("config.json"), config).unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("settings::theme_journey_tests::{name}")])
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

fn desktop_root() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("DESKTOP_CARDS_DESKTOP_ROOT").unwrap())
}

/// Writes one COSMIC user value inside the private fixture root.
fn cosmic(namespace: &str, version: u64, key: &str, value: &str) {
    let dir = desktop_root()
        .join("config/cosmic")
        .join(namespace)
        .join(format!("v{version}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(key), value).unwrap();
}

fn wallpaper(path: &std::path::Path) {
    use crate::desktop::wallpaper as backgrounds;
    cosmic(
        backgrounds::NAMESPACE,
        1,
        "all",
        &ron::to_string(&backgrounds::BgEntry::new(
            "all",
            backgrounds::BgSource::Path(path.to_path_buf()),
        ))
        .unwrap(),
    );
    cosmic(backgrounds::NAMESPACE, 1, "same-on-all", "true");
}

/// The state the user reported: a dark accent from an exact wallpaper swatch
/// on both COSMIC variants, and an older custom active-window outline.
fn reported_cosmic_state(dark: bool) {
    for variant in ["Dark", "Light"] {
        cosmic(
            &format!("com.system76.CosmicTheme.{variant}.Builder"),
            2,
            "accent",
            "Some(\"#40352CFF\")",
        );
    }
    cosmic(
        "com.system76.CosmicTheme.Dark.Builder",
        2,
        "window_hint",
        "Some(\"#63D0DFFF\")",
    );
    cosmic(
        "com.system76.CosmicTheme.Mode",
        1,
        "is_dark",
        &dark.to_string(),
    );
}

/// Background work runs synchronously here, as the app's task runner would.
fn drive(app: &mut Settings) {
    for _ in 0..32 {
        let wallpaper = app.wallpaper.take_jobs();
        let backdrops = app.profiles.take_jobs();
        let themes = app.profiles.take_theme_jobs();
        if wallpaper.is_empty() && backdrops.is_empty() && themes.is_empty() {
            return;
        }
        for job in wallpaper {
            app.wallpaper.update(job.run());
        }
        for job in themes {
            app.profiles.update(job.run());
        }
        for job in backdrops {
            app.profiles.update(job.run());
        }
    }
}

/// Navigates like the sidebar, then reopens the editors whose background
/// work `update` turned into tasks this fixture never launches.
fn goto(app: &mut Settings, page: Page) {
    let _ = app.update(Message::Page(page));
    app.profiles = profiles::ProfilesEditor::default();
    app.profiles.open();
    app.profiles.open_home();
    if matches!(page, Page::DesktopWallpaper | Page::Appearance) {
        app.wallpaper = wallpaper::WallpaperEditor::default();
        app.wallpaper.open();
    }
    drive(app);
}

fn themes(app: &mut Settings, message: profiles::Message) {
    let _ = app.profiles.update(message);
    drive(app);
}

fn prefix() -> String {
    std::env::var("DESKTOP_CARDS_JOURNEY_PREFIX").unwrap_or_else(|_| "journey".into())
}

/// The page top, then each further screenful the viewport can show.
fn capture_page(app: &Settings, size: Size, theme: &cosmic::Theme, name: &str, screens: usize) {
    let mut native = Native::new(size);
    let _ = native.render(app.settings_view(), theme);
    for screen in 0..screens {
        native.scroll(
            app.settings_view(),
            "settings-content",
            screen as f32 * (size.height - 180.),
        );
        native.capture(
            app.settings_view(),
            theme,
            &format!("{}-{name}-{}-{screen}", prefix(), size.width),
        );
    }
}

const FIRST_RUN: &str = r#"{"enabled_widgets":["clock","calendar","weather","devbox","projects"],"location":{"name":"Sample city","latitude":45.0,"longitude":16.0}}"#;
const REPORTED: &str = r#"{"setup_dismissed":true,"theme":"amber","enabled_widgets":["clock","calendar","weather","devbox","projects"],"location":{"name":"Sample city","latitude":45.0,"longitude":16.0},"appearance":{"opacity":92,"corner_radius":24,"border_opacity":20,"color_source":"custom","accent":[64,53,44],"sync_colors":true,"wallpaper_colors":true}}"#;

fn amber() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/wallpapers/amber-1080p.png")
}

#[test]
fn journey_first_run_setup_and_themes() {
    if child("journey_first_run_setup_and_themes", FIRST_RUN) {
        return;
    }
    wallpaper(&amber());
    reported_cosmic_state(true);
    for size in [Size::new(1024., 720.), Size::new(820., 600.)] {
        let (mut app, _) = Settings::init(Core::default(), Page::Overview);
        let _ = app.update(Message::Resized(size));
        goto(&mut app, Page::Overview);
        let _ = app.update(Message::SetupOpen(true));
        drive(&mut app);
        capture_page(&app, size, &cosmic::Theme::dark(), "setup", 2);
        goto(&mut app, Page::Themes);
        capture_page(&app, size, &cosmic::Theme::dark(), "themes-first", 3);
    }
}

#[test]
fn journey_reported_wallpaper_colors_and_outline() {
    if child("journey_reported_wallpaper_colors_and_outline", REPORTED) {
        return;
    }
    wallpaper(&amber());
    reported_cosmic_state(true);
    for size in [Size::new(1024., 720.), Size::new(820., 600.)] {
        let (mut app, _) = Settings::init(Core::default(), Page::Themes);
        let _ = app.update(Message::Resized(size));
        goto(&mut app, Page::Themes);
        capture_page(&app, size, &cosmic::Theme::dark(), "themes-reported", 3);
        themes(
            &mut app,
            profiles::Message::Select(profiles::Selection::Preset("theme-soft")),
        );
        capture_page(&app, size, &cosmic::Theme::dark(), "themes-soft", 3);
        // The whole choice (the style row, color, wallpaper) fits without
        // scrolling; the styles themselves are one click away.
        let mut native = Native::new(size);
        for id in [
            "theme-browse-styles",
            "theme-color-wallpaper",
            "theme-color-custom",
            "theme-matching-wallpaper",
        ] {
            let bounds = native.bounds(app.settings_view(), id);
            assert!(
                bounds.y >= 0. && bounds.y + bounds.height <= size.height - 48.,
                "{id} is visible at {size:?}: {bounds:?}"
            );
        }
        // The browser shows the first styles without scrolling, and leaves
        // the previewed look selected.
        let _ = app.update(Message::StyleBrowser(true));
        for id in ["styles-back", "preset-theme-soft", "preset-theme-outline"] {
            let bounds = native.bounds(app.settings_view(), id);
            assert!(
                bounds.y >= 0. && bounds.y + bounds.height <= size.height - 48.,
                "{id} is visible in the browser at {size:?}: {bounds:?}"
            );
        }
        capture_page(&app, size, &cosmic::Theme::dark(), "themes-styles", 2);
        let _ = app.update(Message::StyleBrowser(false));
        let style = |pointer: &str| {
            app.profiles
                .profile()
                .and_then(|p| p.widget_style.as_ref())
                .and_then(|s| s.pointer(pointer))
                .cloned()
        };
        assert_eq!(
            style("/appearance/wallpaper_colors"),
            Some(true.into()),
            "a wallpaper-colors user keeps that choice"
        );
        let accent: [u8; 3] = serde_json::from_value(style("/appearance/accent").unwrap()).unwrap();
        let tone = crate::desktop_theme::tone::lch(accent);
        assert!(
            tone.l > 0.7 && tone.c > 0.08,
            "not the dark swatch: {accent:?}"
        );
        themes(
            &mut app,
            profiles::Message::Select(profiles::Selection::Preset("theme-paper")),
        );
        capture_page(&app, size, &cosmic::Theme::dark(), "themes-paper", 2);
        goto(&mut app, Page::DesktopWallpaper);
        capture_page(&app, size, &cosmic::Theme::dark(), "wallpaper-colors", 2);
        goto(&mut app, Page::Appearance);
        capture_page(&app, size, &cosmic::Theme::dark(), "widget-style", 1);
    }
}

fn system_wallpaper(file: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new("/usr/share/backgrounds/cosmic").join(file);
    path.is_file().then_some(path)
}

#[test]
fn journey_apply_customize_and_follow_other_wallpapers() {
    if child(
        "journey_apply_customize_and_follow_other_wallpapers",
        REPORTED,
    ) {
        return;
    }
    wallpaper(&amber());
    reported_cosmic_state(true);
    let size = Size::new(1024., 720.);
    let (mut app, _) = Settings::init(Core::default(), Page::Themes);
    let _ = app.update(Message::Resized(size));
    goto(&mut app, Page::Themes);
    // The reported desktop has no theme yet: Wallpaper & colors offers the
    // explicit outline link next to its colors.
    goto(&mut app, Page::DesktopWallpaper);
    capture_page(&app, size, &cosmic::Theme::dark(), "wallpaper-outline", 1);
    goto(&mut app, Page::Themes);
    themes(
        &mut app,
        profiles::Message::Select(profiles::Selection::Preset("theme-soft")),
    );
    let _ = app.update(Message::Profiles(profiles::Message::Apply));
    assert_eq!(
        app.config.appearance.family,
        Some(crate::desktop_theme::Family::Soft)
    );
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    let native = crate::desktop::appearance::AppearanceDraft::load(&store).unwrap();
    for variant in crate::desktop::appearance::ThemeVariant::ALL {
        assert_eq!(
            native
                .color(variant, crate::desktop::appearance::ColorRole::Outline)
                .known(),
            Some(&None),
            "the older outline color now follows the accent"
        );
    }
    goto(&mut app, Page::Themes);
    capture_page(&app, size, &cosmic::Theme::dark(), "themes-applied", 3);
    themes(
        &mut app,
        profiles::Message::ThemeCustom(Some("#5E81AC".into())),
    );
    capture_page(&app, size, &cosmic::Theme::dark(), "themes-custom", 1);
    themes(&mut app, profiles::Message::Discard);
    goto(&mut app, Page::DesktopWallpaper);
    capture_page(&app, size, &cosmic::Theme::dark(), "wallpaper-themed", 1);
    if let Some(orion) = system_wallpaper("orion_nebula_nasa_heic0601a.jpg") {
        wallpaper(&orion);
        goto(&mut app, Page::Themes);
        themes(
            &mut app,
            profiles::Message::Select(profiles::Selection::Preset("theme-outline")),
        );
        capture_page(&app, size, &cosmic::Theme::dark(), "themes-orion", 1);
    }
    if let Some(light) = system_wallpaper("COSMIC-Pattern-Light.png") {
        wallpaper(&light);
        cosmic("com.system76.CosmicTheme.Mode", 1, "is_dark", "false");
        goto(&mut app, Page::Themes);
        themes(
            &mut app,
            profiles::Message::Select(profiles::Selection::Preset("theme-paper")),
        );
        capture_page(&app, size, &cosmic::Theme::light(), "themes-light-mode", 1);
        let small = Size::new(820., 600.);
        let _ = app.update(Message::Resized(small));
        capture_page(&app, small, &cosmic::Theme::light(), "themes-light-mode", 1);
    }
}
