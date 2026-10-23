//! App animations: the one motion preference this COSMIC build passes on to
//! applications.
//!
//! cosmic-comp 4d14528 has no animation settings (its durations are
//! constants), libcosmic dc11d6a has none, and xdg-desktop-portal 1.18 /
//! xdg-desktop-portal-cosmic 9d35e63 publish no `reduced-motion` key. What
//! is honoured is GNOME's `org.gnome.desktop.interface enable-animations`:
//! GTK 3/4 maps it to `gtk-enable-animations` (directly, or through the
//! Settings portal for Flatpak apps), and Firefox and Chromium derive
//! `prefers-reduced-motion` from that GTK setting. It lives in dconf, so it
//! is read and written with `gsettings` in the session's own environment
//! (COSMIC's `DCONF_PROFILE`), and read back after writing.
//!
//! A fixture store keeps it in `<root>/gsettings/<schema>/<key>` ("true" or
//! "false"; the schema directory missing means not installed) and never
//! runs `gsettings`. A read-only store reads but never writes.
use super::DesktopStore;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const SCHEMA: &str = "org.gnome.desktop.interface";
pub const KEY: &str = "enable-animations";
/// The installed schema's default.
pub const DEFAULT: bool = true;
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq)]
enum Backend {
    /// `gsettings` (a program path, normally `/usr/bin/gsettings`).
    Live(PathBuf),
    /// A fixture root's `gsettings` folder.
    Fixture(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppAnimations {
    backend: Backend,
    /// Why writing is off (preview mode), if it is.
    read_only: Option<String>,
    capture: Option<(PathBuf, PathBuf, super::store::Access, Option<PathBuf>)>,
}

impl AppAnimations {
    /// Follows the store: its fixture root (or, for a read-only copy of a
    /// fixture, the root its user tree lives in), else the live session.
    pub fn for_store(store: &DesktopStore) -> Self {
        let read_only = (!store.writable()).then(|| {
            store
                .preflight(&super::DesktopPatch::new())
                .err()
                .map_or_else(|| "Changes are off".to_string(), |error| error.to_string())
        });
        let live_root = dirs::config_dir().map(|dir| dir.join("cosmic"));
        let backend = match store.fixture_root() {
            Some(root) => Backend::Fixture(root.join("gsettings")),
            None if live_root.as_deref() == Some(store.user_root()) => Backend::Live(program()),
            None => Backend::Fixture(
                store
                    .user_root()
                    .ancestors()
                    .nth(2)
                    .unwrap_or(Path::new("/nonexistent"))
                    .join("gsettings"),
            ),
        };
        Self {
            backend,
            read_only,
            capture: Some((
                store.user_root().into(),
                store.state_dir().into(),
                store.access().clone(),
                store.app_config().map(Path::to_path_buf),
            )),
        }
    }
    #[cfg(test)]
    fn with_program(program: PathBuf) -> Self {
        Self {
            backend: Backend::Live(program),
            read_only: None,
            capture: None,
        }
    }
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn writable(&self) -> bool {
        self.read_only.is_none()
    }
    pub fn read_only_reason(&self) -> Option<&str> {
        self.read_only.as_deref()
    }

    /// Whether apps are asked to animate. `Err`: the setting can't be used
    /// here (with a short reason).
    pub fn read(&self) -> Result<bool, String> {
        match &self.backend {
            Backend::Fixture(dir) => {
                let schema = dir.join(SCHEMA);
                if !schema.is_dir() {
                    return Err(NOT_INSTALLED.into());
                }
                match super::recovery::read_motion_fixture(&schema.join(KEY))? {
                    Some(bytes) => parse(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?),
                    None => Ok(DEFAULT),
                }
            }
            Backend::Live(program) => {
                let output = run(program, &["get", SCHEMA, KEY])?;
                parse(&output)
            }
        }
    }

    /// Sets it and reads it back, so the result is what apps now see.
    pub fn write(&self, on: bool) -> Result<bool, String> {
        if let Some(reason) = &self.read_only {
            return Err(reason.clone());
        }
        // Refuse unsupported backends before creating any recovery evidence.
        self.read()?;
        if let Some((root, state, access, config)) = &self.capture {
            if matches!(self.backend, Backend::Live(_))
                && std::env::var("GSETTINGS_BACKEND").is_ok_and(|backend| backend != "dconf")
            {
                return Err(
                    "This GSettings backend cannot preserve the original animation setting".into(),
                );
            }
            let mut store =
                DesktopStore::with_roots(root.clone(), vec![], state.clone(), access.clone());
            if let Some(config) = config {
                store = store.with_app_config(config.clone());
            }
            let backend = match self.backend {
                Backend::Fixture(_) => "fixture",
                Backend::Live(_) => "dconf",
            };
            return store.mutate_motion(backend, || self.original(), || self.write_backend(on));
        }
        self.write_backend(on)
    }
    fn original(&self) -> Result<Option<Vec<u8>>, String> {
        match &self.backend {
            Backend::Fixture(dir) => {
                super::recovery::read_motion_fixture(&dir.join(SCHEMA).join(KEY))
            }
            Backend::Live(_) => {
                let text = run(
                    Path::new("/usr/bin/dconf"),
                    &["read", "/org/gnome/desktop/interface/enable-animations"],
                )?;
                if text.trim().is_empty() {
                    Ok(None)
                } else {
                    parse(&text)?;
                    Ok(Some(text.into_bytes()))
                }
            }
        }
    }
    fn write_backend(&self, on: bool) -> Result<bool, String> {
        let value = if on { "true" } else { "false" };
        match &self.backend {
            Backend::Fixture(dir) => {
                let schema = dir.join(SCHEMA);
                if !schema.is_dir() {
                    return Err(NOT_INSTALLED.into());
                }
                super::recovery::write_motion_fixture(&schema.join(KEY), value.as_bytes())?;
            }
            Backend::Live(program) => {
                run(program, &["set", SCHEMA, KEY, value])?;
            }
        }
        match self.read()? {
            now if now == on => Ok(now),
            _ => Err("The change didn’t take effect; another setting overrides it.".into()),
        }
    }
}

const NOT_INSTALLED: &str = "GTK’s interface settings aren’t installed.";

fn program() -> PathBuf {
    let usual = PathBuf::from("/usr/bin/gsettings");
    if usual.is_file() {
        usual
    } else {
        PathBuf::from("gsettings")
    }
}

fn parse(text: &str) -> Result<bool, String> {
    match text.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("Unexpected value “{other}”")),
    }
}

/// Runs `gsettings`, at most `TIMEOUT`; stdout on success.
fn run(program: &Path, args: &[&str]) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => "gsettings isn’t installed.".to_string(),
            _ => error.to_string(),
        })?;
    let started = std::time::Instant::now();
    loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(_) => break,
            None if started.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("gsettings didn’t answer.".into());
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let error = String::from_utf8_lossy(&output.stderr);
    if error.contains("No such schema") || error.contains("No such key") {
        Err(NOT_INSTALLED.into())
    } else if error.trim().is_empty() {
        Err(format!("gsettings failed ({})", output.status))
    } else {
        Err(error.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::store::tests::Fixture;
    use std::os::unix::fs::PermissionsExt;

    fn installed(fixture: &Fixture) -> PathBuf {
        let dir = fixture.0.join("gsettings").join(SCHEMA);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_fixture_reads_the_default_writes_and_reads_back() {
        let fixture = Fixture::new("motion-fixture");
        let animations = AppAnimations::for_store(&fixture.store());
        assert_eq!(animations.read(), Err(NOT_INSTALLED.into()));
        assert_eq!(animations.write(false), Err(NOT_INSTALLED.into()));
        let dir = installed(&fixture);
        assert_eq!(animations.read(), Ok(true), "absent key: the default");
        assert!(!dir.join(KEY).exists(), "reading wrote nothing");
        assert_eq!(animations.write(false), Ok(false));
        assert_eq!(std::fs::read_to_string(dir.join(KEY)).unwrap(), "false");
        assert_eq!(animations.read(), Ok(false));
        assert_eq!(animations.write(true), Ok(true));
        std::fs::write(dir.join(KEY), "maybe").unwrap();
        assert!(animations.read().unwrap_err().contains("maybe"));
    }

    #[test]
    fn a_read_only_store_reads_its_own_fixture_and_never_writes() {
        let fixture = Fixture::new("motion-read-only");
        let dir = installed(&fixture);
        std::fs::write(dir.join(KEY), "false").unwrap();
        let animations = AppAnimations::for_store(&fixture.store().read_only("Preview"));
        assert_eq!(
            animations.backend,
            Backend::Fixture(fixture.0.join("gsettings"))
        );
        assert!(!animations.writable());
        assert_eq!(animations.read(), Ok(false));
        assert_eq!(animations.write(true), Err("Preview".into()));
        assert_eq!(std::fs::read_to_string(dir.join(KEY)).unwrap(), "false");
    }

    /// The live path, against a stand-in `gsettings` that keeps its value
    /// in a file (no real dconf is touched).
    #[test]
    fn the_live_path_runs_gsettings_get_and_set() {
        let fixture = Fixture::new("motion-live");
        let script = fixture.0.join("gsettings");
        let value = fixture.0.join("value");
        let log = fixture.0.join("log");
        std::fs::write(&value, "true\n").unwrap();
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho \"$@\" >> {log}\n\
                 [ \"$2\" = {SCHEMA} ] && [ \"$3\" = {KEY} ] || {{ echo 'No such schema' >&2; exit 1; }}\n\
                 case $1 in get) cat {value};; set) echo \"$4\" > {value};; esac\n",
                log = log.display(),
                value = value.display(),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let animations = AppAnimations::with_program(script);
        assert_eq!(animations.read(), Ok(true));
        assert_eq!(animations.write(false), Ok(false));
        assert_eq!(std::fs::read_to_string(&value).unwrap(), "false\n");
        assert_eq!(
            std::fs::read_to_string(&log).unwrap(),
            format!(
                "get {SCHEMA} {KEY}\nget {SCHEMA} {KEY}\nset {SCHEMA} {KEY} false\nget {SCHEMA} {KEY}\n"
            )
        );
        // Missing program or schema: unavailable, with a short reason.
        let missing = AppAnimations::with_program(fixture.0.join("absent"));
        assert_eq!(missing.read(), Err("gsettings isn’t installed.".into()));
        let mut other = animations.clone();
        other.backend = Backend::Live(PathBuf::from("/bin/false"));
        assert!(other.read().unwrap_err().starts_with("gsettings failed"));
    }

    #[test]
    fn a_write_that_does_not_stick_is_reported() {
        let fixture = Fixture::new("motion-override");
        let script = fixture.0.join("gsettings");
        // Accepts `set` but always reads `true` (a higher dconf layer).
        std::fs::write(
            &script,
            "#!/bin/sh\n[ \"$1\" = get ] && echo true\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let animations = AppAnimations::with_program(script);
        assert!(animations.write(false).unwrap_err().contains("overrides"));
    }
}
