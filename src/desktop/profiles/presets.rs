//! The built-in setups. Six starter looks each set a whole desktop: a
//! bundled Halcyon wallpaper whose colors the widgets, windows, dock and
//! panel follow, and a widget style. They share one layout: a pill dock,
//! a pill Islands panel on top, a see-through At a glance and the system
//! monitor. "Clean desktop" is a quick switch
//! that only hides things. Every record goes through the same `import` as
//! a shared file, so a preset can express nothing an imported profile could
//! not. Fonts, icon themes, monitor bindings, applet arrangements, locations
//! and projects stay the user's. Loading presets reads nothing and writes
//! nothing; `fitted` adapts one to the user's configuration.
use super::format::{Profile, import};
use super::themes;
use crate::desktop_theme::Family;
use dock_islands_core::Placement;
use serde_json::{Value, json};

/// A built-in profile: stable ID, short description and the record.
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub id: &'static str,
    pub description: &'static str,
    pub profile: Profile,
}

/// The quick switch, listed apart from the starter looks.
pub const CLEAN: &str = "presentation";

/// Earlier releases' presets, kept for schedules that still name them.
const RENAMED: [(&str, &str); 3] = [
    ("minimal", "focus"),
    ("glass", "evening"),
    ("workbench", "halcyon"),
];

/// A starter look differs only in how it looks: wallpaper, style and the
/// colors taken from the wallpaper. What is on the desktop is the same for
/// all of them (see `record`).
struct Look {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    family: Family,
    /// The bundled wallpaper (`wallpaper::BUNDLED`).
    wallpaper: &'static str,
    /// The widget palette nearest the wallpaper.
    palette: &'static str,
    /// The wallpaper's own accent, as the automatic colors pick it. It
    /// seeds the COSMIC, widget, dock and panel colors until the running
    /// cards keep them following the wallpaper.
    seed: [u8; 3],
    light: bool,
}

/// Corner radius COSMIC rounds into a pill at any bar size.
const PILL: u32 = crate::desktop::panels::MAX_RADIUS;

const LOOKS: [Look; 6] = [
    Look {
        id: "halcyon",
        name: "Halcyon",
        description: "Frosted glass over the night sea.",
        family: Family::Frosted,
        wallpaper: "halcyon-night",
        palette: "ocean",
        seed: [0x9b, 0xae, 0xee],
        light: false,
    },
    Look {
        id: "calm",
        name: "Calm",
        description: "Soft cards over still teal water.",
        family: Family::Soft,
        wallpaper: "halcyon-calm",
        palette: "teal",
        seed: [0x6a, 0xc2, 0xc6],
        light: false,
    },
    Look {
        id: "evening",
        name: "Evening",
        description: "Soft lavender cards at dusk.",
        family: Family::Soft,
        wallpaper: "halcyon-evening",
        palette: "lavender",
        seed: [0xb2, 0xa4, 0xf3],
        light: false,
    },
    Look {
        id: "focus",
        name: "Focus",
        description: "Bare text over quiet mountain mist.",
        family: Family::Bare,
        wallpaper: "halcyon-focus",
        palette: "graphite",
        seed: [0x8d, 0xb4, 0xe7],
        light: false,
    },
    Look {
        id: "paper",
        name: "Paper",
        description: "Light mode, warm paper cards and a serif clock.",
        family: Family::Paper,
        wallpaper: "halcyon-paper",
        palette: "amber",
        seed: [0xd5, 0xa7, 0x76],
        light: true,
    },
    Look {
        id: "rose",
        name: "Rosé",
        description: "Rose glass over a warm sunset.",
        family: Family::Glass,
        wallpaper: "halcyon-rose",
        palette: "rose",
        seed: [0xf1, 0x92, 0x95],
        light: false,
    },
];

/// Deep-merges `patch` into `base`; objects merge, everything else replaces.
fn merge(base: &mut Value, patch: Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (key, value) in patch {
                merge(base.entry(key).or_insert(Value::Null), value);
            }
        }
        (base, patch) => *base = patch,
    }
}

fn record(look: &Look) -> Value {
    // The family's own record is what Colors & theme applies: widget style,
    // dock and panel bars, and COSMIC colors from the seed.
    let mut base = themes::record(look.family);
    base.appearance_only = false;
    base.appearance = Some(themes::appearance(look.family, Some(look.seed)));
    let mut value = serde_json::to_value(base).expect("profiles serialize");
    // Dock and panel float as pills in a dark tint of the wallpaper's
    // color, like the windows; the panel shows Islands kept together (a
    // plain bar where the Islands component can't run). Where they sit,
    // their applets and the islands' arrangement stay the person's.
    let tint = look.family.surface(look.seed).map(|v| f32::from(v) / 255.);
    let bar = json!({"background": crate::desktop::panels::Background::Color(tint),
        "floating": true, "margin": 6, "border_radius": PILL});
    let islands = dock_islands_core::Settings {
        enabled: true,
        placement: Placement::Together,
        gap: look.family.spec().bar.island_gap,
        ..Default::default()
    };
    merge(
        &mut value,
        json!({
            "name": look.name,
            "appearance": {"mode": if look.light { "light" } else { "dark" }},
            "widget_style": {"theme": look.palette,
                "appearance": {"accent": look.seed, "wallpaper_colors": true},
                // A see-through At a glance (on a panel over a light
                // wallpaper, where light text wouldn't read), and a system
                // monitor that shows only what every computer has.
                "glance": {"style": if look.light { "panel" } else { "transparent" },
                    "own_background": true, "density": "comfortable"},
                "devbox": {"cpu": true, "gpu": true, "memory": true, "disk": true,
                    "network": true, "uptime": true, "swap": false, "services": false}},
            "wallpaper": {"background": {"source": {"bundled": look.wallpaper}, "fit": "fill"}},
            // Calendar stays off until someone connects one; the clock
            // still shows the date.
            "widget_layout": {"enabled_widgets": ["devbox"],
                "glance": {"enabled": true, "initialized": true, "show_clock": true,
                    "show_weather": true, "show_calendar": false}},
            "dock": bar.clone(),
            "panel": bar,
        }),
    );
    merge(
        &mut value["panel"],
        json!({"spacing": 6, "padding": 1,
            "islands": serde_json::to_value(islands).expect("islands serialize")}),
    );
    value
}

const CLEAN_RECORD: &str = r##"{
  "format": "desktop-cards-profile", "version": 4, "name": "Clean desktop",
  "dock": { "autohide": "Always" },
  "widget_layout": { "enabled_widgets": [],
                     "glance": { "enabled": false, "show_weather": false, "show_calendar": false },
                     "music": { "enabled": false }, "notes": { "enabled": false },
                     "focus": { "enabled": false } }
}"##;

/// The built-in presets in display order: the starter looks, then the
/// quick switch. Pure data: no files are read.
pub fn presets() -> Vec<Preset> {
    LOOKS
        .iter()
        .map(|look| Preset {
            id: look.id,
            description: look.description,
            profile: import(record(look).to_string().as_bytes())
                .expect("built-in presets are valid"),
        })
        .chain(std::iter::once(Preset {
            id: CLEAN,
            description: "Hide all widgets and let the dock hide. Handy for screen sharing.",
            profile: import(CLEAN_RECORD.as_bytes()).expect("built-in presets are valid"),
        }))
        .collect()
}

/// The built-in preset with `id`, including the earlier releases' names.
pub fn preset(id: &str) -> Option<Preset> {
    let id = RENAMED
        .iter()
        .find(|(old, _)| *old == id)
        .map_or(id, |(_, new)| new);
    presets().into_iter().find(|preset| preset.id == id)
}

/// The bundled wallpaper a look shows, by palette name.
fn bundled(preset: &Preset) -> Option<&str> {
    match preset.profile.wallpaper.as_ref()?.background.as_ref()?.source.as_ref()? {
        super::desktop_look::SourceLook::Bundled(name) => Some(name),
        _ => None,
    }
}

fn installed(preset: &Preset, bundled_dir: &std::path::Path) -> bool {
    bundled(preset).is_none_or(|name| {
        crate::desktop::wallpaper::bundled_path(bundled_dir, name).is_file()
    })
}

/// Whether this desktop runs a compatible Islands panel component.
pub fn islands_available(store: &crate::desktop::DesktopStore) -> bool {
    crate::desktop::islands::Client::connect(store)
        .and_then(|mut client| client.capabilities())
        .is_ok()
}

fn wants_islands(preset: &Preset) -> bool {
    [&preset.profile.dock, &preset.profile.panel]
        .into_iter()
        .flatten()
        .any(|look| look.islands.as_ref().is_some_and(|islands| islands.enabled))
}

/// A starter look adds the widgets it places and never takes away what
/// someone set up themselves. What their saved configuration holds is
/// theirs, even when it equals the defaults: a widget list of their own gains
/// the system monitor, an At a glance they set up (on or off) stays as it
/// is, and their system monitor keeps its rows. A new person gets the look's.
fn keep_own_widgets(preset: &mut Preset, config: &crate::config::Config) {
    let saved = config.saved;
    if let Some(Value::Object(layout)) = preset.profile.widget_layout.as_mut() {
        if saved.widgets
            && let Some(Value::Array(widgets)) = layout.get_mut("enabled_widgets")
        {
            for widget in &config.enabled_widgets {
                let widget = Value::String(widget.clone());
                if !widgets.contains(&widget) {
                    widgets.push(widget);
                }
            }
        }
        if saved.glance || config.glance.initialized {
            layout.remove("glance");
        }
    }
    if saved.devbox
        && let Some(Value::Object(style)) = preset.profile.widget_style.as_mut()
    {
        style.remove("devbox");
    }
}

/// `preset` adapted to this person: Projects joins only when there are
/// project folders to show, instead of an empty card; a look whose
/// bundled wallpaper isn't installed (in `bundled_dir`) keeps the current
/// wallpaper; and without the Islands component (`islands` false) the
/// panel stays one bar in the look's shape. None of these refuse the look.
pub fn fitted(
    mut preset: Preset,
    config: &crate::config::Config,
    bundled_dir: &std::path::Path,
    islands: bool,
) -> Preset {
    if config.project_roots.is_empty()
        && let Some(Value::Array(widgets)) = preset
            .profile
            .widget_layout
            .as_mut()
            .and_then(|layout| layout.get_mut("enabled_widgets"))
    {
        widgets.retain(|widget| widget != "projects");
    }
    if preset.id != CLEAN {
        keep_own_widgets(&mut preset, config);
    }
    if !installed(&preset, bundled_dir) {
        preset.profile.wallpaper = None;
    }
    if !islands {
        for look in [&mut preset.profile.dock, &mut preset.profile.panel]
            .into_iter()
            .flatten()
        {
            look.islands = None;
        }
    }
    preset
}

/// What a starter look still needs from the user to look complete.
pub fn missing(
    preset: &Preset,
    bundled_dir: &std::path::Path,
    islands: bool,
) -> Option<&'static str> {
    if !installed(preset, bundled_dir) {
        Some("The built-in wallpapers aren’t installed, so your wallpaper stays. Reinstall them on the Wallpaper page.")
    } else if !islands && wants_islands(preset) {
        Some("The Islands panel isn’t running, so the panel stays one bar.")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::profiles::{Domain, Installed, ProfileLibrary, apply, plan};
    use crate::desktop::store::tests::Fixture;
    use std::collections::BTreeSet;

    const DOCK: &str = "com.system76.CosmicPanel.Dock";
    const PANEL: &str = "com.system76.CosmicPanel.Panel";

    fn desk(name: &str) -> (Fixture, crate::desktop::DesktopStore) {
        let fixture = Fixture::new(name);
        fixture.system(
            "com.system76.CosmicPanel",
            "entries",
            "[\"Panel\",\"Dock\"]",
        );
        fixture.system(DOCK, "anchor", "Bottom");
        fixture.system(DOCK, "size", "L");
        fixture.system(DOCK, "size_wings", "Some((Some(XL), None))");
        fixture.system(DOCK, "output", "Name(\"DP-1\")");
        fixture.system(
            DOCK,
            "plugins_center",
            "Some([\"com.system76.CosmicAppList\"])",
        );
        fixture.system(PANEL, "anchor", "Top");
        fixture.user("com.system76.CosmicTheme.Mode", "is_dark", "true");
        // The installer puts the bundled wallpapers in Pictures/Desktop Cards.
        let bundled = fixture.0.join("pictures/Desktop Cards");
        std::fs::create_dir_all(&bundled).unwrap();
        for (name, _) in crate::desktop::wallpaper::BUNDLED {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("assets/wallpapers/{name}-1080p.png")),
                bundled.join(format!("{name}-4k.png")),
            )
            .unwrap();
        }
        let config = fixture.0.join("app/config.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            r#"{"theme":"teal","location":{"name":"SENTINEL","latitude":1.0,"longitude":2.0},
                "project_roots":["/home/someone/work"],"enabled_widgets":["calendar","projects","devbox","weather","clock"],
                "glance":{"enabled":true,"show_calendar":true,"show_weather":true},
                "appearance":{"match_wallpaper":true,"match_cosmic_colors":true},"future_field":7}"#,
        )
        .unwrap();
        let store = fixture.store().with_app_config(config);
        (fixture, store)
    }

    #[test]
    fn presets_are_distinct_valid_complete_looks_and_loading_writes_nothing() {
        let (fixture, store) = desk("presets-valid");
        let before = fixture.tree();
        let all = presets();
        let ids: Vec<_> = all.iter().map(|preset| preset.id).collect();
        assert_eq!(
            ids,
            ["halcyon", "calm", "evening", "focus", "paper", "rose", CLEAN]
        );
        let installed = Installed::discover(&store);
        let dir = crate::desktop::wallpaper::bundled_dir(&store);
        let config = crate::config::Config::default();
        let mut seen = Vec::new();
        for preset in &all {
            let profile = &preset.profile;
            profile.validate().unwrap();
            // Fonts, icons, monitors and applets stay the user's.
            if let Some(look) = &profile.appearance {
                assert!(look.interface_font.is_none() && look.monospace_font.is_none());
                assert!(look.icon_theme.is_none());
            }
            for look in [&profile.dock, &profile.panel].into_iter().flatten() {
                assert!(look.applets.is_none(), "{}", preset.id);
            }
            if preset.id != CLEAN {
                // A starter look is a whole desktop: wallpaper, style, colors, widgets.
                let domains = profile.domains();
                for domain in [
                    Domain::Wallpaper,
                    Domain::WidgetStyle,
                    Domain::Appearance,
                    Domain::WidgetLayout,
                    Domain::Dock,
                ] {
                    assert!(domains.contains(&domain), "{} lacks {domain:?}", preset.id);
                }
                assert!(super::themes::family(profile).is_some(), "{}", preset.id);
                let wallpaper = serde_json::to_value(&profile.wallpaper).unwrap();
                let name = wallpaper["background"]["source"]["bundled"].as_str().unwrap();
                assert!(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join(format!("assets/wallpapers/{name}-1080p.png"))
                        .exists(),
                    "{name}"
                );
                assert!(crate::desktop::wallpaper::bundled_name(name).is_some(), "{name}");
                let style = profile.widget_style.as_ref().unwrap();
                assert!(
                    crate::config::Palette::CHOICES.iter().any(|(key, _)| style["theme"] == *key),
                    "{}",
                    preset.id
                );
                // Colors keep following the wallpaper after the look is applied.
                assert_eq!(style["appearance"]["wallpaper_colors"], true, "{}", preset.id);
                // One layout for every look: pill dock, pill Islands panel on
                // top kept together, see-through At a glance without the
                // calendar, and only the system monitor.
                let panel = profile.panel.as_ref().unwrap();
                let islands = panel.islands.as_ref().unwrap();
                assert!(islands.enabled, "{}", preset.id);
                assert_eq!(islands.placement, Placement::Together, "{}", preset.id);
                assert!(panel.anchor.is_none(), "where the panel sits stays the person's");
                assert!(profile.dock.as_ref().unwrap().islands.is_none());
                for bar in [panel, profile.dock.as_ref().unwrap()] {
                    assert_eq!(bar.floating, Some(true), "{}", preset.id);
                    assert_eq!(bar.border_radius, Some(PILL), "{}", preset.id);
                    assert!(bar.autohide.is_none(), "{}", preset.id);
                    assert!(matches!(
                        bar.background,
                        Some(crate::desktop::panels::Background::Color(_))
                    ));
                }
                // See-through, except on a panel over the light wallpaper.
                let glance = if preset.id == "paper" { "panel" } else { "transparent" };
                assert_eq!(style["glance"]["style"], glance, "{}", preset.id);
                assert_eq!(style["glance"]["own_background"], true);
                assert_eq!(style["devbox"]["services"], false);
                for metric in ["cpu", "gpu", "memory", "disk", "network", "uptime"] {
                    assert_eq!(style["devbox"][metric], true, "{metric}");
                }
                let layout = profile.widget_layout.as_ref().unwrap();
                assert_eq!(layout["enabled_widgets"], json!(["devbox"]));
                assert_eq!(layout["glance"]["enabled"], true);
                assert_eq!(layout["glance"]["show_calendar"], false);
                for widget in ["music", "notes", "focus"] {
                    assert!(layout.get(widget).is_none(), "{widget} is left alone");
                }
                // The installed COSMIC panel crashes with a frosted background.
                let raw = serde_json::to_string(profile).unwrap();
                assert!(!raw.contains("\"panel\":true"), "{}", preset.id);
            }
            // This fixture has no Islands component, so the looks keep a bar.
            let profile = &fitted(preset.clone(), &config, &dir, false).profile;
            if preset.id != CLEAN {
                assert!(profile.panel.as_ref().unwrap().islands.is_none());
                assert!(missing(preset, &dir, false).is_some());
            }
            let selected: BTreeSet<Domain> = profile.domains().into_iter().collect();
            let plan = plan(&store, &installed, profile, &selected);
            assert!(
                plan.can_apply(),
                "{}: {:?}",
                preset.id,
                plan.blocking_reasons()
            );
            let changes: Vec<_> = plan
                .domains
                .iter()
                .flat_map(|domain| domain.changes.clone())
                .collect();
            assert!(!seen.contains(&changes), "{} repeats a preset", preset.id);
            seen.push(changes);
        }
        assert!(
            ProfileLibrary::new(&store).list().unwrap().is_empty(),
            "presets are not library entries"
        );
        assert_eq!(
            fixture.tree(),
            before,
            "loading and planning presets writes nothing"
        );
    }

    #[test]
    fn earlier_names_resolve_and_looks_fit_what_this_desktop_can_show() {
        assert_eq!(preset("minimal").unwrap().id, "focus");
        assert_eq!(preset("glass").unwrap().id, "evening");
        assert_eq!(preset("workbench").unwrap().id, "halcyon");
        assert!(preset("nonexistent").is_none());
        let (fixture, store) = desk("presets-fitted");
        let dir = crate::desktop::wallpaper::bundled_dir(&store);
        let config = crate::config::Config::default();
        let workbench = preset("calm").unwrap();
        assert!(missing(&workbench, &dir, true).is_none());
        assert!(missing(&workbench, &dir, false).unwrap().contains("Islands"));
        let with = fitted(workbench.clone(), &config, &dir, true);
        assert!(with.profile.wallpaper.is_some());
        assert!(with.profile.panel.as_ref().unwrap().islands.is_some());
        // Without Islands the panel keeps its pill shape as one bar.
        let bar = fitted(workbench.clone(), &config, &dir, false);
        let panel = bar.profile.panel.as_ref().unwrap();
        assert!(panel.islands.is_none());
        assert_eq!(panel.border_radius, Some(PILL));
        // Without the bundled wallpapers the rest of the look still applies.
        std::fs::remove_dir_all(fixture.0.join("pictures")).unwrap();
        assert!(missing(&workbench, &dir, true).unwrap().contains("Wallpaper page"));
        let without = fitted(workbench, &config, &dir, true);
        assert!(without.profile.wallpaper.is_none());
        assert!(without.profile.widget_style.is_some());
    }

    #[test]
    fn presentation_hides_private_cards_and_a_look_sets_the_whole_desktop() {
        let (fixture, store) = desk("presets-apply");
        let installed = Installed::discover(&store);
        let presentation = preset(CLEAN).unwrap();
        let selected: BTreeSet<Domain> = presentation.profile.domains().into_iter().collect();
        apply(
            &store,
            &plan(&store, &installed, &presentation.profile, &selected),
        )
        .unwrap();
        let config: crate::config::Config =
            serde_json::from_slice(&std::fs::read(store.app_config().unwrap()).unwrap()).unwrap();
        use crate::drawer::Widget;
        for widget in [
            Widget::Projects,
            Widget::Calendar,
            Widget::Weather,
            Widget::Devbox,
            Widget::Glance,
        ] {
            assert!(!config.widget_visible(widget), "{widget:?} still shows");
        }
        assert!(!config.glance.show_calendar && !config.glance.show_weather);
        // Private and unrelated members and palette links are untouched.
        let raw = std::fs::read_to_string(store.app_config().unwrap()).unwrap();
        for kept in ["SENTINEL", "/home/someone/work", "future_field", "teal"] {
            assert!(raw.contains(kept), "{kept} lost");
        }
        assert!(config.appearance.match_wallpaper && config.appearance.match_cosmic_colors);

        let dir = crate::desktop::wallpaper::bundled_dir(&store);
        let calm = fitted(preset("calm").unwrap(), &Default::default(), &dir, false);
        let selected: BTreeSet<Domain> = calm.profile.domains().into_iter().collect();
        apply(&store, &plan(&store, &installed, &calm.profile, &selected)).unwrap();
        let config: crate::config::Config =
            serde_json::from_slice(&std::fs::read(store.app_config().unwrap()).unwrap()).unwrap();
        assert_eq!(config.appearance.family, Some(Family::Soft));
        assert_eq!(config.appearance.accent, Some([0x6a, 0xc2, 0xc6]));
        assert!(config.appearance.wallpaper_colors);
        assert!(config.glance.enabled && config.glance.own_background);
        assert!(config.widget_visible(Widget::Devbox));
        assert!(!config.glance.show_calendar);
        assert!(config.devbox.gpu && config.devbox.uptime && !config.devbox.services);
        // After Clean desktop nothing of theirs shows, so only the look's.
        assert!(!config.widget_visible(Widget::Projects));
        // The monitor binding and the applets stay the target's.
        assert_eq!(fixture.read(DOCK, "output"), None);
        assert_eq!(fixture.read(DOCK, "plugins_center"), None);
        let background = std::fs::read_to_string(
            fixture.0.join("config/cosmic/com.system76.CosmicBackground/v1/all"),
        )
        .unwrap_or_default();
        assert!(background.contains("halcyon-calm"), "{background}");
    }

    #[test]
    fn a_look_adds_its_widgets_and_keeps_what_someone_set_up() {
        let dir = std::path::Path::new("/nonexistent");
        let calm = || preset("calm").unwrap();
        let layout = |p: &Preset| p.profile.widget_layout.clone().unwrap();
        // A new person gets exactly the look's widgets.
        let fresh = fitted(calm(), &crate::config::Config::default(), dir, false);
        assert_eq!(layout(&fresh)["enabled_widgets"], json!(["devbox"]));
        assert_eq!(layout(&fresh)["glance"]["show_calendar"], false);
        assert!(fresh.profile.widget_style.as_ref().unwrap().get("devbox").is_some());
        // Someone with their own setup keeps it.
        let own = crate::config::Config::from_bytes(
            br#"{"enabled_widgets":["calendar","weather"],"glance":{"enabled":false},"devbox":{"swap":false}}"#,
        )
        .unwrap();
        let kept = fitted(calm(), &own, dir, false);
        assert_eq!(
            layout(&kept)["enabled_widgets"],
            json!(["devbox", "calendar", "weather"])
        );
        // At a glance they turned off stays off.
        assert!(layout(&kept).get("glance").is_none());
        assert!(kept.profile.widget_style.as_ref().unwrap().get("devbox").is_none());
        // The look's style still applies.
        assert_eq!(kept.profile.widget_style.as_ref().unwrap()["glance"]["style"], "transparent");
        // Choices equal to the defaults are still theirs once saved.
        let defaults_saved = crate::config::Config::from_bytes(
            serde_json::to_vec(&json!({"enabled_widgets": crate::config::Config::default().enabled_widgets}))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let kept = fitted(calm(), &defaults_saved, dir, false);
        for widget in &crate::config::Config::default().enabled_widgets {
            assert!(
                layout(&kept)["enabled_widgets"].as_array().unwrap().contains(&json!(widget)),
                "{widget} kept"
            );
        }
        // Clean desktop still hides everything.
        let clean = fitted(preset(CLEAN).unwrap(), &own, dir, false);
        assert_eq!(layout(&clean)["enabled_widgets"], json!([]));
    }
}
