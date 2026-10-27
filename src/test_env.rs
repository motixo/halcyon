//! Environment boundary for re-executed tests. Never mutates this process's env.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const ROOT: &str = "HALCYON_TEST_ROOT";
// Inject a second writer after a durable preference commit, before its caller
// resumes. Thread-local and one-shot; never compiled into the application.
thread_local! {
    pub static AFTER_PREFERENCE_COMMIT: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}
const DIRECTORIES: [(&str, &str); 8] = [
    ("HOME", "home"),
    ("XDG_CONFIG_HOME", "home/.config"),
    ("XDG_CONFIG_DIRS", "system-config"),
    ("XDG_DATA_HOME", "home/.local/share"),
    ("XDG_DATA_DIRS", "data"),
    ("XDG_STATE_HOME", "home/.local/state"),
    ("XDG_CACHE_HOME", "home/.cache"),
    ("XDG_RUNTIME_DIR", "runtime"),
];

pub fn configure(command: &mut Command, root: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let root = root.canonicalize().expect("fixture root exists");
    command.env(ROOT, &root);
    for (key, relative) in DIRECTORIES {
        let path = root.join(relative);
        std::fs::create_dir_all(&path).unwrap();
        command.env(key, path);
    }
    std::fs::set_permissions(root.join("runtime"), std::fs::Permissions::from_mode(0o700)).unwrap();
    // Explicitly unavailable fixture sockets prevent both inherited session-bus
    // access and D-Bus autolaunch. Tests with a private bus may override these.
    for (key, name) in [
        ("DBUS_SESSION_BUS_ADDRESS", "session-bus"),
        ("DBUS_SYSTEM_BUS_ADDRESS", "system-bus"),
    ] {
        command.env(
            key,
            format!("unix:path={}", root.join("runtime").join(name).display()),
        );
    }
    for key in [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "WAYLAND_SOCKET",
        "XAUTHORITY",
        "DBUS_STARTER_ADDRESS",
        "DBUS_STARTER_BUS_TYPE",
        "DESKTOP_CARDS_CONFIG",
        "DESKTOP_CARDS_DESKTOP_ROOT",
        "DESKTOP_CARDS_CAPTURE_DIR",
        "DESKTOP_CARDS_SETTINGS_CAPTURE_DIR",
        "DESKTOP_CARDS_SETTINGS_SIZE",
    ] {
        command.env_remove(key);
    }
}

pub fn assert_isolated() -> PathBuf {
    let root = PathBuf::from(std::env::var_os(ROOT).expect("isolated child root"));
    assert!(root.is_absolute());
    for (key, _) in DIRECTORIES {
        let path = PathBuf::from(std::env::var_os(key).expect(key));
        assert!(
            path.canonicalize().unwrap().starts_with(&root),
            "{key} escapes fixture"
        );
    }
    for key in ["DBUS_SESSION_BUS_ADDRESS", "DBUS_SYSTEM_BUS_ADDRESS"] {
        let address = std::env::var(key).unwrap();
        let path = Path::new(address.strip_prefix("unix:path=").expect("fixture bus"));
        assert!(path.starts_with(&root), "{key} escapes fixture");
    }
    for key in ["DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"] {
        assert!(std::env::var_os(key).is_none(), "inherited {key}");
    }
    for path in [
        crate::config::Config::path(),
        crate::paths::state_dir(),
        crate::paths::cache_dir(),
        crate::paths::runtime_dir(),
    ] {
        assert!(
            path.unwrap().starts_with(&root),
            "application path escapes fixture"
        );
    }
    root
}

/// Select production worker dispatch independently of any provider stub.
pub fn native_worker(root: &Path, enabled: bool) {
    assert_eq!(
        std::env::var_os("DESKTOP_CARDS_DESKTOP_ROOT")
            .map(PathBuf::from)
            .as_deref(),
        Some(root),
        "worker mode belongs to the active isolated desktop"
    );
    let path = root.join("native-worker-enabled");
    if enabled {
        std::fs::write(path, b"1").unwrap();
    } else if let Err(error) = std::fs::remove_file(path) {
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}

#[test]
fn worker_mode_is_independent_of_probe_files() {
    if crate::displays::test_support::isolated(
        "test_env::worker_mode_is_independent_of_probe_files",
    ) {
        return;
    }
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    let root = store.fixture_root().unwrap();
    use crate::desktop::islands::Installation;
    assert!(!Installation::needs_worker(&store));
    std::fs::write(root.join("status-islands-probe.py"), "print('{}')\n").unwrap();
    assert!(
        !Installation::needs_worker(&store),
        "a probe does not select dispatch"
    );
    std::fs::remove_file(root.join("status-islands-probe.py")).unwrap();
    native_worker(root, true);
    assert!(
        Installation::needs_worker(&store),
        "worker needs no provider script"
    );
    native_worker(root, false);
    assert!(!Installation::needs_worker(&store));
}

#[test]
fn isolated_child_covers_paths_buses_and_cleanup() {
    if crate::displays::test_support::isolated(
        "test_env::isolated_child_covers_paths_buses_and_cleanup",
    ) {
        return;
    }
    let root = assert_isolated();
    for (key, _) in DIRECTORIES {
        let directory = PathBuf::from(std::env::var_os(key).unwrap());
        std::fs::write(directory.join("containment-marker"), key).unwrap();
        assert!(
            directory
                .join("containment-marker")
                .canonicalize()
                .unwrap()
                .starts_with(&root)
        );
    }
}
