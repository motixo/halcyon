use super::*;
use crate::settings::{Page, Settings};
use cosmic::{
    Application,
    app::Core,
    iced::{Event, Point, Size, keyboard},
};
#[test]
fn fb18_native_package_review_export_cancel_and_import_never_apply() {
    if crate::displays::test_support::isolated(
        "settings::profiles::packages::tests::fb18_native_package_review_export_cancel_and_import_never_apply",
    ) {
        return;
    }
    let config = Config::path().unwrap();
    fs::write(&config,br#"{"onboarded":true,"setup_dismissed":true,"theme":"teal","location":{"name":"PRIVATE","latitude":1,"longitude":2}}"#).unwrap();
    let store = crate::desktop::actions::history_store().unwrap();
    let image = store.fixture_root().unwrap().join("package-source.png");
    image::RgbaImage::from_pixel(64, 36, image::Rgba([30, 70, 80, 255]))
        .save(&image)
        .unwrap();
    let mut profile = Profile::new("Morning travel").unwrap();
    profile.wallpaper = Some(profiles::WallpaperLook {
        background: Some(profiles::BackgroundLook {
            source: Some(profiles::SourceLook::Image(image)),
            ..Default::default()
        }),
        ..Default::default()
    });
    let id = ProfileLibrary::new(&store).create(&profile).unwrap();
    let before = fs::read(&config).unwrap();
    let (mut app, _) = Settings::init(Core::default(), Page::DesktopProfiles);
    app.profiles.update(Message::Select(Selection::Saved(id)));
    let selected = app.profiles.profile().cloned();
    app.page_scroll
        .insert(Page::DesktopProfiles.key().into(), 500.);
    crate::settings::tests::send_settings(
        &mut app,
        crate::settings::Message::Profiles(Message::Package(Edit::Open)),
    );
    assert_eq!(app.page_scroll.get(Page::DesktopProfiles.key()), Some(&0.));
    crate::settings::tests::send_settings(&mut app, crate::settings::Message::Back);
    assert!(app.profiles.package.is_none());
    assert_eq!(app.profiles.profile(), selected.as_ref());
    crate::settings::tests::send_settings(
        &mut app,
        crate::settings::Message::Profiles(Message::Package(Edit::Open)),
    );
    app.native_busy = true;
    let _ = app.update(crate::settings::Message::Profiles(Message::Package(
        Edit::Author("Stale edit".into()),
    )));
    assert!(app.native_deferred.is_empty());
    assert!(
        app.profiles
            .package
            .as_ref()
            .unwrap()
            .credits
            .author
            .is_empty()
    );
    app.native_busy = false;
    app.profiles.update(Message::Package(Edit::Picture(true)));
    assert_eq!(
        app.profiles.update(Message::Package(Edit::Export)),
        Outcome::None
    );
    assert!(matches!(app.profiles.notice, Some(Notice::Error(_))));
    for (w, h) in [(820., 600.), (1280., 800.)] {
        crate::settings::tests::send_settings(
            &mut app,
            crate::settings::Message::Resized(Size::new(w, h)),
        );
        let mut native = crate::native_test::Native::new(Size::new(w, h));
        let bounds = native.bounds(app.settings_view(), "package-export");
        assert!(bounds.y + bounds.height <= h);
        native.capture(
            app.settings_view(),
            &cosmic::Theme::dark(),
            &format!("fb18-package-review-{w}"),
        );
        crate::settings::tests::native_reveal(&app, &mut native, "package-author");
        let author = native.bounds(app.settings_view(), "package-author");
        assert!(author.y + author.height <= h);
        native.capture(
            app.settings_view(),
            &cosmic::Theme::dark(),
            &format!("fb18-package-credits-{w}"),
        );
    }
    for edit in [
        Edit::Author("Example artist".into()),
        Edit::License("CC0-1.0".into()),
    ] {
        app.profiles.update(Message::Package(edit));
    }
    let mut native = crate::native_test::Native::new(Size::new(1280., 800.));
    native.operate(
        app.settings_view(),
        Box::new(cosmic::iced::advanced::widget::operation::focusable::focus(
            cosmic::widget::Id::new("package-export"),
        )),
    );
    let key = keyboard::Key::Named(keyboard::key::Named::Enter);
    let (messages, _) = native.event(
        app.settings_view(),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        }),
        Point::new(1., 1.),
    );
    let [crate::settings::Message::Profiles(edit)] = messages.as_slice() else {
        panic!("keyboard export action");
    };
    assert!(edit.blocking());
    let Outcome::Dialog(Request::Export {
        id,
        file_name,
        bytes,
        confine,
    }) = app.profiles.update(edit.clone())
    else {
        panic!("export dialog");
    };
    assert!(file_name.ends_with(".halcyon"));
    assert!(confine.is_some());
    assert!(!String::from_utf8_lossy(&bytes).contains("PRIVATE"));
    assert_eq!(fs::read(&config).unwrap(), before);
    app.profiles.update(Message::Exported(id, Ok(None)));
    assert!(!app.profiles.busy());
    let Outcome::Dialog(Request::Import { id }) = app.profiles.update(Message::Import) else {
        panic!("import dialog");
    };
    crate::settings::tests::send_settings(&mut app, crate::settings::Message::Page(Page::General));
    let task = app.update(crate::settings::Message::Profiles(Message::Imported(
        id,
        Ok(Some(bytes)),
    )));
    assert!(
        app.native_busy,
        "package import stays on its worker even after navigation"
    );
    crate::settings::tests::settle_settings(&mut app, task);
    assert_eq!(app.page, Page::General);
    assert!(matches!(app.profiles.notice, Some(Notice::Done(_))));
    assert!(app.profiles.profile().unwrap().package.is_some());
    assert_eq!(fs::read(&config).unwrap(), before);
}
#[test]
fn fb18_package_draft_is_invalidated_by_selection_and_large_plain_profiles_stay_bounded() {
    use crate::desktop::store::tests::Fixture;
    let f = Fixture::new("fb18-ui-stale");
    let store = f.store();
    let mut editor = ProfilesEditor::with_store(store);
    editor.update(Message::Select(Selection::Preset("paper")));
    editor.update(Message::Package(Edit::Open));
    assert!(editor.package.is_some());
    editor.update(Message::Select(Selection::Preset("tidal")));
    assert!(editor.package.is_none());
    assert_eq!(editor.update(Message::Package(Edit::Export)), Outcome::None);
    let path = f.0.join("large.json");
    fs::write(&path, vec![b' '; MAX_PROFILE_BYTES + 1]).unwrap();
    assert!(read_import(&path).is_err());
    let profile = Profile::new("No picture").unwrap();
    let (package, _) = Package::build(&profile, false, Credits::default()).unwrap();
    fs::write(&path, package.encode().unwrap()).unwrap();
    assert!(Package::parse(&read_import(&path).unwrap()).is_ok());
}

#[test]
fn fb18_package_export_refuses_a_picture_replaced_since_review() {
    use crate::desktop::store::tests::Fixture;
    let f = Fixture::new("fb18-export-source");
    let store = f.store();
    let path = f.0.join("picture.png");
    image::RgbaImage::from_pixel(16, 16, image::Rgba([20, 40, 70, 255]))
        .save(&path)
        .unwrap();
    let mut profile = Profile::new("Reviewed picture").unwrap();
    profile.wallpaper = Some(profiles::WallpaperLook {
        background: Some(profiles::BackgroundLook {
            source: Some(profiles::SourceLook::Image(path.clone())),
            ..Default::default()
        }),
        ..Default::default()
    });
    let id = ProfileLibrary::new(&store).create(&profile).unwrap();
    let mut editor = ProfilesEditor::with_store(store);
    editor.update(Message::Select(Selection::Saved(id)));
    editor.update(Message::Package(Edit::Open));
    for edit in [
        Edit::Picture(true),
        Edit::Author("Example".into()),
        Edit::License("CC0-1.0".into()),
    ] {
        editor.update(Message::Package(edit));
    }
    let replacement = f.0.join("replacement.png");
    image::RgbaImage::from_pixel(16, 16, image::Rgba([70, 40, 20, 255]))
        .save(&replacement)
        .unwrap();
    fs::rename(&replacement, &path).unwrap();
    assert_eq!(editor.update(Message::Package(Edit::Export)), Outcome::None);
    assert!(matches!(editor.notice,Some(Notice::Error(ref s)) if s.contains("changed")));
    assert!(!editor.busy());
}
