//! Where Halcyon keeps things.
//!
//! Per-person folders are named `halcyon` (`~/.config/halcyon` and so on).
//! Releases before the rename used `cosmic-desktop-cards`; `migrate` copies
//! those over once and leaves the originals in place.
//!
//! Installed files sit under the prefix the program runs from: `/usr` for the
//! package, `~/.local` for an install from source (`<prefix>/bin/halcyon`).
//! See packaging/README.md.
use std::path::{Path, PathBuf};

/// The per-person folder name.
pub const NAME: &str = "halcyon";
/// The folder name before the rename.
pub const LEGACY: &str = "cosmic-desktop-cards";
/// The installed package name, for its library and shared folders.
pub const PACKAGE: &str = "cosmic-ext-halcyon";
/// The user service.
pub const SERVICE: &str = "halcyon.service";
/// The command-line controller (shortcuts and the edge menu use it).
pub const CONTROLLER: &str = "halcyon-ctl";

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(NAME))
}
pub fn data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(NAME))
}
pub fn state_dir() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|dir| dir.join(NAME))
}
pub fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join(NAME))
}
/// Files that only live while the session does (commands, layout state).
pub fn runtime_dir() -> Option<PathBuf> {
    dirs::runtime_dir().map(|dir| dir.join(NAME))
}

/// The installation prefix: two levels above the running program, or
/// `HALCYON_PREFIX`.
fn prefix() -> Option<PathBuf> {
    if let Some(prefix) = std::env::var_os("HALCYON_PREFIX") {
        return Some(PathBuf::from(prefix));
    }
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    Some(exe.parent()?.parent()?.to_path_buf())
}

/// The first of `candidates` that exists, else the first one.
fn first_existing(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|path| path.exists())
        .or(candidates.first())
        .cloned()
}

/// A helper program Halcyon runs (`calendar-sync.py`, `project-status.py`).
pub fn helper(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(prefix) = prefix() {
        candidates.push(prefix.join("lib").join(PACKAGE).join(name));
    }
    // An install from before the rename keeps its helpers here.
    if let Some(data) = dirs::data_dir() {
        candidates.push(data.join(LEGACY).join(name));
    }
    first_existing(candidates)
}

/// A shared, read-only file installed with Halcyon.
pub fn shared(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(prefix) = prefix() {
        candidates.push(prefix.join("share").join(PACKAGE).join(name));
    }
    if let Some(data) = dirs::data_local_dir() {
        candidates.push(data.join(LEGACY).join(name));
        candidates.push(data.join(LEGACY).join("source/docs").join(name));
    }
    first_existing(candidates)
}

/// The controller installed beside the program (`halcyon-ctl`), or the one
/// an install from before the rename put in `~/.local/bin`.
pub fn controller() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(prefix) = prefix() {
        candidates.push(prefix.join("bin").join(CONTROLLER));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/bin/desktop-cards"));
    }
    first_existing(candidates).unwrap_or_else(|| PathBuf::from(CONTROLLER))
}

/// The built-in wallpapers. Installed with Halcyon under the prefix; an
/// install from before the rename copied them to `~/Pictures/Desktop Cards`.
pub fn backgrounds() -> Option<PathBuf> {
    let installed = prefix().map(|prefix| prefix.join("share/backgrounds").join(NAME));
    if let Some(installed) = installed.as_ref().filter(|dir| dir.is_dir()) {
        return Some(installed.clone());
    }
    let legacy = dirs::picture_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Pictures")))
        .map(|pictures| pictures.join("Desktop Cards"));
    legacy.filter(|dir| dir.is_dir()).or(installed)
}

/// Copies each per-person folder from before the rename to its new place,
/// once: only when the new folder doesn't exist yet. The old folders stay,
/// so an older version still finds its settings. Both Halcyon processes may
/// start at once; a lock makes one do it while the other waits.
pub fn migrate() {
    if std::env::var_os("DESKTOP_CARDS_CONFIG").is_some() {
        return;
    }
    let pairs = [
        (dirs::config_dir(), config_dir()),
        (dirs::data_dir(), data_dir()),
        (dirs::state_dir().or_else(dirs::data_local_dir), state_dir()),
        (dirs::cache_dir(), cache_dir()),
    ];
    let Some(lock_dir) = dirs::config_dir() else {
        return;
    };
    let _lock = lock(&lock_dir.join(format!(".{NAME}-migrate.lock")));
    for (base, new) in pairs {
        let (Some(base), Some(new)) = (base, new) else {
            continue;
        };
        let old = base.join(LEGACY);
        if new.exists() || !old.is_dir() {
            continue;
        }
        let staging = base.join(format!(".{NAME}.migrating"));
        let _ = std::fs::remove_dir_all(&staging);
        if copy_tree(&old, &staging, SKIP).is_ok() {
            if std::fs::rename(&staging, &new).is_err() {
                let _ = std::fs::remove_dir_all(&staging);
            }
        } else {
            let _ = std::fs::remove_dir_all(&staging);
        }
    }
}

/// An exclusive lock held while the returned file is open.
fn lock(path: &Path) -> Option<std::fs::File> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(path)
        .ok()?;
    // SAFETY: flock on a descriptor this function owns.
    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    Some(file)
}

/// What the old data folder holds that Halcyon doesn't take along: the old
/// installer's copy of its sources, binary backups, and the Islands panel
/// bundle (which stays where an installed panel launcher expects it).
const SKIP: &[&str] = &["source", "backups", "companion"];

/// Regular files and folders with their permissions; links, sockets and
/// pipes are skipped (lock files and sockets are recreated as needed), and so
/// are top-level entries named in `skip`.
fn copy_tree(from: &Path, to: &Path, skip: &[&str]) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let mode = std::fs::symlink_metadata(from)?.permissions().mode() & 0o7777;
    std::fs::DirBuilder::new().mode(mode | 0o700).create(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if skip.iter().any(|name| entry.file_name() == *name) {
            continue;
        }
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target, &[])?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_folders_with_their_files_and_permissions_but_not_sockets() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("halcyon-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let old = root.join("old");
        std::fs::create_dir_all(old.join("nested")).unwrap();
        std::fs::write(old.join("config.json"), "{}").unwrap();
        std::fs::set_permissions(
            old.join("config.json"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        std::fs::write(old.join("nested/note.json"), "note").unwrap();
        std::os::unix::fs::symlink("/etc/passwd", old.join("link")).unwrap();
        let _socket = std::os::unix::net::UnixListener::bind(old.join("socket")).unwrap();
        std::fs::create_dir_all(old.join("source")).unwrap();
        copy_tree(&old, &root.join("new"), SKIP).unwrap();
        let new = root.join("new");
        assert_eq!(
            std::fs::read_to_string(new.join("config.json")).unwrap(),
            "{}"
        );
        assert_eq!(
            std::fs::metadata(new.join("config.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::read_to_string(new.join("nested/note.json")).unwrap(),
            "note"
        );
        assert!(!new.join("link").exists() && !new.join("socket").exists());
        assert!(!new.join("source").exists());
        assert!(old.join("config.json").exists(), "the original stays");
        let _ = std::fs::remove_dir_all(&root);
    }
}
