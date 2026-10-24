//! First-run setup through the production Settings view in the headless
//! renderer, on a private fixture desktop: every screen at the smallest and
//! a common window size, a look and dock choices applied live, Cancel
//! putting the desktop back and Finish keeping it.
use super::*;
use crate::native_test::Native;
use cosmic::iced::{Event, mouse};

const DOCK: &str = "com.system76.CosmicPanel.Dock";
const PANEL: &str = "com.system76.CosmicPanel.Panel";

fn child(name: &str) -> bool {
    if std::env::var("DESKTOP_CARDS_WIZARD_TEST").as_deref() == Ok(name) {
        return false;
    }
    let root = std::env::temp_dir().join(format!("cards-wizard-{}-{name}", std::process::id()));
    for dir in ["config", "data", "cache", "state", "desktop"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join("dconf-profile"), "").unwrap();
    std::fs::write(root.join("config.json"), "{}").unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("settings::wizard_tests::{name}"),
            "--include-ignored",
            "--nocapture",
        ])
        .env("DESKTOP_CARDS_WIZARD_TEST", name)
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
    if matches!(name, "responsiveness_probe" | "rendering_probe") {
        eprint!("{}", String::from_utf8_lossy(&result.stdout));
    }
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

/// A stock dock and panel, and the bundled wallpapers where the installer
/// puts them.
fn seed_desktop() {
    let system = |namespace: &str, key: &str, value: &str| {
        let path = desktop_root()
            .join("system/cosmic")
            .join(namespace)
            .join("v1")
            .join(key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    };
    system(
        "com.system76.CosmicPanel",
        "entries",
        "[\"Panel\",\"Dock\"]",
    );
    system(DOCK, "anchor", "Bottom");
    system(
        DOCK,
        "plugins_center",
        "Some([\"com.system76.CosmicAppList\"])",
    );
    system(PANEL, "anchor", "Top");
    system(
        PANEL,
        "plugins_wings",
        "Some(([\"com.system76.CosmicPanelWorkspacesButton\"], [\"com.system76.CosmicAppletTime\"]))",
    );
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    let bundled = crate::desktop::wallpaper::bundled_dir(&store);
    std::fs::create_dir_all(&bundled).unwrap();
    for (name, _) in crate::desktop::wallpaper::BUNDLED {
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("assets/wallpapers/{name}.jpg")),
            crate::desktop::wallpaper::bundled_path(&bundled, name),
        )
        .unwrap();
    }
}

/// The user's own value of a COSMIC key, if they have one.
fn user(namespace: &str, key: &str) -> Option<String> {
    std::fs::read_to_string(
        desktop_root()
            .join("config/cosmic")
            .join(namespace)
            .join("v1")
            .join(key),
    )
    .ok()
}

fn click(app: &mut Settings, native: &mut Native, id: &str) {
    let bounds = native.bounds(app.settings_view(), id);
    assert!(
        bounds.x >= 0.
            && bounds.x + bounds.width <= native.size.width
            && bounds.y >= 0.
            && bounds.y + bounds.height <= native.size.height,
        "{id} is off screen: {bounds:?}"
    );
    let point = bounds.center();
    for input in [
        mouse::Event::CursorMoved { position: point },
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        for message in native
            .event(app.settings_view(), Event::Mouse(input), point)
            .0
        {
            let _ = app.update(message);
        }
    }
}

fn open(size: Size) -> (Settings, Native) {
    OPEN_WIZARD.store(true, std::sync::atomic::Ordering::Relaxed);
    let (mut app, _) = Settings::init(Core::default(), Page::Overview);
    let _ = app.update(Message::Resized(size));
    assert!(app.wizard.is_some());
    (app, Native::new(size))
}

fn step(app: &Settings) -> wizard::Step {
    app.wizard.as_ref().unwrap().step
}

/// Explicit diagnostic, not a wall-time CI assertion. Uses an isolated desktop
/// and the real widget tree; software-rendered timings are not GPU frame rates.
#[test]
#[ignore]
fn responsiveness_probe() {
    if child("responsiveness_probe") {
        return;
    }
    seed_desktop();
    let measure = |label: &str, f: &mut dyn FnMut()| {
        let start = std::time::Instant::now();
        f();
        println!(
            "PERF {label}: {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.
        );
    };
    let start = std::time::Instant::now();
    let (mut app, mut native) = open(Size::new(1280., 800.));
    println!(
        "PERF init_and_renderer: {:.3} ms",
        start.elapsed().as_secs_f64() * 1000.
    );
    let theme = controls_theme(&app.config);
    measure("welcome_cold_view", &mut || {
        drop(app.settings_view());
    });
    measure("welcome_render", &mut || {
        native.render(app.settings_view(), &theme);
    });
    measure("start_click", &mut || {
        click(&mut app, &mut native, "wizard-start");
    });
    measure("look_cold_view", &mut || {
        drop(app.settings_view());
    });
    measure("look_warm_views_20", &mut || {
        for _ in 0..20 {
            drop(app.settings_view());
        }
    });
    app.wizard.as_mut().unwrap().step = wizard::Step::Bars;
    measure("bars_resize_views_80", &mut || {
        for n in 0..80 {
            app.window = Size::new(820. + n as f32 * 3., 600. + n as f32);
            drop(app.settings_view());
        }
    });
    app.wizard = None;
    app.window = Size::new(1280., 800.);
    for page in [
        Page::Overview,
        Page::Themes,
        Page::General,
        Page::DesktopDock,
        Page::Calendar,
        Page::Appearance,
    ] {
        measure(&format!("page_{page:?}"), &mut || {
            let _ = app.update(Message::Page(page));
        });
        measure(&format!("view_{page:?}_20"), &mut || {
            for _ in 0..20 {
                drop(app.settings_view());
            }
        });
        if matches!(page, Page::Themes | Page::General | Page::DesktopDock) {
            native.render(app.settings_view(), &theme);
            measure(&format!("software_scroll_{page:?}_10"), &mut || {
                for _ in 0..10 {
                    native.event(
                        app.settings_view(),
                        Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Lines { x: 0., y: -1. },
                        }),
                        cosmic::iced::Point::new(700., 400.),
                    );
                    native.render(app.settings_view(), &theme);
                }
            });
        }
    }
}

/// Offline CPU/GPU comparison on the same widget tree. Screenshot readback is
/// included, so these timings are not compositor presentation rates.
#[test]
#[ignore]
fn rendering_probe() {
    if child("rendering_probe") {
        return;
    }
    seed_desktop();
    let (mut app, _) = open(Size::new(1280., 800.));
    app.wizard = None;
    let theme = controls_theme(&app.config);
    for backend in ["tiny-skia", "wgpu"] {
        let mut native = Native::with_backend(Size::new(1280., 800.), backend);
        println!("RENDERER {}", native.renderer_name());
        for page in [Page::Themes, Page::General, Page::DesktopDock] {
            let _ = app.update(Message::Page(page));
            native.render(app.settings_view(), &theme);
            let start = std::time::Instant::now();
            for n in 0..30 {
                native.event(
                    app.settings_view(),
                    Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Lines {
                            x: 0.,
                            y: if n < 15 { -1. } else { 1. },
                        },
                    }),
                    cosmic::iced::Point::new(700., 400.),
                );
                let pixels = native.render(app.settings_view(), &theme);
                assert_eq!(pixels.len(), 1280 * 800 * 4);
                assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
            }
            println!(
                "RENDER {backend} {page:?} 30 scroll/draw/readback cycles: {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.
            );
        }
    }
}

#[test]
fn every_screen_fits_and_cancel_puts_the_desktop_back() {
    if child("every_screen_fits_and_cancel_puts_the_desktop_back") {
        return;
    }
    seed_desktop();
    for size in [Size::new(820., 600.), Size::new(1280., 800.)] {
        let (mut app, mut native) = open(size);
        let theme = controls_theme(&app.config);
        let shot = |app: &Settings, native: &mut Native, name: &str| {
            native.capture(
                app.settings_view(),
                &theme,
                &format!("wizard-{name}-{}", size.width),
            )
        };
        shot(&app, &mut native, "1-welcome");
        click(&mut app, &mut native, "wizard-start");
        assert_eq!(step(&app), wizard::Step::Look);
        click(&mut app, &mut native, "wizard-look-calm");
        let wizard = app.wizard.as_ref().unwrap();
        assert_eq!(wizard.error, None);
        assert_eq!(wizard.look, Some("calm"));
        assert_eq!(
            Config::load().appearance.family,
            Some(crate::desktop_theme::Family::Soft)
        );
        shot(&app, &mut native, "2-look");
        // Trying another look replaces the first.
        click(&mut app, &mut native, "wizard-look-evening");
        assert_eq!(app.wizard.as_ref().unwrap().look, Some("evening"));
        click(&mut app, &mut native, "wizard-next");
        assert_eq!(step(&app), wizard::Step::Bars);
        click(&mut app, &mut native, "wizard-dock-rounded");
        click(&mut app, &mut native, "wizard-dock-hides");
        shot(&app, &mut native, "3-bars");
        click(&mut app, &mut native, "wizard-next");
        assert_eq!(app.wizard.as_ref().unwrap().error, None);
        assert_eq!(step(&app), wizard::Step::Weather);
        assert_eq!(user(DOCK, "autohide").as_deref(), Some("OnOverlap"));
        assert_eq!(user(DOCK, "border_radius").as_deref(), Some("12"));
        // Applets and their order are never part of a look.
        assert_eq!(user(DOCK, "plugins_center"), None);
        assert_eq!(user(PANEL, "plugins_wings"), None);
        shot(&app, &mut native, "4-weather");
        click(&mut app, &mut native, "wizard-next");
        assert_eq!(step(&app), wizard::Step::Ready);
        shot(&app, &mut native, "5-ready");
        click(&mut app, &mut native, "wizard-cancel");
        assert!(
            app.wizard.is_none(),
            "{:?}",
            app.wizard.as_ref().and_then(|w| w.error.clone())
        );
        let config = Config::load();
        assert!(config.onboarded);
        assert_eq!(config.appearance.family, None, "the look is undone");
        for key in ["autohide", "border_radius"] {
            assert!(user(DOCK, key).is_none(), "{key}: {:?}", user(DOCK, key));
        }
        // The next size starts from a fresh first run.
        Config::save_changes(serde_json::json!({"onboarded": false})).unwrap();
    }
}

#[test]
fn finishing_keeps_the_look_and_skip_leaves_the_desktop_alone() {
    if child("finishing_keeps_the_look_and_skip_leaves_the_desktop_alone") {
        return;
    }
    seed_desktop();
    let size = Size::new(1020., 760.);
    let (mut app, mut native) = open(size);
    click(&mut app, &mut native, "wizard-leave");
    assert!(app.wizard.is_none());
    assert!(Config::load().onboarded);
    assert_eq!(Config::load().appearance.family, None);
    assert_eq!(user(DOCK, "border_radius"), None, "Skip changes nothing");

    let (mut app, mut native) = open(size);
    click(&mut app, &mut native, "wizard-start");
    click(&mut app, &mut native, "wizard-look-focus");
    for _ in 0..3 {
        click(&mut app, &mut native, "wizard-next");
    }
    assert_eq!(step(&app), wizard::Step::Ready);
    click(&mut app, &mut native, "wizard-finish");
    assert!(app.wizard.is_none());
    assert_eq!(app.page, Page::Overview);
    let config = Config::load();
    assert_eq!(
        config.appearance.family,
        Some(crate::desktop_theme::Family::Bare)
    );
    assert!(config.widget_visible(crate::drawer::Widget::Devbox));
    assert!(config.glance.enabled && !config.glance.show_calendar);
    // The old checklist doesn't follow the wizard.
    assert!(!app.needs_setup());
}

#[test]
fn an_untouched_dock_and_panel_step_keeps_the_persons_own_setup() {
    if child("an_untouched_dock_and_panel_step_keeps_the_persons_own_setup") {
        return;
    }
    seed_desktop();
    // Their own dock: hidden, square, attached.
    let own = |key: &str, value: &str| {
        let path = desktop_root()
            .join("config/cosmic")
            .join(DOCK)
            .join("v1")
            .join(key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    };
    own("autohide", "Always");
    own("border_radius", "0");
    let (mut app, mut native) = open(Size::new(1020., 760.));
    let bars = app.wizard.as_ref().unwrap().bars;
    assert!(bars.hides, "the step shows their dock as it is");
    assert_eq!(bars.dock_shape, wizard::Shape::Square);
    click(&mut app, &mut native, "wizard-start");
    click(&mut app, &mut native, "wizard-skip");
    assert_eq!(step(&app), wizard::Step::Bars);
    click(&mut app, &mut native, "wizard-next");
    assert_eq!(step(&app), wizard::Step::Weather);
    assert_eq!(user(DOCK, "autohide").as_deref(), Some("Always"));
    assert_eq!(user(DOCK, "border_radius").as_deref(), Some("0"));
    assert_eq!(user(DOCK, "anchor_gap"), None);
    assert_eq!(user(PANEL, "border_radius"), None);
    // Changing one thing writes only that.
    click(&mut app, &mut native, "wizard-back");
    click(&mut app, &mut native, "wizard-dock-pill");
    click(&mut app, &mut native, "wizard-next");
    assert_eq!(user(DOCK, "autohide").as_deref(), Some("Always"));
    assert_eq!(user(DOCK, "border_radius").as_deref(), Some("160"));
    assert_eq!(user(PANEL, "border_radius"), None);
}

#[test]
fn cancel_undoes_setup_after_a_city_and_after_reopening() {
    if child("cancel_undoes_setup_after_a_city_and_after_reopening") {
        return;
    }
    seed_desktop();
    let size = Size::new(1020., 760.);
    let (mut app, mut native) = open(size);
    click(&mut app, &mut native, "wizard-start");
    click(&mut app, &mut native, "wizard-look-calm");
    assert!(Config::load().appearance.family.is_some());
    // Close Settings halfway: setup resumes, with its changes.
    drop(app);
    let (mut app, mut native) = open(size);
    let wizard = app.wizard.as_ref().unwrap();
    assert_eq!(wizard.step, wizard::Step::Look);
    assert_eq!(wizard.look, Some("calm"));
    assert!(wizard.has_changes());
    // A newer, unrelated change: choosing a city.
    click(&mut app, &mut native, "wizard-next");
    click(&mut app, &mut native, "wizard-next");
    assert_eq!(step(&app), wizard::Step::Weather);
    app.cities = vec![City {
        name: "Belgrade".into(),
        latitude: 44.8,
        longitude: 20.5,
        admin1: String::new(),
        country: "Serbia".into(),
    }];
    let _ = app.update(Message::ChooseCity(0));
    assert!(Config::load().location.is_some());
    click(&mut app, &mut native, "wizard-cancel");
    assert!(
        app.wizard.is_none(),
        "{:?}",
        app.wizard.as_ref().and_then(|wizard| wizard.error.clone())
    );
    let config = Config::load();
    assert_eq!(config.appearance.family, None, "the look is undone");
    assert!(config.location.is_some(), "the city stays");
    // Nothing left to resume.
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    assert!(!wizard::Wizard::pending(&store));
}

#[test]
fn skipping_dock_and_panel_keeps_them_and_ready_says_so() {
    if child("skipping_dock_and_panel_keeps_them_and_ready_says_so") {
        return;
    }
    seed_desktop();
    let (mut app, mut native) = open(Size::new(1020., 760.));
    click(&mut app, &mut native, "wizard-start");
    click(&mut app, &mut native, "wizard-skip");
    let before = app.wizard.as_ref().unwrap().bars;
    click(&mut app, &mut native, "wizard-dock-square");
    assert_ne!(app.wizard.as_ref().unwrap().bars, before);
    click(&mut app, &mut native, "wizard-skip");
    assert_eq!(step(&app), wizard::Step::Weather);
    assert_eq!(
        app.wizard.as_ref().unwrap().bars,
        before,
        "the summary shows what's applied"
    );
    assert_eq!(user(DOCK, "border_radius"), None, "nothing was written");
}

#[test]
fn busy_sliders_keep_latest_sample_and_preserve_completion_and_undo_order() {
    if child("busy_sliders_keep_latest_sample_and_preserve_completion_and_undo_order") {
        return;
    }
    seed_desktop();
    let (mut app, _) = Settings::init(Core::default(), Page::DesktopDock);
    app.native_busy = true;
    app.last_desktop_change = Some((
        crate::desktop::Owner::DockPanel,
        "Earlier action".into(),
        "shown-id".into(),
    ));
    for value in 0..100 {
        let _ = app.update(Message::DesktopEditor(desktop::Message::Opacity(value)));
    }
    assert_eq!(
        app.native_deferred.len(),
        1,
        "obsolete pointer samples must not form a backlog"
    );
    let _ = app.update(Message::DesktopEditor(desktop::Message::Apply));
    let _ = app.update(Message::DesktopEditor(desktop::Message::Select(
        crate::desktop::panels::PanelKind::Panel,
    )));
    for value in 0..50 {
        let _ = app.update(Message::DesktopEditor(desktop::Message::Opacity(value)));
    }
    let _ = app.update(Message::DesktopEditor(desktop::Message::Apply));
    let _ = app.update(Message::UndoDesktop);
    let queued = &app.native_deferred;
    assert_eq!(queued.len(), 6);
    assert!(matches!(
        &queued[0],
        Message::DesktopEditor(desktop::Message::Opacity(99))
    ));
    assert!(matches!(
        &queued[1],
        Message::DesktopEditor(desktop::Message::Apply)
    ));
    assert!(matches!(
        &queued[2],
        Message::DesktopEditor(desktop::Message::Select(
            crate::desktop::panels::PanelKind::Panel
        ))
    ));
    assert!(matches!(
        &queued[3],
        Message::DesktopEditor(desktop::Message::Opacity(49))
    ));
    assert!(matches!(
        &queued[4],
        Message::DesktopEditor(desktop::Message::Apply)
    ));
    assert!(matches!(&queued[5], Message::UndoDesktopShown(Some(id)) if id == "shown-id"));
    app.native_deferred.pop_back(); // Verify final slider values before Undo.
    app.native_busy = false;
    while let Some(message) = app.native_deferred.pop_front() {
        let _ = app.update(message);
    }
    assert_eq!(user(DOCK, "opacity").as_deref(), Some("0.99"));
    assert_eq!(user(PANEL, "opacity").as_deref(), Some("0.49"));
}

#[test]
fn dock_panel_navigation_does_not_wait_for_islands_and_checks_discover_once() {
    if child("dock_panel_navigation_does_not_wait_for_islands_and_checks_discover_once") {
        return;
    }
    seed_desktop();
    let (mut app, _) = Settings::init(Core::default(), Page::DesktopDock);
    let root = desktop_root();
    crate::desktop::islands::fixture_installed(&root);
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    app.desktop.islands.installation = crate::desktop::islands::Installation::discover(&store);
    let counter = root.join("check-count");
    std::fs::write(root.join("slow-islands-probe.py"), format!(
        "import pathlib,time\np=pathlib.Path({:?})\np.write_text(str(int(p.read_text())+1 if p.exists() else 1))\ntime.sleep(0.3)\nprint(pathlib.Path({:?}).read_text())\n",
        counter.to_str().unwrap(), root.join("islands-installation.json").to_str().unwrap()
    )).unwrap();
    for kind in [
        crate::desktop::panels::PanelKind::Panel,
        crate::desktop::panels::PanelKind::Dock,
    ] {
        let _ = app.update(Message::DesktopEditor(desktop::Message::Select(kind)));
        assert_eq!(
            app.desktop.selected, kind,
            "tab selection updates immediately"
        );
        assert!(!app.native_busy, "tab selection is not a native write job");
        assert!(app.native_deferred.is_empty());
        assert!(
            !counter.exists(),
            "plain navigation must not start compatibility discovery"
        );
    }
    // Real checking still observes installation afresh, but only once per read.
    app.desktop.check_islands();
    assert_eq!(std::fs::read_to_string(counter).unwrap(), "1");
}
