//! Local library copies never serve as mutable references from the desktop.
use super::*;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};
const MAX_LIBRARY_BYTES: usize = MAX_LIBRARY * MAX_BYTES;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub version: u32,
    pub entries: BTreeMap<String, Recipe>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            entries: BTreeMap::new(),
        }
    }
}
pub fn path() -> Result<PathBuf, String> {
    let store = crate::desktop::actions::history_store().map_err(|e| e.to_string())?;
    store
        .state_file("widget-recipes.json")
        .map_err(|e| e.to_string())
}
pub fn load(path: &Path) -> Result<Library, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Library::default()),
        Err(e) => return Err(e.to_string()),
    };
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Recipe library must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_LIBRARY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_LIBRARY_BYTES {
        return Err("Recipe library is too large".into());
    }
    let library: Library = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if library.version != 1 || library.entries.len() > MAX_LIBRARY {
        return Err("Unsupported recipe library".into());
    }
    for (id, r) in &library.entries {
        r.validate()?;
        if id != &r.id {
            return Err("Recipe library identity mismatch".into());
        }
    }
    Ok(library)
}
/// Expected snapshot fences another Settings window or external library edit.
pub fn save(path: &Path, recipe: &Recipe, expected: Option<&Recipe>) -> Result<Recipe, String> {
    recipe.validate()?;
    let _lock = crate::config::lock_config(path)?;
    let mut library = load(path)?;
    if expected.is_some_and(|old| old.kind != recipe.kind) {
        return Err("A saved widget keeps its original kind; create a new widget instead".into());
    }
    if library.entries.get(&recipe.id) != expected {
        return Err("This saved widget changed. Reload it before saving.".into());
    }
    if expected.is_none() && library.entries.len() >= MAX_LIBRARY {
        return Err("The widget library is full (32 saved widgets)".into());
    }
    let recipe = recipe.clone();
    library.entries.insert(recipe.id.clone(), recipe.clone());
    write(path, &library)?;
    Ok(recipe)
}
pub fn delete(path: &Path, expected: &Recipe) -> Result<(), String> {
    let _lock = crate::config::lock_config(path)?;
    let mut library = load(path)?;
    if library.entries.get(&expected.id) != Some(expected) {
        return Err("This saved widget changed. Reload it before deleting.".into());
    }
    library.entries.remove(&expected.id);
    write(path, &library)
}
fn write(path: &Path, library: &Library) -> Result<(), String> {
    let text = serde_json::to_string_pretty(library).map_err(|e| e.to_string())?;
    if text.len() > MAX_LIBRARY_BYTES {
        return Err("Recipe library is too large".into());
    }
    crate::config::write_config_text(path, Some(&text))?;
    std::fs::File::open(path.parent().ok_or("No library directory")?)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())
}
