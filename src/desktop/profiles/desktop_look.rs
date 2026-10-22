//! The version 3 profile sections: `wallpaper` and `desktop_icons`.
//!
//! `wallpaper` describes the shared background and, in local profiles
//! only, the display arrangement (`displays`) and display-specific
//! backgrounds (`per_display`). A source names a bundled palette wallpaper,
//! a picture or folder relative to a trusted system `backgrounds` folder, a
//! colour or a gradient; personal pictures and folders are absolute paths
//! and also stay on this computer. `desktop_icons` holds cosmic-files'
//! desktop members under plain names. Every member is optional: absent
//! leaves that setting alone. Values are validated with the same native
//! validators the store applies, so nothing here can describe an entry the
//! Wallpaper or Desktop icons page could not write.
use crate::desktop::appearance;
use crate::desktop::desktop_icons::{self, DesktopIcons, IconEdits};
use crate::desktop::store::{RawValue, rule_for};
use crate::desktop::wallpaper::{
    self, BgColor, BgEntry, BgGradient, BgSource, Displays, MAX_ROTATION, SamplingMethod,
    ScalingMode,
};
use cosmic::cosmic_theme::palette::Srgba;
use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};

const MAX_PATH_BYTES: usize = 4096;
const MAX_PARTS: usize = 16;

/// What portable profiles leave on this computer, as the export notice
/// names it.
pub const OWN_WALLPAPER: &str = "your own wallpaper";
pub const DISPLAY_BACKGROUNDS: &str = "display-specific backgrounds";
pub const DISPLAY_ARRANGEMENT: &str = "the display arrangement";

/// Whether every display shows the shared background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplaysName {
    Same,
    Separate,
}

impl From<Displays> for DisplaysName {
    fn from(value: Displays) -> Self {
        match value {
            Displays::Same => Self::Same,
            Displays::Separate => Self::Separate,
        }
    }
}
impl From<DisplaysName> for Displays {
    fn from(value: DisplaysName) -> Self {
        match value {
            DisplaysName::Same => Self::Same,
            DisplaysName::Separate => Self::Separate,
        }
    }
}

/// An opaque colour: `"#RRGGBB"`, or exact `[r, g, b]` components in 0–1
/// when a captured value is not 8-bit exact. Solid colours and the
/// letterbox are sRGB; for gradient stops a hex value is sRGB (converted to
/// linear light when applied) and exact components are linear, as stored.
#[derive(Clone, Debug, PartialEq)]
pub enum Rgb {
    Hex(String),
    Exact([f32; 3]),
}

fn hex_rgb(text: &str) -> Option<[f32; 3]> {
    (text.len() == 7 && text.starts_with('#'))
        .then(|| appearance::parse_hex(text, false))
        .flatten()
        .map(|color| [color.red, color.green, color.blue])
}
fn rgb_hex(rgb: [f32; 3]) -> String {
    appearance::hex(Srgba::new(rgb[0], rgb[1], rgb[2], 1.), false)
}
fn in_range(rgb: [f32; 3]) -> Result<[f32; 3], String> {
    if rgb.iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)) {
        Ok(rgb)
    } else {
        Err("colour components must be between 0 and 1".into())
    }
}

impl Rgb {
    /// sRGB components (solid colours, the letterbox).
    pub fn srgb(&self) -> Result<[f32; 3], String> {
        match self {
            Self::Hex(text) => hex_rgb(text).ok_or_else(|| format!("invalid color {text}")),
            Self::Exact(rgb) => in_range(*rgb),
        }
    }
    /// Linear components (gradient stops).
    pub fn linear(&self) -> Result<[f32; 3], String> {
        match self {
            Self::Hex(_) => self.srgb().map(|rgb| rgb.map(wallpaper::srgb_to_linear)),
            Self::Exact(rgb) => in_range(*rgb),
        }
    }
    /// Hex when it gives back exactly `rgb` (sRGB), else the components.
    pub fn of_srgb(rgb: [f32; 3]) -> Self {
        let text = rgb_hex(rgb);
        if hex_rgb(&text) == Some(rgb) {
            Self::Hex(text)
        } else {
            Self::Exact(rgb)
        }
    }
    /// Hex when converting it back gives exactly `rgb` (linear), else the
    /// linear components.
    pub fn of_linear(rgb: [f32; 3]) -> Self {
        let text = rgb_hex(rgb.map(wallpaper::linear_to_srgb));
        let back = Self::Hex(text);
        if back.linear() == Ok(rgb) {
            back
        } else {
            Self::Exact(rgb)
        }
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Hex(text) => serializer.serialize_str(text),
            Self::Exact(components) => components.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::String(text) if text.len() <= 16 => Ok(Self::Hex(text)),
            Value::Array(items) if items.len() == 3 => {
                let mut components = [0f32; 3];
                for (slot, item) in components.iter_mut().zip(&items) {
                    *slot = item
                        .as_f64()
                        .ok_or_else(|| de::Error::custom("color components must be numbers"))?
                        as f32;
                }
                Ok(Self::Exact(components))
            }
            _ => Err(de::Error::custom("a color is \"#RRGGBB\" or [r, g, b]")),
        }
    }
}

/// A gradient: 2–8 stops and cosmic-bg's angle (0 bottom→top, 90
/// left→right, 180 top→bottom, 270 right→left).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradientLook {
    pub stops: Vec<Rgb>,
    pub angle: f32,
}

/// Where a background comes from. `image` and `folder` are local only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLook {
    /// A bundled palette wallpaper, by palette name.
    Bundled(String),
    /// A picture or folder relative to a system `backgrounds` folder.
    System(String),
    Color(Rgb),
    Gradient(GradientLook),
    Image(PathBuf),
    Folder(PathBuf),
}

impl SourceLook {
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Image(_) | Self::Folder(_))
    }
    /// The native colour of a colour or gradient source.
    pub fn color(&self) -> Result<Option<BgColor>, String> {
        Ok(match self {
            Self::Color(rgb) => Some(BgColor::Single(rgb.srgb()?)),
            Self::Gradient(gradient) => Some(BgColor::Gradient(BgGradient {
                colors: gradient
                    .stops
                    .iter()
                    .map(Rgb::linear)
                    .collect::<Result<_, _>>()?,
                radius: gradient.angle,
            })),
            _ => None,
        })
    }
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Bundled(name) => {
                if crate::desktop::wallpaper::bundled_name(name).is_none() {
                    return Err(format!(
                        "“{}” is not a built-in wallpaper",
                        name.chars().take(40).collect::<String>()
                    ));
                }
            }
            Self::System(relative) => {
                if !valid_relative(relative) {
                    return Err("a COSMIC wallpaper is named by a plain relative path".into());
                }
            }
            Self::Color(_) | Self::Gradient(_) => {
                if let Some(BgColor::Gradient(gradient)) = self.color()? {
                    if !(2..=8).contains(&gradient.colors.len()) {
                        return Err("a gradient has 2 to 8 colors".into());
                    }
                    if !gradient.radius.is_finite() || !(0.0..=360.0).contains(&gradient.radius) {
                        return Err("the gradient angle is between 0 and 360".into());
                    }
                }
            }
            Self::Image(path) | Self::Folder(path) => {
                let plain = path.to_str().is_some_and(|text| {
                    path.is_absolute()
                        && text.len() <= MAX_PATH_BYTES
                        && !text.chars().any(char::is_control)
                });
                if !plain {
                    return Err("a picture is a plain absolute path".into());
                }
            }
        }
        Ok(())
    }
}

/// `cosmic/<file or folder>`: 1–16 parts of `[A-Za-z0-9._ -]`, none of
/// them empty, `.` or `..` (the same rule `wallpaper::classify` applies).
pub fn valid_relative(relative: &str) -> bool {
    let parts: Vec<&str> = relative.split('/').collect();
    relative.len() <= MAX_PATH_BYTES
        && parts.len() <= MAX_PARTS
        && parts.iter().all(|part| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && part.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b' ' | b'-')
                })
        })
}

/// cosmic-bg's scaling: Fill covers and crops, Fit letterboxes, Stretch
/// ignores the aspect ratio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitName {
    Fill,
    Fit,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderName {
    /// By full path.
    InOrder,
    Shuffle,
}

impl From<SamplingMethod> for OrderName {
    fn from(value: SamplingMethod) -> Self {
        match value {
            SamplingMethod::Alphanumeric => Self::InOrder,
            SamplingMethod::Random => Self::Shuffle,
        }
    }
}
impl From<OrderName> for SamplingMethod {
    fn from(value: OrderName) -> Self {
        match value {
            OrderName::InOrder => Self::Alphanumeric,
            OrderName::Shuffle => Self::Random,
        }
    }
}

/// A folder's slideshow: seconds per picture (0 pauses) and order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideshowLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<OrderName>,
}

/// One background. Members cosmic-bg stores but doesn't use (filter by
/// theme, filter method) are never part of a profile: the target keeps
/// its own.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackgroundLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceLook>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit: Option<FitName>,
    /// The letterbox colour (sRGB) with `fit: "fit"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit_color: Option<Rgb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slideshow: Option<SlideshowLook>,
}

impl BackgroundLook {
    pub fn is_local(&self) -> bool {
        self.source.as_ref().is_some_and(SourceLook::is_local)
    }
    /// The scaling this look sets; `current` is the target's, whose
    /// letterbox colour a bare `fit` keeps (black otherwise).
    pub fn scaling(&self, current: Option<&ScalingMode>) -> Result<Option<ScalingMode>, String> {
        Ok(match self.fit {
            None => None,
            Some(FitName::Fill) => Some(ScalingMode::Zoom),
            Some(FitName::Stretch) => Some(ScalingMode::Stretch),
            Some(FitName::Fit) => Some(ScalingMode::Fit(match (&self.fit_color, current) {
                (Some(color), _) => color.srgb()?,
                (None, Some(ScalingMode::Fit(color))) => *color,
                (None, _) => [0., 0., 0.],
            })),
        })
    }
    pub fn rotation(&self) -> Option<u64> {
        self.slideshow.as_ref().and_then(|show| show.every)
    }
    pub fn sampling(&self) -> Option<SamplingMethod> {
        self.slideshow
            .as_ref()
            .and_then(|show| show.order)
            .map(Into::into)
    }
    fn validate(&self) -> Result<(), String> {
        if let Some(source) = &self.source {
            source.validate()?;
        }
        if self.fit_color.is_some() && self.fit != Some(FitName::Fit) {
            return Err("a letterbox color needs “fit”: “fit”".into());
        }
        if let Some(color) = &self.fit_color {
            color.srgb()?;
        }
        if self.slideshow.is_some()
            && !matches!(
                self.source,
                None | Some(SourceLook::System(_) | SourceLook::Folder(_))
            )
        {
            return Err("a slideshow needs a folder".into());
        }
        if self.rotation().is_some_and(|every| every > MAX_ROTATION) {
            return Err("the slideshow interval is longer than a week".into());
        }
        // The native entry validator, on a sample entry with these members.
        let source = match &self.source {
            Some(SourceLook::Image(path) | SourceLook::Folder(path)) => {
                BgSource::Path(path.clone())
            }
            Some(look) => match look.color()? {
                Some(color) => BgSource::Color(color),
                None => BgSource::Path(PathBuf::from("/backgrounds/sample.png")),
            },
            None => BgSource::Color(BgColor::Single([0., 0., 0.])),
        };
        let mut entry = BgEntry::new(wallpaper::SHARED, source);
        if let Some(scaling) = self.scaling(None)? {
            entry.scaling_mode = scaling;
        }
        if let Some(every) = self.rotation() {
            entry.rotation_frequency = every;
        }
        let value = RawValue::encode(&entry).map_err(|error| error.to_string())?;
        wallpaper::check_entry(&value)
    }
}

/// The wallpaper section.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallpaperLook {
    /// Local only. With `per_display`, the arrangement is replaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub displays: Option<DisplaysName>,
    /// The shared background (also what displays without their own show).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<BackgroundLook>,
    /// Local only: displays with their own background, by connector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_display: Option<BTreeMap<String, BackgroundLook>>,
}

impl WallpaperLook {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
    /// The first member a shared profile may not have.
    pub fn local_member(&self) -> Option<&'static str> {
        if self.displays.is_some() {
            return Some("displays");
        }
        if self.per_display.is_some() {
            return Some("per_display");
        }
        match self
            .background
            .as_ref()
            .and_then(|look| look.source.as_ref())
        {
            Some(SourceLook::Image(_)) => Some("image"),
            Some(SourceLook::Folder(_)) => Some("folder"),
            _ => None,
        }
    }
    /// Removes everything a shared profile may not have and returns what
    /// was left out, as the export notice names it. A personal source
    /// takes its whole background with it, so its fit and slideshow can't
    /// change someone else's current wallpaper.
    pub fn strip_local(&mut self) -> Vec<&'static str> {
        let mut omitted = Vec::new();
        if self
            .background
            .as_ref()
            .is_some_and(BackgroundLook::is_local)
        {
            self.background = None;
            omitted.push(OWN_WALLPAPER);
        }
        if self.per_display.take().is_some_and(|map| !map.is_empty()) {
            self.displays = None;
            omitted.push(DISPLAY_BACKGROUNDS);
        } else if self.displays.take().is_some() {
            omitted.push(DISPLAY_ARRANGEMENT);
        }
        omitted
    }
    pub fn validate(&self) -> Result<(), String> {
        if let Some(look) = &self.background {
            look.validate()
                .map_err(|message| format!("Wallpaper: {message}"))?;
        }
        if let Some(map) = &self.per_display {
            match self.displays {
                None => {
                    return Err(
                        "Wallpaper: display-specific backgrounds need “displays”: “separate”"
                            .into(),
                    );
                }
                Some(DisplaysName::Same) if !map.is_empty() => {
                    return Err(
                        "Wallpaper: with the same background on all displays, no display has its own"
                            .into(),
                    );
                }
                _ => {}
            }
            if map.len() > wallpaper::MAX_OUTPUTS {
                return Err("Wallpaper: at most 16 displays have their own background".into());
            }
            for (name, look) in map {
                if !wallpaper::valid_output_name(name) {
                    return Err("Wallpaper: a display name this version can’t use".into());
                }
                look.validate()
                    .map_err(|message| format!("Wallpaper for {name}: {message}"))?;
            }
        }
        Ok(())
    }
}

/// The desktop icons section: percentages (1–500) and what shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopIconsLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_size: Option<u16>,
    /// Applies in both directions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<u16>,
    /// The Desktop folder's contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_folder: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_drives: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_trash: Option<bool>,
}

impl DesktopIconsLook {
    /// Every member of a native value, exactly.
    pub fn of(config: &DesktopIcons) -> Self {
        Self {
            icon_size: Some(config.icon_size.get()),
            spacing: Some(config.grid_spacing.get()),
            show_folder: Some(config.show_content),
            show_drives: Some(config.show_mounted_drives),
            show_trash: Some(config.show_trash),
        }
    }
    /// The member edits this look makes (sizes already validated).
    pub fn edits(&self) -> IconEdits {
        IconEdits {
            icon_size: self.icon_size.and_then(NonZeroU16::new),
            grid_spacing: self.spacing.and_then(NonZeroU16::new),
            show_content: self.show_folder,
            show_mounted_drives: self.show_drives,
            show_trash: self.show_trash,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        for (what, value) in [("icon size", self.icon_size), ("spacing", self.spacing)] {
            if value.is_some_and(|value| !(1..=desktop_icons::SCALE_MAX).contains(&value)) {
                return Err(format!("Desktop icons: the {what} is between 1 and 500 %"));
            }
        }
        let mut config = DesktopIcons::default();
        self.edits().apply_to(&mut config);
        let value = RawValue::encode(&config).map_err(|error| error.to_string())?;
        let rule = rule_for(&desktop_icons::key()).ok_or("Desktop icons are not editable")?;
        (rule.validate)(&value).map_err(|message| format!("Desktop icons: {message}"))
    }
}

/// A short, recognisable name for a picture or folder in notices.
pub fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
