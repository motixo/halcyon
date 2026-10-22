//! Read-only discovery of installed font families and icon themes.
//!
//! Fonts come from the font database this process already loaded for
//! rendering. Icon themes come from `index.theme` files in the XDG icon
//! directories, user first. Nothing is installed, written or executed; a
//! name read from a file is only ever displayed or stored as a setting.
use cosmic::iced::font::Weight;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// One installed family and the upright/italic weights it offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFamily {
    pub name: String,
    pub weights: Vec<Weight>,
    pub italic: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontCatalog {
    pub interface: Vec<FontFamily>,
    pub monospace: Vec<FontFamily>,
}

/// The CSS-style weight number fontdb reports, mapped to iced's names.
pub fn weight_from_number(number: u16) -> Weight {
    match number {
        0..=149 => Weight::Thin,
        150..=249 => Weight::ExtraLight,
        250..=349 => Weight::Light,
        350..=449 => Weight::Normal,
        450..=549 => Weight::Medium,
        550..=649 => Weight::Semibold,
        650..=749 => Weight::Bold,
        750..=849 => Weight::ExtraBold,
        _ => Weight::Black,
    }
}

pub fn weight_title(weight: Weight) -> &'static str {
    match weight {
        Weight::Thin => "Thin",
        Weight::ExtraLight => "Extra light",
        Weight::Light => "Light",
        Weight::Normal => "Regular",
        Weight::Medium => "Medium",
        Weight::Semibold => "Semibold",
        Weight::Bold => "Bold",
        Weight::ExtraBold => "Extra bold",
        Weight::Black => "Black",
    }
}

/// A face as the catalog needs it: family, monospace, weight number,
/// italic/oblique and normal stretch.
pub struct Face<'a> {
    pub family: &'a str,
    pub monospace: bool,
    pub weight: u16,
    pub italic: bool,
    pub normal_stretch: bool,
}

impl FontCatalog {
    /// Groups faces by family like cosmic-settings: monospace families are
    /// offered for the monospace font, the rest for the interface font.
    pub fn from_faces<'a>(faces: impl IntoIterator<Item = Face<'a>>) -> Self {
        let mut families: BTreeMap<(bool, String), FontFamily> = BTreeMap::new();
        for face in faces {
            let name = face.family.trim();
            if name.is_empty() || !face.normal_stretch || name.chars().any(char::is_control) {
                continue;
            }
            let family = families
                .entry((face.monospace, name.to_string()))
                .or_insert_with(|| FontFamily {
                    name: name.to_string(),
                    weights: Vec::new(),
                    italic: false,
                });
            if face.italic {
                family.italic = true;
            } else {
                let weight = weight_from_number(face.weight);
                if !family.weights.contains(&weight) {
                    family.weights.push(weight);
                }
            }
        }
        let mut catalog = Self::default();
        for ((monospace, _), mut family) in families {
            family.weights.sort_by_key(|weight| *weight as u8);
            if family.weights.is_empty() {
                continue;
            }
            if monospace {
                catalog.monospace.push(family);
            } else {
                catalog.interface.push(family);
            }
        }
        catalog
    }
    /// Families from the renderer's font database (already loaded; reads only).
    pub fn installed() -> Self {
        let system = cosmic::iced::advanced::graphics::text::font_system();
        let Ok(mut system) = system.write() else {
            return Self::default();
        };
        let db = system.raw().db();
        Self::from_faces(db.faces().flat_map(|face| {
            face.families.first().map(|(family, _)| Face {
                family: family.as_str(),
                monospace: face.monospaced,
                weight: face.weight.0,
                italic: face.style
                    != cosmic::iced::advanced::graphics::text::cosmic_text::fontdb::Style::Normal,
                normal_stretch: face.stretch
                    == cosmic::iced::advanced::graphics::text::cosmic_text::fontdb::Stretch::Normal,
            })
        }))
    }
    /// Fixture catalog: `interface<TAB>Family<TAB>400,700<TAB>italic` lines.
    pub fn from_manifest(text: &str) -> Self {
        let mut faces: Vec<(String, bool, u16, bool)> = Vec::new();
        for line in text.lines().take(4096) {
            let mut parts = line.split('\t');
            let (Some(kind), Some(family)) = (parts.next(), parts.next()) else {
                continue;
            };
            let monospace = kind.trim() == "monospace";
            let weights = parts.next().unwrap_or("400");
            let italic = parts.next().is_some_and(|value| value.trim() == "italic");
            for weight in weights
                .split(',')
                .filter_map(|value| value.trim().parse().ok())
            {
                faces.push((family.to_string(), monospace, weight, false));
            }
            if italic {
                faces.push((family.to_string(), monospace, 400, true));
            }
        }
        Self::from_faces(
            faces
                .iter()
                .map(|(family, monospace, weight, italic)| Face {
                    family,
                    monospace: *monospace,
                    weight: *weight,
                    italic: *italic,
                    normal_stretch: true,
                }),
        )
    }
    pub fn families(&self, monospace: bool) -> &[FontFamily] {
        if monospace {
            &self.monospace
        } else {
            &self.interface
        }
    }
    pub fn family(&self, monospace: bool, name: &str) -> Option<&FontFamily> {
        self.families(monospace)
            .iter()
            .find(|family| family.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconTheme {
    /// Directory name; what COSMIC stores in `icon_theme`.
    pub id: String,
    /// Localized display name from `index.theme`.
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IconCatalog {
    pub themes: Vec<IconTheme>,
    /// Directories that exist but could not be listed.
    pub errors: Vec<String>,
}

/// XDG icon directories in lookup order: `$XDG_DATA_HOME/icons`,
/// `~/.icons`, then each `$XDG_DATA_DIRS/icons`.
pub fn icon_dirs(
    data_home: Option<PathBuf>,
    home: Option<PathBuf>,
    data_dirs: Option<String>,
) -> Vec<PathBuf> {
    let dirs = data_dirs
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    let mut result = Vec::new();
    for dir in data_home
        .map(|dir| dir.join("icons"))
        .into_iter()
        .chain(home.map(|home| home.join(".icons")))
        .chain(dirs.split(':').map(|dir| Path::new(dir).join("icons")))
    {
        if dir.is_absolute() && !result.contains(&dir) {
            result.push(dir);
        }
    }
    result
}

/// Language tags to try for `Name[...]`, most specific first.
pub fn languages() -> Vec<String> {
    let value = ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
        .unwrap_or_default();
    let mut tags = Vec::new();
    for item in value.split(':') {
        let tag = item.split(['.', '@']).next().unwrap_or("");
        if tag.is_empty() || tag == "C" || tag == "POSIX" {
            continue;
        }
        tags.push(tag.to_string());
        if let Some((language, _)) = tag.split_once('_') {
            tags.push(language.to_string());
        }
    }
    tags.dedup();
    tags
}

/// Parses the `[Icon Theme]` group: display name and whether the theme has
/// icon directories (cursor-only themes have none) and is visible.
fn parse_index(text: &str, languages: &[String]) -> Option<String> {
    let mut in_group = false;
    let mut name = None;
    let mut localized: Vec<(usize, String)> = Vec::new();
    let mut directories = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_group = line == "[Icon Theme]";
            continue;
        }
        if !in_group {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "Name" => name = Some(value.to_string()),
            "Hidden" if value.eq_ignore_ascii_case("true") => return None,
            "Directories" | "ScaledDirectories" if !value.is_empty() => directories = true,
            _ => {
                if let Some(tag) = key
                    .strip_prefix("Name[")
                    .and_then(|rest| rest.strip_suffix(']'))
                    && let Some(rank) = languages.iter().position(|language| language == tag)
                {
                    localized.push((rank, value.to_string()));
                }
            }
        }
    }
    if !directories {
        return None;
    }
    localized.sort();
    localized
        .into_iter()
        .map(|(_, value)| value)
        .chain(name)
        .find(|value| !value.is_empty() && !value.chars().any(char::is_control))
}

impl IconCatalog {
    pub fn discover(dirs: &[PathBuf], languages: &[String]) -> Self {
        let mut catalog = Self::default();
        let mut seen: BTreeMap<String, ()> = BTreeMap::new();
        for dir in dirs {
            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    catalog.errors.push(format!("{}: {error}", dir.display()));
                    continue;
                }
            };
            let mut found: Vec<IconTheme> = Vec::new();
            for entry in entries.flatten().take(2048) {
                let Some(id) = entry.file_name().to_str().map(String::from) else {
                    continue;
                };
                if id.starts_with('.') || seen.contains_key(&id) {
                    continue;
                }
                let path = entry.path();
                let mut text = String::new();
                let read = std::fs::File::open(path.join("index.theme"))
                    .and_then(|file| file.take(64 * 1024).read_to_string(&mut text));
                if read.is_err() {
                    continue;
                }
                if let Some(name) = parse_index(&text, languages) {
                    found.push(IconTheme { id, name, path });
                }
            }
            for theme in found {
                seen.insert(theme.id.clone(), ());
                catalog.themes.push(theme);
            }
        }
        catalog
            .themes
            .sort_by_key(|theme| (theme.name.to_lowercase(), theme.id.clone()));
        catalog
    }
    pub fn installed() -> Self {
        Self::discover(
            &icon_dirs(
                dirs::data_dir(),
                dirs::home_dir(),
                std::env::var("XDG_DATA_DIRS").ok(),
            ),
            &languages(),
        )
    }
    pub fn get(&self, id: &str) -> Option<&IconTheme> {
        self.themes.iter().find(|theme| theme.id == id)
    }
}

/// Case-insensitive substring filter shared by both pickers.
pub fn matches(query: &str, values: &[&str]) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(&query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::store::tests::Fixture;

    #[test]
    fn fonts_group_by_family_and_role_with_upright_weights() {
        let catalog = FontCatalog::from_manifest(
            "interface\tOpen Sans\t400,700,300\titalic\nmonospace\tNoto Sans Mono\t400\n\
             interface\tOpen Sans\t700\nbroken line\ninterface\t \t400\n",
        );
        assert_eq!(catalog.interface.len(), 1);
        let open = &catalog.interface[0];
        assert_eq!(open.name, "Open Sans");
        assert_eq!(open.weights, [Weight::Light, Weight::Normal, Weight::Bold]);
        assert!(open.italic);
        assert_eq!(catalog.monospace[0].name, "Noto Sans Mono");
        assert!(catalog.family(false, "open sans").is_some());
        assert!(catalog.family(true, "Open Sans").is_none());
    }

    #[test]
    fn icon_themes_follow_xdg_precedence_and_localized_names() {
        let fixture = Fixture::new("icons");
        let user = fixture.0.join("user/icons");
        let system = fixture.0.join("system/icons");
        let theme = |root: &Path, id: &str, text: &str| {
            std::fs::create_dir_all(root.join(id)).unwrap();
            std::fs::write(root.join(id).join("index.theme"), text).unwrap();
        };
        theme(
            &system,
            "Pop",
            "[Icon Theme]\nName=Pop\nName[de]=Pop (System)\nDirectories=a\n",
        );
        theme(&user, "Pop", "[Icon Theme]\nName=My Pop\nDirectories=a\n");
        theme(
            &system,
            "Cosmic",
            "[Icon Theme]\nName=COSMIC\nDirectories=scalable\n",
        );
        theme(&system, "Cursor", "[Icon Theme]\nName=Cursor only\n");
        theme(
            &system,
            "Hidden",
            "[Icon Theme]\nName=Hidden\nHidden=true\nDirectories=a\n",
        );
        theme(
            &system,
            "Other",
            "[Other]\nName=Wrong\n[Icon Theme]\nDirectories=a\n",
        );
        std::fs::create_dir_all(system.join("empty")).unwrap();
        let languages = vec!["de_DE".to_string(), "de".to_string()];
        let catalog = IconCatalog::discover(&[user.clone(), system.clone()], &languages);
        let ids: Vec<_> = catalog
            .themes
            .iter()
            .map(|theme| (theme.id.as_str(), theme.name.as_str()))
            .collect();
        // The user copy wins; cursor-only, hidden and nameless themes are skipped.
        assert_eq!(ids, [("Cosmic", "COSMIC"), ("Pop", "My Pop")]);
        let catalog = IconCatalog::discover(&[system], &languages);
        assert_eq!(catalog.get("Pop").unwrap().name, "Pop (System)");
        assert!(
            IconCatalog::discover(&[fixture.0.join("missing")], &[])
                .themes
                .is_empty()
        );
        assert_eq!(
            icon_dirs(
                Some("/h/.local/share".into()),
                Some("/h".into()),
                Some("/usr/share:relative".into())
            ),
            [
                PathBuf::from("/h/.local/share/icons"),
                PathBuf::from("/h/.icons"),
                PathBuf::from("/usr/share/icons")
            ]
        );
        assert!(matches("pop", &["My Pop", "Pop"]));
        assert!(!matches("zzz", &["My Pop"]));
    }
}
