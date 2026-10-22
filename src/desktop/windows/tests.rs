use super::*;
use crate::desktop::store::tests::Fixture;
use crate::desktop::store::{Access, Owner};
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::cosmic_theme::ThemeBuilder;
use cosmic::cosmic_theme::palette::Srgba;
use std::collections::BTreeMap;
use std::path::PathBuf;

const COMP: &str = "com.system76.CosmicComp";
const TK: &str = "com.system76.CosmicTk";
const XKB: &str = "(rules:\"\",model:\"pc105\",layout:\"us,de\",variant:\"\",options:Some(\"grp:sentinel\"),repeat_delay:600,repeat_rate:25)";
const INPUT: &str = "(state:Enabled,acceleration:Some((profile:Some(Flat),speed:0.25)))";

fn config_files(fixture: &Fixture) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut tree = fixture.tree();
    tree.retain(|path, _| path.starts_with(fixture.0.join("config")));
    tree
}

fn theme_file(fixture: &Fixture, namespace: &str, key: &str) -> Option<String> {
    std::fs::read_to_string(
        fixture
            .0
            .join("config/cosmic")
            .join(namespace)
            .join("v2")
            .join(key),
    )
    .ok()
}

/// Exact pinned defaults when no layer defines a key; scalar edits write
/// their own key only; unrelated compositor members (input, XKB) and other
/// toolkit keys keep their bytes; loading and planning write nothing; a
/// read-only store refuses without writing. A second root with its own
/// system defaults shows no value comes from anywhere else.
#[test]
fn windows_change_only_touched_keys() {
    let empty = Fixture::new("windows-defaults");
    let draft = WindowsDraft::load(&empty.store()).unwrap();
    assert_eq!(empty.tree().len(), 0, "loading creates nothing");
    let expect = [
        (Behavior::Autotile, Choice::Flag(false)),
        (Behavior::Scope, Choice::Scope(TileBehavior::Global)),
        (Behavior::ActiveHint, Choice::Flag(true)),
        (Behavior::FocusFollowsCursor, Choice::Flag(false)),
        (Behavior::CursorFollowsFocus, Choice::Flag(false)),
        (Behavior::FocusDelay, Choice::Millis(250)),
        (Behavior::EdgeSnap, Choice::Pixels(0)),
        (Behavior::ShowMinimize, Choice::Flag(true)),
        (Behavior::ShowMaximize, Choice::Flag(true)),
    ];
    for (behavior, choice) in expect {
        assert_eq!(
            draft.value(behavior),
            Setting::Known(choice),
            "{behavior:?}"
        );
    }

    // The second root: its own system layer, nothing from the live desktop.
    let other = Fixture::new("windows-system");
    other.system(COMP, "autotile", "true");
    other.system(COMP, "autotile_behavior", "PerWorkspace");
    other.system(TK, "show_minimize", "false");
    let draft = WindowsDraft::load(&other.store()).unwrap();
    assert_eq!(draft.flag(Behavior::Autotile), Setting::Known(true));
    assert_eq!(draft.scope(), Setting::Known(TileBehavior::PerWorkspace));
    assert_eq!(draft.flag(Behavior::ShowMinimize), Setting::Known(false));
    assert_eq!(
        WindowsDraft::load(&empty.store()).unwrap().scope(),
        Setting::Known(TileBehavior::Global),
        "one root never sees the other's values"
    );

    let fixture = Fixture::new("windows-touched");
    fixture.user(COMP, "xkb_config", XKB);
    fixture.user(COMP, "input_default", INPUT);
    fixture.user(
        COMP,
        "appearance_settings",
        "(clip_floating_windows:false,clip_tiled_windows:true,shadow_tiled_windows:false)",
    );
    fixture.user(COMP, "focus_follows_cursor_delay", "100000");
    fixture.user(TK, "icon_theme", "\"Sentinel\"");
    fixture.user(TK, "show_maximize", "false");
    let store = fixture.store();
    let before = config_files(&fixture);
    let mut draft = WindowsDraft::load(&store).unwrap();
    // An existing out-of-range value is shown as it is, not clamped.
    assert_eq!(draft.focus_delay(), Setting::Known(100_000));
    draft.set(Behavior::Autotile, Choice::Flag(true));
    draft.set(Behavior::ActiveHint, Choice::Flag(false));
    draft.set(Behavior::EdgeSnap, Choice::Pixels(EDGE_SNAP_ON));
    draft.set(Behavior::ShowMinimize, Choice::Flag(false));
    // Setting the current value again is no edit; a wrong type is ignored.
    draft.set(Behavior::ShowMaximize, Choice::Flag(false));
    draft.set(Behavior::Scope, Choice::Flag(true));
    let plan = draft.plan();
    assert!(plan.blocked.is_empty(), "{:?}", plan.blocked);
    let mut written: Vec<String> = plan.patch.keys().map(ToString::to_string).collect();
    written.sort();
    assert_eq!(
        written,
        [
            "com.system76.CosmicComp/v1/active_hint",
            "com.system76.CosmicComp/v1/autotile",
            "com.system76.CosmicComp/v1/edge_snap_threshold",
            "com.system76.CosmicTk/v1/show_minimize",
        ]
    );
    assert_eq!(config_files(&fixture), before, "planning writes nothing");

    // Read-only: the same plan is refused before anything is written.
    let read_only = DesktopStore::with_roots(
        fixture.0.join("config/cosmic"),
        vec![fixture.0.join("system/cosmic")],
        fixture.0.join("state"),
        Access::ReadOnly("preview".into()),
    );
    assert!(matches!(
        read_only.apply_owned(&plan.patch, None, Owner::Windows, "Windows"),
        Err(DesktopError::ReadOnly(_))
    ));
    assert_eq!(config_files(&fixture), before);
    assert!(!fixture.0.join("state").exists(), "nothing created");

    store
        .apply_owned(&plan.patch, None, Owner::Windows, "Windows")
        .unwrap();
    let after = config_files(&fixture);
    for (path, contents) in &before {
        assert_eq!(after.get(path), Some(contents), "{} kept", path.display());
    }
    assert_eq!(fixture.read(COMP, "autotile").as_deref(), Some("true"));
    assert_eq!(fixture.read(COMP, "active_hint").as_deref(), Some("false"));
    assert_eq!(
        fixture.read(COMP, "edge_snap_threshold").as_deref(),
        Some("10")
    );
    assert_eq!(fixture.read(TK, "show_minimize").as_deref(), Some("false"));
    assert_eq!(fixture.read(COMP, "xkb_config").as_deref(), Some(XKB));
    assert_eq!(fixture.read(COMP, "input_default").as_deref(), Some(INPUT));
    assert_eq!(
        fixture.read(COMP, "focus_follows_cursor_delay").as_deref(),
        Some("100000")
    );
    assert_eq!(
        store.latest_change().unwrap().map(|latest| latest.owner),
        Some(Owner::Windows)
    );
    // Every other compositor key the adapter never writes stays refused.
    let mut foreign = DesktopPatch::new();
    foreign.set(
        ConfigKey::new(COMP, 1, "xkb_config").unwrap(),
        RawValue::parse(XKB).ok(),
        RawValue::parse("(rules:\"\",model:\"\",layout:\"fr\",variant:\"\",options:None)").ok(),
    );
    assert!(matches!(
        store.apply(&foreign),
        Err(DesktopError::NotAllowed(_))
    ));

    // A value this version can't decode blocks only its own control.
    let odd = Fixture::new("windows-odd");
    odd.user(COMP, "autotile", "Maybe");
    let mut draft = WindowsDraft::load(&odd.store()).unwrap();
    assert!(matches!(draft.flag(Behavior::Autotile), Setting::Unsupported(raw) if raw == "Maybe"));
    draft.set(Behavior::Autotile, Choice::Flag(true));
    draft.set(Behavior::CursorFollowsFocus, Choice::Flag(true));
    let plan = draft.plan();
    assert_eq!(plan.blocked.len(), 1);
    assert_eq!(plan.blocked[0].key, Behavior::Autotile.key());
    assert!(
        plan.patch.get(&Behavior::Autotile.key()).is_none(),
        "never replaced"
    );
    assert!(
        plan.patch
            .get(&Behavior::CursorFollowsFocus.key())
            .is_some()
    );
}

/// cosmic-comp reads `autotile` and `autotile_behavior` separately and
/// retiles existing windows under whatever scope it holds, so a change of
/// both in one step is refused in the core plan. The first step keeps the
/// scope in between PerWorkspace in every direction: it writes only its own
/// key, guards the other at its original bytes and leaves the rest of the
/// draft (desired values and baselines) for the next Apply.
#[test]
fn tiling_scope_transition_never_uses_stale_global_scope() {
    use TileBehavior::{Global, PerWorkspace};
    for (tiled, scope, target, first) in [
        (false, Global, PerWorkspace, Behavior::Scope),
        (true, Global, PerWorkspace, Behavior::Scope),
        (true, PerWorkspace, Global, Behavior::Autotile),
        (false, PerWorkspace, Global, Behavior::Autotile),
    ] {
        let case = format!("{tiled} {scope:?} → {} {target:?}", !tiled);
        let fixture = Fixture::new("windows-transition");
        fixture.user(COMP, "autotile", &tiled.to_string());
        fixture.user(COMP, "autotile_behavior", &format!("{scope:?}"));
        let store = fixture.store();
        let mut draft = WindowsDraft::load(&store).unwrap();
        draft.set(Behavior::Scope, Choice::Scope(target));
        draft.set(Behavior::Autotile, Choice::Flag(!tiled));
        draft.set(Behavior::ActiveHint, Choice::Flag(false));
        draft.set_gap(ThemeVariant::Dark, Gap::Outer, 12);
        let plan = draft.plan();
        assert!(plan.blocked.iter().any(is_transition), "{case}: {plan:?}");
        assert!(tiling_transition(&plan.patch).is_some());
        assert_eq!(first_tiling_step(target), first, "{case}");
        let (step, part) = draft.first_step().unwrap();
        assert_eq!(step, first, "{case}");
        assert!(part.blocked.is_empty(), "{case}: {:?}", part.blocked);
        assert_eq!(
            part.patch.keys().cloned().collect::<Vec<_>>(),
            [first.key()]
        );
        let other = match first {
            Behavior::Scope => Behavior::Autotile,
            _ => Behavior::Scope,
        };
        let original = fixture
            .read(COMP, other.name())
            .map(|raw| RawValue::parse(raw).unwrap());
        assert_eq!(
            part.patch.guards().collect::<Vec<_>>(),
            [(&other.key(), &original)],
            "{case}: the other key is guarded at its original bytes"
        );
        // The rest keeps its desired values and original baselines.
        let rest = draft.without(first);
        assert!(!rest.edited(first) && rest.edited(other) && rest.edited(Behavior::ActiveHint));
        assert_eq!(rest.gap_edits(), [(ThemeVariant::Dark, Gap::Outer, 12)]);
        assert_eq!(rest.snapshot(), draft.snapshot());
        assert!(!rest.plan().blocked.iter().any(is_transition));
    }

    // The step never runs under a coupled value changed meanwhile.
    let fixture = Fixture::new("windows-transition-guard");
    fixture.user(COMP, "autotile", "true");
    fixture.user(COMP, "autotile_behavior", "PerWorkspace");
    let store = fixture.store();
    let mut draft = WindowsDraft::load(&store).unwrap();
    draft.set(Behavior::Scope, Choice::Scope(TileBehavior::Global));
    draft.set(Behavior::Autotile, Choice::Flag(false));
    fixture.user(COMP, "autotile_behavior", "Global");
    let files = config_files(&fixture);
    let (_, part) = draft.first_step().unwrap();
    assert!(matches!(
        store.apply_owned(&part.patch, None, Owner::Windows, "Windows"),
        Err(DesktopError::Conflict(keys)) if keys == [Behavior::Scope.key()]
    ));
    assert_eq!(config_files(&fixture), files, "nothing written");
    assert_eq!(store.latest_change().unwrap(), None);

    // Changing only the scope, or only tiling, is never refused, and an
    // edit back to the current value is no edit.
    let mut alone = WindowsDraft::load(&store).unwrap();
    alone.set(Behavior::Autotile, Choice::Flag(false));
    assert!(alone.plan().blocked.is_empty() && alone.first_step().is_none());
    alone.set(Behavior::Scope, Choice::Scope(TileBehavior::Global));
    assert!(!alone.edited(Behavior::Scope));
    assert!(alone.plan().blocked.is_empty());
}

/// Both variants written by the official types with distinct gaps and
/// outlines, a hand-customized button and an unknown theme key.
fn themed(name: &str) -> Fixture {
    let fixture = Fixture::new(name);
    let root = fixture.0.join("config");
    for (variant, mut builder, gaps, outline) in [
        (ThemeVariant::Dark, ThemeBuilder::dark(), (2, 8), 3),
        (ThemeVariant::Light, ThemeBuilder::light(), (4, 1), 4),
    ] {
        builder.gaps = gaps;
        builder.active_hint = outline;
        let b = Config::with_custom_path(variant.builder_namespace(), 2, root.clone()).unwrap();
        builder.write_entry(&b).unwrap();
        let mut theme = builder.build();
        theme.button.base = Srgba::new(0.2, 0.25, 0.3, 0.8);
        let t = Config::with_custom_path(variant.theme_namespace(), 2, root.clone()).unwrap();
        theme.write_entry(&t).unwrap();
    }
    fixture.user("com.system76.CosmicTheme.Mode", "is_dark", "true");
    fixture
}

/// Gap edits go through the Appearance adapter: one variant at a time,
/// inner never below that variant's outline, outer alone keeps the inner
/// value exactly (even one below the outline), and unrelated roles and the
/// mode stay. Profiles carry gaps under Appearance and compose them with
/// the outline width; a Windows-only selection never writes them.
#[test]
fn gap_profile_composes_with_outline_and_preserves_variants() {
    use crate::desktop::profiles::{self, Domain, Installed};
    let fixture = themed("windows-gaps");
    let store = fixture.store();
    let dark_button = theme_file(&fixture, "com.system76.CosmicTheme.Dark", "button");
    let light_builder = theme_file(&fixture, "com.system76.CosmicTheme.Light.Builder", "gaps");
    let mut draft = WindowsDraft::load(&store).unwrap();
    assert_eq!(draft.gaps(ThemeVariant::Dark), Setting::Known((2, 8)));
    assert_eq!(draft.gaps(ThemeVariant::Light), Setting::Known((4, 1)));
    assert_eq!(draft.active_variant(), ThemeVariant::Dark);
    // Inner below the outline widens to it, and says so through the value.
    draft.set_gap(ThemeVariant::Dark, Gap::Inner, 0);
    assert_eq!(draft.gaps(ThemeVariant::Dark), Setting::Known((2, 3)));
    assert!(draft.gaps_dirty() && !draft.behavior_dirty());
    assert_eq!(draft.gaps(ThemeVariant::Light), Setting::Known((4, 1)));
    let plan = draft.plan();
    assert!(plan.blocked.is_empty(), "{:?}", plan.blocked);
    assert!(
        plan.patch
            .keys()
            .all(|key| key.namespace().starts_with("com.system76.CosmicTheme.Dark")),
        "only the Dark theme: {:?}",
        plan.patch.keys().collect::<Vec<_>>()
    );
    store
        .apply_owned(&plan.patch, None, Owner::Windows, "Windows")
        .unwrap();
    let gaps = |variant: &str, builder: bool| -> (u32, u32) {
        let namespace = if builder {
            format!("com.system76.CosmicTheme.{variant}.Builder")
        } else {
            format!("com.system76.CosmicTheme.{variant}")
        };
        RawValue::parse(theme_file(&fixture, &namespace, "gaps").unwrap())
            .unwrap()
            .decode()
            .unwrap()
    };
    assert_eq!(gaps("Dark", true), (2, 3));
    assert_eq!(gaps("Dark", false), (2, 3), "the derived theme follows");
    assert_eq!(
        theme_file(&fixture, "com.system76.CosmicTheme.Dark", "button"),
        dark_button,
        "a customized role stays"
    );
    assert_eq!(
        theme_file(&fixture, "com.system76.CosmicTheme.Light.Builder", "gaps"),
        light_builder
    );
    assert_eq!(
        fixture
            .read("com.system76.CosmicTheme.Mode", "is_dark")
            .as_deref(),
        Some("true")
    );

    // Dark's inner gap now equals its outline: dragging it lower widens
    // back to the same value, so it is no edit (nothing to apply, nothing
    // for other pages to wait for). A real inner edit stays one.
    let mut draft = WindowsDraft::load(&store).unwrap();
    draft.set_gap(ThemeVariant::Dark, Gap::Inner, 1);
    assert_eq!(draft.gaps(ThemeVariant::Dark), Setting::Known((2, 3)));
    assert!(!draft.dirty(), "{:?}", draft.gap_edits());
    assert!(draft.plan().patch.is_empty() && draft.gap_plan().patch.guards().count() == 0);
    draft.set_gap(ThemeVariant::Dark, Gap::Inner, 5);
    assert_eq!(draft.gaps(ThemeVariant::Dark), Setting::Known((2, 5)));
    assert_eq!(draft.gap_edits(), [(ThemeVariant::Dark, Gap::Inner, 5)]);
    // Light's inner 1 is below its 4 px outline: an inner edit, even to 1,
    // is a real change to the outline width.
    draft.set_gap(ThemeVariant::Light, Gap::Inner, 1);
    assert_eq!(draft.gaps(ThemeVariant::Light), Setting::Known((4, 4)));
    assert!(draft.gap_edited(ThemeVariant::Light));

    // Outer alone keeps an inner gap below the outline exactly as it is.
    let mut draft = WindowsDraft::load(&store).unwrap();
    draft.set_gap(ThemeVariant::Light, Gap::Outer, 20);
    assert_eq!(draft.gaps(ThemeVariant::Light), Setting::Known((20, 1)));
    store
        .apply_owned(&draft.plan().patch, None, Owner::Windows, "Windows")
        .unwrap();
    assert_eq!(gaps("Light", true), (20, 1));
    assert_eq!(gaps("Dark", true), (2, 3), "the other variant stays");

    // Profiles: gaps per variant under Appearance, composed with the outline.
    let mut profile = profiles::Profile::new("Gaps").unwrap();
    profile.appearance = Some(profiles::AppearanceLook {
        light: Some(profiles::VariantLook {
            outline_width: Some(6),
            gaps: Some((10, 2)),
            ..Default::default()
        }),
        ..Default::default()
    });
    profile.windows = Some(profiles::WindowsBehaviorLook {
        active_hint: Some(false),
        ..Default::default()
    });
    let profile = profiles::import(&profiles::export(&profile).unwrap().bytes).unwrap();
    let installed = Installed::default();
    let windows_only = profiles::plan(
        &store,
        &installed,
        &profile,
        &[Domain::Windows].into_iter().collect(),
    );
    assert!(
        windows_only.can_apply(),
        "{:?}",
        windows_only.blocking_reasons()
    );
    assert!(
        windows_only
            .patch()
            .keys()
            .all(|key| key.namespace() == COMP),
        "a Windows selection never writes gaps"
    );
    let appearance = profiles::plan(
        &store,
        &installed,
        &profile,
        &[Domain::Appearance].into_iter().collect(),
    );
    assert!(
        appearance.can_apply(),
        "{:?}",
        appearance.blocking_reasons()
    );
    profiles::apply(&store, &appearance).unwrap();
    assert_eq!(
        gaps("Light", true),
        (10, 6),
        "inner widened to the new outline"
    );
    assert_eq!(gaps("Light", false), (10, 6));
    assert_eq!(gaps("Dark", true), (2, 3));
    assert_eq!(
        theme_file(&fixture, "com.system76.CosmicTheme.Dark", "button"),
        dark_button
    );
    assert_eq!(fixture.read(COMP, "active_hint"), None);
}
