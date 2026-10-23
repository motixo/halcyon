//! Small runtime geometry snapshot shared with the Settings placement preview.
use crate::{
    config::Config,
    drawer::{self, Widget},
    layout,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

static GENERATION: AtomicU64 = AtomicU64::new(0);
static PUBLISHED: Mutex<u64> = Mutex::new(0);

pub fn next_generation() -> u64 {
    GENERATION.fetch_add(1, Ordering::Relaxed) + 1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetGeometry {
    pub key: String,
    pub size: [f32; 2],
    #[serde(default)]
    pub natural_height: Option<f32>,
    #[serde(default)]
    pub natural_size: Option<[f32; 2]>,
    pub position: [f32; 2],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayState {
    pub output: Option<String>,
    pub ready: bool,
    pub epoch: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub display: Option<DisplayState>,
    pub screen: [f32; 2],
    pub widgets: Vec<WidgetGeometry>,
}

pub fn path() -> Option<PathBuf> {
    if std::env::var_os("DESKTOP_CARDS_CONFIG").is_some() {
        Config::path().map(|path| path.with_file_name("layout-state.json"))
    } else {
        crate::paths::runtime_dir().map(|dir| dir.join("layout-state.json"))
    }
}

fn cached() -> Option<Snapshot> {
    let bytes = std::fs::read(path()?).ok()?;
    let state: Snapshot = serde_json::from_slice(&bytes).ok()?;
    state.valid().then_some(state)
}

impl Snapshot {
    fn valid(&self) -> bool {
        self.screen
            .iter()
            .all(|v| v.is_finite() && *v >= 1. && *v <= 32768.)
            && self.widgets.len() == Widget::ALL.len()
            && Widget::ALL
                .iter()
                .all(|w| self.widgets.iter().filter(|g| g.key == w.key()).count() == 1)
            && self.widgets.iter().all(|g| {
                g.size
                    .iter()
                    .all(|v| v.is_finite() && *v > 0. && *v <= 32768.)
                    && g.natural_size.is_none_or(crate::layout::valid_size)
                    && g.natural_height
                        .is_none_or(|height| height.is_finite() && height > 0.)
                    && g.position.iter().all(|v| v.is_finite())
            })
    }
}

fn fallback(config: &Config) -> Snapshot {
    Snapshot {
        display: None,
        screen: [1920., 1080.],
        widgets: Widget::ALL
            .into_iter()
            .map(|widget| WidgetGeometry {
                natural_size: None,
                key: widget.key().into(),
                size: match widget {
                    Widget::Clock => [490., 200.],
                    Widget::Calendar => [360., 320.],
                    Widget::Weather => [360., 180.],
                    Widget::Devbox => [360., 280.],
                    Widget::Projects => [360., 200.],
                    Widget::Glance => [config.glance.auto_width(config.glance_panel()), 220.],
                    tool => crate::tools::cards::size(tool, config),
                },
                natural_height: (widget == Widget::Glance).then_some(220.),
                position: [0., 0.],
            })
            .collect(),
    }
}

fn size_for_tool(state: &Snapshot, widget: Widget) -> [f32; 2] {
    state
        .widgets
        .iter()
        .find(|g| g.key == widget.key())
        .map(|g| g.size)
        .unwrap_or([280., 160.])
}

fn default_positions(mut state: Snapshot, config: &Config) -> Snapshot {
    let ordered: Vec<_> = config
        .ordered_right_cards()
        .into_iter()
        .filter(|w| config.widget_visible(*w))
        .collect();
    let size_for = |widget: Widget| {
        state
            .widgets
            .iter()
            .find(|g| g.key == widget.key())
            .unwrap()
            .size
    };
    let total = ordered.iter().map(|w| size_for(*w)[1]).sum::<f32>()
        + ordered.len().saturating_sub(1) as f32 * drawer::CARD_GAP;
    let mut right_y = (state.screen[1] - total) / 2.;
    for widget in ordered {
        let g = state
            .widgets
            .iter_mut()
            .find(|g| g.key == widget.key())
            .unwrap();
        g.position = layout::clamp(
            [state.screen[0] - g.size[0] - 42., right_y],
            g.size,
            state.screen,
        );
        right_y += g.size[1] + drawer::CARD_GAP;
    }
    let focus_height = size_for_tool(&state, Widget::Focus)[1];
    for g in &mut state.widgets {
        if let Some(tool) = Widget::parse(&g.key).filter(|widget| widget.is_tool()) {
            g.position = layout::tool_position(tool, g.size, focus_height, state.screen);
            continue;
        }
        let point = match g.key.as_str() {
            "clock" | "glance" => [82., 88.],
            "calendar" => [42., (state.screen[1] - g.size[1]) / 2.],
            _ => g.position,
        };
        if g.key == "glance" && !config.widget_sizes.contains_key("glance") {
            g.size[1] = g
                .natural_height
                .unwrap_or(g.size[1])
                .min((state.screen[1] - point[1] - 48.).max(1.));
            g.position = layout::clamp(point, [g.size[0], 1.], state.screen);
        } else {
            g.position = layout::clamp(point, g.size, state.screen);
        }
    }
    state
}

/// Glance grows below its top edge. Persist that edge against a stable vertical
/// reference instead of normalizing it against the changing agenda height.
pub fn encode_position(
    widget: Widget,
    point: [f32; 2],
    size: [f32; 2],
    screen: [f32; 2],
) -> [f32; 2] {
    layout::encode(
        point,
        if widget == Widget::Glance {
            [size[0], 1.]
        } else {
            size
        },
        screen,
    )
}
pub fn decode_position(
    widget: Widget,
    point: [f32; 2],
    size: [f32; 2],
    screen: [f32; 2],
) -> [f32; 2] {
    layout::decode(
        point,
        if widget == Widget::Glance {
            [size[0], 1.]
        } else {
            size
        },
        screen,
    )
}
/// Moving a busy agenda must remain possible. Its viewport shrinks below the
/// new anchor rather than forcing a full-height widget back to the screen top.
pub fn placement_size(widget: Widget, size: [f32; 2]) -> [f32; 2] {
    if widget == Widget::Glance {
        [size[0], size[1].min(180.)]
    } else {
        size
    }
}

pub fn defaults(config: &Config) -> Snapshot {
    let mut state = cached().unwrap_or_else(|| fallback(config));
    for g in &mut state.widgets {
        if let Some(size) = g.natural_size {
            g.size = size;
        }
    }
    let mut automatic = config.clone();
    automatic.widget_sizes.clear();
    default_positions(state, &automatic)
}
pub fn read(config: &Config) -> Snapshot {
    project(defaults(config), config)
}

/// Project a preview from cached geometry without reading the desktop again.
pub fn project(mut state: Snapshot, config: &Config) -> Snapshot {
    for g in &mut state.widgets {
        if let Some(size) = config.widget_sizes.get(&g.key) {
            g.size = layout::sized(Widget::parse(&g.key).unwrap(), *size, state.screen);
        }
    }
    state = default_positions(state, config);
    for g in &mut state.widgets {
        if let Some(position) = config.widget_positions.get(&g.key) {
            g.position = decode_position(
                Widget::parse(&g.key).unwrap(),
                *position,
                g.size,
                state.screen,
            );
            if config.widget_sizes.contains_key(&g.key) {
                g.position = layout::clamp(g.position, g.size, state.screen);
            }
            if g.key == "glance" && !config.widget_sizes.contains_key("glance") {
                g.size[1] = g
                    .natural_height
                    .unwrap_or(g.size[1])
                    .min((state.screen[1] - g.position[1] - 48.).max(1.));
            }
        }
    }
    state
}

pub fn publish(state: &Snapshot, generation: u64) {
    let Some(path) = path() else {
        return;
    };
    let Ok(mut published) = PUBLISHED.lock() else {
        return;
    };
    publish_ordered(&path, state, generation, &mut published);
}
fn publish_ordered(path: &std::path::Path, state: &Snapshot, generation: u64, published: &mut u64) {
    if generation < *published {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(state) else {
        return;
    };
    if !state.valid() {
        return;
    }
    *published = generation;
    if std::fs::read(path).ok().as_deref() == Some(bytes.as_slice()) {
        return;
    }
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let temporary = path.with_extension(format!("{}-{generation}.tmp", std::process::id()));
    if std::fs::write(&temporary, bytes).is_ok() {
        let _ = std::fs::rename(&temporary, path);
    }
    let _ = std::fs::remove_file(temporary);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_glance_inherits_a_safe_viewport_from_an_automatic_bottom_anchor() {
        let mut config = Config::default();
        config.glance.enabled = true;
        config.widget_positions.insert("glance".into(), [0.5, 0.99]);
        config.widget_sizes.insert("glance".into(), [480., 320.]);
        let state = read(&config);
        let g = state.widgets.iter().find(|g| g.key == "glance").unwrap();
        assert!(g.position[1] + g.size[1] <= state.screen[1] - 48.);
    }

    #[test]
    fn snapshot_rejects_missing_duplicate_and_nonfinite_geometry() {
        let mut snapshot = fallback(&Config::default());
        assert!(snapshot.valid());
        snapshot.screen[0] = f32::NAN;
        assert!(!snapshot.valid());
        snapshot.screen[0] = 1920.;
        snapshot.widgets[0].key = snapshot.widgets[1].key.clone();
        assert!(!snapshot.valid());
    }
    #[test]
    fn default_preview_uses_the_actual_card_sizes_and_visible_order() {
        let mut config = Config::default();
        config.enabled_widgets = vec!["weather".into(), "projects".into()];
        let state = default_positions(fallback(&Config::default()), &config);
        let weather = state.widgets.iter().find(|g| g.key == "weather").unwrap();
        let projects = state.widgets.iter().find(|g| g.key == "projects").unwrap();
        assert_eq!(
            projects.position[1] - weather.position[1],
            weather.size[1] + drawer::CARD_GAP
        );
        assert_eq!(weather.position[0], projects.position[0]);
    }
    #[test]
    fn glance_anchor_is_independent_of_agenda_height_and_preview_restores_growth() {
        let screen = [1920., 1080.];
        let anchor = [430., 400.];
        let encoded = encode_position(Widget::Glance, anchor, [312., 220.], screen);
        assert_eq!(
            decode_position(Widget::Glance, encoded, [312., 632.], screen),
            anchor
        );
        let mut state = fallback(&Config::default());
        let glance = state
            .widgets
            .iter_mut()
            .find(|g| g.key == "glance")
            .unwrap();
        glance.natural_height = Some(1400.);
        glance.size[1] = 632.;
        let state = default_positions(state, &Config::default());
        let glance = state.widgets.iter().find(|g| g.key == "glance").unwrap();
        assert_eq!(glance.position, [82., 88.]);
        assert_eq!(glance.size[1], 1080. - 88. - 48.);
        assert_eq!(placement_size(Widget::Glance, glance.size), [280., 180.]);
    }
    #[test]
    fn older_snapshot_cannot_overwrite_newer_geometry() {
        let path = std::env::temp_dir().join(format!(
            "desktop-cards-geometry-{}.json",
            std::process::id()
        ));
        let older = fallback(&Config::default());
        let mut newer = older.clone();
        newer.screen = [2560., 1440.];
        let mut generation = 0;
        publish_ordered(&path, &newer, 2, &mut generation);
        publish_ordered(&path, &older, 1, &mut generation);
        let stored: Snapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(stored.screen, newer.screen);
    }
}
