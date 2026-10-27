//! Two bounded presentation layers. Persistence and timed activation remain
//! owned by Config, the desktop store and the existing automation engine.
use crate::config::{Appearance, ClockPreferences, Config};
use crate::desktop_theme::Family;
use crate::widget_style::resolver::{ClockFace, ClockSurface, ClockWeight};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Day,
    Night,
}
impl Phase {
    pub const ALL: [Self; 2] = [Self::Day, Self::Night];
    pub fn key(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Night => "night",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Day => "Day",
            Self::Night => "Night",
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clock {
    pub face: Option<ClockFace>,
    pub weight: Option<ClockWeight>,
    pub italic: Option<bool>,
    pub surface: Option<ClockSurface>,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Look {
    pub family: Option<Family>,
    pub clock: Clock,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Moments {
    pub active: Option<Phase>,
    pub day: Look,
    pub night: Look,
}
impl Default for Moments {
    fn default() -> Self {
        Self {
            active: None,
            day: Look {
                family: Some(Family::Paper),
                ..Default::default()
            },
            night: Look {
                family: Some(Family::Midnight),
                ..Default::default()
            },
        }
    }
}
// Missing family retains that phase's default; an explicit null means inherit.
// This distinction matters when the first saved edit is only a Clock property.
impl<'de> Deserialize<'de> for Moments {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Day {
            family: Option<Family>,
            clock: Clock,
        }
        impl Default for Day {
            fn default() -> Self {
                Self {
                    family: Some(Family::Paper),
                    clock: Clock::default(),
                }
            }
        }
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Night {
            family: Option<Family>,
            clock: Clock,
        }
        impl Default for Night {
            fn default() -> Self {
                Self {
                    family: Some(Family::Midnight),
                    clock: Clock::default(),
                }
            }
        }
        #[derive(Default, Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Record {
            active: Option<Phase>,
            day: Day,
            night: Night,
        }
        let raw = Record::deserialize(d)?;
        Ok(Self {
            active: raw.active,
            day: Look {
                family: raw.day.family,
                clock: raw.day.clock,
            },
            night: Look {
                family: raw.night.family,
                clock: raw.night.clock,
            },
        })
    }
}
impl Moments {
    pub fn get(&self, phase: Phase) -> &Look {
        match phase {
            Phase::Day => &self.day,
            Phase::Night => &self.night,
        }
    }
    pub fn selected(&self) -> Option<&Look> {
        self.active.map(|phase| self.get(phase))
    }
    pub fn pointer(phase: Option<Phase>, key: &str) -> String {
        phase.map_or_else(
            || format!("/clock/{key}"),
            |p| format!("/moments/{}/clock/{key}", p.key()),
        )
    }
    pub fn clock_member(key: &str) -> bool {
        matches!(key, "face" | "weight" | "italic" | "surface")
    }
}
impl Config {
    pub fn moment_appearance(&self) -> Appearance {
        let mut appearance = self.appearance;
        if let Some(family) = self.moments.selected().and_then(|look| look.family) {
            appearance.family = Some(family);
        }
        appearance
    }
    pub fn moment_clock(&self) -> ClockPreferences {
        let mut clock = self.clock;
        if let Some(look) = self.moments.selected() {
            clock.face = look.clock.face.or(clock.face);
            clock.weight = look.clock.weight.or(clock.weight);
            clock.italic = look.clock.italic.or(clock.italic);
            clock.surface = look.clock.surface.or(clock.surface);
        }
        clock
    }
    /// Inspector choices show the edited layer, not a higher inherited value.
    pub fn clock_layer(&self, phase: Option<Phase>) -> ClockPreferences {
        let mut clock = self.clock;
        if let Some(phase) = phase {
            let look = self.moments.get(phase);
            clock.face = look.clock.face;
            clock.weight = look.clock.weight;
            clock.italic = look.clock.italic;
            clock.surface = look.clock.surface;
        }
        clock
    }
    pub fn save_moment(pointer: &str, value: serde_json::Value) -> Result<Self, String> {
        if !pointer.starts_with("/moments/") {
            return Err("Unsupported moment choice".into());
        }
        let path = Self::path().ok_or("Configuration directory unavailable")?;
        let saved = crate::config::patch_config(&path, |raw| {
            crate::desktop::store::widget::put(raw, pointer, Some(value))
        })?;
        Self::from_saved_value(saved)
    }
    pub fn save_moment_clock(
        phase: Option<Phase>,
        key: &str,
        value: serde_json::Value,
    ) -> Result<Self, String> {
        if !Moments::clock_member(key) {
            return Err("Unsupported moment clock choice".into());
        }
        let path = Self::path().ok_or("Configuration directory unavailable")?;
        let change = |raw: &mut serde_json::Value| {
            let latest: Config = serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
            if phase.is_some() && latest.moments.active != phase {
                return Err("The active moment changed. Review the clock and try again.".into());
            }
            crate::desktop::store::widget::put(raw, &Moments::pointer(phase, key), Some(value))
        };
        let saved = if let Some(label) = crate::desktop::actions::current_label() {
            crate::desktop::actions::preferences_guarded(
                &path,
                &label,
                phase.map(|p| ("/moments/active", Some(serde_json::json!(p)))),
                change,
            )?
        } else {
            crate::config::patch_config(&path, change)?
        };
        Self::from_saved_value(saved)
    }
}

#[cfg(test)]
mod tests;
