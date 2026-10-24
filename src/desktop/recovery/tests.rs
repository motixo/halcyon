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

#[test]
fn review_r1_lock_links_cannot_create_native_state_before_capture() {
    for which in ["config", "desktop"] {
        let fixture = Fixture::new(&format!("review-r1-{which}-lock-link"));
        let app = fixture.0.join("app/config.json");
        fs::create_dir_all(app.parent().unwrap()).unwrap();
        fs::write(&app, b"{\"theme\":\"teal\"}\n").unwrap();
        let user = fixture.0.join("config/cosmic");
        let state = fixture.0.join("state");
        fs::create_dir_all(user.join(DOCK).join("v1")).unwrap();
        fs::create_dir_all(&state).unwrap();
        let target = user.join(DOCK).join("v1/opacity");
        let lock = if which == "config" {
            app.with_extension("json.lock")
        } else {
            state.join("desktop.lock")
        };
        symlink(&target, &lock).unwrap();
        // Exercise live access semantics against entirely private roots;
        // fixture preflight must not mask the live lock-opener defect.
        let store = DesktopStore::with_roots(
            user.clone(),
            vec![],
            state,
            crate::desktop::store::Access::Live,
        )
        .with_app_config(app.clone());
        let mut widget = crate::desktop::WidgetPatch::new();
        widget
            .set("/theme", Some("teal".into()), Some("rose".into()))
            .unwrap();
        assert!(
            store
                .apply_owned(
                    &DesktopPatch::new(),
                    Some(&widget),
                    crate::desktop::Owner::Appearance,
                    "Palette"
                )
                .is_err(),
            "{which}"
        );
        assert!(!target.exists(), "{which} lock created its dangling target");
        assert_eq!(fs::read(&app).unwrap(), b"{\"theme\":\"teal\"}\n");
        assert!(!store.state_dir().join("desktop-recovery/original").exists());
    }
}

#[test]
fn review_r1_lock_ancestors_and_nonregular_leaves_are_rejected() {
    for which in ["config", "desktop"] {
        for kind in ["ancestor", "dangling-ancestor", "directory", "fifo"] {
            let fixture = Fixture::new(&format!("review-r1-lock-{which}-{kind}"));
            let outside = Fixture::new("review-r1-lock-outside");
            let app = fixture.0.join("app/config.json");
            fs::create_dir_all(app.parent().unwrap()).unwrap();
            fs::write(&app, b"{\"theme\":\"teal\"}\n").unwrap();
            let state = fixture.0.join("state");
            fs::create_dir_all(&state).unwrap();
            let (parent, name) = if which == "config" {
                (app.parent().unwrap().to_path_buf(), "config.json.lock")
            } else {
                (state.clone(), "desktop.lock")
            };
            if kind.ends_with("ancestor") {
                if which == "config" {
                    fs::rename(&app, outside.0.join("config.json")).unwrap();
                }
                fs::remove_dir(&parent).unwrap();
                symlink(
                    if kind == "ancestor" {
                        outside.0.clone()
                    } else {
                        outside.0.join("absent")
                    },
                    &parent,
                )
                .unwrap();
            } else if kind == "directory" {
                fs::create_dir(parent.join(name)).unwrap();
            } else {
                let name =
                    std::ffi::CString::new(parent.join(name).as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            let before = outside.tree();
            let store = DesktopStore::with_roots(
                fixture.0.join("config/cosmic"),
                vec![],
                state.clone(),
                crate::desktop::store::Access::Live,
            )
            .with_app_config(app.clone());
            let result = if which == "config" {
                crate::config::lock_config(&app)
                    .map(drop)
                    .map_err(|e| e.to_string())
            } else {
                store
                    .capture_baseline(BaselineKind::OriginalDesktop)
                    .map(drop)
                    .map_err(|e| e.to_string())
            };
            assert!(result.is_err(), "{which} {kind}");
            assert_eq!(
                outside.tree(),
                before,
                "{which} {kind}: outside target changed"
            );
            assert!(!state.join("desktop-recovery/original").exists());
        }
    }
}

#[test]
fn task2_undo_rejects_even_whitespace_only_external_edits() {
    let f = Fixture::new("restore-exact-guard");
    f.user(DOCK, "opacity", " 0.8\n");
    let store = f.store();
    let applied = store.apply(&change(&store, "0.5")).unwrap();
    f.user(DOCK, "opacity", " 0.5\n");
    assert!(
        store.undo(&applied).is_err(),
        "Undo must not claim externally rewritten bytes"
    );
    assert_eq!(f.read(DOCK, "opacity").unwrap(), " 0.5\n");
}

#[test]
fn task2_restore_is_selective_exact_and_undoable_after_restart() {
    let f = Fixture::new("restore-selective");
    f.user(DOCK, "opacity", " \n0.8\n ");
    f.user(DOCK, "anchor", "Bottom");
    let app = f.0.join("config.json");
    fs::write(
        &app,
        r#"{"appearance":{"opacity":85},"notes":{"text":"private"}}"#,
    )
    .unwrap();
    let store = f.store().with_app_config(app.clone());
    store.apply(&change(&store, "0.5")).unwrap();
    let original = fs::read(
        store
            .state_dir()
            .join("desktop-recovery/original/manifest.json"),
    )
    .unwrap();
    fs::write(
        &app,
        r#"{"appearance":{"opacity":75},"notes":{"text":"new private"}}"#,
    )
    .unwrap();
    let plan = plan_restore(&store, &[RecoveryDomain::DockPanel]).unwrap();
    let restored = apply_restore(&store, &plan).unwrap();
    assert_eq!(f.read(DOCK, "opacity").unwrap(), " \n0.8\n ");
    assert!(fs::read_to_string(&app).unwrap().contains("new private"));
    let reopened = f.store().with_app_config(app);
    reopened.undo_latest(&restored.id).unwrap();
    assert_eq!(f.read(DOCK, "opacity").unwrap(), "0.5");
    assert_eq!(
        fs::read(
            store
                .state_dir()
                .join("desktop-recovery/original/manifest.json")
        )
        .unwrap(),
        original
    );
    assert!(
        store
            .state_dir()
            .join("desktop-recovery/pre-restore")
            .is_dir()
    );
}

#[test]
fn task2_restore_keeps_opaque_evidence_and_rejects_changed_preview() {
    let f = Fixture::new("restore-opaque");
    f.user(DOCK, "opacity", "0.8");
    let opaque = f.0.join("config/cosmic").join(DOCK).join("v1/size");
    fs::write(&opaque, b"\xff\x00").unwrap();
    let store = f.store();
    store.apply(&change(&store, "0.5")).unwrap();
    let plan = plan_restore(&store, &[RecoveryDomain::DockPanel]).unwrap();
    assert!(!plan.skipped.is_empty());
    f.user(DOCK, "opacity", " 0.5\n");
    assert!(apply_restore(&store, &plan).is_err());
    assert_eq!(fs::read(opaque).unwrap(), b"\xff\x00");
    let exported = export_baseline(&store).unwrap();
    assert!(exported.contains("/wA="));
    assert!(!exported.contains("desktop-journal"));
}

#[test]
fn task2_restore_interruption_retains_pre_restore_and_recovers_exact_bytes() {
    let f = Fixture::new("restore-interrupt");
    f.user(DOCK, "opacity", " 0.8\n");
    let store = f.store();
    store.apply(&change(&store, "0.5")).unwrap();
    let mut patch = DesktopPatch::new();
    let key = ConfigKey::new(DOCK, 1, "anchor").unwrap();
    patch.set(key, None, Some(RawValue::parse("Top").unwrap()));
    store.apply(&patch).unwrap();
    let plan = plan_restore(&store, &[RecoveryDomain::DockPanel]).unwrap();
    store.inject(Some(crate::desktop::store::Fault::InterruptAfter(1)));
    assert!(apply_restore(&store, &plan).is_err());
    let reopened = f.store();
    assert!(reopened.pending_recovery().unwrap().is_some());
    reopened.recover().unwrap();
    assert_eq!(f.read(DOCK, "opacity").unwrap(), "0.5");
    assert_eq!(f.read(DOCK, "anchor").unwrap(), "Top");
}

#[test]
fn task2_corrupt_manifest_is_preserved_and_scoped_evidence_is_exportable() {
    let f = Fixture::new("restore-corrupt-export");
    f.user(DOCK, "opacity", "0.8");
    let store = f.store();
    store.apply(&change(&store, "0.5")).unwrap();
    let path = store
        .state_dir()
        .join("desktop-recovery/original/manifest.json");
    let mut info: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    info["domains"][0]["sha256"] = "broken".into();
    info["tokens"] = "must not export".into();
    fs::write(&path, serde_json::to_vec(&info).unwrap()).unwrap();
    assert!(plan_restore(&store, &[RecoveryDomain::DockPanel]).is_err());
    let export = export_baseline(&store).unwrap();
    assert!(export.contains("MC44"));
    assert!(!export.contains("must not export"));
    assert_eq!(f.read(DOCK, "opacity").unwrap(), "0.5");
}

#[test]
fn task2_restore_removes_only_owned_dynamic_keys_and_keeps_other_scopes() {
    let f = Fixture::new("restore-dynamic");
    f.user(DOCK, "opacity", "0.8");
    let store = f.store();
    store.apply(&change(&store, "0.5")).unwrap();
    let output = ConfigKey::new(crate::desktop::wallpaper::NAMESPACE, 1, "output.DP-9").unwrap();
    let entry = crate::desktop::wallpaper::BgEntry::new(
        "output.DP-9",
        crate::desktop::wallpaper::BgSource::Color(crate::desktop::wallpaper::BgColor::Single([
            0.1, 0.2, 0.3,
        ])),
    );
    let mut patch = DesktopPatch::new();
    patch.set(
        output.clone(),
        None,
        Some(RawValue::encode(&entry).unwrap()),
    );
    store.apply(&patch).unwrap();
    f.user(
        crate::desktop::wallpaper::NAMESPACE,
        "output.DP-8",
        &ron::to_string(&entry).unwrap(),
    );
    let plan = plan_restore(&store, &[RecoveryDomain::Wallpaper]).unwrap();
    apply_restore(&store, &plan).unwrap();
    assert!(!store.user_path(&output).exists());
    assert!(
        f.read(crate::desktop::wallpaper::NAMESPACE, "output.DP-8")
            .is_some()
    );
    assert_eq!(f.read(DOCK, "opacity").unwrap(), "0.5");
}

#[test]
fn task2_missing_wallpaper_after_preview_is_rejected() {
    let f = Fixture::new("restore-missing-wallpaper");
    let path = f.0.join("saved.png");
    fs::write(&path, "image evidence").unwrap();
    let entry = crate::desktop::wallpaper::BgEntry::new(
        "all",
        crate::desktop::wallpaper::BgSource::Path(path.clone()),
    );
    f.user(
        crate::desktop::wallpaper::NAMESPACE,
        "all",
        &ron::to_string(&entry).unwrap(),
    );
    let store = f.store();
    store.apply(&change(&store, "0.5")).unwrap();
    let plan = plan_restore(&store, &[RecoveryDomain::Wallpaper]).unwrap();
    fs::remove_file(path).unwrap();
    assert!(apply_restore(&store, &plan).is_err());
}

#[test]
fn task2_restore_undo_review_keeps_newer_app_field_and_reverses_native() {
    let f = Fixture::new("restore-review-fields");
    f.user(DOCK, "opacity", "0.8");
    let path = f.0.join("config.json");
    fs::write(&path, r#"{"appearance":{"opacity":80}}"#).unwrap();
    let store = f.store().with_app_config(path.clone());
    store.apply(&change(&store, "0.5")).unwrap();
    fs::write(&path, r#"{"appearance":{"opacity":90}}"#).unwrap();
    let plan = plan_restore(
        &store,
        &[RecoveryDomain::DockPanel, RecoveryDomain::WidgetAppearance],
    )
    .unwrap();
    let restored = apply_restore(&store, &plan).unwrap();
    fs::write(&path, r#"{"appearance":{"opacity":95}}"#).unwrap();
    assert!(store.undo_latest(&restored.id).is_err());
    let review = store.review_undo(&restored.id).unwrap();
    assert!(!review.conflicts.is_empty());
    store.accept_undo_review(&review).unwrap();
    assert_eq!(f.read(DOCK, "opacity").unwrap(), "0.5");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(path).unwrap()).unwrap()["appearance"]
            ["opacity"],
        95
    );
}

#[test]
fn task2r1_opaque_fixed_wallpaper_does_not_block_restore_undo_or_recovery() {
    for interrupted in [false, true] {
        let f = Fixture::new("opaque-fixed-wallpaper");
        f.user(DOCK, "opacity", "0.8");
        f.user(crate::desktop::wallpaper::NAMESPACE, "all", "placeholder");
        let wallpaper =
            f.0.join("config/cosmic/com.system76.CosmicBackground/v1/all");
        fs::write(&wallpaper, b"\xff\x00opaque").unwrap();
        let store = f.store();
        store.apply(&change(&store, "0.5")).unwrap();
        let original = fs::read(
            store
                .state_dir()
                .join("desktop-recovery/original/manifest.json"),
        )
        .unwrap();
        let plan = plan_restore(&store, &[RecoveryDomain::DockPanel]).unwrap();
        if interrupted {
            store.inject(Some(crate::desktop::store::Fault::InterruptBeforeRecord));
            assert!(apply_restore(&store, &plan).is_err());
            let reopened = f.store();
            reopened.recover().unwrap();
            assert!(reopened.pending_recovery().unwrap().is_none());
        } else {
            let restored = apply_restore(&store, &plan).unwrap();
            assert_eq!(f.read(DOCK, "opacity").unwrap(), "0.8");
            store.undo_latest(&restored.id).unwrap();
        }
        assert_eq!(f.read(DOCK, "opacity").unwrap(), "0.5");
        assert_eq!(fs::read(&wallpaper).unwrap(), b"\xff\x00opaque");
        assert_eq!(
            fs::read(
                store
                    .state_dir()
                    .join("desktop-recovery/original/manifest.json")
            )
            .unwrap(),
            original
        );
        assert!(
            store
                .palette_seen(store.restore_epoch().unwrap(), 0)
                .unwrap()
        );
        fs::remove_file(&wallpaper).unwrap();
        let draft = crate::desktop::wallpaper::WallpaperDraft::load(&store).unwrap();
        let later = crate::desktop::wallpaper::automatic::source(&store, &draft).stamp;
        assert_ne!(later, 0);
        assert!(
            !store
                .palette_seen(store.restore_epoch().unwrap(), later)
                .unwrap()
        );
    }
}
