//! One installation contract. IPC proves a running host, never installation.
use super::*;
use serde::Deserialize;
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Installation {
    #[default]
    Pending,
    Absent,
    Installed {
        provenance: Provenance,
        enabled: bool,
        compatible: Option<bool>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Package,
    Manual,
}
#[derive(Deserialize)]
struct Status {
    format: String,
    version: u32,
    installation: String,
    provenance: Option<Provenance>,
    enabled: bool,
    compatible: Option<bool>,
}
impl Installation {
    pub fn installed(&self) -> bool {
        matches!(self, Self::Installed { .. })
    }
    fn parse(bytes: &[u8]) -> Self {
        if bytes.len() > 65536 {
            return Self::Pending;
        }
        let Ok(status) = serde_json::from_slice::<Status>(bytes) else {
            return Self::Pending;
        };
        if status.format != "halcyon-islands-installation" || status.version != 1 {
            return Self::Pending;
        }
        match (status.installation.as_str(), status.provenance) {
            ("absent", None) => Self::Absent,
            ("installed", Some(provenance)) => Self::Installed {
                provenance,
                enabled: status.enabled,
                compatible: status.compatible,
            },
            _ => Self::Pending,
        }
    }
    pub fn discover(store: &DesktopStore) -> Self {
        if let Some(root) = store.fixture_root() {
            return match crate::desktop::recovery::read_path(
                &root.join("islands-installation.json"),
                65536,
            ) {
                Ok(Some((bytes, _))) => Self::parse(&bytes),
                Ok(None) => Self::Absent,
                Err(_) => Self::Pending,
            };
        }
        if std::env::var_os("DESKTOP_CARDS_CONFIG").is_some() {
            return Self::Pending;
        }
        let helper = std::path::Path::new("/usr/lib/cosmic-ext-halcyon/islands-status.py");
        if !helper.is_file() {
            return Self::Absent;
        }
        let Ok(mut child) = std::process::Command::new("/usr/bin/python3")
            .args(["-B", helper.to_str().unwrap()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            return Self::Pending;
        };
        let started = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if started.elapsed() < Duration::from_secs(6) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Self::Pending;
                }
            }
        }
        match child.wait_with_output() {
            Ok(output) if output.status.success() => Self::parse(&output.stdout),
            _ => Self::Pending,
        }
    }
}
#[cfg(test)]
pub(crate) fn fixture_installed(root: &std::path::Path) {
    std::fs::write(root.join("islands-installation.json"), br#"{"format":"halcyon-islands-installation","version":1,"installation":"installed","provenance":"manual","enabled":false,"compatible":false}"#).unwrap();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task2_installed_disabled_incompatible_and_unknown_are_distinct() {
        let f = crate::desktop::store::tests::Fixture::new("installed-contract");
        assert_eq!(Installation::discover(&f.store()), Installation::Absent);
        fixture_installed(&f.0);
        assert!(matches!(
            Installation::discover(&f.store()),
            Installation::Installed {
                enabled: false,
                compatible: Some(false),
                ..
            }
        ));
        std::fs::write(f.0.join("islands-installation.json"), b"{broken").unwrap();
        assert_eq!(Installation::discover(&f.store()), Installation::Pending);
        std::fs::remove_file(f.0.join("islands-installation.json")).unwrap();
        f.user(NAMESPACE, "Dock", "(enabled:true)");
        assert_eq!(Installation::discover(&f.store()), Installation::Absent);
    }
}
