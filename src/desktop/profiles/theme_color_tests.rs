//! Native color roles a shared theme coordinates: the active-window outline
//! and accents readable in each COSMIC variant. Isolated fixtures only.
use super::*;
use crate::desktop::appearance::{Control, Value as Edit};
use crate::desktop::store::tests::Fixture;
use cosmic::cosmic_theme::palette::{FromColor, Oklab, Srgb};
use std::fs;

fn write_v2(fixture: &Fixture, namespace: &str, key: &str, value: &str) {
    let path = fixture
        .0
        .join("config/cosmic")
        .join(namespace)
        .join("v2")
        .join(key);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value).unwrap();
}

fn lightness(color: cosmic::cosmic_theme::palette::Srgba) -> f32 {
    Oklab::from_color(Srgb::new(color.red, color.green, color.blue)).l
}

/// The reported desktop: an exact dark wallpaper swatch as the accent on
/// both variants and an older custom active-window outline color.
fn reported(name: &str, dark: bool) -> (Fixture, DesktopStore, Installed) {
    let fixture = Fixture::new(name);
    for variant in ["Dark", "Light"] {
        write_v2(
            &fixture,
            &format!("com.system76.CosmicTheme.{variant}.Builder"),
            "accent",
            "Some(\"#40352CFF\")",
        );
    }
    write_v2(
        &fixture,
        "com.system76.CosmicTheme.Dark.Builder",
        "window_hint",
        "Some(\"#63D0DFFF\")",
    );
    fixture.user(
        "com.system76.CosmicTheme.Mode",
        "is_dark",
        &dark.to_string(),
    );
    let path = fixture.0.join("config.json");
    fs::write(
        &path,
        r#"{"theme":"amber","appearance":{"accent":[64,53,44],"color_source":"custom","sync_colors":true,"wallpaper_colors":true}}"#,
    )
    .unwrap();
    let store = fixture.store().with_app_config(path);
    let installed = Installed::discover(&store);
    (fixture, store, installed)
}

#[test]
fn applying_a_theme_links_the_window_outline_and_keeps_accents_readable_in_both_modes() {
    let mut failures = Vec::new();
    for dark in [true, false] {
        for family in Family::ALL {
            let (_fixture, store, installed) =
                reported(&format!("theme-outline-{}-{dark}", family.id()), dark);
            let mut p = record(family);
            preserve_custom(&store, &installed, &mut p).unwrap();
            let plan = plan(&store, &installed, &p, &p.domains().into_iter().collect());
            assert!(plan.can_apply(), "{:?}", plan.blocking_reasons());
            apply(&store, &plan).unwrap();
            let draft = AppearanceDraft::load(&store).unwrap();
            let mode = if dark { "dark mode" } else { "light mode" };
            for variant in ThemeVariant::ALL {
                let outline = draft.color(variant, ColorRole::Outline).known().copied();
                if outline != Some(None) {
                    failures.push(format!(
                        "{family:?} in {mode}: {variant:?} active window outline stays {:?} instead of following the accent",
                        outline.flatten().map(|c| appearance::hex(c, false))
                    ));
                }
            }
            let accent = |variant| {
                draft
                    .color(variant, ColorRole::Accent)
                    .known()
                    .copied()
                    .flatten()
                    .expect("themes set an accent")
            };
            // COSMIC's own palettes: pastel accents in Dark, deep ones in Light.
            for (variant, ok, want) in [
                (ThemeVariant::Dark, 0.68..=1.0, ">= 0.68"),
                (ThemeVariant::Light, 0.0..=0.56, "<= 0.56"),
            ] {
                let color = accent(variant);
                let l = lightness(color);
                if !ok.contains(&l) {
                    failures.push(format!(
                        "{family:?} in {mode}: {variant:?} accent {} has Oklab L {l:.2}, want {want}",
                        appearance::hex(color, false)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn outline(
    store: &DesktopStore,
    variant: ThemeVariant,
) -> Option<Option<cosmic::cosmic_theme::palette::Srgba>> {
    AppearanceDraft::load(store)
        .unwrap()
        .color(variant, ColorRole::Outline)
        .known()
        .copied()
}
fn switch(store: &DesktopStore, installed: &Installed, family: Family) {
    let mut p = record(family);
    preserve_custom(store, installed, &mut p).unwrap();
    let plan = plan(store, installed, &p, &p.domains().into_iter().collect());
    assert!(plan.can_apply(), "{:?}", plan.blocking_reasons());
    apply(store, &plan).unwrap();
}

#[test]
fn style_changes_keep_an_outline_chosen_after_themes_and_link_an_older_one() {
    let (fixture, store, installed) = reported("theme-outline-preserve", true);
    switch(&store, &installed, Family::Soft);
    assert_eq!(outline(&store, ThemeVariant::Dark), Some(None));
    // Chosen in Theme & fonts after the first theme: kept like any Custom.
    let chosen = appearance::parse_hex("#FFAD00", false);
    let mut native = AppearanceDraft::load(&store).unwrap();
    native.set(
        Control::Color(ThemeVariant::Dark, ColorRole::Outline),
        Edit::Color(chosen),
    );
    store.apply(&native.plan().patch).unwrap();
    switch(&store, &installed, Family::Paper);
    assert_eq!(outline(&store, ThemeVariant::Dark), Some(chosen));
    assert_eq!(outline(&store, ThemeVariant::Light), Some(None));
    // The outline that predates themes (still the one Before Themes keeps)
    // follows the accent on the next style change, as an older release
    // applied themes without linking it.
    write_v2(
        &fixture,
        "com.system76.CosmicTheme.Dark.Builder",
        "window_hint",
        "Some(\"#63D0DFFF\")",
    );
    switch(&store, &installed, Family::Outline);
    assert_eq!(outline(&store, ThemeVariant::Dark), Some(None));
}

#[test]
fn surfaces_and_accents_from_the_previous_release_stay_linked() {
    for family in Family::ALL {
        let (fixture, store, installed) =
            reported(&format!("theme-previous-release-{}", family.id()), true);
        // Write the previous release's derivation as if it applied `family`.
        let (accent, surface) = desktop_theme::previous_native(family, None);
        let hex = |[r, g, b]: [u8; 3]| format!("Some(\"#{r:02X}{g:02X}{b:02X}FF\")");
        let owned = if family.light() { "Light" } else { "Dark" };
        for variant in ["Dark", "Light"] {
            write_v2(
                &fixture,
                &format!("com.system76.CosmicTheme.{variant}.Builder"),
                "accent",
                &hex(accent),
            );
        }
        for key in ["bg_color", "primary_container_bg"] {
            write_v2(
                &fixture,
                &format!("com.system76.CosmicTheme.{owned}.Builder"),
                key,
                &hex(surface),
            );
        }
        let mut config: serde_json::Value =
            serde_json::from_slice(&fs::read(store.app_config().unwrap()).unwrap()).unwrap();
        config["appearance"]["family"] = serde_json::json!(family);
        config["appearance"]["accent"] = serde_json::Value::Null;
        config["appearance"]["wallpaper_colors"] = false.into();
        fs::write(store.app_config().unwrap(), config.to_string()).unwrap();
        let (_, typed) = crate::desktop::wallpaper::automatic::read_config(&store).unwrap();
        let draft = AppearanceDraft::load(&store).unwrap();
        let state = native_custom(&draft, typed.appearance);
        assert!(
            !state.surfaces,
            "{family:?}: previous surfaces are not Custom"
        );
        // The stale outline is the one thing that does not follow yet.
        assert!(state.windows, "{family:?}");
        // A recolor moves previous-release surfaces to the current tones.
        let mut next = AppearanceDraft::load(&store).unwrap();
        linked_colors(&mut next, typed.appearance, [200, 120, 60]);
        let variant = if family.light() {
            ThemeVariant::Light
        } else {
            ThemeVariant::Dark
        };
        let moved = next
            .color(variant, ColorRole::Background)
            .known()
            .copied()
            .flatten();
        let target = family.surface([200, 120, 60]);
        assert!(
            moved.is_some_and(|c| {
                [c.red, c.green, c.blue].map(|v| (v * 255.).round() as u8) == target
            }),
            "{family:?}: {moved:?}"
        );
        drop(installed);
    }
}

#[test]
fn unset_window_surfaces_follow_the_style_and_wallpaper_tint() {
    // COSMIC's default leaves the window background unset. That is no one's
    // choice, so windows take the style's tint instead of staying gray.
    let (fixture, store, installed) = reported("theme-unset-surfaces", true);
    for key in ["bg_color", "primary_container_bg"] {
        write_v2(&fixture, "com.system76.CosmicTheme.Dark.Builder", key, "None");
    }
    let mut config: serde_json::Value =
        serde_json::from_slice(&fs::read(store.app_config().unwrap()).unwrap()).unwrap();
    config["appearance"]["family"] = serde_json::json!(Family::Soft);
    config["appearance"]["accent"] = serde_json::json!([224, 138, 243]);
    fs::write(store.app_config().unwrap(), config.to_string()).unwrap();
    let (_, typed) = crate::desktop::wallpaper::automatic::read_config(&store).unwrap();
    let draft = AppearanceDraft::load(&store).unwrap();
    assert!(!native_custom(&draft, typed.appearance).surfaces);
    let mut next = AppearanceDraft::load(&store).unwrap();
    linked_colors(&mut next, typed.appearance, [90, 140, 200]);
    let byte = |role| {
        next.color(ThemeVariant::Dark, role)
            .known()
            .copied()
            .flatten()
            .map(|c| [c.red, c.green, c.blue].map(|v| (v * 255.).round() as u8))
    };
    assert_eq!(byte(ColorRole::Background), Some(Family::Soft.surface([90, 140, 200])));
    assert_eq!(byte(ColorRole::Primary), Some(Family::Soft.container([90, 140, 200])));
    // An explicit background stays the user's own.
    write_v2(&fixture, "com.system76.CosmicTheme.Dark.Builder", "bg_color", "Some(\"#102030FF\")");
    let mut kept = AppearanceDraft::load(&store).unwrap();
    linked_colors(&mut kept, typed.appearance, [90, 140, 200]);
    assert!(!kept.edited(Control::Color(ThemeVariant::Dark, ColorRole::Background)));
    drop(installed);
}

#[test]
fn material_survives_theme_apply_and_captured_setup_restores_native_bars() {
    use crate::materials::{Finish, Kind, Target};
    let (_fixture, store, installed) = reported("material-profile", true);
    crate::desktop::materials::apply(&store, Target::Shared, Some(Finish::new(Kind::Liquid)))
        .unwrap();
    let theme = record(Family::Soft);
    let proposal = plan(
        &store,
        &installed,
        &theme,
        &theme.domains().into_iter().collect(),
    );
    assert!(proposal.can_apply(), "{:?}", proposal.blocking_reasons());
    apply(&store, &proposal).unwrap();
    let (_, cfg) = crate::desktop::wallpaper::automatic::read_config(&store).unwrap();
    assert_eq!(cfg.materials.shared.unwrap().kind, Kind::Liquid);
    let captured = crate::desktop::profiles::capture(&store, "Material setup")
        .unwrap()
        .profile;
    assert_eq!(captured.written_version(), 8);
    crate::desktop::materials::apply(&store, Target::Shared, Some(Finish::new(Kind::Smoked)))
        .unwrap();
    let proposal = plan(&store, &installed, &captured, &[Domain::WidgetStyle].into());
    assert!(proposal.can_apply(), "{:?}", proposal.blocking_reasons());
    let change = apply(&store, &proposal).unwrap();
    for bar in ["Dock", "Panel"] {
        let material = store
            .read_user(&crate::desktop::materials::key(bar))
            .unwrap()
            .unwrap()
            .decode::<dock_islands_core::material::BarMaterial>()
            .unwrap();
        assert_eq!(material.finish.kind, Kind::Liquid);
    }
    store.undo_latest(&change.id).unwrap();
    let (_, cfg) = crate::desktop::wallpaper::automatic::read_config(&store).unwrap();
    assert_eq!(cfg.materials.shared.unwrap().kind, Kind::Smoked);
}
