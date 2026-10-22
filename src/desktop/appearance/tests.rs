use super::*;
use crate::desktop::store::Fault;
use crate::desktop::store::tests::Fixture;
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::cosmic_theme::{CosmicPalette, Theme};
use std::collections::BTreeMap;
use std::path::PathBuf;

const DARK: &str = "com.system76.CosmicTheme.Dark";
const LIGHT: &str = "com.system76.CosmicTheme.Light";
const DARK_BUILDER: &str = "com.system76.CosmicTheme.Dark.Builder";
const LIGHT_BUILDER: &str = "com.system76.CosmicTheme.Light.Builder";
/// The seeded hand-customized button base, as COSMIC stores it.
const CUSTOM_BUTTON: &str = "#33404CCC";

fn read(fixture: &Fixture, namespace: &str, version: u64, key: &str) -> Option<String> {
    std::fs::read_to_string(
        fixture
            .0
            .join("config/cosmic")
            .join(namespace)
            .join(format!("v{version}"))
            .join(key),
    )
    .ok()
}
fn write(fixture: &Fixture, namespace: &str, version: u64, key: &str, value: &str) {
    let path = fixture
        .0
        .join("config/cosmic")
        .join(namespace)
        .join(format!("v{version}"))
        .join(key);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, value).unwrap();
}
/// Config files only (the state directory holds the lock and journal).
fn config(fixture: &Fixture) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut tree = fixture.tree();
    tree.retain(|path, _| path.starts_with(fixture.0.join("config")));
    tree
}

/// Both variants written by the official types, with customizations a
/// person could have made in cosmic-settings, an unknown future key and a
/// customized derived role that no edit should reset.
fn seeded(name: &str) -> Fixture {
    let fixture = Fixture::new(name);
    let root = fixture.0.join("config");
    for (variant, builder) in [
        (ThemeVariant::Dark, ThemeBuilder::dark()),
        (ThemeVariant::Light, ThemeBuilder::light()),
    ] {
        let mut builder = builder.bg_color(Srgba::new(0.18, 0.21, 0.26, 1.));
        builder.spacing = Density::Compact.into();
        builder.gaps = (2, 8);
        builder.window_hint = Some(Srgb::new(0.7, 0.3, 0.2));
        let b = Config::with_custom_path(variant.builder_namespace(), 2, root.clone()).unwrap();
        builder.write_entry(&b).unwrap();
        let mut theme = builder.build();
        // A hand-customized ordinary button: the base color is not derived.
        theme.button.base = Srgba::new(0.2, 0.25, 0.3, 0.8);
        let t = Config::with_custom_path(variant.theme_namespace(), 2, root.clone()).unwrap();
        theme.write_entry(&t).unwrap();
        write(
            &fixture,
            variant.theme_namespace(),
            2,
            "future_control",
            "(keep: true)",
        );
    }
    write(&fixture, MODE, 1, "is_dark", "true");
    write(&fixture, MODE, 1, "auto_switch", "false");
    write(&fixture, TOOLKIT, 1, "interface_density", "Compact");
    write(&fixture, TOOLKIT, 1, "header_size", "Compact");
    write(
        &fixture,
        "com.system76.CosmicPanel.Dock",
        1,
        "border_radius",
        "160",
    );
    write(&fixture, "com.system76.CosmicPanel.Dock", 1, "spacing", "0");
    fixture
}

fn draft(fixture: &Fixture) -> AppearanceDraft {
    AppearanceDraft::load(&fixture.store()).unwrap()
}

#[test]
fn loading_and_previewing_write_nothing_and_decode_live_formats() {
    let fixture = seeded("appearance-load");
    // An absent variant and the live hex encoding of a builder color.
    write(&fixture, LIGHT_BUILDER, 2, "accent", "Some(\"#88BDFFFF\")");
    let before = fixture.tree();
    let draft = draft(&fixture);
    assert_eq!(draft.mode(), Setting::Known(SystemMode::Dark));
    assert_eq!(draft.density(), Setting::Known(Density::Compact));
    assert_eq!(
        draft.color(ThemeVariant::Light, ColorRole::Accent),
        Setting::Known(Some(parse_hex("#88BDFF", false).unwrap()))
    );
    assert_eq!(
        draft.color(ThemeVariant::Dark, ColorRole::Accent),
        Setting::Known(None),
        "automatic accent"
    );
    assert_eq!(draft.outline_width(ThemeVariant::Dark), Setting::Known(3));
    assert_eq!(draft.icon_theme(), Setting::Known("Cosmic".into()));
    assert_eq!(
        draft.window(WindowEffect::ShadowTiled),
        Setting::Known(false),
        "absent: cosmic-comp default"
    );
    for variant in ThemeVariant::ALL {
        let preview = draft.preview(variant);
        assert_eq!(hex(preview.button, true), CUSTOM_BUTTON);
        assert_eq!(preview.outline_width, 3);
    }
    assert!(!draft.dirty());
    assert!(draft.patch().is_empty());
    assert_eq!(fixture.tree(), before);
    assert!(!fixture.0.join("state").exists());
}

#[test]
fn a_light_accent_edit_keeps_dark_mode_custom_roles_and_unknown_keys() {
    let fixture = seeded("appearance-light");
    let store = fixture.store();
    let mut draft = draft(&fixture);
    let before = config(&fixture);
    let accent = parse_hex("#E5A50A", false).unwrap();
    draft.set(
        Control::Color(ThemeVariant::Light, ColorRole::Accent),
        Value::Color(Some(accent)),
    );
    let patch = draft.patch();
    assert!(
        patch
            .keys()
            .all(|key| key.namespace() == LIGHT || key.namespace() == LIGHT_BUILDER),
        "{:?}",
        patch.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        fixture.tree().len(),
        before.len(),
        "planning writes nothing"
    );
    store.apply(&patch).unwrap();
    let after = config(&fixture);
    for (path, contents) in &before {
        let path_text = path.to_string_lossy();
        if !path_text.contains(&format!("/{LIGHT}/"))
            && !path_text.contains(&format!("/{LIGHT_BUILDER}/"))
        {
            assert_eq!(after.get(path), Some(contents), "{path_text} changed");
        }
    }
    assert_eq!(
        read(&fixture, LIGHT_BUILDER, 2, "accent").as_deref(),
        Some("Some(\"#E5A50AFF\")")
    );
    let light = AppearanceDraft::load(&store).unwrap();
    let preview = light.preview(ThemeVariant::Light);
    let expected = {
        let mut builder = light.base_builder(ThemeVariant::Light).unwrap();
        builder.accent = Some(Srgb::new(accent.red, accent.green, accent.blue));
        builder.build()
    };
    assert_eq!(hex(preview.accent, true), hex(expected.accent.base, true));
    // The customized button base stays; its accent-derived focus follows.
    assert_eq!(hex(preview.button, true), CUSTOM_BUTTON);
    let button: Component = RawValue::parse(read(&fixture, LIGHT, 2, "button").unwrap())
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(hex(button.focus, true), hex(expected.button.focus, true));
    for key in [
        "future_control",
        "spacing",
        "gaps",
        "window_hint",
        "corner_radii",
        "palette",
    ] {
        assert_eq!(
            read(&fixture, LIGHT, 2, key),
            before
                .get(
                    &fixture
                        .0
                        .join("config/cosmic")
                        .join(LIGHT)
                        .join("v2")
                        .join(key)
                )
                .cloned()
                .flatten()
                .map(|bytes| String::from_utf8(bytes).unwrap()),
            "{key}"
        );
    }
    assert_eq!(read(&fixture, MODE, 1, "is_dark").as_deref(), Some("true"));
}

#[test]
fn a_mode_edit_touches_only_the_mode_and_undo_restores_absence() {
    let fixture = seeded("appearance-mode");
    let store = fixture.store();
    std::fs::remove_file(
        fixture
            .0
            .join("config/cosmic")
            .join(MODE)
            .join("v1/auto_switch"),
    )
    .unwrap();
    let mut draft = draft(&fixture);
    draft.set(Control::Mode, Value::Mode(SystemMode::Automatic));
    let patch = draft.patch();
    assert_eq!(
        patch.keys().map(ToString::to_string).collect::<Vec<_>>(),
        ["com.system76.CosmicTheme.Mode/v1/auto_switch"]
    );
    let before = config(&fixture);
    let change = store.apply(&patch).unwrap();
    assert_eq!(
        read(&fixture, MODE, 1, "auto_switch").as_deref(),
        Some("true")
    );
    assert_eq!(read(&fixture, MODE, 1, "is_dark").as_deref(), Some("true"));
    store.undo(&change).unwrap();
    assert_eq!(config(&fixture), before, "absence restored");
    // Choosing the current mode is not an edit.
    let mut draft = AppearanceDraft::load(&store).unwrap();
    draft.set(Control::Mode, Value::Mode(SystemMode::Dark));
    assert!(!draft.dirty());
    draft.set(Control::Mode, Value::Mode(SystemMode::Light));
    let keys: Vec<_> = draft
        .patch()
        .keys()
        .map(|key| key.key().to_string())
        .collect();
    assert_eq!(keys, ["is_dark"]);
}

#[test]
fn global_controls_write_both_variants_and_never_panel_settings() {
    let fixture = seeded("appearance-global");
    let store = fixture.store();
    let mut draft = draft(&fixture);
    draft.set(Control::Roundness, Value::Roundness(Roundness::Square));
    draft.set(Control::Density, Value::Density(Density::Spacious));
    draft.set(Control::OutlineWidth, Value::Number(12));
    let patch = draft.patch();
    assert!(
        patch
            .keys()
            .all(|key| !key.namespace().starts_with("com.system76.CosmicPanel"))
    );
    store.apply(&patch).unwrap();
    for namespace in [DARK_BUILDER, LIGHT_BUILDER, DARK, LIGHT] {
        // The inner gap follows a wider outline, as in cosmic-settings.
        let gaps: (u32, u32) = RawValue::parse(read(&fixture, namespace, 2, "gaps").unwrap())
            .unwrap()
            .decode()
            .unwrap();
        assert_eq!(gaps, (2, 12), "{namespace}");
        let radii: CornerRadii =
            RawValue::parse(read(&fixture, namespace, 2, "corner_radii").unwrap())
                .unwrap()
                .decode()
                .unwrap();
        assert_eq!(radii, Roundness::Square.into());
    }
    assert_eq!(
        read(&fixture, TOOLKIT, 1, "interface_density").as_deref(),
        Some("Spacious")
    );
    assert_eq!(
        read(&fixture, TOOLKIT, 1, "header_size").as_deref(),
        Some("Spacious")
    );
    assert_eq!(
        read(
            &fixture,
            "com.system76.CosmicPanel.Dock",
            1,
            "border_radius"
        )
        .as_deref(),
        Some("160")
    );
    assert_eq!(
        read(&fixture, "com.system76.CosmicPanel.Dock", 1, "spacing").as_deref(),
        Some("0")
    );
    let draft = AppearanceDraft::load(&store).unwrap();
    assert_eq!(
        draft.roundness(ThemeVariant::Light),
        Setting::Known(Roundness::Square)
    );
    assert_eq!(draft.preview(ThemeVariant::Dark).outline_width, 12);
}

#[test]
fn fonts_icons_effects_and_window_options_write_only_their_keys() {
    let fixture = seeded("appearance-toolkit");
    write(
        &fixture,
        COMPOSITOR,
        1,
        "appearance_settings",
        "(clip_floating_windows: true, clip_tiled_windows: false, shadow_tiled_windows: true)",
    );
    let store = fixture.store();
    let mut draft = draft(&fixture);
    let mut font = FontRole::Interface.default();
    font.family = "Fira Sans".into();
    font.weight = cosmic::iced::font::Weight::Semibold;
    draft.set(
        Control::Font(FontRole::Interface),
        Value::Font(font.clone()),
    );
    draft.set(Control::IconTheme, Value::Name("Pop".into()));
    draft.set(
        Control::Window(WindowEffect::ClipFloating),
        Value::Flag(false),
    );
    draft.set(Control::Frosted(Surface::Panel), Value::Flag(true));
    draft.set(Control::Blur, Value::Blur(BlurStrength::High));
    let patch = draft.patch();
    store.apply(&patch).unwrap();
    let stored: FontConfig = RawValue::parse(read(&fixture, TOOLKIT, 1, "interface_font").unwrap())
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(stored, font, "stretch and style kept");
    assert!(read(&fixture, TOOLKIT, 1, "monospace_font").is_none());
    assert_eq!(
        read(&fixture, TOOLKIT, 1, "icon_theme").as_deref(),
        Some("\"Pop\"")
    );
    let comp: CompositorAppearance =
        RawValue::parse(read(&fixture, COMPOSITOR, 1, "appearance_settings").unwrap())
            .unwrap()
            .decode()
            .unwrap();
    assert_eq!(
        comp,
        CompositorAppearance {
            clip_floating_windows: false,
            clip_tiled_windows: false,
            shadow_tiled_windows: true,
        }
    );
    for namespace in [DARK_BUILDER, DARK] {
        assert_eq!(
            read(&fixture, namespace, 2, "frosted_panel").as_deref(),
            Some("true")
        );
        assert_eq!(
            read(&fixture, namespace, 2, "frosted").as_deref(),
            Some("High")
        );
    }
    // Frosting changes the derived transparent containers' alpha.
    let draft = AppearanceDraft::load(&store).unwrap();
    assert!(draft.preview(ThemeVariant::Dark).frosted);
    let transparent: Container =
        RawValue::parse(read(&fixture, DARK, 2, "transparent_background").unwrap())
            .unwrap()
            .decode()
            .unwrap();
    let alpha = AlphaMap::default().blurred_alpha(BlurStrength::High);
    assert!(
        (transparent.base.alpha - alpha).abs() < 0.01,
        "{}",
        transparent.base.alpha
    );
}

#[test]
fn unknown_and_malformed_values_stay_raw_and_block_only_dependent_edits() {
    let fixture = seeded("appearance-unknown");
    // A future member inside a composite, a malformed font and a future
    // compositor member.
    let button = read(&fixture, LIGHT, 2, "button").unwrap();
    let future = button.trim_end().trim_end_matches(')').to_string() + "    future_glow: 0.5,\n)";
    write(&fixture, LIGHT, 2, "button", &future);
    write(
        &fixture,
        TOOLKIT,
        1,
        "monospace_font",
        "(family: \"Mono\", weight: Wobbly)",
    );
    write(
        &fixture,
        COMPOSITOR,
        1,
        "appearance_settings",
        "(clip_floating_windows: true, clip_tiled_windows: true, shadow_tiled_windows: false, rounded_popups: true)",
    );
    let store = fixture.store();
    let mut draft = draft(&fixture);
    assert!(
        matches!(draft.font(FontRole::Monospace), Setting::Unsupported(raw) if raw.contains("Wobbly"))
    );
    assert!(matches!(
        draft.window(WindowEffect::ClipTiled),
        Setting::Unsupported(_)
    ));
    // A light accent edit would rewrite the button: blocked, with the reason.
    draft.set(
        Control::Color(ThemeVariant::Light, ColorRole::Accent),
        Value::Color(parse_hex("#33D17A", false)),
    );
    draft.set(Control::Window(WindowEffect::ClipTiled), Value::Flag(false));
    let blocked = draft.blocked();
    let keys: Vec<_> = blocked.iter().map(|b| b.key.to_string()).collect();
    assert_eq!(
        keys,
        [
            "com.system76.CosmicTheme.Light/v2/button",
            "com.system76.CosmicComp/v1/appearance_settings"
        ]
    );
    assert!(blocked[0].reason.contains("does not recognize"));
    // Nothing for the blocked edits reaches the patch; the dark variant and
    // the unknown values are untouched by an unrelated dark edit.
    let patch = draft.patch();
    assert!(
        patch
            .keys()
            .all(|key| key.namespace() != LIGHT && key.namespace() != COMPOSITOR)
    );
    let mut dark = AppearanceDraft::load(&store).unwrap();
    dark.set(
        Control::Color(ThemeVariant::Dark, ColorRole::Accent),
        Value::Color(parse_hex("#33D17A", false)),
    );
    assert!(dark.blocked().is_empty());
    store.apply(&dark.patch()).unwrap();
    assert_eq!(
        read(&fixture, LIGHT, 2, "button").as_deref(),
        Some(future.as_str())
    );
    assert!(
        read(&fixture, TOOLKIT, 1, "monospace_font")
            .unwrap()
            .contains("Wobbly")
    );
    // A builder input that cannot be read makes that variant's colors unsupported.
    write(&fixture, DARK_BUILDER, 2, "palette", "Dark(())");
    let dark = AppearanceDraft::load(&store).unwrap();
    assert!(matches!(
        dark.color(ThemeVariant::Dark, ColorRole::Accent),
        Setting::Unsupported(_)
    ));
    assert!(matches!(
        dark.color(ThemeVariant::Light, ColorRole::Accent),
        Setting::Known(_)
    ));
    assert!(decode_exact::<CompositorAppearance>(
        &RawValue::parse("(clip_floating_windows: true, clip_tiled_windows: true, shadow_tiled_windows: false)").unwrap()
    )
    .is_ok());
}

#[test]
fn a_failed_multi_key_apply_restores_every_original() {
    let fixture = seeded("appearance-rollback");
    let store = fixture.store();
    let mut draft = draft(&fixture);
    draft.set(
        Control::Color(ThemeVariant::Dark, ColorRole::Background),
        Value::Color(parse_hex("#101820", true)),
    );
    draft.set(Control::IconTheme, Value::Name("Pop".into()));
    let patch = draft.patch();
    assert!(patch.len() > 5);
    let last = patch.keys().last().unwrap().clone();
    let before = config(&fixture);
    store.inject(Some(Fault::FailWrite(last)));
    let error = store.apply(&patch).unwrap_err();
    assert!(matches!(error, DesktopError::RolledBack(_)), "{error:?}");
    assert_eq!(config(&fixture), before);
    assert_eq!(store.pending_recovery().unwrap(), None);
}

#[test]
fn outside_changes_to_inputs_conflict_coherently_and_unrelated_edits_survive() {
    let fixture = seeded("appearance-conflict");
    let store = fixture.store();
    let mut draft = draft(&fixture);
    draft.set(
        Control::Color(ThemeVariant::Dark, ColorRole::Accent),
        Value::Color(parse_hex("#C061CB", false)),
    );
    draft.set(Control::IconTheme, Value::Name("Pop".into()));
    // Not written by the accent edit, but an input to its derivation.
    let guarded = draft
        .patch()
        .guards()
        .any(|(key, _)| key.to_string() == "com.system76.CosmicTheme.Dark.Builder/v2/bg_color");
    assert!(guarded);
    // Someone changes the dark background (an input) and the light accent
    // (unrelated) while the draft is open.
    write(&fixture, DARK_BUILDER, 2, "bg_color", "Some(\"#000000FF\")");
    write(&fixture, LIGHT_BUILDER, 2, "accent", "Some(\"#FF0000FF\")");
    draft.refresh(store.load(&AppearanceDraft::keys()).unwrap());
    assert_eq!(
        draft
            .conflicts()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["com.system76.CosmicTheme.Dark.Builder/v2/bg_color"]
    );
    assert_eq!(
        draft.color(ThemeVariant::Light, ColorRole::Accent),
        Setting::Known(parse_hex("#FF0000", false)),
        "the unrelated change is adopted"
    );
    let before = config(&fixture);
    let DesktopError::Conflict(keys) = store.apply(&draft.patch()).unwrap_err() else {
        panic!()
    };
    assert_eq!(config(&fixture), before, "nothing written");
    assert_eq!(key_label(&keys[0]), "Dark background");
    // Using the current value drops the accent edit (it depended on the
    // background) and keeps the icon edit.
    draft.resolve_conflicts(store.load(&AppearanceDraft::keys()).unwrap(), &keys);
    assert!(draft.conflicts().is_empty());
    assert_eq!(
        draft.edits().map(|(c, _)| *c).collect::<Vec<_>>(),
        [Control::IconTheme]
    );
    store.apply(&draft.patch()).unwrap();
    assert_eq!(
        read(&fixture, DARK_BUILDER, 2, "bg_color").as_deref(),
        Some("Some(\"#000000FF\")")
    );
    assert_eq!(
        read(&fixture, TOOLKIT, 1, "icon_theme").as_deref(),
        Some("\"Pop\"")
    );
}

#[test]
fn background_rebase_preserves_concurrent_custom_accent_text() {
    let fixture = Fixture::new("appearance-custom-accent-rebase");
    let store = fixture.store();
    let root = fixture.0.join("config");
    let builder = ThemeBuilder::dark().accent(Srgb::new(51. / 255., 102. / 255., 204. / 255.));
    builder
        .write_entry(&Config::with_custom_path(DARK_BUILDER, 2, root.clone()).unwrap())
        .unwrap();
    builder
        .build()
        .write_entry(&Config::with_custom_path(DARK, 2, root).unwrap())
        .unwrap();
    let background = Control::Color(ThemeVariant::Dark, ColorRole::Background);
    let mut pending = draft(&fixture);
    pending.set(background, Value::Color(parse_hex("#EEEEEE", true)));
    pending.set(Control::IconTheme, Value::Name("Pop".into()));
    let saved = ron::to_string(&pending).unwrap();

    let mut external = draft(&fixture);
    external.set(
        Control::Color(ThemeVariant::Dark, ColorRole::Accent),
        Value::Color(parse_hex("#CC3366", false)),
    );
    store.apply(&external.patch()).unwrap();
    let custom = "Some(\"#00FF00FF\")";
    assert_ne!(
        read(&fixture, DARK, 2, "accent_text").as_deref(),
        Some(custom)
    );
    write(&fixture, DARK, 2, "accent_text", custom);
    let before = config(&fixture);

    let mut recovered: AppearanceDraft = ron::from_str(&saved).unwrap();
    // Reopening and repeated refresh must not adopt a lossy reverse derivation.
    for _ in 0..2 {
        recovered.refresh(store.load(&AppearanceDraft::keys()).unwrap());
        assert!(
            recovered
                .conflicts()
                .contains(&theme_key(ThemeVariant::Dark, "accent_text")),
            "the concurrent custom accent text requires explicit resolution"
        );
        assert!(recovered.edited(background));
        assert!(matches!(
            store.apply(&recovered.patch()),
            Err(DesktopError::Conflict(_))
        ));
        assert_eq!(
            config(&fixture),
            before,
            "failed Apply preserves all values"
        );
    }
    // The existing Use current value action resolves the coupled theme edit
    // without losing either the external customization or unrelated draft work.
    recovered.resolve_conflicts(store.load(&AppearanceDraft::keys()).unwrap(), &[]);
    assert!(recovered.conflicts().is_empty());
    assert!(!recovered.edited(background));
    assert!(recovered.edited(Control::IconTheme));
    store.apply(&recovered.patch()).unwrap();
    assert_eq!(
        read(&fixture, DARK, 2, "accent_text").as_deref(),
        Some(custom)
    );
    assert_eq!(
        read(&fixture, TOOLKIT, 1, "icon_theme").as_deref(),
        Some("\"Pop\"")
    );
}

#[test]
fn independent_background_rebase_and_undo_accept_generated_and_absent_defaults() {
    for variant in ThemeVariant::ALL {
        for explicit in [false, true] {
            let fixture = Fixture::new("appearance-independent-rebase-undo");
            let store = fixture.store();
            if explicit {
                let root = fixture.0.join("config");
                let builder = variant.default_builder().accent(Srgb::new(
                    51. / 255.,
                    102. / 255.,
                    204. / 255.,
                ));
                builder
                    .write_entry(
                        &Config::with_custom_path(variant.builder_namespace(), 2, root.clone())
                            .unwrap(),
                    )
                    .unwrap();
                builder
                    .build()
                    .write_entry(
                        &Config::with_custom_path(variant.theme_namespace(), 2, root).unwrap(),
                    )
                    .unwrap();
            }
            let config_files = || {
                config(&fixture)
                    .into_iter()
                    .filter(|(_, value)| value.is_some())
                    .collect::<BTreeMap<_, _>>()
            };
            let original = config_files();
            let mut pending = draft(&fixture);
            let original_accent = pending.color(variant, ColorRole::Accent);
            let color = parse_hex("#EEEEEE", true);
            pending.set(
                Control::Color(variant, ColorRole::Background),
                Value::Color(color),
            );
            let mut external = draft(&fixture);
            external.set(
                Control::Color(variant, ColorRole::Accent),
                Value::Color(parse_hex("#CC3366", false)),
            );
            let change = store.apply(&external.patch()).unwrap();
            for undo in [false, true] {
                if undo {
                    store.undo(&change).unwrap();
                    assert_eq!(
                        config_files(),
                        original,
                        "Undo restores exact bytes/absence"
                    );
                }
                pending.refresh(store.load(&AppearanceDraft::keys()).unwrap());
                assert!(
                    pending.conflicts().is_empty(),
                    "{variant:?}, explicit={explicit}, undo={undo}: {:?}",
                    pending.conflicts()
                );
                assert_eq!(
                    pending.color(variant, ColorRole::Background),
                    Setting::Known(color)
                );
            }
            store.apply(&pending.patch()).unwrap();
            let applied = draft(&fixture);
            assert_eq!(applied.color(variant, ColorRole::Accent), original_accent);
            assert_eq!(
                applied.color(variant, ColorRole::Background),
                Setting::Known(color)
            );
        }
    }
}

/// The derivation palette matching used before the shared writer, kept
/// here as the reference the shared derivation must reproduce.
fn legacy_recolor(current: &Theme, builder: &ThemeBuilder, accent: Srgb) -> Theme {
    let old = builder.clone().accent(current.accent_color().color).build();
    let new = builder.clone().accent(accent).build();
    let mut changed = current.clone();
    macro_rules! components {
        ($($field:ident),+) => { $(
            theme::merge_component(&mut changed.$field, &old.$field, &new.$field);
        )+ };
    }
    components!(
        button,
        icon_button,
        link_button,
        list_button,
        text_button,
        success,
        success_button,
        warning,
        warning_button,
        destructive,
        destructive_button
    );
    changed.accent = new.accent;
    changed.accent_button = new.accent_button;
    changed.accent_text = new.accent_text;
    changed
}

#[test]
fn palette_matching_reproduces_the_legacy_derivation_through_one_patch() {
    let fixture = seeded("appearance-palette");
    let store = fixture.store();
    let accent = Srgb::new(0.53, 0.74, 1.);
    let snapshot = store.load(&AppearanceDraft::keys()).unwrap();
    let root = fixture.0.join("config");
    let originals: Vec<(ThemeBuilder, Theme)> = ThemeVariant::ALL
        .into_iter()
        .map(|variant| {
            let builder =
                Config::with_custom_path(variant.builder_namespace(), 2, root.clone()).unwrap();
            let theme =
                Config::with_custom_path(variant.theme_namespace(), 2, root.clone()).unwrap();
            (
                ThemeBuilder::get_entry(&builder).unwrap(),
                Theme::get_entry(&theme).unwrap(),
            )
        })
        .collect();
    let patch = palette_patch(&snapshot, accent).unwrap();
    assert!(patch.keys().all(|key| ThemeVariant::of(key).is_some()));
    store.apply(&patch).unwrap();
    for (variant, (original_builder, original)) in ThemeVariant::ALL.into_iter().zip(originals) {
        let builder_config =
            Config::with_custom_path(variant.builder_namespace(), 2, root.clone()).unwrap();
        let theme_config =
            Config::with_custom_path(variant.theme_namespace(), 2, root.clone()).unwrap();
        let builder = ThemeBuilder::get_entry(&builder_config).unwrap();
        assert_eq!(
            builder.accent,
            Some(Srgb::<u8>::new(135, 189, 255).into_format())
        );
        let updated = Theme::get_entry(&theme_config).unwrap();
        let expected = legacy_recolor(&original, &original_builder, accent);
        // Colors are stored as hex, so compare what COSMIC stores.
        let stored = |value: &dyn erased::Stored| value.stored();
        assert_eq!(stored(&updated.accent), stored(&expected.accent));
        assert_eq!(
            stored(&updated.accent_button),
            stored(&expected.accent_button)
        );
        assert_eq!(stored(&updated.accent_text), stored(&expected.accent_text));
        assert_eq!(stored(&updated.button), stored(&expected.button));
        assert_eq!(stored(&updated.text_button), stored(&expected.text_button));
        // Legacy matching also moved containers' accent-derived component
        // fields (selection, focus) while keeping their base and text.
        let old = original_builder
            .clone()
            .accent(original.accent_color().color)
            .build();
        let new = original_builder.clone().accent(accent).build();
        for transparent in [false, true] {
            let mut container = original.background(transparent).clone();
            theme::merge_component(
                &mut container.component,
                &old.background(transparent).component,
                &new.background(transparent).component,
            );
            assert_eq!(stored(updated.background(transparent)), stored(&container));
        }
        assert_eq!(hex(updated.button.base, true), CUSTOM_BUTTON);
        assert_eq!(updated.spacing, original.spacing);
        assert_eq!(updated.gaps, original.gaps);
        assert_eq!(updated.window_hint, original.window_hint);
    }
    assert_eq!(read(&fixture, MODE, 1, "is_dark").as_deref(), Some("true"));
    assert!(read(&fixture, DARK, 2, "future_control").is_some());
}

#[test]
fn every_planned_key_is_whitelisted_and_hex_input_round_trips() {
    let fixture = seeded("appearance-rules");
    let mut draft = draft(&fixture);
    for role in ColorRole::ALL {
        for variant in ThemeVariant::ALL {
            draft.set(
                Control::Color(variant, role),
                Value::Color(parse_hex("#3584E4", true)),
            );
        }
    }
    draft.set(Control::Glass, Value::Number(80));
    draft.set(Control::Density, Value::Density(Density::Standard));
    for surface in Surface::ALL {
        draft.set(Control::Frosted(surface), Value::Flag(true));
    }
    let patch = draft.patch();
    for (key, entry) in patch.iter() {
        let rule = crate::desktop::store::rule_for(key).unwrap_or_else(|| panic!("{key}"));
        (rule.validate)(entry.desired.as_ref().unwrap()).unwrap();
    }
    assert_eq!(rules().count(), 15 * 2 + 35 * 2 + 8);
    assert_eq!(parse_hex("#3584e4", false), parse_hex("3584E4", false));
    assert_eq!(parse_hex("#3584E480", false), None, "no alpha for accents");
    assert_eq!(
        hex(parse_hex("#3584E480", true).unwrap(), true),
        "#3584E480"
    );
    assert_eq!(hex(parse_hex("#3584E4", true).unwrap(), true), "#3584E4");
    assert_eq!(parse_hex("#35zz", false), None);
    assert_eq!(glass_percent(&glass_alpha_map(80)), 80);
}

mod erased {
    pub trait Stored {
        fn stored(&self) -> String;
    }
    impl<T: serde::Serialize> Stored for T {
        fn stored(&self) -> String {
            super::RawValue::encode(self).unwrap().as_str().to_string()
        }
    }
}

#[test]
fn the_installed_untagged_dark_builder_palette_is_read_not_replaced() {
    // cosmic-settings 1.10 installs the Dark builder palette without its
    // `Dark(...)` tag. With no user-layer builder that default is the
    // effective palette, so Appearance and palette matching must read it.
    let installed = include_str!("testdata/installed-dark-builder-palette.ron");
    assert!(
        installed.trim_start().starts_with('('),
        "untagged as installed"
    );
    let fixture = seeded("appearance-installed-palette");
    std::fs::remove_dir_all(fixture.0.join("config/cosmic").join(DARK_BUILDER)).unwrap();
    let system = fixture
        .0
        .join("system/cosmic")
        .join(DARK_BUILDER)
        .join("v2");
    std::fs::create_dir_all(&system).unwrap();
    // A recognizable gray proves the installed values are used, not the
    // built-in palette.
    std::fs::write(
        system.join("palette"),
        installed.replacen("gray_1: \"#1B1B1BFF\"", "gray_1: \"#202122FF\"", 1),
    )
    .unwrap();
    let store = fixture.store();
    let before = config(&fixture);
    let mut draft = AppearanceDraft::load(&store).unwrap();
    let builder = draft.base_builder(ThemeVariant::Dark).unwrap();
    let CosmicPalette::Dark(palette) = &builder.palette else {
        panic!("tagged by its namespace: {:?}", builder.palette)
    };
    assert_eq!(hex(palette.gray_1, true), "#202122");
    assert_eq!(
        draft.color(ThemeVariant::Dark, ColorRole::Accent),
        Setting::Known(None)
    );
    draft.set(
        Control::Color(ThemeVariant::Dark, ColorRole::Accent),
        Value::Color(parse_hex("#33D17A", false)),
    );
    assert!(draft.blocked().is_empty());
    let patch = draft.patch();
    assert!(
        patch
            .keys()
            .all(|key| key.key() != "palette" || key.namespace() != DARK_BUILDER),
        "an input palette is never written"
    );
    assert!(
        palette_patch(
            &store.load(&AppearanceDraft::keys()).unwrap(),
            Srgb::new(0.2, 0.8, 0.5)
        )
        .is_ok()
    );
    assert_eq!(
        config(&fixture),
        before,
        "reading and planning write nothing"
    );
    // A palette with a member this version does not know still blocks.
    std::fs::write(
        system.join("palette"),
        installed.replacen("name:", "future_shade: 0.5,\n    name:", 1),
    )
    .unwrap();
    let draft = AppearanceDraft::load(&store).unwrap();
    assert!(matches!(
        draft.color(ThemeVariant::Dark, ColorRole::Accent),
        Setting::Unsupported(_)
    ));
}

/// COSMIC draws an outline color only the derived theme holds (another tool
/// wrote it without the builder). Edits of other roles and looks whose
/// builder outline already follows keep it; an explicit "follow the accent"
/// clears that color alone and is no edit where nothing needs relinking.
#[test]
fn follow_accent_relinks_an_outline_only_the_derived_theme_colors() {
    let fixture = Fixture::new("appearance-derived-outline");
    let cyan = "Some((red:0.3882353,green:0.8156863,blue:0.8745098))";
    write(&fixture, DARK, 2, "window_hint", cyan);
    let dark = ThemeVariant::Dark;
    let mut look = draft(&fixture);
    assert!(look.derived_outline(dark).is_some());
    assert!(!look.outline_follows(dark));
    assert!(look.outline_follows(ThemeVariant::Light));
    let preview = look.preview(dark);
    assert_ne!(preview.outline, preview.accent, "COSMIC draws it");
    let mut values = VariantValues::default();
    values.colors.insert(ColorRole::Outline, None);
    values
        .colors
        .insert(ColorRole::Accent, parse_hex("#E66100", false));
    look.set(Control::Variant(dark), Value::Variant(Box::new(values)));
    let plan = look.plan();
    assert!(
        !plan.patch.keys().any(|key| key.key() == "window_hint"),
        "a look whose outline already follows leaves it"
    );
    look.set(Control::Color(dark, ColorRole::Outline), Value::Color(None));
    assert!(look.outline_follows(dark));
    let plan = look.plan();
    assert!(plan.blocked.is_empty(), "{:?}", plan.blocked);
    let written: Vec<_> = plan
        .patch
        .keys()
        .filter(|key| key.key() == "window_hint")
        .cloned()
        .collect();
    assert_eq!(
        written,
        vec![theme_key(dark, "window_hint")],
        "the derived color alone"
    );
    let preview = look.preview(dark);
    assert_eq!(preview.outline, preview.accent);
    fixture.store().apply(&plan.patch).unwrap();
    assert_eq!(read(&fixture, DARK_BUILDER, 2, "window_hint"), None);
    let mut after = draft(&fixture);
    assert!(after.derived_outline(dark).is_none() && after.outline_follows(dark));
    after.set(Control::Color(dark, ColorRole::Outline), Value::Color(None));
    assert!(!after.dirty(), "nothing left to relink");
}
