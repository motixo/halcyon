//! Sidecar access uses the same journal and conflict checks as native keys.
use super::panels::PanelKind;
use super::store::{ConfigKey, DesktopStore, KeyRule, Namespace, RawValue};
use dock_islands_core::{Capabilities, MAX_WIRE_BYTES, NAMESPACE, Request, Response, Settings};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::{
    fs::{FileTypeExt, MetadataExt},
    net::UnixStream,
};
use std::path::PathBuf;
use std::time::Duration;

pub fn key(kind: PanelKind) -> ConfigKey {
    ConfigKey::new(NAMESPACE, 1, kind.entry()).expect("fixed island key")
}
fn validate(raw: &RawValue) -> Result<(), String> {
    if raw.as_str().len() > MAX_WIRE_BYTES {
        return Err("Island settings are too large".into());
    }
    raw.decode::<Settings>()?.validate()
}
pub static RULES: [KeyRule; 2] = [
    KeyRule {
        namespace: Namespace::Exact(NAMESPACE),
        version: 1,
        key: "Dock",
        validate,
    },
    KeyRule {
        namespace: Namespace::Exact(NAMESPACE),
        version: 1,
        key: "Panel",
        validate,
    },
];

#[derive(Debug, PartialEq)]
enum InactiveReason {
    NotInstalled,
    Disabled,
    CosmicChanged,
    Failed,
    NotRunning,
}
impl InactiveReason {
    fn message(&self) -> &'static str {
        match self {
            Self::NotInstalled => {
                "Install and enable the Islands add-on to use this style. The standard dock and panel remain available."
            }
            Self::Disabled => {
                "Islands is disabled. Run halcyon-islands enable, then log out and back in."
            }
            Self::CosmicChanged => {
                "Islands needs an update for this COSMIC version. Install the matching add-on, run halcyon-islands enable, then log out and back in. Your saved layout is kept."
            }
            Self::Failed => {
                "Islands stopped unexpectedly, so the standard panel was selected. See the Islands troubleshooting guide before enabling it again."
            }
            Self::NotRunning => {
                "Islands is not running. Startup checks may have selected the standard panel. After enabling or updating the add-on, log out and back in; check the Islands status and session log if it remains unavailable."
            }
        }
    }
}

fn read_record(path: &std::path::Path) -> Option<serde_json::Value> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= 65536)
        .then(|| serde_json::from_slice(&bytes).ok())
        .flatten()
}

// Explanatory mirror of the selector's reviewed-source-v1 policy. This never
// activates a host; ABI/global checks remain the launcher's responsibility.
fn source_revision<'a>(package: &str, version: &'a str) -> Option<&'a str> {
    static FORMAT: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let captures = FORMAT
        .get_or_init(|| {
            regex::Regex::new(r"^(\d+(?:\.\d+)*)~\d+~[0-9.]+~([0-9a-f]{7,40})$")
                .expect("fixed package pattern")
        })
        .captures(version)?;
    let release: Vec<u64> = captures[1]
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if package != "cosmic-comp" && release.as_slice() < [1, 10].as_slice() {
        return None;
    }
    Some(captures.get(2)?.as_str())
}

fn inactive_reason(
    bundle: &std::path::Path,
    state: &std::path::Path,
    installed_version: impl Fn(&str) -> Option<String>,
) -> InactiveReason {
    if !bundle.join("manifest.json").is_file() || !bundle.join("cosmic-panel-islands").is_file() {
        return InactiveReason::NotInstalled;
    }
    let selection = read_record(&bundle.join("selection.json"));
    if selection.as_ref().and_then(|v| v["enabled"].as_bool()) == Some(false) {
        return InactiveReason::Disabled;
    }
    if state.join("failed").is_file() {
        return InactiveReason::Failed;
    }
    if let Some(manifest) = read_record(&bundle.join("manifest.json")) {
        // New add-ons advertise tested combinations. Older manifests retain
        // their single combination; missing evidence must not invent an upgrade.
        let source_policy =
            manifest["source_policy"] == "reviewed-source-v1" && manifest["runtime_probe"] == 1;
        let legacy = &manifest["packages"];
        let candidates = manifest["supported_packages"]
            .as_array()
            .filter(|rows| !rows.is_empty())
            .map(Vec::as_slice)
            .unwrap_or_else(|| std::slice::from_ref(legacy));
        let versions = ["cosmic-panel", "cosmic-comp", "cosmic-session"]
            .map(|name| (name, installed_version(name)));
        if candidates.iter().all(|candidate| {
            versions.iter().any(|(name, actual)| {
                candidate[*name]
                    .as_str()
                    .zip(actual.as_deref())
                    .is_some_and(|(expected, actual)| {
                        expected != actual
                            && !(source_policy
                                && source_revision(name, expected)
                                    .zip(source_revision(name, actual))
                                    .is_some_and(|(a, b)| a == b))
                    })
            })
        }) {
            return InactiveReason::CosmicChanged;
        }
    }
    InactiveReason::NotRunning
}

fn inactive_message(store: &DesktopStore) -> String {
    if store.fixture_root().is_some() || std::env::var_os("DESKTOP_CARDS_CONFIG").is_some() {
        return "The isolated Islands panel component is not active".into();
    }
    let Some(data) = dirs::data_dir() else {
        return InactiveReason::NotRunning.message().into();
    };
    let Some(state) = dirs::state_dir() else {
        return InactiveReason::NotRunning.message().into();
    };
    inactive_reason(
        &data.join("cosmic-desktop-cards/companion"),
        &state.join("desktop-cards-panel"),
        |package| {
            let result = std::process::Command::new("/usr/bin/dpkg-query")
                .args(["-W", "-f=${Version}", package])
                .output()
                .ok()?;
            if !result.status.success() {
                return None;
            }
            let version = String::from_utf8(result.stdout).ok()?.trim().to_string();
            (!version.is_empty()).then_some(version)
        },
    )
    .message()
    .into()
}

pub struct Client {
    stream: BufReader<UnixStream>,
}
impl Client {
    pub fn connect(store: &DesktopStore) -> Result<Self, String> {
        if Installation::on_native_thread() && Installation::needs_worker(store) {
            return Err("Islands runtime status is refreshed in the background".into());
        }
        if !Installation::discover(store).installed() {
            return Err(
                "Islands is not installed or its installation could not be verified".into(),
            );
        }
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or("No desktop session")?;
        let path = if store.fixture_root().is_some() {
            let path = std::env::var_os("DESKTOP_CARDS_ISLANDS_SOCKET")
                .map(PathBuf::from)
                .ok_or("No isolated panel component")?;
            if path.parent() != Some(runtime.as_path()) {
                return Err(
                    "The isolated panel socket must be in the isolated runtime directory".into(),
                );
            }
            path
        } else if std::env::var_os("DESKTOP_CARDS_CONFIG").is_some() {
            return Err("An isolated desktop root is needed for panel previews".into());
        } else {
            runtime.join("desktop-cards-islands-v1.sock")
        };
        Self::connect_at(&path, || inactive_message(store))
    }

    fn connect_at(path: &std::path::Path, inactive: impl Fn() -> String) -> Result<Self, String> {
        let meta = std::fs::symlink_metadata(path).map_err(|_| inactive())?;
        if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
            return Err("The panel socket is not owned by this user".into());
        }
        let stream = UnixStream::connect(path).map_err(|error| match error.kind() {
            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound => inactive(),
            _ => "The Islands panel component is not responding".into(),
        })?;
        stream
            .set_read_timeout(Some(Duration::from_millis(750)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_millis(750)))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            stream: BufReader::new(stream),
        })
    }
    pub fn request(&mut self, request: &Request) -> Result<Response, String> {
        let mut bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        if bytes.len() > MAX_WIRE_BYTES {
            return Err("Island request is too large".into());
        }
        self.stream
            .get_mut()
            .write_all(&bytes)
            .map_err(|_| "The Islands panel component disconnected")?;
        let mut data = Vec::new();
        self.stream
            .by_ref()
            .take((MAX_WIRE_BYTES + 1) as u64)
            .read_until(b'\n', &mut data)
            .map_err(|_| "The panel did not acknowledge the change")?;
        if data.is_empty() {
            return Err("The Islands panel component disconnected".into());
        }
        if data.len() > MAX_WIRE_BYTES || !data.ends_with(b"\n") {
            return Err("Invalid panel reply".into());
        }
        match serde_json::from_slice(&data).map_err(|_| "Invalid panel reply")? {
            Response::Error(error) => Err(error),
            response => Ok(response),
        }
    }
    pub fn capabilities(&mut self) -> Result<Capabilities, String> {
        match self.request(&Request::Capabilities)? {
            Response::Capabilities(cap) if cap.compatible() => Ok(cap),
            _ => Err("The panel component needs a compatible Dock Islands version".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::{
        panels::{PanelDraft, Setting},
        store::{Owner, tests::Fixture},
    };
    #[test]
    fn stale_socket_reports_failure_and_respects_isolation() {
        let fixture = Fixture::new("islands-stale-socket");
        let socket = fixture.0.join("panel.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        drop(listener);
        assert!(
            std::fs::symlink_metadata(&socket)
                .unwrap()
                .file_type()
                .is_socket()
        );
        let bundle = fixture.0.join("bundle");
        let state = fixture.0.join("state");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(bundle.join("manifest.json"), "{}").unwrap();
        std::fs::write(bundle.join("cosmic-panel-islands"), "fixture").unwrap();
        std::fs::write(state.join("failed"), "crashed").unwrap();
        let result = Client::connect_at(&socket, || {
            inactive_reason(&bundle, &state, |_| {
                panic!("failure needs no package query")
            })
            .message()
            .into()
        });
        assert_eq!(result.err().unwrap(), InactiveReason::Failed.message());
        assert_eq!(
            Client::connect_at(&socket, || inactive_message(&fixture.store()))
                .err()
                .unwrap(),
            "The isolated Islands panel component is not active"
        );
        std::fs::remove_file(state.join("failed")).unwrap();
        assert_eq!(
            Client::connect_at(&socket, || inactive_reason(&bundle, &state, |_| None)
                .message()
                .into())
            .err()
            .unwrap(),
            InactiveReason::NotRunning.message()
        );
    }

    #[test]
    fn invalid_socket_is_rejected_before_diagnosis() {
        let fixture = Fixture::new("islands-invalid-socket");
        let socket = fixture.0.join("panel.sock");
        std::fs::write(&socket, "not a socket").unwrap();
        assert_eq!(
            Client::connect_at(&socket, || panic!("invalid socket must not diagnose"))
                .err()
                .unwrap(),
            "The panel socket is not owned by this user"
        );
    }

    #[test]
    fn inactive_status_distinguishes_upgrade_from_disabled_and_failure() {
        let fixture = Fixture::new("islands-status");
        let bundle = fixture.0.join("bundle");
        let state = fixture.0.join("state");
        assert_eq!(
            inactive_reason(&bundle, &state, |_| None),
            InactiveReason::NotInstalled
        );
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("cosmic-panel-islands"), "fixture").unwrap();
        std::fs::write(
            bundle.join("manifest.json"),
            r#"{"packages":{"cosmic-panel":"1.11","cosmic-comp":"2","cosmic-session":"3"}}"#,
        )
        .unwrap();
        std::fs::write(bundle.join("selection.json"), r#"{"enabled":false}"#).unwrap();
        assert_eq!(
            inactive_reason(&bundle, &state, |_| panic!(
                "disabled must not query packages"
            )),
            InactiveReason::Disabled
        );
        std::fs::write(bundle.join("selection.json"), r#"{"enabled":true}"#).unwrap();
        for changed in ["cosmic-panel", "cosmic-comp", "cosmic-session"] {
            let status = inactive_reason(&bundle, &state, |name| {
                Some(
                    if name == changed {
                        "new"
                    } else {
                        match name {
                            "cosmic-panel" => "1.11",
                            "cosmic-comp" => "2",
                            _ => "3",
                        }
                    }
                    .into(),
                )
            });
            assert_eq!(status, InactiveReason::CosmicChanged);
        }
        assert_eq!(
            inactive_reason(&bundle, &state, |name| Some(
                match name {
                    "cosmic-panel" => "1.11",
                    "cosmic-comp" => "2",
                    _ => "3",
                }
                .into()
            )),
            InactiveReason::NotRunning
        );
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("failed"), "previous crash").unwrap();
        assert_eq!(
            inactive_reason(&bundle, &state, |_| panic!("failure has priority")),
            InactiveReason::Failed
        );
    }

    #[test]
    fn source_policy_diagnoses_rebuilds_without_claiming_unknown_revisions() {
        let fixture = Fixture::new("islands-source-policy");
        let bundle = &fixture.0;
        std::fs::write(bundle.join("cosmic-panel-islands"), "fixture").unwrap();
        std::fs::write(
            bundle.join("manifest.json"),
            r#"{
            "source_policy":"reviewed-source-v1", "runtime_probe":1,
            "supported_packages":[{"cosmic-panel":"1.11.0~1~24.04~ca20ce3",
                "cosmic-comp":"0.1~1~24.04~8cb1802", "cosmic-session":"1.11.0~1~24.04~4b0a0a2"}]
        }"#,
        )
        .unwrap();
        let actual = |name: &str| {
            Some(
                match name {
                    "cosmic-panel" => "1.11.0~2~26.04~ca20ce3",
                    "cosmic-comp" => "0.1~2~26.04~8cb1802",
                    _ => "1.11.0~2~26.04~4b0a0a2",
                }
                .into(),
            )
        };
        assert_eq!(
            inactive_reason(bundle, bundle, actual),
            InactiveReason::NotRunning
        );
        assert_eq!(
            inactive_reason(bundle, bundle, |name| {
                if name == "cosmic-comp" {
                    Some("0.1~2~26.04~abcdef0".into())
                } else {
                    actual(name)
                }
            }),
            InactiveReason::CosmicChanged
        );
    }

    #[test]
    fn isolated_inactive_message_never_reads_host_companion_state() {
        let fixture = Fixture::new("islands-status-isolated");
        let before = fixture.tree();
        assert_eq!(
            inactive_message(&fixture.store()),
            "The isolated Islands panel component is not active"
        );
        assert_eq!(fixture.tree(), before);
    }

    #[test]
    fn inactive_status_accepts_each_supported_combination_but_not_mixed_releases() {
        let fixture = Fixture::new("islands-status-matrix");
        let bundle = &fixture.0;
        std::fs::write(bundle.join("cosmic-panel-islands"), "fixture").unwrap();
        std::fs::write(bundle.join("selection.json"), r#"{"enabled":true}"#).unwrap();
        std::fs::write(
            bundle.join("manifest.json"),
            r#"{
            "packages":{"cosmic-panel":"1.11","cosmic-comp":"new","cosmic-session":"1.11"},
            "supported_packages":[
                {"cosmic-panel":"1.11","cosmic-comp":"new","cosmic-session":"1.11"},
                {"cosmic-panel":"1.10","cosmic-comp":"old","cosmic-session":"1.10"}
            ]
        }"#,
        )
        .unwrap();
        for (panel, compositor, session, expected) in [
            ("1.10", "old", "1.10", InactiveReason::NotRunning),
            ("1.11", "new", "1.11", InactiveReason::NotRunning),
            ("1.10", "new", "1.10", InactiveReason::CosmicChanged),
            ("1.12", "newer", "1.12", InactiveReason::CosmicChanged),
        ] {
            assert_eq!(
                inactive_reason(bundle, &bundle.join("state"), |name| Some(
                    match name {
                        "cosmic-panel" => panel,
                        "cosmic-comp" => compositor,
                        _ => session,
                    }
                    .into()
                )),
                expected
            );
        }
    }

    #[test]
    fn inactive_status_does_not_invent_an_upgrade_without_version_evidence() {
        let fixture = Fixture::new("islands-status-unknown");
        let bundle = &fixture.0;
        std::fs::write(bundle.join("cosmic-panel-islands"), "fixture").unwrap();
        std::fs::write(bundle.join("selection.json"), r#"{"enabled":true}"#).unwrap();
        for manifest in ["{}", "invalid", r#"{"packages":{"cosmic-panel":"1.11"}}"#] {
            std::fs::write(bundle.join("manifest.json"), manifest).unwrap();
            assert_eq!(
                inactive_reason(bundle, &fixture.0.join("state"), |_| None),
                InactiveReason::NotRunning
            );
        }
    }

    #[test]
    fn sidecar_apply_conflict_and_persistent_undo_preserve_native_lists() {
        let fixture = Fixture::new("islands-transaction");
        fixture_installed(&fixture.0);
        fixture.system(
            "com.system76.CosmicPanel.Dock",
            "plugins_center",
            "Some([\"unknown\",\"a\",\"a\"])",
        );
        let store = fixture.store();
        let mut draft = PanelDraft::load(&store, PanelKind::Dock).unwrap();
        let Setting::Known(mut settings) = draft.islands() else {
            panic!()
        };
        settings.enabled = true;
        settings.gap = 32;
        draft.set_islands(settings.clone());
        let before = fixture.tree();
        assert!(!before.iter().any(|p| format!("{p:?}").contains(NAMESPACE)));
        let applied = store
            .apply_owned(&draft.patch(), None, Owner::DockPanel, "Dock Islands")
            .unwrap();
        assert_eq!(
            PanelDraft::load(&store, PanelKind::Dock)
                .unwrap()
                .islands()
                .known(),
            Some(&settings)
        );
        assert!(
            fixture
                .read("com.system76.CosmicPanel.Dock", "plugins_center")
                .is_none()
        );
        let reopened = fixture.store();
        reopened.undo_latest(&applied.id).unwrap();
        assert!(fixture.read(NAMESPACE, "Dock").is_none());
        let mut other = PanelDraft::load(&store, PanelKind::Dock).unwrap();
        other.set_islands(settings.clone());
        fixture.user(
            NAMESPACE,
            "Dock",
            &ron::to_string(&Settings {
                gap: 1,
                ..Settings::default()
            })
            .unwrap(),
        );
        assert!(
            store
                .apply_owned(&other.patch(), None, Owner::DockPanel, "Dock Islands")
                .is_err()
        );
        assert_eq!(
            fixture.read("com.system76.CosmicPanel.Dock", "plugins_center"),
            None
        );
    }
    #[test]
    fn unknown_extension_is_kept_by_native_edits_and_saved_draft() {
        let f = Fixture::new("islands-unknown");
        f.user(NAMESPACE, "Dock", "(version:99,new_field:true)");
        let mut draft = PanelDraft::load(&f.store(), PanelKind::Dock).unwrap();
        assert!(matches!(draft.islands(), Setting::Unsupported(_)));
        draft.set_anchor(crate::desktop::panels::Anchor::Left);
        let restored: PanelDraft = ron::from_str(&ron::to_string(&draft).unwrap()).unwrap();
        f.store()
            .apply_owned(&restored.patch(), None, Owner::DockPanel, "Dock & Panel")
            .unwrap();
        assert_eq!(
            f.read(NAMESPACE, "Dock").as_deref(),
            Some("(version:99,new_field:true)")
        );
    }
}

#[cfg(test)]
mod installation_tests {
    use super::*;
    use crate::desktop::store::tests::Fixture;
    #[test]
    fn task2_absent_islands_cannot_write_even_a_disabled_sidecar() {
        let f = Fixture::new("absent-islands-write");
        let store = f.store();
        let mut patch = super::super::DesktopPatch::new();
        patch.set(
            key(PanelKind::Dock),
            None,
            Some(RawValue::encode(&Settings::default()).unwrap()),
        );
        assert!(
            store.apply(&patch).is_err(),
            "leftovers/default settings must not install a sidecar"
        );
        assert!(!store.user_path(&key(PanelKind::Dock)).exists());
    }
}

mod installation;
#[cfg(test)]
pub(crate) use installation::fixture_installed;
pub use installation::{CompatibilityStatus, Installation};

#[cfg(test)]
mod installation_races {
    use super::*;
    #[test]
    fn task2_removal_after_preview_blocks_sidecar_but_preserves_stored_preferences() {
        let f = crate::desktop::store::tests::Fixture::new("islands-removed");
        fixture_installed(&f.0);
        let store = f.store();
        let mut patch = crate::desktop::DesktopPatch::new();
        let key = key(PanelKind::Panel);
        patch.set(
            key.clone(),
            None,
            Some(RawValue::encode(&Settings::default()).unwrap()),
        );
        store.apply(&patch).unwrap();
        let saved = std::fs::read(store.user_path(&key)).unwrap();
        let mut changed = Settings::default();
        changed.gap = 42;
        patch.set(
            key.clone(),
            store.read_user(&key).unwrap(),
            Some(RawValue::encode(&changed).unwrap()),
        );
        std::fs::remove_file(f.0.join("islands-installation.json")).unwrap();
        assert!(store.apply(&patch).is_err());
        assert_eq!(std::fs::read(store.user_path(&key)).unwrap(), saved);
    }
}
