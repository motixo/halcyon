//! *Yours*: the app's own list of personal pictures and slideshow folders
//! (`<store state>/wallpaper-collection.json`), plus COSMIC Settings' custom
//! images, read only.
//!
//! Loading reads only (the read-only state path, never the write-checked
//! one) and creates nothing; a missing file is an empty collection, a
//! damaged or oversized one an error that is never rewritten by loading.
//! Only an explicit save writes, through the store's write and confinement
//! checks, atomically with mode 0600. A damaged original is first kept in
//! a unique sibling that never replaces an earlier one; if that fails,
//! nothing is saved. Nothing is copied, moved or deleted in the pictures.
use crate::desktop::store::{ConfigKey, DesktopStore};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const FILE: &str = "wallpaper-collection.json";
pub const UNREADABLE: &str = "Your saved collection couldn’t be read";
const FORMAT: &str = "desktop-cards-wallpapers";
const VERSION: u32 = 1;
const MAX_ITEMS: usize = 200;
const MAX_BYTES: u64 = 256 * 1024;
const MAX_PATH_BYTES: usize = 4096;
/// Damage backups tried before giving up.
const MAX_BACKUPS: usize = 64;

static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Image,
    Folder,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub kind: Kind,
    pub path: PathBuf,
}

impl Item {
    /// A plain absolute UTF-8 path without control characters.
    fn valid(&self) -> bool {
        self.path.is_absolute()
            && self.path.to_str().is_some_and(|text| {
                text.len() <= MAX_PATH_BYTES && !text.chars().any(char::is_control)
            })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    version: u32,
    items: Vec<Item>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Collection {
    items: Vec<Item>,
}

fn read(path: &Path) -> Result<Vec<Item>, String> {
    let unreadable = || UNREADABLE.to_string();
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(unreadable()),
        Ok(metadata) if !metadata.is_file() || metadata.len() > MAX_BYTES => {
            return Err(unreadable());
        }
        Ok(_) => {}
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(MAX_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|_| unreadable())?;
    let stored: Stored = serde_json::from_slice(&bytes).map_err(|_| unreadable())?;
    if bytes.len() as u64 > MAX_BYTES
        || stored.format != FORMAT
        || stored.version != VERSION
        || stored.items.len() > MAX_ITEMS
        || !stored.items.iter().all(Item::valid)
    {
        return Err(unreadable());
    }
    Ok(stored.items)
}

impl Collection {
    /// Reads `<state>/wallpaper-collection.json`; creates nothing.
    pub fn load(store: &DesktopStore) -> Result<Self, String> {
        read(&store.state_dir().join(FILE)).map(|items| Self { items })
    }
    /// Adds a valid item not listed yet, up to 200.
    pub fn add(&mut self, item: Item) -> bool {
        if !item.valid()
            || self.items.len() >= MAX_ITEMS
            || self.items.iter().any(|known| known.path == item.path)
        {
            return false;
        }
        self.items.push(item);
        true
    }
    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.path != path);
        self.items.len() != before
    }
    pub fn items(&self) -> &[Item] {
        &self.items
    }
    pub fn contains(&self, path: &Path) -> bool {
        self.items.iter().any(|item| item.path == path)
    }
    /// Writes the collection (write access and confinement checked first).
    pub fn save(&self, store: &DesktopStore) -> Result<(), String> {
        let path = store.state_file(FILE).map_err(|error| error.to_string())?;
        if std::fs::symlink_metadata(&path).is_ok() && read(&path).is_err() {
            keep_damaged(store, &path)?;
        }
        let bytes = serde_json::to_vec_pretty(&Stored {
            format: FORMAT.into(),
            version: VERSION,
            items: self.items.clone(),
        })
        .map_err(|error| error.to_string())?;
        write_private(&path, &bytes)
            .map_err(|error| format!("Couldn’t save your collection: {error}"))
    }
}

/// Keeps the damaged original as a new hard link next to it; an existing
/// backup is never replaced.
fn keep_damaged(store: &DesktopStore, path: &Path) -> Result<(), String> {
    for index in 1..=MAX_BACKUPS {
        let name = match index {
            1 => format!("{FILE}.damaged"),
            index => format!("{FILE}.damaged-{index}"),
        };
        let backup = store.state_file(&name).map_err(|error| error.to_string())?;
        match std::fs::hard_link(path, &backup) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Couldn’t keep a copy of the damaged collection ({error}); nothing was saved"
                ));
            }
        }
    }
    Err("Too many damaged collection copies; nothing was saved".into())
}

/// Same-directory temporary file (0600) + fsync + rename.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("no parent directory"))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{FILE}.{}.{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        std::fs::File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// COSMIC Settings' own custom images
/// (`com.system76.CosmicSettings.Wallpaper/v1/custom-images`), read only;
/// absolute paths, at most 200.
pub fn cosmic_settings_images(store: &DesktopStore) -> Vec<PathBuf> {
    let Ok(key) = ConfigKey::new("com.system76.CosmicSettings.Wallpaper", 1, "custom-images")
    else {
        return Vec::new();
    };
    store
        .load(std::slice::from_ref(&key))
        .ok()
        .and_then(|snapshot| snapshot.effective(&key)?.decode::<Vec<PathBuf>>().ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|path| path.is_absolute() && path.to_str().is_some())
        .take(MAX_ITEMS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::store::tests::Fixture;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn item(kind: Kind, path: &str) -> Item {
        Item {
            kind,
            path: PathBuf::from(path),
        }
    }
    fn changed(
        before: &std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>,
        after: &std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>,
    ) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = before
            .keys()
            .chain(after.keys())
            .filter(|path| before.get(*path) != after.get(*path))
            .cloned()
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }

    #[test]
    fn collection_writes_only_its_file_and_survives_damage() {
        let fixture = Fixture::new("collection");
        let store = fixture.store();
        fs::create_dir_all(store.state_dir()).unwrap();
        fixture.user(crate::desktop::wallpaper::NAMESPACE, "all", "kept");
        let file = store.state_dir().join(FILE);
        // A missing file is an empty collection, and loading creates nothing.
        let before = fixture.tree();
        let mut collection = Collection::load(&store).unwrap();
        assert!(collection.items().is_empty());
        assert_eq!(fixture.tree(), before);

        assert!(collection.add(item(Kind::Image, "/pictures/a.png")));
        assert!(collection.add(item(Kind::Folder, "/pictures/slides")));
        assert!(
            !collection.add(item(Kind::Image, "/pictures/a.png")),
            "no duplicates"
        );
        assert!(
            !collection.add(item(Kind::Image, "relative.png")),
            "absolute only"
        );
        collection.save(&store).unwrap();
        assert_eq!(changed(&before, &fixture.tree()), vec![file.clone()]);
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let loaded = Collection::load(&store).unwrap();
        assert_eq!(loaded, collection);
        let before = fixture.tree();
        let mut collection = loaded;
        assert!(collection.remove(Path::new("/pictures/a.png")));
        collection.save(&store).unwrap();
        assert_eq!(changed(&before, &fixture.tree()), vec![file.clone()]);
        assert_eq!(Collection::load(&store).unwrap().items().len(), 1);

        // Damaged: loading reports it and never rewrites it.
        fs::write(&file, b"{ not json").unwrap();
        let before = fixture.tree();
        assert_eq!(Collection::load(&store), Err(UNREADABLE.to_string()));
        assert_eq!(fixture.tree(), before);
        // Oversized counts as damaged too.
        fs::write(&file, vec![b' '; 300 * 1024]).unwrap();
        assert!(Collection::load(&store).is_err());
        fs::write(&file, b"{ not json").unwrap();

        // An explicit save keeps the damaged original in a unique sibling.
        let mut fresh = Collection::default();
        fresh.add(item(Kind::Image, "/pictures/b.png"));
        fresh.save(&store).unwrap();
        let first = store.state_dir().join(format!("{FILE}.damaged"));
        assert_eq!(fs::read(&first).unwrap(), b"{ not json");
        assert_eq!(Collection::load(&store).unwrap(), fresh);
        // A second damage never replaces the earlier backup.
        fs::write(&file, b"second damage").unwrap();
        fresh.save(&store).unwrap();
        assert_eq!(fs::read(&first).unwrap(), b"{ not json");
        assert_eq!(
            fs::read(store.state_dir().join(format!("{FILE}.damaged-2"))).unwrap(),
            b"second damage"
        );

        // If the original can't be kept, nothing is saved.
        fs::write(&file, b"third damage").unwrap();
        let state = store.state_dir().to_path_buf();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o500)).unwrap();
        let result = fresh.save(&store);
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read(&file).unwrap(), b"third damage");

        // A read-only (preview) store refuses to save and writes nothing.
        let before = fixture.tree();
        let preview = fixture.store().read_only("Preview");
        assert!(fresh.save(&preview).is_err());
        assert_eq!(fixture.tree(), before);
    }

    #[test]
    fn cosmic_settings_images_are_read_only() {
        let fixture = Fixture::new("collection-cosmic-settings");
        let store = fixture.store();
        assert!(cosmic_settings_images(&store).is_empty());
        fixture.user(
            "com.system76.CosmicSettings.Wallpaper",
            "custom-images",
            "[\"/pictures/x.jpg\", \"relative.jpg\"]",
        );
        let before = fixture.tree();
        assert_eq!(
            cosmic_settings_images(&store),
            vec![PathBuf::from("/pictures/x.jpg")]
        );
        assert_eq!(fixture.tree(), before);
    }
}
