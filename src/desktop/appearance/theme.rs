//! COSMIC theme derivation, shared by the Appearance editor and palette
//! matching. Mirrors libcosmic dc11d6a `cosmic-theme` (`ThemeBuilder` and
//! `Theme`, both v2).
//!
//! A builder holds the user's choices; `ThemeBuilder::build` derives every
//! theme role from it. COSMIC apps read the derived `Theme` keys, which the
//! user (or another tool) may also have customized. An edit therefore
//! rewrites only the theme fields whose official derivation changed between
//! the old and the new builder, inside each component or container, and keeps
//! every other customized role. Values are decoded only when an edit needs to
//! rewrite them; a value this pin cannot represent exactly blocks the edit
//! instead of being reserialized without its unknown members.
use super::decode_exact;
use crate::desktop::store::{ConfigKey, DesktopPatch, RawValue, Snapshot};
use cosmic::cosmic_theme::color::{ColorRepr, ColorReprOption};
use cosmic::cosmic_theme::palette::{Srgb, Srgba};
use cosmic::cosmic_theme::{
    AlphaMap, BlurStrength, Component, Container, CornerRadii, CosmicPalette, CosmicPaletteInner,
    Spacing, Theme, ThemeBuilder,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const VERSION: u64 = 2;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ThemeVariant {
    #[default]
    Dark,
    Light,
}

impl ThemeVariant {
    pub const ALL: [Self; 2] = [Self::Light, Self::Dark];
    pub fn title(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
    pub fn theme_namespace(self) -> &'static str {
        match self {
            Self::Dark => "com.system76.CosmicTheme.Dark",
            Self::Light => "com.system76.CosmicTheme.Light",
        }
    }
    pub fn builder_namespace(self) -> &'static str {
        match self {
            Self::Dark => "com.system76.CosmicTheme.Dark.Builder",
            Self::Light => "com.system76.CosmicTheme.Light.Builder",
        }
    }
    /// Upstream defaults for keys neither the user nor the system defines.
    pub fn default_builder(self) -> ThemeBuilder {
        match self {
            Self::Dark => ThemeBuilder::dark(),
            Self::Light => ThemeBuilder::light(),
        }
    }
    pub fn of(key: &ConfigKey) -> Option<(Self, bool)> {
        [Self::Dark, Self::Light].into_iter().find_map(|variant| {
            if key.namespace() == variant.builder_namespace() {
                Some((variant, true))
            } else if key.namespace() == variant.theme_namespace() {
                Some((variant, false))
            } else {
                None
            }
        })
    }
}

pub fn builder_key(variant: ThemeVariant, name: &str) -> ConfigKey {
    ConfigKey::new(variant.builder_namespace(), VERSION, name).expect("builder keys are valid")
}
pub fn theme_key(variant: ThemeVariant, name: &str) -> ConfigKey {
    ConfigKey::new(variant.theme_namespace(), VERSION, name).expect("theme keys are valid")
}

/// Builder fields the editor may write.
pub const BUILDER_WRITES: [&str; 15] = [
    "accent",
    "bg_color",
    "primary_container_bg",
    "secondary_container_bg",
    "window_hint",
    "active_hint",
    "gaps",
    "corner_radii",
    "spacing",
    "frosted",
    "alpha_map",
    "frosted_windows",
    "frosted_system_interface",
    "frosted_panel",
    "frosted_applets",
];
/// Builder fields only read as derivation inputs.
pub const BUILDER_INPUTS: [&str; 6] = [
    "palette",
    "neutral_tint",
    "text_tint",
    "success",
    "warning",
    "destructive",
];
const COMPONENTS: [&str; 13] = [
    "button",
    "accent",
    "success",
    "destructive",
    "warning",
    "accent_button",
    "success_button",
    "destructive_button",
    "warning_button",
    "icon_button",
    "link_button",
    "list_button",
    "text_button",
];
const CONTAINERS: [&str; 6] = [
    "background",
    "primary",
    "secondary",
    "transparent_background",
    "transparent_primary",
    "transparent_secondary",
];
/// Theme fields an edit may rewrite. `name`, `is_dark` and
/// `is_high_contrast` are never written.
pub const THEME_WRITES: [&str; 35] = [
    "button",
    "accent",
    "success",
    "destructive",
    "warning",
    "accent_button",
    "success_button",
    "destructive_button",
    "warning_button",
    "icon_button",
    "link_button",
    "list_button",
    "text_button",
    "background",
    "primary",
    "secondary",
    "transparent_background",
    "transparent_primary",
    "transparent_secondary",
    "palette",
    "spacing",
    "corner_radii",
    "gaps",
    "active_hint",
    "window_hint",
    "frosted",
    "frosted_windows",
    "frosted_system_interface",
    "frosted_panel",
    "frosted_applets",
    "alpha_map",
    "shade",
    "accent_text",
    "control_tint",
    "text_tint",
];
/// Wholly accent-specific components, including contrast-aware label colors:
/// replaced together rather than per field.
const WHOLE: [&str; 3] = ["accent", "accent_button", "accent_text"];

/// A value an edit needs but cannot rewrite faithfully.
#[derive(Clone, Debug, PartialEq)]
pub struct Blocked {
    pub key: ConfigKey,
    pub reason: String,
}

fn colors_rgb(value: ColorReprOption) -> Option<Srgb> {
    value.into()
}
fn colors_rgba(value: ColorReprOption) -> Option<Srgba> {
    value.into()
}

/// The effective builder: user values, then system defaults, then the
/// upstream default for the variant, field by field like `get_entry`.
pub fn read_builder(snapshot: &Snapshot, variant: ThemeVariant) -> Result<ThemeBuilder, Blocked> {
    let mut builder = variant.default_builder();
    macro_rules! field {
        ($name:ident: $ty:ty => $convert:expr) => {
            let key = builder_key(variant, stringify!($name));
            if let Some(raw) = snapshot.effective(&key) {
                let value: $ty = decode_exact(raw).map_err(|reason| Blocked {
                    key: key.clone(),
                    reason,
                })?;
                builder.$name = $convert(value);
            }
        };
    }
    let key = builder_key(variant, "palette");
    if let Some(raw) = snapshot.effective(&key) {
        builder.palette = decode_palette(raw, variant).map_err(|reason| Blocked {
            key: key.clone(),
            reason,
        })?;
    }
    field!(spacing: Spacing => |value| value);
    field!(corner_radii: CornerRadii => |value| value);
    field!(neutral_tint: ColorReprOption => colors_rgb);
    field!(bg_color: ColorReprOption => colors_rgba);
    field!(primary_container_bg: ColorReprOption => colors_rgba);
    field!(secondary_container_bg: ColorReprOption => colors_rgba);
    field!(text_tint: ColorReprOption => colors_rgb);
    field!(accent: ColorReprOption => colors_rgb);
    field!(success: ColorReprOption => colors_rgb);
    field!(warning: ColorReprOption => colors_rgb);
    field!(destructive: ColorReprOption => colors_rgb);
    field!(frosted: BlurStrength => |value| value);
    field!(gaps: (u32, u32) => |value| value);
    field!(active_hint: u32 => |value| value);
    field!(window_hint: ColorReprOption => colors_rgb);
    field!(frosted_windows: bool => |value| value);
    field!(frosted_system_interface: bool => |value| value);
    field!(frosted_panel: bool => |value| value);
    field!(frosted_applets: bool => |value| value);
    field!(alpha_map: AlphaMap => |value| value);
    Ok(builder)
}

/// A builder palette. cosmic-settings 1.10 installs the Dark builder default
/// as an untagged palette (the derived-theme form, without `Dark(...)`),
/// which `get_entry` silently replaces with the built-in palette. Read that
/// exact installed palette instead, tagged by the namespace it lives in; any
/// other undecodable or unknown value still blocks.
fn decode_palette(raw: &RawValue, variant: ThemeVariant) -> Result<CosmicPalette, String> {
    decode_exact::<CosmicPalette>(raw).or_else(|tagged| {
        let inner: CosmicPaletteInner = decode_exact(raw).map_err(|_| tagged)?;
        Ok(match variant {
            ThemeVariant::Dark => CosmicPalette::Dark(inner),
            ThemeVariant::Light => CosmicPalette::Light(inner),
        })
    })
}

/// Writes the builder fields `edited` changed and guards every other field
/// the derivation read.
pub fn builder_patch(
    patch: &mut DesktopPatch,
    snapshot: &Snapshot,
    variant: ThemeVariant,
    base: &ThemeBuilder,
    edited: &ThemeBuilder,
) -> Result<(), String> {
    macro_rules! write {
        ($name:ident => $encode:expr) => {{
            let key = builder_key(variant, stringify!($name));
            let original = snapshot.user(&key).cloned();
            if base.$name != edited.$name {
                let value = RawValue::encode(&$encode(&edited.$name)).map_err(|e| e.to_string())?;
                patch.set(key, original, Some(value));
            } else {
                patch.guard(key, original);
            }
        }};
    }
    let rgb = |value: &Option<Srgb>| ColorReprOption::from(*value);
    let rgba = |value: &Option<Srgba>| ColorReprOption::from(*value);
    write!(accent => rgb);
    write!(bg_color => rgba);
    write!(primary_container_bg => rgba);
    write!(secondary_container_bg => rgba);
    write!(window_hint => rgb);
    write!(active_hint => |value: &u32| *value);
    write!(gaps => |value: &(u32, u32)| *value);
    write!(corner_radii => |value: &CornerRadii| *value);
    write!(spacing => |value: &Spacing| *value);
    write!(frosted => |value: &BlurStrength| *value);
    write!(alpha_map => |value: &AlphaMap| *value);
    write!(frosted_windows => |value: &bool| *value);
    write!(frosted_system_interface => |value: &bool| *value);
    write!(frosted_panel => |value: &bool| *value);
    write!(frosted_applets => |value: &bool| *value);
    for name in BUILDER_INPUTS {
        let key = builder_key(variant, name);
        let original = snapshot.user(&key).cloned();
        patch.guard(key, original);
    }
    Ok(())
}

/// Copies only fields whose official derivation changed. For example, a
/// customized ordinary button background stays while its focus ring follows
/// a new accent.
pub fn merge_component(current: &mut Component, old: &Component, new: &Component) {
    macro_rules! fields {
        ($($field:ident),+ $(,)?) => { $(
            if old.$field != new.$field { current.$field = new.$field; }
        )+ };
    }
    fields!(
        base,
        hover,
        pressed,
        selected,
        selected_text,
        focus,
        divider,
        on,
        disabled,
        on_disabled,
        border,
        disabled_border
    );
}
fn merge_container(current: &mut Container, old: &Container, new: &Container) {
    merge_component(&mut current.component, &old.component, &new.component);
    macro_rules! fields {
        ($($field:ident),+ $(,)?) => { $(
            if old.$field != new.$field { current.$field = new.$field; }
        )+ };
    }
    fields!(base, divider, on, small_widget);
}
fn replace<T: Clone + PartialEq>(current: &mut T, old: &T, new: &T) {
    if old != new {
        *current = new.clone();
    }
}

/// When the wholly accent-specific components are replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whole {
    /// Only when their derivation changed (editor).
    WhenChanged,
    /// Always, the long-standing palette-matching behavior.
    Always,
}

/// A derivation: the theme the current values were derived from, the theme
/// the edited builder derives, and the stand-in for keys that are absent.
pub struct Derivation<'a> {
    pub fallback: &'a Theme,
    pub old: &'a Theme,
    pub new: &'a Theme,
    pub whole: Whole,
}

struct Sink<'a> {
    patch: &'a mut DesktopPatch,
    snapshot: &'a Snapshot,
    variant: ThemeVariant,
    whole: Whole,
    blocked: Vec<Blocked>,
}

impl Sink<'_> {
    #[allow(clippy::too_many_arguments)]
    fn key<T: Clone + PartialEq>(
        &mut self,
        name: &'static str,
        fallback: &T,
        old: &T,
        new: &T,
        merge: impl Fn(&mut T, &T, &T),
        decode: impl Fn(&RawValue) -> Result<T, String>,
        encode: impl Fn(&T) -> Result<RawValue, String>,
    ) {
        let whole = WHOLE.contains(&name);
        let always = whole && self.whole == Whole::Always;
        if old == new && !always {
            return;
        }
        let key = theme_key(self.variant, name);
        let original = self.snapshot.user(&key).cloned();
        let current = match self.snapshot.effective(&key) {
            Some(raw) => match decode(raw) {
                Ok(value) => value,
                Err(reason) => {
                    self.blocked.push(Blocked { key, reason });
                    return;
                }
            },
            None => fallback.clone(),
        };
        let mut result = current.clone();
        if whole {
            result = new.clone();
        } else {
            merge(&mut result, old, new);
        }
        if result == current {
            self.patch.guard(key, original);
            return;
        }
        match encode(&result) {
            Ok(value) => self.patch.set(key, original, Some(value)),
            Err(reason) => self.blocked.push(Blocked { key, reason }),
        }
    }
    fn plain<T: Clone + PartialEq + Serialize + DeserializeOwned>(
        &mut self,
        name: &'static str,
        fallback: &T,
        old: &T,
        new: &T,
        merge: impl Fn(&mut T, &T, &T),
    ) {
        self.key(
            name,
            fallback,
            old,
            new,
            merge,
            decode_exact::<T>,
            |value| RawValue::encode(value).map_err(|error| error.to_string()),
        );
    }
    /// Colors stored as `ColorRepr` (hex or struct) through cosmic-config.
    fn repr<T: Clone + PartialEq>(
        &mut self,
        name: &'static str,
        fallback: &T,
        old: &T,
        new: &T,
        from: fn(ColorReprOption) -> T,
        to: fn(&T) -> ColorReprOption,
    ) {
        self.key(
            name,
            fallback,
            old,
            new,
            replace,
            |raw| decode_exact::<ColorReprOption>(raw).map(from),
            |value| RawValue::encode(&to(value)).map_err(|error| error.to_string()),
        );
    }
}

fn container<'a>(theme: &'a Theme, name: &str) -> &'a Container {
    match name {
        "background" => theme.background(false),
        "primary" => theme.primary(false),
        "secondary" => theme.secondary(false),
        "transparent_background" => theme.background(true),
        "transparent_primary" => theme.primary(true),
        _ => theme.secondary(true),
    }
}
fn component<'a>(theme: &'a Theme, name: &str) -> &'a Component {
    match name {
        "button" => &theme.button,
        "accent" => &theme.accent,
        "success" => &theme.success,
        "destructive" => &theme.destructive,
        "warning" => &theme.warning,
        "accent_button" => &theme.accent_button,
        "success_button" => &theme.success_button,
        "destructive_button" => &theme.destructive_button,
        "warning_button" => &theme.warning_button,
        "icon_button" => &theme.icon_button,
        "link_button" => &theme.link_button,
        "list_button" => &theme.list_button,
        _ => &theme.text_button,
    }
}

/// Adds the theme keys the derivation changes to `patch`, guarding the ones
/// it read but left alone. Nothing is added for a variant with blocked keys.
pub fn theme_patch(
    patch: &mut DesktopPatch,
    snapshot: &Snapshot,
    variant: ThemeVariant,
    derivation: &Derivation<'_>,
) -> Result<(), Vec<Blocked>> {
    let mut staged = DesktopPatch::new();
    let mut sink = Sink {
        patch: &mut staged,
        snapshot,
        variant,
        whole: derivation.whole,
        blocked: Vec::new(),
    };
    let (fallback, old, new) = (derivation.fallback, derivation.old, derivation.new);
    for name in COMPONENTS {
        sink.plain(
            name,
            component(fallback, name),
            component(old, name),
            component(new, name),
            merge_component,
        );
    }
    for name in CONTAINERS {
        sink.plain(
            name,
            container(fallback, name),
            container(old, name),
            container(new, name),
            merge_container,
        );
    }
    sink.plain::<CosmicPaletteInner>(
        "palette",
        &fallback.palette,
        &old.palette,
        &new.palette,
        replace,
    );
    sink.plain(
        "spacing",
        &fallback.spacing,
        &old.spacing,
        &new.spacing,
        replace,
    );
    sink.plain(
        "corner_radii",
        &fallback.corner_radii,
        &old.corner_radii,
        &new.corner_radii,
        replace,
    );
    sink.plain("gaps", &fallback.gaps, &old.gaps, &new.gaps, replace);
    sink.plain(
        "active_hint",
        &fallback.active_hint,
        &old.active_hint,
        &new.active_hint,
        replace,
    );
    // The theme stores window_hint as a plain color struct.
    sink.plain(
        "window_hint",
        &fallback.window_hint,
        &old.window_hint,
        &new.window_hint,
        replace,
    );
    sink.plain(
        "frosted",
        &fallback.frosted,
        &old.frosted,
        &new.frosted,
        replace,
    );
    macro_rules! flags {
        ($($field:ident),+) => { $(
            sink.plain(stringify!($field), &fallback.$field, &old.$field, &new.$field, replace);
        )+ };
    }
    flags!(
        frosted_windows,
        frosted_system_interface,
        frosted_panel,
        frosted_applets
    );
    sink.plain(
        "alpha_map",
        &fallback.alpha_map,
        &old.alpha_map,
        &new.alpha_map,
        replace,
    );
    sink.key(
        "shade",
        &fallback.shade,
        &old.shade,
        &new.shade,
        replace,
        |raw| decode_exact::<ColorRepr>(raw).map(Srgba::from),
        |value| RawValue::encode(&ColorRepr::from(*value)).map_err(|error| error.to_string()),
    );
    sink.repr(
        "accent_text",
        &fallback.accent_text,
        &old.accent_text,
        &new.accent_text,
        colors_rgba,
        |value| ColorReprOption::from(*value),
    );
    sink.repr(
        "control_tint",
        &fallback.control_tint,
        &old.control_tint,
        &new.control_tint,
        colors_rgb,
        |value| ColorReprOption::from(*value),
    );
    sink.repr(
        "text_tint",
        &fallback.text_tint,
        &old.text_tint,
        &new.text_tint,
        colors_rgb,
        |value| ColorReprOption::from(*value),
    );
    let blocked = std::mem::take(&mut sink.blocked);
    if !blocked.is_empty() {
        return Err(blocked);
    }
    patch.extend(staged);
    Ok(())
}

/// Verify a generated transition before rebasing another semantic edit.
/// Compare typed values (including absent defaults), so an Undo that restores
/// absent files works too. A direct customization of a derived field still
/// conflicts; only the forward derivation is evidence. Reversing it can erase
/// custom fields and falsely match the old snapshot.
pub(super) fn derived_transition_matches(
    before: &Snapshot,
    after: &Snapshot,
    variant: ThemeVariant,
    old: &Theme,
    new: &Theme,
    keys: &[ConfigKey],
) -> bool {
    let mut patch = DesktopPatch::new();
    if theme_patch(
        &mut patch,
        before,
        variant,
        &Derivation {
            fallback: old,
            old,
            new,
            whole: Whole::WhenChanged,
        },
    )
    .is_err()
    {
        return false;
    }
    let mut expected = before.clone();
    for (key, entry) in patch.iter() {
        let mut value = expected.entry(key).cloned().unwrap_or_default();
        value.user = entry.desired.clone();
        expected.insert(key.clone(), value);
    }
    fn same<T: PartialEq + Serialize + DeserializeOwned>(
        a: Option<&RawValue>,
        b: Option<&RawValue>,
        fallback: &T,
    ) -> bool {
        let value = |raw| match raw {
            Some(raw) => decode_exact::<T>(raw).ok(),
            // Theme colors serialize to 8-bit hex. Compare missing defaults
            // at that same precision, including when Undo removes the file.
            None => RawValue::encode(fallback)
                .ok()
                .and_then(|raw| decode_exact::<T>(&raw).ok()),
        };
        match (value(a), value(b)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
    keys.iter()
        .filter(|key| key.namespace() == variant.theme_namespace())
        .all(|key| {
            let a = expected.effective(key);
            let b = after.effective(key);
            let name = key.key();
            if COMPONENTS.contains(&name) {
                return same(a, b, component(new, name));
            }
            if CONTAINERS.contains(&name) {
                return same(a, b, container(new, name));
            }
            macro_rules! plain {
            ($($field:ident),+ $(,)?) => { match name {
                $(stringify!($field) => same(a, b, &new.$field),)+
                "shade" => same(a, b, &ColorRepr::from(new.shade)),
                "accent_text" => same(a, b, &ColorReprOption::from(new.accent_text)),
                "control_tint" => same(a, b, &ColorReprOption::from(new.control_tint)),
                "text_tint" => same(a, b, &ColorReprOption::from(new.text_tint)),
                _ => false,
            }};
        }
            plain!(
                palette,
                spacing,
                corner_radii,
                gaps,
                active_hint,
                window_hint,
                frosted,
                frosted_windows,
                frosted_system_interface,
                frosted_panel,
                frosted_applets,
                alpha_map
            )
        })
}

/// The value a theme key will have after `patch`, else its current value,
/// else the derived one. Used by the preview; reads only.
pub fn theme_value<T: DeserializeOwned + Serialize>(
    patch: &DesktopPatch,
    snapshot: &Snapshot,
    variant: ThemeVariant,
    name: &str,
) -> Option<T> {
    let key = theme_key(variant, name);
    let raw = match patch.get(&key) {
        Some(entry) => entry.desired.as_ref(),
        None => snapshot.effective(&key),
    }?;
    decode_exact(raw).ok()
}
