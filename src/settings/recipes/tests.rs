use super::*;
#[test]
fn fb17_editor_reorder_primary_protection_and_copy_identity() {
    let mut editor = Editor::default();
    let id = editor.draft.id.clone();
    editor.update(Message::Remove(0));
    assert!(editor.draft.parts.contains(&Part::Face));
    editor.update(Message::Add(Part::Caption));
    editor.update(Message::Move(2, true));
    assert_eq!(editor.draft.parts[1], Part::Caption);
    editor.update(Message::Arrangement(true, Arrangement::Stack));
    assert_eq!(editor.draft.wide, Arrangement::Stack);
    editor.update(Message::Duplicate);
    assert_ne!(editor.draft.id, id);
    assert!(editor.expected.is_none());
    editor.draft.validate().unwrap();
}
#[test]
fn fb17_native_builder_save_apply_reset_undo_and_library_delete_are_independent() {
    if crate::displays::test_support::isolated(
        "settings::recipes::tests::fb17_native_builder_save_apply_reset_undo_and_library_delete_are_independent",
    ) {
        return;
    }
    use crate::settings::tests::send_settings;
    std::fs::write(
        Config::path().unwrap(),
        br#"{"onboarded":true,"setup_dismissed":true,"theme":"teal"}"#,
    )
    .unwrap();
    let store = crate::desktop::actions::history_store().unwrap();
    store.migrate_design().unwrap();
    let frozen = std::fs::read(Config::path().unwrap()).unwrap();
    let (mut app, _) = Settings::init(Core::default(), Page::Builder);
    let send = |app: &mut Settings, m| send_settings(app, super::super::Message::Recipe(m));
    send(&mut app, Message::Name("Morning paper".into()));
    send(&mut app, Message::Add(Part::Caption));
    send(&mut app, Message::Save);
    assert!(!app.recipe_editor.error, "{:?}", app.recipe_editor.notice);
    let saved = app.recipe_editor.expected.clone().unwrap();
    send(&mut app, Message::Apply);
    assert!(!app.recipe_editor.error, "{:?}", app.recipe_editor.notice);
    assert_eq!(app.config.recipes.clock.as_ref(), Some(&saved));
    assert_eq!(std::fs::read(Config::path().unwrap()).unwrap(), frozen);
    send(&mut app, Message::Delete);
    send(&mut app, Message::ConfirmDelete);
    assert!(app.recipe_editor.library.entries.is_empty());
    assert_eq!(
        Config::try_load().unwrap().recipes.clock,
        Some(saved.clone())
    );
    send(&mut app, Message::UseBuiltin);
    assert!(app.config.recipes.clock.is_none());
    send_settings(&mut app, super::super::Message::UndoDesktop);
    assert_eq!(app.config.recipes.clock, Some(saved));
    for (w, h) in [(820., 600.), (1280., 800.)] {
        send_settings(&mut app, super::super::Message::Resized(Size::new(w, h)));
        let mut native = crate::native_test::Native::new(Size::new(w, h));
        let bounds = native.bounds(app.settings_view(), "recipe-apply");
        assert!(bounds.y >= 0. && bounds.y + bounds.height <= h);
        native.capture(
            app.settings_view(),
            &cosmic::Theme::dark(),
            &format!("fb17-builder-{w}"),
        );
    }
}
#[test]
fn fb17_recipe_apply_requires_migration_and_stale_baseline_is_refused() {
    if crate::displays::test_support::isolated(
        "settings::recipes::tests::fb17_recipe_apply_requires_migration_and_stale_baseline_is_refused",
    ) {
        return;
    }
    std::fs::write(Config::path().unwrap(), br#"{"onboarded":true}"#).unwrap();
    let mut editor = Editor::default();
    assert!(editor.work(Message::Apply, None).is_err());
    let store = crate::desktop::actions::history_store().unwrap();
    store.migrate_design().unwrap();
    editor.work(Message::Apply, None).unwrap();
    let saved = Config::try_load().unwrap().recipes.clock.unwrap();
    editor.draft.name = "New name".into();
    editor.draft.seal();
    assert!(editor.work(Message::Apply, None).is_err());
    editor
        .work(Message::Apply, Some(serde_json::to_value(saved).unwrap()))
        .unwrap();
    assert_eq!(
        Config::try_load().unwrap().recipes.clock.unwrap().name,
        "New name"
    );
}
#[test]
fn fb17_capture_preserves_legacy_versions_but_keeps_personal_and_reset_intent() {
    if crate::displays::test_support::isolated(
        "settings::recipes::tests::fb17_capture_preserves_legacy_versions_but_keeps_personal_and_reset_intent",
    ) {
        return;
    }
    std::fs::write(
        Config::path().unwrap(),
        br#"{"onboarded":true,"theme":"teal"}"#,
    )
    .unwrap();
    let store = crate::desktop::actions::history_store().unwrap();
    store.migrate_design().unwrap();
    let capture = |name| {
        crate::desktop::profiles::capture(&store, name)
            .unwrap()
            .profile
    };
    assert!(capture("Legacy").written_version() < 13);
    let mut editor = Editor::default();
    editor.work(Message::Apply, None).unwrap();
    let applied = capture("Personal");
    assert_eq!(applied.written_version(), 13);
    assert_eq!(
        applied.widget_style.unwrap()["recipes"]["clock"]["id"],
        editor.draft.id
    );
    let destination = Config::path().unwrap().with_file_name("legacy-export.json");
    assert!(crate::design::commands::export(&Config::path().unwrap(), &destination, true).is_err());
    assert!(!destination.exists());
    editor
        .work(
            Message::UseBuiltin,
            Some(serde_json::to_value(&editor.draft).unwrap()),
        )
        .unwrap();
    let reset = capture("Reset");
    assert_eq!(reset.written_version(), 13);
    assert!(reset.widget_style.unwrap()["recipes"]["clock"].is_null());
    crate::design::commands::export(&Config::path().unwrap(), &destination, true).unwrap();
}
