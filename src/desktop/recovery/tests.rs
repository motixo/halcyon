use crate::desktop::store::tests::Fixture;
use crate::desktop::{ConfigKey, DesktopPatch, RawValue};
use std::fs;

const DOCK: &str = "com.system76.CosmicPanel.Dock";
fn change(store: &crate::desktop::DesktopStore, value: &str) -> DesktopPatch {
    let key = ConfigKey::new(DOCK, 1, "opacity").unwrap();
    let mut patch = DesktopPatch::new();
    patch.set(
        key.clone(),
        store.read_user(&key).unwrap(),
        Some(RawValue::parse(value).unwrap()),
    );
    patch
}

#[test]
fn first_desktop_write_captures_baseline_before_mutation() {
    let fixture = Fixture::new("baseline-first");
    fixture.user(DOCK, "opacity", " 0.80\n");
    let store = fixture
        .store()
        .with_app_config(fixture.0.join("config.json"));
    store.apply(&change(&store, "0.5")).unwrap();
    assert!(
        store
            .state_dir()
            .join("desktop-recovery/original/manifest.json")
            .is_file(),
        "first write must create its immutable baseline"
    );
}

use super::*;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{PermissionsExt, symlink};
fn baseline(store: &DesktopStore) -> BaselineInfo {
    load_baseline(store).unwrap().unwrap()
}
fn record<'a>(info: &'a BaselineInfo, namespace: &str, key: &[u8]) -> &'a NativeRecord {
    info.domains
        .iter()
        .flat_map(|domain| &domain.native)
        .find(|record| {
            record.namespace == namespace && record.key_name().unwrap().as_bytes() == key
        })
        .unwrap()
}

#[test]
fn baseline_preserves_raw_bytes_absence_unknown_names_and_private_scope() {
    let fixture = Fixture::new("baseline-bytes");
    fixture.user(DOCK, "opacity", " \n0.8\n ");
    let ns = fixture.0.join("config/cosmic").join(DOCK).join("v1");
    fs::write(
        ns.join(OsString::from_vec(b"unknown-\xff".to_vec())),
        b"\xff\x00future\n",
    )
    .unwrap();
    fixture.user(
        "com.system76.CosmicBackground",
        "output.DP-2",
        "future unknown value\n",
    );
    fixture.user(
        "com.system76.CosmicSettings.Wallpaper",
        "custom-images",
        "[\"/fixture/a.png\"]\n",
    );
    let config = fixture.0.join("config.json");
    fs::write(&config, r#"{"appearance":{"accent":null},"notes":{"text":"secret note"},"calendar":{"token":"secret token"},"project_roots":["private path"]}"#).unwrap();
    let store = fixture.store().with_app_config(config);
    store.apply(&change(&store, "0.6")).unwrap();
    let info = baseline(&store);
    assert_eq!(
        record(&info, DOCK, b"opacity").contents().unwrap().unwrap(),
        b" \n0.8\n "
    );
    assert_eq!(
        record(&info, DOCK, b"unknown-\xff")
            .contents()
            .unwrap()
            .unwrap(),
        b"\xff\x00future\n"
    );
    assert_eq!(record(&info, DOCK, b"background").contents().unwrap(), None);
    assert!(
        record(&info, "com.system76.CosmicBackground", b"output.DP-2")
            .contents()
            .unwrap()
            .is_some()
    );
    assert!(
        record(
            &info,
            "com.system76.CosmicSettings.Wallpaper",
            b"custom-images"
        )
        .contents()
        .unwrap()
        .is_some()
    );
    let app: Vec<_> = info.domains.iter().flat_map(|d| &d.app).collect();
    let accent = app
        .iter()
        .find(|leaf| leaf.pointer == "/appearance/accent")
        .unwrap();
    assert!(accent.present && accent.value.is_null());
    assert!(
        !app.iter()
            .find(|leaf| leaf.pointer == "/appearance/family")
            .unwrap()
            .present
    );
    let serialized = serde_json::to_string(&info).unwrap();
    for private in [
        "secret note",
        "secret token",
        "private path",
        "project_roots",
    ] {
        assert!(!serialized.contains(private));
    }
    let original = store.state_dir().join("desktop-recovery/original");
    assert_eq!(
        fs::metadata(&original).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(original.join("manifest.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn baseline_is_not_rotated_by_undo_history() {
    let fixture = Fixture::new("baseline-immutable");
    fixture.user(DOCK, "opacity", "0.8");
    let store = fixture.store();
    let first = store
        .apply_owned(
            &change(&store, "0.6"),
            None,
            crate::desktop::Owner::DockPanel,
            "Opacity",
        )
        .unwrap();
    let original = fs::read(
        store
            .state_dir()
            .join("desktop-recovery/original/manifest.json"),
    )
    .unwrap();
    for value in ["0.7", "0.9", "0.4"] {
        store
            .apply_owned(
                &change(&store, value),
                None,
                crate::desktop::Owner::DockPanel,
                "Opacity",
            )
            .unwrap();
    }
    let last = store.latest_change().unwrap().unwrap();
    assert_ne!(last.id, first.id);
    store.undo_latest(&last.id).unwrap();
    assert_eq!(
        fs::read(
            store
                .state_dir()
                .join("desktop-recovery/original/manifest.json")
        )
        .unwrap(),
        original
    );
}

#[test]
fn legacy_install_is_labeled_current_desktop() {
    for evidence in ["config", "history", "missing-app-path", "fresh"] {
        let fixture = Fixture::new(evidence);
        let app = fixture.0.join("config.json");
        if evidence == "config" {
            fs::write(&app, r#"{"onboarded":true}"#).unwrap();
        }
        if evidence == "history" {
            fs::create_dir_all(fixture.0.join("state")).unwrap();
            fs::write(
                fixture.0.join("state/cosmic-colors-original.json"),
                "legacy",
            )
            .unwrap();
        }
        let store = if evidence == "missing-app-path" {
            fixture.store()
        } else {
            fixture.store().with_app_config(app)
        };
        let info = ensure_baseline(&store, BaselineKind::OriginalDesktop).unwrap();
        assert_eq!(
            info.kind,
            if evidence == "fresh" {
                BaselineKind::OriginalDesktop
            } else {
                BaselineKind::CurrentDesktop
            }
        );
    }
}

#[test]
fn failed_capture_blocks_mutation() {
    let fixture = Fixture::new("baseline-failure");
    fixture.user(DOCK, "opacity", "0.8");
    let bad = fixture.0.join("state/desktop-recovery/original");
    fs::create_dir_all(&bad).unwrap();
    fs::write(bad.join("manifest.json"), b"broken evidence").unwrap();
    let config = fixture.0.join("config.json");
    fs::write(&config, "{\"theme\":\"teal\"}\n").unwrap();
    let store = fixture.store().with_app_config(config.clone());
    let mut widget = crate::desktop::WidgetPatch::new();
    widget
        .set("/theme", Some("teal".into()), Some("rose".into()))
        .unwrap();
    assert!(
        store
            .apply_owned(
                &change(&store, "0.4"),
                Some(&widget),
                crate::desktop::Owner::Appearance,
                "Colors"
            )
            .is_err()
    );
    assert_eq!(fixture.read(DOCK, "opacity").unwrap(), "0.8");
    assert_eq!(fs::read(&config).unwrap(), b"{\"theme\":\"teal\"}\n");
    assert_eq!(
        fs::read(bad.join("manifest.json")).unwrap(),
        b"broken evidence"
    );
    assert!(!store.state_dir().join("desktop-journal.json").exists());
}

#[test]
fn baseline_rejects_links_and_escapes() {
    for kind in ["leaf", "namespace", "state", "app"] {
        let fixture = Fixture::new(&format!("baseline-link-{kind}"));
        let outside = Fixture::new("baseline-outside");
        fixture.user(DOCK, "opacity", "0.8");
        let store = fixture
            .store()
            .with_app_config(fixture.0.join("config.json"));
        match kind {
            "leaf" => {
                fs::write(outside.0.join("secret"), b"private").unwrap();
                symlink(
                    outside.0.join("secret"),
                    store.user_root().join(DOCK).join("v1/future"),
                )
                .unwrap();
            }
            "namespace" => {
                symlink(
                    &outside.0,
                    store.user_root().join("com.system76.CosmicTheme.Dark"),
                )
                .unwrap();
            }
            "state" => {
                fs::create_dir_all(store.state_dir()).unwrap();
                symlink(&outside.0, store.state_dir().join("desktop-recovery")).unwrap();
            }
            "app" => {
                fs::write(outside.0.join("secret"), b"{}").unwrap();
                symlink(outside.0.join("secret"), store.app_config().unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        let before = outside.tree();
        assert!(store.apply(&change(&store, "0.5")).is_err(), "{kind}");
        assert_eq!(fixture.read(DOCK, "opacity").unwrap(), "0.8");
        assert_eq!(outside.tree(), before);
    }
}

#[test]
fn concurrent_first_writes_share_one_complete_snapshot() {
    let fixture = Fixture::new("baseline-concurrent");
    fixture.user(DOCK, "opacity", "0.8");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(6));
    let threads: Vec<_> = (0..6)
        .map(|_| {
            let root = fixture.0.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let store = DesktopStore::fixture(&root)
                    .unwrap()
                    .with_app_config(root.join("config.json"));
                let patch = change(&store, "0.5");
                barrier.wait();
                store.apply(&patch).is_ok()
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|t| usize::from(t.join().unwrap()))
            .sum::<usize>(),
        1
    );
    let info = baseline(&fixture.store());
    assert_eq!(info.domains.len(), RecoveryDomain::ALL.len());
    assert_eq!(
        record(&info, DOCK, b"opacity").contents().unwrap().unwrap(),
        b"0.8"
    );
    assert_eq!(
        fs::read_dir(fixture.0.join("state/desktop-recovery"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn oversized_capture_and_incomplete_original_are_not_promoted() {
    let fixture = Fixture::new("baseline-bounds");
    fixture.user(DOCK, "opacity", "0.8");
    let path = fixture.0.join("config/cosmic").join(DOCK).join("v1/future");
    fs::write(&path, vec![0xff; MAX_FILE_BYTES + 1]).unwrap();
    let store = fixture.store();
    assert!(store.apply(&change(&store, "0.7")).is_err());
    assert!(!store.state_dir().join("desktop-recovery/original").exists());
    fs::write(&path, vec![0xff; MAX_FILE_BYTES]).unwrap();
    store.apply(&change(&store, "0.7")).unwrap();
    assert_eq!(
        record(&baseline(&store), DOCK, b"future")
            .contents()
            .unwrap()
            .unwrap()
            .len(),
        MAX_FILE_BYTES
    );
    fs::remove_file(
        store
            .state_dir()
            .join("desktop-recovery/original/manifest.json"),
    )
    .unwrap();
    assert!(store.apply(&change(&store, "0.6")).is_err());
    assert_eq!(fixture.read(DOCK, "opacity").unwrap(), "0.7");
}

#[test]
fn app_animation_write_captures_its_scoped_value_before_mutation() {
    let fixture = Fixture::new("baseline-gtk-motion");
    let dir = fixture
        .0
        .join("gsettings")
        .join(crate::desktop::motion::SCHEMA);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(crate::desktop::motion::KEY), "true\n").unwrap();
    let store = fixture
        .store()
        .with_app_config(fixture.0.join("config.json"));
    crate::desktop::motion::AppAnimations::for_store(&store)
        .write(false)
        .unwrap();
    let info = baseline(&store);
    let motion = info
        .domains
        .iter()
        .find(|d| d.domain == RecoveryDomain::AppAnimations)
        .unwrap()
        .motion
        .as_ref()
        .unwrap();
    assert_eq!(
        STANDARD.decode(motion.bytes.as_ref().unwrap()).unwrap(),
        b"true\n"
    );
    assert_eq!(
        fs::read(dir.join(crate::desktop::motion::KEY)).unwrap(),
        b"false"
    );
}

#[test]
fn newly_supported_domain_gets_its_own_current_date_without_rewriting_original_evidence() {
    let fixture = Fixture::new("baseline-new-domain");
    fixture.user(DOCK, "opacity", "0.8");
    let store = fixture
        .store()
        .with_app_config(fixture.0.join("config.json"));
    let first = ensure_baseline(&store, BaselineKind::OriginalDesktop).unwrap();
    let motion_dir = fixture
        .0
        .join("gsettings")
        .join(crate::desktop::motion::SCHEMA);
    fs::create_dir_all(&motion_dir).unwrap();
    crate::desktop::motion::AppAnimations::for_store(&store)
        .write(false)
        .unwrap();
    let after = baseline(&store);
    assert_eq!(first.kind, after.kind);
    assert_eq!(first.captured_at, after.captured_at);
    for domain in &first.domains {
        assert_eq!(
            after
                .domains
                .iter()
                .find(|d| d.domain == domain.domain)
                .unwrap(),
            domain
        );
    }
    let added = after
        .domains
        .iter()
        .find(|d| d.domain == RecoveryDomain::AppAnimations)
        .unwrap();
    assert_eq!(added.kind, BaselineKind::CurrentDesktop);
    assert!(added.captured_at >= first.captured_at);
    assert!(
        added.motion.as_ref().unwrap().bytes.is_none(),
        "absence remains different from default true"
    );
    let bytes = fs::read(
        store
            .state_dir()
            .join("desktop-recovery/original/manifest.json"),
    )
    .unwrap();
    crate::desktop::motion::AppAnimations::for_store(&store)
        .write(true)
        .unwrap();
    assert_eq!(
        fs::read(
            store
                .state_dir()
                .join("desktop-recovery/original/manifest.json")
        )
        .unwrap(),
        bytes
    );
}

#[test]
fn total_capture_limit_blocks_writes_and_stale_pending_evidence_is_never_promoted() {
    let fixture = Fixture::new("baseline-total-bound");
    fixture.user(DOCK, "opacity", "0.8");
    let store = fixture.store();
    let directory = store.user_root().join(DOCK).join("v1");
    for index in 0..65 {
        fs::write(
            directory.join(format!("future-{index}")),
            vec![0xff; MAX_FILE_BYTES],
        )
        .unwrap();
    }
    assert!(store.apply(&change(&store, "0.6")).is_err());
    assert_eq!(fixture.read(DOCK, "opacity").unwrap(), "0.8");
    assert!(!store.state_dir().join("desktop-recovery/original").exists());
    for index in 63..65 {
        fs::remove_file(directory.join(format!("future-{index}"))).unwrap();
    }
    let pending = store
        .state_dir()
        .join("desktop-recovery/pending-interrupted");
    fs::create_dir_all(&pending).unwrap();
    fs::write(pending.join("manifest.json"), b"partial evidence").unwrap();
    store.apply(&change(&store, "0.6")).unwrap();
    assert_eq!(
        record(&baseline(&store), DOCK, b"opacity")
            .contents()
            .unwrap()
            .unwrap(),
        b"0.8"
    );
    assert_eq!(
        fs::read(pending.join("manifest.json")).unwrap(),
        b"partial evidence"
    );
}

#[test]
fn animation_capture_failure_keeps_setting_and_corrupt_evidence() {
    let fixture = Fixture::new("baseline-motion-failure");
    let store = fixture.store();
    let path = fixture
        .0
        .join("gsettings")
        .join(crate::desktop::motion::SCHEMA)
        .join(crate::desktop::motion::KEY);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"true\n").unwrap();
    let manifest = store
        .state_dir()
        .join("desktop-recovery/original/manifest.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(&manifest, b"invalid baseline").unwrap();
    assert!(
        crate::desktop::motion::AppAnimations::for_store(&store)
            .write(false)
            .is_err()
    );
    assert_eq!(fs::read(path).unwrap(), b"true\n");
    assert_eq!(fs::read(manifest).unwrap(), b"invalid baseline");
}

#[test]
fn animation_fixture_never_follows_links_even_after_baseline_capture() {
    let fixture = Fixture::new("baseline-motion-link");
    let outside = Fixture::new("baseline-motion-outside");
    let path = fixture
        .0
        .join("gsettings")
        .join(crate::desktop::motion::SCHEMA)
        .join(crate::desktop::motion::KEY);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let store = fixture.store();
    let motion = crate::desktop::motion::AppAnimations::for_store(&store);
    motion.write(false).unwrap();
    fs::write(outside.0.join("secret"), b"true\n").unwrap();
    fs::remove_file(&path).unwrap();
    symlink(outside.0.join("secret"), &path).unwrap();
    assert!(motion.write(false).is_err());
    assert_eq!(fs::read(outside.0.join("secret")).unwrap(), b"true\n");
    assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn corrupted_header_cannot_relabel_current_evidence_as_original() {
    let fixture = Fixture::new("baseline-label-corruption");
    fixture.user(DOCK, "opacity", "0.8");
    let store = fixture.store();
    let mut info = ensure_baseline(&store, BaselineKind::OriginalDesktop).unwrap();
    assert_eq!(info.kind, BaselineKind::CurrentDesktop);
    info.kind = BaselineKind::OriginalDesktop;
    let manifest = store
        .state_dir()
        .join("desktop-recovery/original/manifest.json");
    let bytes = serde_json::to_vec(&info).unwrap();
    fs::write(&manifest, &bytes).unwrap();
    assert!(store.apply(&change(&store, "0.6")).is_err());
    assert_eq!(fs::read(&manifest).unwrap(), bytes);
    assert_eq!(fixture.read(DOCK, "opacity").unwrap(), "0.8");
}
