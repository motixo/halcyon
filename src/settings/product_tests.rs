// Uses the existing isolated native Settings harness.
#[test]
fn product_help_report_and_history_are_read_only_and_keep_private_data_out() {
    let name = "product_help_report_and_history_are_read_only_and_keep_private_data_out";
    if desktop_test_child(
        name,
        DesktopEnv {
            app_config: true,
            fixture: true,
            ..Default::default()
        },
    ) {
        return;
    }
    let root = isolated_root();
    let mut app = open_isolated_editor(&root);
    let before = fixture_files(&root.join("cosmic-root"));
    let config = std::fs::read(Config::path().unwrap()).unwrap();
    let sentinel = "PRIVATE_SENTINEL@example.invalid/private/calendar.ics?token=secret";
    app.calendar.connection.account = sentinel.into();
    app.calendar.connection.calendar_id = sentinel.into();
    app.calendar.connection.credential = sentinel.into();
    app.calendar.connection.calendar_name = sentinel.into();
    app.calendar.error = Some(sentinel.into());
    app.updates.store_error = Some(sentinel.into());
    send_settings(&mut app, Message::AppSettingsPage(AppSettingsPage::Help));
    send_settings(&mut app, Message::Support(support::Message::Preview(true)));
    let report = app.support.report.clone().unwrap();
    assert!(!report.contains("PRIVATE_SENTINEL"));
    assert!(!report.contains("secret"));
    assert!(!report.contains(&root.display().to_string()));
    assert!(report.contains("Calendar: Needs attention"));
    app.calendar.error = None;
    assert_eq!(
        app.support.report.as_deref(),
        Some(report.as_str()),
        "preview is the immutable copy/export payload"
    );
    send_settings(&mut app, Message::Back);
    assert!(app.support.report.is_none());
    send_settings(&mut app, Message::Support(support::Message::Recent(true)));
    assert!(app.support.recent_open);
    assert_eq!(app.app_settings_page, AppSettingsPage::Restore);
    send_settings(&mut app, Message::Back);
    assert!(!app.support.recent_open);
    for query in ["diagnostics", "why did my desktop change"] {
        send_settings(&mut app, Message::SearchSettings(query.into()));
        send_settings(&mut app, Message::SearchFirst);
        assert_eq!(app.page, Page::General);
        assert_eq!(app.support.recent_open, query.starts_with("why"));
    }
    assert_eq!(fixture_files(&root.join("cosmic-root")), before);
    assert_eq!(std::fs::read(Config::path().unwrap()).unwrap(), config);
}
#[test]
fn product_help_tabs_and_primary_actions_fit_compact_and_wide_windows() {
    let name = "product_help_tabs_and_primary_actions_fit_compact_and_wide_windows";
    if desktop_test_child(
        name,
        DesktopEnv {
            app_config: true,
            fixture: true,
            ..Default::default()
        },
    ) {
        return;
    }
    let root = isolated_root();
    let mut app = open_isolated_editor(&root);
    for size in [Size::new(820., 600.), Size::new(1280., 900.)] {
        send_settings(&mut app, Message::Resized(size));
        send_settings(&mut app, Message::AppSettingsPage(AppSettingsPage::Help));
        let mut native = crate::native_test::Native::new(size);
        for page in AppSettingsPage::ALL {
            let bounds =
                native.bounds(app.settings_view(), &format!("app-settings-{}", page.key()));
            assert!(
                bounds.x >= 0.
                    && bounds.x + bounds.width <= size.width
                    && bounds.y + bounds.height <= size.height,
                "{page:?}: {bounds:?}"
            );
        }
        let primary = native.bounds(app.settings_view(), "help-preview-report");
        assert!(
            primary.y + primary.height < size.height,
            "primary action visible on entry"
        );
        for (name, theme) in [
            ("dark", cosmic::Theme::dark()),
            ("light", cosmic::Theme::light()),
        ] {
            native.capture_hover(
                app.settings_view(),
                &theme,
                cosmic::iced::Point::new(-1., -1.),
                &format!("product-help-{}-{name}", size.width),
            );
        }
        send_settings(&mut app, Message::Support(support::Message::Preview(true)));
        native_reveal(&app, &mut native, "help-copy-report");
    }
}

#[test]
fn product_direct_help_loads_history_without_opening_a_desktop_editor() {
    let name = "product_direct_help_loads_history_without_opening_a_desktop_editor";
    if desktop_test_child(
        name,
        DesktopEnv {
            app_config: true,
            fixture: true,
            ..Default::default()
        },
    ) {
        return;
    }
    let (mut app, _) = Settings::init(Core::default(), Page::General);
    send_settings(&mut app, Message::AppSettingsPage(AppSettingsPage::Help));
    assert!(
        !app.support.local.problem,
        "A missing history file is a healthy empty state"
    );
    assert!(app.support.local.store.is_some());
    assert!(app.support.local.recent.entries.is_empty());
}
