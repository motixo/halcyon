use super::*;
use crate::displays::{
    OutputEvent,
    test_support::{Wire, info, isolated},
};
use cosmic::Application;

#[test]
fn monitor_pinning_settings_save_undo_and_failure() {
    if isolated("settings::display_tests::monitor_pinning_settings_save_undo_and_failure") {
        return;
    }
    let wire = Wire::new();
    let a = wire.output(1);
    let b = wire.output(2);
    let (mut app, _) = Settings::init(Core::default(), Page::WidgetLayout);
    let _ = app.update(Message::DisplayOutput(
        OutputEvent::Created(Some(info(Some("DP-1"), (1920, 1080)))),
        a,
    ));
    let _ = app.update(Message::DisplayOutput(
        OutputEvent::Created(Some(info(Some("DP-2"), (1920, 1080)))),
        b.clone(),
    ));
    let _ = app.update(Message::WidgetDisplay(Some("DP-2".into())));
    assert!(!app.error, "{}", app.status);
    assert_eq!(
        Config::try_load().unwrap().widget_display.as_deref(),
        Some("DP-2")
    );
    assert!(app.last_desktop_change.is_some(), "durable Undo must exist");
    // Unrelated concurrent edits must survive the actual transaction Undo.
    Config::save_changes(serde_json::json!({"clock":{"show_date":false},"future_preference":7}))
        .unwrap();
    let _ = app.update(Message::UndoPreferences);
    let saved = Config::try_load().unwrap();
    assert_eq!(saved.widget_display, None);
    assert!(!saved.clock.show_date);
    let _ = app.update(Message::WidgetDisplay(Some("DP-2".into())));
    let _ = app.update(Message::DisplayOutput(OutputEvent::Removed, b));
    assert!(
        app.displays
            .choices(app.config.widget_display.as_deref())
            .iter()
            .any(|(label, value)| label.contains("not connected")
                && value.as_deref() == Some("DP-2"))
    );
    assert!(app.displays.status(Some("DP-2")).contains("DP-1"));
    // Disconnected choice is retained; an obsolete menu action cannot pick it anew.
    let _ = app.update(Message::WidgetDisplay(Some("missing".into())));
    assert!(app.error);
    assert_eq!(
        Config::try_load().unwrap().widget_display.as_deref(),
        Some("DP-2")
    );
    let path = Config::path().unwrap();
    let before = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let _ = app.update(Message::WidgetDisplay(None));
    assert!(app.error);
    assert_eq!(app.config.widget_display.as_deref(), Some("DP-2"));
    std::fs::remove_dir(&path).unwrap();
    std::fs::write(&path, before).unwrap();
}

#[test]
fn monitor_pinning_native_selector_and_ambiguous_names() {
    if isolated("settings::display_tests::monitor_pinning_native_selector_and_ambiguous_names") {
        return;
    }
    let wire = Wire::new();
    let a = wire.output(1);
    let b = wire.output(2);
    let (mut app, _) = Settings::init(Core::default(), Page::WidgetLayout);
    let _ = app.update(Message::DisplayOutput(
        OutputEvent::Created(Some(info(Some("DP-1"), (1920, 1080)))),
        a.clone(),
    ));
    let _ = app.update(Message::DisplayOutput(
        OutputEvent::Created(Some(info(Some("DP-2"), (1920, 1080)))),
        b.clone(),
    ));
    let mut native = crate::native_test::Native::new(cosmic::iced::Size::new(640., 240.));
    let bounds = native.bounds(app.display_selector(), "widget-display-DP-2");

    assert!(bounds.width > 40. && bounds.x + bounds.width <= 640.);
    let point = bounds.center();
    // Exercise the real selector: a button click must emit and save the choice.
    let mut emitted = false;
    for event in [
        cosmic::iced::mouse::Event::CursorMoved { position: point },
        cosmic::iced::mouse::Event::ButtonPressed(cosmic::iced::mouse::Button::Left),
        cosmic::iced::mouse::Event::ButtonReleased(cosmic::iced::mouse::Button::Left),
    ] {
        let (messages, _) = native.event(
            app.display_selector(),
            cosmic::iced::Event::Mouse(event),
            point,
        );
        for message in messages {
            emitted |= matches!(&message,Message::WidgetDisplay(Some(name)) if name=="DP-2");
            let _ = app.update(message);
        }
    }
    assert!(emitted);
    assert_eq!(
        Config::try_load().unwrap().widget_display.as_deref(),
        Some("DP-2")
    );
    let theme = cosmic::Theme::dark();
    native.capture_hover(
        app.display_selector(),
        &theme,
        point,
        "monitor-selector-connected",
    );
    for (width, height) in [(1024., 760.), (820., 600.)] {
        app.config.setup_dismissed = true;
        let _ = app.update(Message::Resized(cosmic::iced::Size::new(width, height)));
        let mut full = crate::native_test::Native::new(cosmic::iced::Size::new(width, height));
        let button = full.bounds(app.settings_view(), "widget-display-DP-2");
        assert!(
            button.x >= 0.
                && button.y >= 0.
                && button.x + button.width <= width
                && button.y + button.height <= height
        );
        full.capture(
            app.settings_view(),
            &theme,
            &format!("monitor-settings-{}", width as u32),
        );
    }
    let _ = app.update(Message::WidgetDisplay(Some("DP-2".into())));
    let _ = app.update(Message::DisplayOutput(OutputEvent::Removed, b.clone()));
    native.capture(
        app.display_selector(),
        &theme,
        "monitor-selector-disconnected",
    );
    let _ = app.update(Message::DisplayOutput(
        OutputEvent::Created(Some(info(Some("DP-1"), (1920, 1080)))),
        b,
    ));
    assert!(app.displays.unique("DP-1").is_none());
    assert!(
        !app.displays
            .choices(None)
            .iter()
            .any(|(_, v)| v.as_deref() == Some("DP-1"))
    );
    let unnamed = wire.output(3);
    let _ = app.update(Message::DisplayOutput(
        OutputEvent::Created(Some(info(None, (1920, 1080)))),
        unnamed,
    ));
    assert!(app.displays.status(None).contains("cannot be pinned"));
    native.capture(app.display_selector(), &theme, "monitor-selector-ambiguous");
    let _ = app.update(Message::WidgetDisplay(Some("DP-1".into())));
    assert!(app.error);
}

#[test]
fn monitor_pinning_config_validation_and_portable_boundary() {
    if isolated("settings::display_tests::monitor_pinning_config_validation_and_portable_boundary")
    {
        return;
    }
    assert!(
        crate::config::check_config_value(&serde_json::json!({}))
            .unwrap()
            .widget_display
            .is_none()
    );
    for value in [
        serde_json::json!(""),
        serde_json::json!("DP\n1"),
        serde_json::json!("x".repeat(129)),
        serde_json::json!(4),
    ] {
        assert!(
            crate::config::check_config_value(&serde_json::json!({"widget_display":value}))
                .is_err()
        );
    }
    Config::save_changes(serde_json::json!({"widget_display":"DP-2","future_key":{"value":42},"widget_positions":{"weather":[0.7,0.4]}})).unwrap();
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    let mut captured = crate::desktop::profiles::capture(&store, "Display regression").unwrap();
    let encoded = serde_json::to_string(&captured.profile).unwrap();
    assert!(!encoded.contains("widget_display"));
    assert!(!encoded.contains("DP-2"));
    let exported = crate::desktop::profiles::export(&captured.profile).unwrap();
    let mut imported = crate::desktop::profiles::import(&exported.bytes).unwrap();
    imported.widget_layout = Some(serde_json::json!({"widget_positions":{"weather":[0.2,0.2]}}));
    let plan = crate::desktop::profiles::plan(
        &store,
        &crate::desktop::profiles::Installed::default(),
        &imported,
        &[crate::desktop::profiles::Domain::WidgetLayout]
            .into_iter()
            .collect(),
    );
    crate::desktop::profiles::apply(&store, &plan).unwrap();
    assert_eq!(
        Config::try_load().unwrap().widget_display.as_deref(),
        Some("DP-2")
    );
    assert_eq!(
        Config::try_load().unwrap().widget_positions["weather"],
        [0.2, 0.2]
    );
    captured.profile.widget_layout = Some(serde_json::json!({"widget_display":"DP-1"}));
    assert!(captured.profile.validate().is_err());
    assert_eq!(
        Config::try_load().unwrap().widget_display.as_deref(),
        Some("DP-2")
    );
}
