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

pub struct Client {
    stream: BufReader<UnixStream>,
}
impl Client {
    pub fn connect(store: &DesktopStore) -> Result<Self, String> {
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
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|_| "The Islands panel component is not active")?;
        if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
            return Err("The panel socket is not owned by this user".into());
        }
        let stream = UnixStream::connect(path)
            .map_err(|_| "The Islands panel component is not responding")?;
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
    fn sidecar_apply_conflict_and_persistent_undo_preserve_native_lists() {
        let fixture = Fixture::new("islands-transaction");
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
