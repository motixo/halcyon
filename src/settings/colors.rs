//! One app-owned color choice on Wallpaper & colors. Only the
//! color preferences participate; unrelated and future JSON is never
//! copied from this draft into the live configuration.
use crate::config::{self, Config, Palette};
use crate::desktop::store::{DesktopStore, WidgetPatch, widget};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KEYS: [&str; 10] = [
    "/theme",
    "/appearance/accent",
    "/appearance/color_source",
    "/appearance/sync_colors",
    "/appearance/wallpaper_colors",
    "/appearance/wallpaper_color_stamp",
    "/appearance/family",
    "/appearance/color_policy",
    "/appearance/overrides",
    "/appearance/native_mode_palettes",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Colors {
    #[serde(default)]
    pub family: Option<crate::desktop_theme::Family>,
    #[serde(default)]
    pub policy: Option<crate::config::ColorPolicy>,
    #[serde(default)]
    pub overrides: u64,
    pub theme: String,
    pub accent: Option<[u8; 3]>,
    pub sync: bool,
    #[serde(default)]
    pub automatic: bool,
    #[serde(default)]
    pub stamp: u64,
    /// A pending human choice survives an outside wallpaper refresh.
    #[serde(default)]
    pub adjusted: bool,
    /// The active window outline will follow the accent again.
    #[serde(default)]
    pub outline: bool,
    // JSON scalars encoded as strings keep the session draft RON-compatible.
    original: BTreeMap<String, Option<String>>,
}

fn read(store: &DesktopStore) -> Result<(serde_json::Value, Config), String> {
    let path = store
        .app_config()
        .ok_or("Widget settings aren’t available here")?;
    let text = config::read_config_text(path)?;
    let raw = config::parse_config_text(text.as_deref())?;
    let typed = config::check_config_value(&raw)?;
    Ok((raw, typed))
}

impl Colors {
    /// The family and accent that owned native surfaces when editing began.
    pub fn baseline_appearance(&self) -> crate::config::Appearance {
        crate::config::Appearance {
            family: self.family,
            color_policy: self.policy,
            overrides: self.overrides,
            native_mode_palettes: self
                .original
                .get("/appearance/native_mode_palettes")
                .and_then(Option::as_ref)
                .and_then(|value| serde_json::from_str(value).ok())
                .unwrap_or(0),
            accent: self
                .original
                .get("/appearance/accent")
                .and_then(Option::as_ref)
                .and_then(|v| serde_json::from_str(v).ok())
                .flatten(),
            ..Default::default()
        }
    }
    pub fn accept_baseline(&mut self, fresh: Self) {
        self.family = fresh.family;
        self.original = fresh.original;
    }
    pub fn load(store: &DesktopStore) -> Result<Self, String> {
        let (raw, config) = read(store)?;
        Ok(Self {
            family: config.appearance.family,
            policy: config.appearance.color_policy,
            overrides: config.appearance.overrides,
            theme: config.theme,
            accent: config.appearance.accent,
            sync: config.appearance.sync_colors,
            automatic: config.appearance.wallpaper_colors,
            stamp: config.appearance.wallpaper_color_stamp,
            adjusted: false,
            outline: false,
            original: KEYS
                .into_iter()
                .map(|key| {
                    (
                        key.into(),
                        widget::get(&raw, key).map(|value| value.to_string()),
                    )
                })
                .collect(),
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if !Palette::CHOICES.iter().any(|(key, _)| *key == self.theme)
            || ![4, 6, 7, 9, KEYS.len()].contains(&self.original.len())
            || !KEYS[..4].iter().all(|key| self.original.contains_key(*key))
            || self
                .original
                .keys()
                .any(|key| !KEYS.contains(&key.as_str()))
            || self
                .original
                .values()
                .flatten()
                .any(|value| value.len() > 1024)
        {
            return Err("The saved colors contain unsupported preferences".into());
        }
        self.patch().map(|_| ())
    }
    pub fn rgb(&self) -> [u8; 3] {
        self.accent.unwrap_or_else(|| {
            if let Some(family) = self.family {
                return family.accent();
            }
            let color = Palette::named(&self.theme).accent;
            [color.r, color.g, color.b].map(|value| (value * 255.).round() as u8)
        })
    }
    pub fn palette(&self) -> Palette {
        if let Some(family) = self.family {
            return crate::desktop_theme::resolve(crate::config::Appearance {
                family: Some(family),
                accent: self.accent,
                ..Default::default()
            })
            .card;
        }
        self.accent
            .map_or_else(|| Palette::named(&self.theme), Palette::from_accent)
    }
    pub fn patch(&self) -> Result<WidgetPatch, String> {
        let mut patch = WidgetPatch::new();
        for (key, desired) in [
            ("/theme", serde_json::json!(self.theme)),
            (
                "/appearance/color_policy",
                serde_json::json!(if self.automatic {
                    crate::config::ColorPolicy::Wallpaper
                } else {
                    self.policy.unwrap_or(crate::config::ColorPolicy::Custom)
                }),
            ),
            ("/appearance/accent", serde_json::json!(self.accent)),
            ("/appearance/overrides", serde_json::json!(self.overrides)),
            ("/appearance/color_source", serde_json::json!("custom")),
            ("/appearance/sync_colors", serde_json::json!(self.sync)),
            (
                "/appearance/wallpaper_colors",
                serde_json::json!(self.automatic),
            ),
            (
                "/appearance/wallpaper_color_stamp",
                serde_json::json!(self.stamp),
            ),
        ] {
            let original = self
                .original
                .get(key)
                .and_then(Option::as_ref)
                .map(|value| serde_json::from_str(value).map_err(|error| error.to_string()))
                .transpose()?;
            let desired = if original.is_none()
                && ((key == "/appearance/color_policy" && self.policy.is_none())
                    || (key == "/appearance/overrides" && self.overrides == 0)
                    || (key == "/appearance/wallpaper_colors" && !self.automatic)
                    || (key == "/appearance/wallpaper_color_stamp" && self.stamp == 0))
            {
                None
            } else {
                Some(desired)
            };
            patch.set(key, original, desired)?;
        }
        for key in ["/appearance/family", "/appearance/native_mode_palettes"] {
            if let Some(original) = self.original.get(key) {
                let original = original
                    .as_ref()
                    .map(|value| serde_json::from_str(value).map_err(|error| error.to_string()))
                    .transpose()?;
                // Generated surfaces also depend on the original family. This is
                // a guard only, never a family write from Wallpaper & colors.
                patch.set(key, original.clone(), original)?;
            }
        }
        Ok(patch)
    }
    pub fn conflicts(&self, store: &DesktopStore) -> Result<bool, String> {
        let (raw, _) = read(store)?;
        Ok(self
            .patch()?
            .iter()
            .any(|(pointer, entry)| widget::get(&raw, pointer) != entry.original))
    }
}
