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
        status: Option<CompatibilityStatus>,
        diagnostics: String,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Package,
    Manual,
}
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub enum CompatibilityStatus {
    #[serde(rename = "Ready")]
    Ready,
    #[serde(rename = "Log out to activate")]
    LogoutRequired,
    #[serde(rename = "Using standard panel")]
    Standard,
    #[serde(rename = "Compatibility update needed")]
    UpdateNeeded,
}
impl CompatibilityStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::LogoutRequired => "Log out to activate",
            Self::Standard => "Using standard panel",
            Self::UpdateNeeded => "Compatibility update needed",
        }
    }
}
#[derive(Deserialize)]
struct Status {
    format: String,
    version: u32,
    installation: String,
    provenance: Option<Provenance>,
    enabled: bool,
    compatible: Option<bool>,
    #[serde(default)]
    status_version: Option<u32>,
    #[serde(default)]
    status: Option<CompatibilityStatus>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    installed_generation: Option<String>,
    #[serde(default)]
    active_generation: Option<String>,
    #[serde(default)]
    package_versions: std::collections::BTreeMap<String, String>,
}
thread_local! {
    static NATIVE_VIEW: std::cell::RefCell<Option<Installation>> = const { std::cell::RefCell::new(None) };
}
pub struct NativeView(Option<Installation>);
impl Drop for NativeView {
    fn drop(&mut self) {
        NATIVE_VIEW.with(|v| {
            v.replace(self.0.take());
        });
    }
}
impl Installation {
    /// Native read-only planning consumes the last background result. A write
    /// must run outside this context and re-discover under transaction locks.
    pub fn native_view(state: Self) -> NativeView {
        NativeView(NATIVE_VIEW.with(|v| v.replace(Some(state))))
    }
    pub fn on_native_thread() -> bool {
        NATIVE_VIEW.with(|v| v.borrow().is_some())
    }
    pub fn needs_worker(store: &DesktopStore) -> bool {
        match store.fixture_root() {
            None => true,
            Some(_root) => {
                #[cfg(test)]
                {
                    return _root.join("slow-islands-probe.py").is_file();
                }
                #[cfg(not(test))]
                {
                    false
                }
            }
        }
    }
    pub(crate) fn for_write(store: &DesktopStore) -> Self {
        if Self::on_native_thread() && Self::needs_worker(store) {
            // Fail closed if a future UI path bypasses the background executor.
            Self::Pending
        } else {
            Self::discover(store)
        }
    }
    pub fn installed(&self) -> bool {
        matches!(self, Self::Installed { .. })
    }
    pub fn status_label(&self) -> &'static str {
        match self {
            Self::Installed {
                status: Some(status),
                ..
            } => status.label(),
            _ => "Using standard panel",
        }
    }
    pub fn diagnostics(&self) -> Option<&str> {
        match self {
            Self::Installed { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
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
        if status.status_version.is_some_and(|v| v != 1)
            || (status.status.is_some() && status.status_version != Some(1))
        {
            return Self::Pending;
        }
        let safe_hash = |s: &Option<String>| {
            s.as_ref()
                .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
                .cloned()
        };
        let reason = status
            .reason
            .as_ref()
            .filter(|v| v.len() <= 80 && v.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'));
        let packages: std::collections::BTreeMap<_, _> = status
            .package_versions
            .iter()
            .filter(|(name, value)| {
                matches!(
                    name.as_str(),
                    "cosmic-panel" | "cosmic-comp" | "cosmic-session" | "cosmic-settings-daemon"
                ) && value.len() <= 128
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".+:~_-".contains(&b))
            })
            .collect();
        let diagnostics = serde_json::json!({
            "version": 1, "enabled": status.enabled, "compatible": status.compatible,
            "status": status.status.map(CompatibilityStatus::label), "reason": reason,
            "installed_generation": safe_hash(&status.installed_generation),
            "active_generation": safe_hash(&status.active_generation), "package_versions": packages,
        })
        .to_string();
        match (status.installation.as_str(), status.provenance) {
            ("absent", None) => Self::Absent,
            ("installed", Some(provenance)) => Self::Installed {
                provenance,
                enabled: status.enabled,
                compatible: status.compatible,
                status: status.status,
                diagnostics,
            },
            _ => Self::Pending,
        }
    }
    pub fn discover(store: &DesktopStore) -> Self {
        if Self::needs_worker(store) {
            if let Some(state) = NATIVE_VIEW.with(|v| v.borrow().clone()) {
                return state;
            }
        }
        if let Some(root) = store.fixture_root() {
            #[cfg(test)]
            if root.join("slow-islands-probe.py").is_file() {
                let output = std::process::Command::new("/usr/bin/python3")
                    .arg("-B")
                    .arg(root.join("slow-islands-probe.py"))
                    .output()
                    .unwrap();
                return Self::parse(&output.stdout);
            }
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
    fn task3_status_is_versioned_and_copy_diagnostics_excludes_untrusted_paths() {
        let data = br#"{"format":"halcyon-islands-installation","version":1,"installation":"installed","provenance":"package","enabled":true,"compatible":true,"status_version":1,"status":"Log out to activate","reason":"installed_running_differ","installed_generation":"/home/private/token","package_versions":{"cosmic-panel":"1.11.0~24.04~ca20ce3","private":"/home/private"}}"#;
        let status = Installation::parse(data);
        assert_eq!(status.status_label(), "Log out to activate");
        let diagnostic = status.diagnostics().unwrap();
        assert!(!diagnostic.contains("/home"));
        assert!(!diagnostic.contains("private"));
        assert!(diagnostic.contains("ca20ce3"));
        let unknown = String::from_utf8(data.to_vec())
            .unwrap()
            .replace("status_version\":1", "status_version\":2");
        assert_eq!(
            Installation::parse(unknown.as_bytes()),
            Installation::Pending
        );
    }
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
