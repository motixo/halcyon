use super::*;
use crate::desktop::Owner;
use crate::desktop::store::tests::Fixture;
use crate::desktop::wallpaper::{BgEntry, NAMESPACE};
use cosmic::cosmic_theme::palette::Srgba;
use std::fs;

fn fixture(name: &str) -> (Fixture, DesktopStore) {
    let fixture = Fixture::new(name);
    let store = fixture
        .store()
        .with_app_config(fixture.0.join("config.json"));
    fs::write(
        store.app_config().unwrap(),
        r#"{"appearance":{"wallpaper_colors":true},"future":{"keep":42}}"#,
    )
    .unwrap();
    set_color(&fixture, [0.2, 0.5, 0.7]);
    (fixture, store)
}
fn set_color(fixture: &Fixture, rgb: [f32; 3]) {
    let entry = BgEntry::new("all", BgSource::Color(BgColor::Single(rgb)));
    fixture.user(NAMESPACE, "all", &ron::to_string(&entry).unwrap());
}
fn raw(store: &DesktopStore) -> serde_json::Value {
    serde_json::from_slice(&fs::read(store.app_config().unwrap()).unwrap()).unwrap()
}
/// A picture's suggestion keeps its hue at a lively accent lightness.
fn assert_toned_from(accent: serde_json::Value, picture: [u8; 3]) {
    use crate::desktop_theme::tone;
    let accent: [u8; 3] = serde_json::from_value(accent).unwrap();
    let (got, from) = (tone::lch(accent), tone::lch(picture));
    let turn = (got.h - from.h).rem_euclid(std::f32::consts::TAU);
    assert!(
        turn.min(std::f32::consts::TAU - turn) < 0.12,
        "{accent:?} keeps the hue of {picture:?}"
    );
    assert!((got.l - 0.76).abs() < 0.03, "{accent:?} is a lively tone");
}

#[test]
fn automatic_colors_runtime_changes_widgets_and_both_variants_then_undo_stays_put() {
    let (_fixture, store) = fixture("automatic-runtime");
    let before = fs::read(store.app_config().unwrap()).unwrap();
    let planned = prepare(&store).unwrap().unwrap();
    let rgb = recommended(&planned.source.analyse().unwrap()).unwrap();
    let change = commit(&store, planned).unwrap().unwrap();
    assert_eq!(raw(&store)["appearance"]["accent"], serde_json::json!(rgb));
    assert_eq!(raw(&store)["future"]["keep"], 42);
    let draft = AppearanceDraft::load(&store).unwrap();
    // Both variants follow, each at the tone COSMIC uses for that variant.
    for (variant, native) in crate::desktop::profiles::themes::native_accents(rgb) {
        let color = draft
            .color(variant, crate::desktop::appearance::ColorRole::Accent)
            .known()
            .unwrap()
            .unwrap();
        assert_eq!(
            [color.red, color.green, color.blue].map(|v| (v * 255.).round() as u8),
            native
        );
        assert_eq!(
            draft
                .color(variant, crate::desktop::appearance::ColorRole::Outline)
                .known(),
            Some(&None),
            "the active window outline keeps following the accent"
        );
    }
    assert!(
        prepare(&store).unwrap().is_none(),
        "no duplicate transaction on restart"
    );
    assert!(store.latest_change().unwrap().is_none());
    assert!(store.undo_latest(&change.id).is_err());
    assert_ne!(fs::read(store.app_config().unwrap()).unwrap(), before);
    assert!(
        prepare(&store).unwrap().is_none(),
        "Undo is respected until the next wallpaper"
    );
}

#[test]
fn automatic_colors_revisits_wallpaper_after_runtime_undo_and_restart() {
    let (fixture, store) = fixture("automatic-revisit-runtime-undo");
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    let first = fs::read(store.app_config().unwrap()).unwrap();
    set_color(&fixture, [0.8, 0.3, 0.2]);
    let second = commit(&store, prepare(&store).unwrap().unwrap())
        .unwrap()
        .unwrap();
    assert!(store.undo_latest(&second.id).is_err());
    assert_ne!(fs::read(store.app_config().unwrap()).unwrap(), first);
    drop(store);
    let restarted = fixture
        .store()
        .with_app_config(fixture.0.join("config.json"));
    assert!(
        prepare(&restarted).unwrap().is_none(),
        "Undo survives restart"
    );
    set_color(&fixture, [0.2, 0.5, 0.7]);
    let before = fixture.tree();
    let plan = prepare(&restarted).unwrap();
    assert_eq!(fixture.tree(), before, "preparation is read-only");
    if let Some(plan) = plan {
        let preview = fixture
            .store()
            .with_app_config(fixture.0.join("config.json"))
            .read_only("Preview");
        assert!(prepare(&preview).unwrap().is_none());
        assert!(commit(&preview, plan.clone()).is_err());
        assert_eq!(fixture.tree(), before, "preview writes no acknowledgement");
        assert!(commit(&restarted, plan).unwrap().is_some());
    }
    assert_eq!(fs::read(restarted.app_config().unwrap()).unwrap(), first);
    assert!(
        restarted.latest_change().unwrap().is_none(),
        "matching colors need no new Undo entry"
    );
    assert!(prepare(&restarted).unwrap().is_none());
    set_color(&fixture, [0.8, 0.3, 0.2]);
    let plan = prepare(&restarted)
        .unwrap()
        .expect("Undo suppression ends after observing a different wallpaper");
    commit(&restarted, plan).unwrap();
    assert_toned_from(
        raw(&restarted)["appearance"]["accent"].clone(),
        [204, 77, 51],
    );
}

#[test]
fn automatic_colors_stale_picture_disabled_mode_and_new_draft_cannot_commit() {
    let (fixture, store) = fixture("automatic-stale");
    let plan = prepare(&store).unwrap().unwrap();
    set_color(&fixture, [0.8, 0.3, 0.2]);
    assert!(commit(&store, plan).is_err());
    assert!(raw(&store)["appearance"]["accent"].is_null());
    let plan = prepare(&store).unwrap().unwrap();
    let mut config = raw(&store);
    config["appearance"]["wallpaper_colors"] = false.into();
    fs::write(store.app_config().unwrap(), config.to_string()).unwrap();
    assert!(commit(&store, plan).is_err());
    assert!(prepare(&store).unwrap().is_none());
    config["appearance"]["wallpaper_colors"] = true.into();
    fs::write(store.app_config().unwrap(), config.to_string()).unwrap();
    let plan = prepare(&store).unwrap().unwrap();
    fs::create_dir_all(store.state_dir()).unwrap();
    fs::write(
        store.state_dir().join("settings-draft-appearance.json"),
        "pending",
    )
    .unwrap();
    assert!(
        commit(&store, plan).is_ok(),
        "old proposals never hold applied colors"
    );
    assert!(prepare(&store).unwrap().is_none());
    assert!(store.latest_change().unwrap().is_none());
}

#[test]
fn automatic_colors_manual_adjustment_lasts_until_next_wallpaper() {
    let (fixture, store) = fixture("automatic-override");
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    let mut config = raw(&store);
    config["appearance"]["accent"] = serde_json::json!([211, 71, 98]);
    fs::write(store.app_config().unwrap(), config.to_string()).unwrap();
    assert!(prepare(&store).unwrap().is_none());
    set_color(&fixture, [0.1, 0.7, 0.3]);
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    assert_ne!(
        raw(&store)["appearance"]["accent"],
        serde_json::json!([211, 71, 98])
    );
    assert_eq!(raw(&store)["appearance"]["wallpaper_colors"], true);
}

#[test]
fn automatic_colors_use_one_display_and_current_slideshow_frame() {
    let (fixture, store) = fixture("automatic-slideshow");
    fs::write(fixture.0.join("outputs"), "HDMI-1\nDP-1\neDP-1\n").unwrap();
    let folder = fixture.0.join("slides");
    fs::create_dir_all(&folder).unwrap();
    let first = folder.join("one.png");
    let second = folder.join("two.png");
    for (path, color) in [(&first, [30, 130, 220]), (&second, [210, 70, 100])] {
        image::RgbImage::from_pixel(16, 16, image::Rgb(color))
            .save(path)
            .unwrap();
    }
    fixture.user(
        NAMESPACE,
        "all",
        &ron::to_string(&BgEntry::new("all", BgSource::Path(folder))).unwrap(),
    );
    let state = media::state_path(&store).unwrap();
    fs::create_dir_all(state.parent().unwrap()).unwrap();
    let write_frame = |picture: &Path| {
        fs::write(
            &state,
            ron::to_string(&vec![
                ("DP-1", BgSource::Path(second.clone())),
                ("eDP-1", BgSource::Path(picture.to_path_buf())),
            ])
            .unwrap(),
        )
        .unwrap()
    };
    write_frame(&first);
    let a = source(&store, &WallpaperDraft::load(&store).unwrap());
    assert!(a.label.contains("Built-in"));
    assert_toned_from(
        serde_json::json!(recommended(&a.analyse().unwrap()).unwrap()),
        [30, 130, 220],
    );
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    write_frame(&second);
    let b = source(&store, &WallpaperDraft::load(&store).unwrap());
    assert_ne!(a.stamp, b.stamp);
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    assert_toned_from(raw(&store)["appearance"]["accent"].clone(), [210, 70, 100]);
}

#[test]
fn automatic_colors_failure_preserves_last_valid_color_and_replaced_files_recompute() {
    let (fixture, store) = fixture("automatic-failure");
    let path = fixture.0.join("wallpaper.png");
    fixture.user(
        NAMESPACE,
        "all",
        &ron::to_string(&BgEntry::new("all", BgSource::Path(path.clone()))).unwrap(),
    );
    assert!(prepare(&store).unwrap_err().contains("missing"));
    image::RgbImage::from_pixel(16, 16, image::Rgb([44, 160, 140]))
        .save(&path)
        .unwrap();
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    let valid = raw(&store);
    fs::write(&path, "not a picture").unwrap();
    assert!(prepare(&store).is_err());
    assert_eq!(raw(&store), valid);
    image::RgbImage::from_pixel(20, 20, image::Rgb([220, 100, 50]))
        .save(&path)
        .unwrap();
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    assert_toned_from(raw(&store)["appearance"]["accent"].clone(), [220, 100, 50]);
}

#[test]
fn automatic_colors_profiles_preserve_wallpaper_link_unless_they_choose_explicit_colors() {
    use crate::desktop::profiles::{
        self, BackgroundLook, Domain, Installed, Profile, SourceLook, WallpaperLook,
    };
    let (_fixture, store) = fixture("automatic-profile");
    let mut profile = Profile::new("Background only").unwrap();
    profile.wallpaper = Some(WallpaperLook {
        background: Some(BackgroundLook {
            source: Some(SourceLook::Color(profiles::Rgb::Hex("#D34762".into()))),
            ..Default::default()
        }),
        ..Default::default()
    });
    let plan = profiles::plan(
        &store,
        &Installed::default(),
        &profile,
        &[Domain::Wallpaper].into(),
    );
    profiles::apply(&store, &plan).unwrap();
    assert_eq!(raw(&store)["appearance"]["wallpaper_colors"], true);
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    let mut profile = Profile::new("Fixed rose").unwrap();
    profile.widget_style = Some(serde_json::json!({"theme":"rose"}));
    let plan = profiles::plan(
        &store,
        &Installed::default(),
        &profile,
        &[Domain::WidgetStyle].into(),
    );
    let change = profiles::apply(&store, &plan).unwrap();
    assert_eq!(raw(&store)["appearance"]["wallpaper_colors"], false);
    assert!(prepare(&store).unwrap().is_none());
    store.undo_latest(&change.id).unwrap();
    assert_eq!(raw(&store)["appearance"]["wallpaper_colors"], true);
    // An appearance-only profile with an explicit theme accent also wins.
    profile.widget_style = None;
    profile.appearance = Some(profiles::AppearanceLook {
        dark: Some(profiles::VariantLook {
            accent: Some(profiles::ColorChoice::Hex("#336699".into())),
            ..Default::default()
        }),
        ..Default::default()
    });
    let plan = profiles::plan(
        &store,
        &Installed::default(),
        &profile,
        &[Domain::Appearance].into(),
    );
    profiles::apply(&store, &plan).unwrap();
    assert_eq!(raw(&store)["appearance"]["wallpaper_colors"], false);
}

#[test]
fn shared_themes_follow_wallpaper_without_erasing_custom_native_surface_or_geometry() {
    use crate::desktop::appearance::{ColorRole, Control, ThemeVariant, Value as Edit};
    use crate::desktop::profiles::{self, themes};
    use crate::desktop_theme::Family;
    let (fixture, store) = fixture("shared-themes-auto");
    let mut p = themes::record(Family::Paper);
    p.widget_style.as_mut().unwrap()["appearance"]["wallpaper_colors"] = true.into();
    let installed = profiles::Installed::discover(&store);
    profiles::apply(
        &store,
        &profiles::plan(&store, &installed, &p, &p.domains().into_iter().collect()),
    )
    .unwrap();
    let plan = prepare(&store).unwrap().unwrap();
    commit(&store, plan).unwrap();
    let mut custom = AppearanceDraft::load(&store).unwrap();
    let explicit = Srgba::new(0.81, 0.87, 0.95, 1.);
    custom.set(
        Control::Color(ThemeVariant::Light, ColorRole::Background),
        Edit::Color(Some(explicit)),
    );
    store
        .apply_owned(
            &custom.plan().patch,
            None,
            Owner::Appearance,
            "Custom background",
        )
        .unwrap();
    let before = raw(&store);
    set_color(&fixture, [0.9, 0.85, 0.7]);
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    let after = raw(&store);
    assert_eq!(after["appearance"]["family"], "paper");
    assert_eq!(
        after["appearance"]["corner_radius"],
        before["appearance"]["corner_radius"]
    );
    assert_ne!(
        after["appearance"]["accent"],
        before["appearance"]["accent"]
    );
    assert_eq!(after["appearance"]["wallpaper_light"], true);
    let color = AppearanceDraft::load(&store)
        .unwrap()
        .color(ThemeVariant::Light, ColorRole::Background)
        .known()
        .copied()
        .flatten()
        .unwrap();
    assert!((color.red - explicit.red).abs() < 0.005 && (color.blue - explicit.blue).abs() < 0.005);
    set_color(&fixture, [0.1, 0.2, 0.3]);
    let prepared = prepare(&store).unwrap().unwrap();
    let mut config = raw(&store);
    config["appearance"]["family"] = "outline".into();
    fs::write(store.app_config().unwrap(), config.to_string()).unwrap();
    assert!(
        commit(&store, prepared).is_err(),
        "a family change invalidates color analysis"
    );
}

#[test]
fn shared_theme_fixed_colors_keep_the_theme_undo_until_wallpaper_polarity_changes() {
    use crate::desktop::profiles::{self, themes};
    use crate::desktop_theme::Family;
    let (fixture, store) = fixture("shared-theme-undo-polarity");
    set_color(&fixture, [0.04, 0.06, 0.05]);
    let p = themes::record(Family::Bare);
    let plan = profiles::plan(
        &store,
        &profiles::Installed::discover(&store),
        &p,
        &p.domains().into_iter().collect(),
    );
    let change = profiles::apply(&store, &plan).unwrap();
    let before = fs::read(store.app_config().unwrap()).unwrap();
    // An absent default false must not become a spurious automatic transaction.
    assert!(raw(&store)["appearance"]["wallpaper_light"].is_null());
    assert!(
        commit(&store, prepare(&store).unwrap().unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(store.latest_change().unwrap().unwrap().id, change.id);
    assert_eq!(fs::read(store.app_config().unwrap()).unwrap(), before);
    // Settings acknowledges fixed colors too, so reopening/restarting keeps Undo.
    keep_manual_colors(&store).unwrap();
    assert!(prepare(&store).unwrap().is_none());
    set_color(&fixture, [0.9, 0.9, 0.85]);
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    assert_eq!(raw(&store)["appearance"]["wallpaper_light"], true);
    assert_eq!(raw(&store)["appearance"]["family"], "bare");
    assert!(raw(&store)["appearance"]["accent"].is_null());
}

#[test]
fn explicit_policy_and_override_intent_invalidate_inflight_colors() {
    let (_fixture, store) = fixture("automatic-policy-race");
    let plan = prepare(&store).unwrap().unwrap();
    let mut value = raw(&store);
    value["appearance"]["color_policy"] = "custom".into();
    fs::write(store.app_config().unwrap(), value.to_string()).unwrap();
    assert!(commit(&store, plan).is_err());
    assert!(
        prepare(&store).unwrap().is_none(),
        "explicit policy wins over a legacy boolean"
    );
    value["appearance"]["color_policy"] = "wallpaper".into();
    fs::write(store.app_config().unwrap(), value.to_string()).unwrap();
    let plan = prepare(&store).unwrap().unwrap();
    value["appearance"]["overrides"] = (1u64 << 24).into();
    fs::write(store.app_config().unwrap(), value.to_string()).unwrap();
    assert!(
        commit(&store, plan).is_err(),
        "an equal-valued manual override is still newer intent"
    );
}

#[test]
fn task2_restore_invalidates_prepared_palette_even_when_all_values_match() {
    let (f, store) = fixture("epoch-prepared");
    let prepared = prepare(&store).unwrap().unwrap();
    crate::desktop::recovery::ensure_baseline(
        &store,
        crate::desktop::recovery::BaselineKind::CurrentDesktop,
    )
    .unwrap();
    let plan = crate::desktop::recovery::plan_restore(
        &store,
        &[crate::desktop::recovery::RecoveryDomain::DockPanel],
    )
    .unwrap();
    crate::desktop::recovery::apply_restore(&store, &plan).unwrap();
    let before = fs::read(store.app_config().unwrap()).unwrap();
    assert!(
        commit(&store, prepared).is_err(),
        "matching values do not grant stale ownership"
    );
    assert_eq!(fs::read(store.app_config().unwrap()).unwrap(), before);
    assert!(
        prepare(&store).unwrap().is_none(),
        "manual restore owns this wallpaper"
    );
    set_color(&f, [0.8, 0.3, 0.2]);
    assert!(
        prepare(&store).unwrap().is_some(),
        "the next wallpaper must still update"
    );
}

#[test]
fn task2_noop_ack_cannot_cross_restore_or_its_undo() {
    let (f, store) = fixture("epoch-noop");
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    // Delete only observation evidence to prepare a genuine no-op result.
    let mut history: serde_json::Value =
        serde_json::from_slice(&fs::read(store.state_dir().join("desktop-undo.json")).unwrap())
            .unwrap();
    history.as_object_mut().unwrap().remove("palette_ack");
    fs::write(
        store.state_dir().join("desktop-undo.json"),
        serde_json::to_vec(&history).unwrap(),
    )
    .unwrap();
    let _ = fs::remove_file(store.state_dir().join(ACK));
    let prepared = prepare(&store).unwrap().unwrap();
    assert!(prepared.patch.is_empty() && prepared.widgets.is_empty());
    let plan = crate::desktop::recovery::plan_restore(
        &store,
        &[crate::desktop::recovery::RecoveryDomain::DockPanel],
    )
    .unwrap();
    let restored = crate::desktop::recovery::apply_restore(&store, &plan).unwrap();
    assert!(commit(&store, prepared.clone()).is_err());
    store.undo_latest(&restored.id).unwrap();
    assert!(commit(&store, prepared).is_err());
    let restarted = f.store().with_app_config(f.0.join("config.json"));
    assert!(prepare(&restarted).unwrap().is_none());
    set_color(&f, [0.4, 0.7, 0.2]);
    assert!(prepare(&restarted).unwrap().is_some());
}

#[test]
fn task2_interrupted_palette_never_acknowledges_before_its_transaction() {
    for fault in [
        crate::desktop::store::Fault::InterruptBeforeRecord,
        crate::desktop::store::Fault::InterruptAfterRecord,
    ] {
        let (f, store) = fixture("palette-killed");
        let prepared = prepare(&store).unwrap().unwrap();
        store.inject(Some(fault));
        assert!(commit(&store, prepared).is_err());
        let restarted = f.store().with_app_config(f.0.join("config.json"));
        assert!(restarted.pending_recovery().unwrap().is_some());
        restarted.recover().unwrap();
        assert!(
            prepare(&restarted).unwrap().is_some(),
            "recovery must restore both colors and observation"
        );
    }
}

#[test]
fn task2_manual_undo_preserves_latest_palette_observation() {
    let (f, store) = fixture("manual-undo-observation");
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    let mut widgets = WidgetPatch::new();
    widgets
        .set("/appearance/opacity", None, Some(serde_json::json!(90)))
        .unwrap();
    let manual = store
        .apply_action(
            &DesktopPatch::new(),
            Some(&widgets),
            Owner::Preferences,
            "Opacity",
            crate::desktop::store::Source::Manual,
        )
        .unwrap();
    set_color(&f, [0.8, 0.3, 0.2]);
    commit(&store, prepare(&store).unwrap().unwrap()).unwrap();
    store.undo_latest(&manual.id).unwrap();
    assert!(
        prepare(&store).unwrap().is_none(),
        "Undo must retain the latest source observation"
    );
}

#[test]
fn task2_interrupted_restore_recovery_keeps_manual_colors_and_invalidates_worker() {
    let (f, store) = fixture("restore-crash-observation");
    let stale = prepare(&store).unwrap().unwrap();
    crate::desktop::recovery::ensure_baseline(
        &store,
        crate::desktop::recovery::BaselineKind::CurrentDesktop,
    )
    .unwrap();
    let plan = crate::desktop::recovery::plan_restore(
        &store,
        &[crate::desktop::recovery::RecoveryDomain::DockPanel],
    )
    .unwrap();
    store.inject(Some(crate::desktop::store::Fault::InterruptBeforeRecord));
    assert!(crate::desktop::recovery::apply_restore(&store, &plan).is_err());
    let restarted = f.store().with_app_config(f.0.join("config.json"));
    restarted.recover().unwrap();
    assert!(commit(&restarted, stale).is_err());
    assert!(
        prepare(&restarted).unwrap().is_none(),
        "recovery is still a manual choice"
    );
    set_color(&f, [0.8, 0.3, 0.2]);
    assert!(prepare(&restarted).unwrap().is_some());
}
