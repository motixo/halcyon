//! Checked merges, the persistent latest change and the combined
//! COSMIC + widget transaction, including restart recovery at each boundary.
//! Every test writes only below its own temporary fixture root; the widget
//! configuration is an explicit file inside that root.
use super::tests::Fixture;
use super::*;
use serde_json::json;

const DOCK: &str = "com.system76.CosmicPanel.Dock";

fn key(name: &str) -> ConfigKey {
    ConfigKey::new(DOCK, 1, name).unwrap()
}
fn raw(text: &str) -> RawValue {
    RawValue::parse(text).unwrap()
}

/// A fixture with two Dock keys and a widget configuration holding portable
/// members, excluded private members and an unknown future member.
struct Case {
    fixture: Fixture,
    config: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let fixture = Fixture::new(name);
        fixture.user(DOCK, "anchor", "Bottom");
        fixture.user(DOCK, "opacity", "0.8");
        let config = fixture.0.join("app/config.json");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(
            &config,
            r#"{"theme":"teal","appearance":{"opacity":90},"location":{"name":"SENTINEL-CITY","latitude":1.0,"longitude":2.0},"project_roots":["/home/sentinel/secret"],"future_setting":{"keep":true}}"#,
        )
        .unwrap();
        Self { fixture, config }
    }
    fn store(&self) -> DesktopStore {
        self.fixture.store().with_app_config(self.config.clone())
    }
    fn text(&self) -> String {
        fs::read_to_string(&self.config).unwrap()
    }
    fn undo(&self) -> PathBuf {
        self.fixture.0.join("state").join(UNDO)
    }
    fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.text()).unwrap()
    }
    fn patch(&self) -> (DesktopPatch, WidgetPatch) {
        let mut patch = DesktopPatch::new();
        patch.set(key("anchor"), Some(raw("Bottom")), Some(raw("Left")));
        patch.set(key("opacity"), Some(raw("0.8")), Some(raw("0.5")));
        let mut widget = WidgetPatch::new();
        widget
            .set("/theme", Some(json!("teal")), Some(json!("rose")))
            .unwrap();
        widget
            .set("/appearance/opacity", Some(json!(90)), Some(json!(75)))
            .unwrap();
        widget
            .set("/glance/enabled", None, Some(json!(true)))
            .unwrap();
        (patch, widget)
    }
}

#[test]
fn checked_merge_dedupes_identical_parts_and_rejects_every_disagreement() {
    let mut base = DesktopPatch::new();
    base.set(key("anchor"), Some(raw("Bottom")), Some(raw("Left")));
    base.guard(key("size"), Some(raw("L")));

    let mut same = DesktopPatch::new();
    same.set(key("anchor"), Some(raw("Bottom")), Some(raw("Left")));
    same.guard(key("size"), Some(raw("L")));
    same.set(key("margin"), None, Some(raw("4")));
    let mut merged = base.clone();
    merged.merge(same).unwrap();
    assert_eq!(merged.len(), 2, "identical write kept once");
    assert_eq!(merged.guards().count(), 1, "identical guard kept once");

    let cases: Vec<(&str, DesktopPatch)> = vec![
        ("different desired value", {
            let mut p = DesktopPatch::new();
            p.set(key("anchor"), Some(raw("Bottom")), Some(raw("Top")));
            p
        }),
        ("different original", {
            let mut p = DesktopPatch::new();
            p.set(key("anchor"), None, Some(raw("Left")));
            p
        }),
        ("guard disagrees with written original", {
            let mut p = DesktopPatch::new();
            p.guard(key("anchor"), Some(raw("Top")));
            p
        }),
        ("guard on a key the other part writes", {
            let mut p = DesktopPatch::new();
            p.guard(key("anchor"), Some(raw("Bottom")));
            p
        }),
        ("write over the other part's dependency", {
            let mut p = DesktopPatch::new();
            p.set(key("size"), Some(raw("L")), Some(raw("M")));
            p
        }),
        ("conflicting guards", {
            let mut p = DesktopPatch::new();
            p.guard(key("size"), Some(raw("S")));
            p
        }),
    ];
    for (name, other) in cases {
        let mut target = base.clone();
        let error = target.merge(other).unwrap_err();
        assert!(
            matches!(error, DesktopError::Overlap(_)),
            "{name}: {error:?}"
        );
        assert_eq!(target, base, "{name}: nothing was combined");
    }

    // Widget patches: identical members dedupe, different ones overlap.
    let mut first = WidgetPatch::new();
    first
        .set("/theme", Some(json!("teal")), Some(json!("rose")))
        .unwrap();
    let mut second = WidgetPatch::new();
    second
        .set("/theme", Some(json!("teal")), Some(json!("rose")))
        .unwrap();
    first.merge(second).unwrap();
    let mut third = WidgetPatch::new();
    third
        .set("/theme", Some(json!("teal")), Some(json!("amber")))
        .unwrap();
    assert_eq!(
        first.merge(third).unwrap_err(),
        vec!["Card palette".to_string()]
    );
}

#[test]
fn widget_patches_accept_only_allowlisted_members_and_valid_values() {
    let mut patch = WidgetPatch::new();
    for pointer in [
        "/location",
        "/project_roots",
        "/refresh_intervals/unknown",
        "/../x",
        "/appearance/match_wallpaper",
    ] {
        assert!(
            patch.set(pointer, None, Some(json!(1))).is_err(),
            "{pointer}"
        );
    }
    assert!(
        patch
            .set("/appearance/opacity", None, Some(json!(150)))
            .is_err()
    );
    assert!(patch.set("/theme", None, Some(json!("neon"))).is_err());
    assert!(
        patch
            .set("/widget_positions/clock", None, Some(json!([0.5, 1.5])))
            .is_err()
    );
    assert!(
        patch
            .set("/enabled_widgets", None, Some(json!(["clock", "clock"])))
            .is_err()
    );
    assert!(patch.is_empty());
}

#[test]
fn owned_apply_survives_restart_and_undo_restores_exact_bytes_once() {
    let case = Case::new("owned-restart");
    let before_config = case.text();
    let before_tree = {
        let mut tree = case.fixture.tree();
        tree.retain(|path, _| path.starts_with(case.fixture.0.join("config")));
        tree
    };
    let (patch, widget) = case.patch();
    let latest = case
        .store()
        .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile “Glass”")
        .unwrap();
    assert_eq!(case.fixture.read(DOCK, "anchor").as_deref(), Some("Left"));
    let after = case.json();
    assert_eq!(after["theme"], "rose");
    assert_eq!(after["glance"]["enabled"], true);
    assert_eq!(
        after["location"]["name"], "SENTINEL-CITY",
        "excluded member kept"
    );
    assert_eq!(
        after["future_setting"],
        json!({"keep": true}),
        "unknown member kept"
    );
    assert!(!case.fixture.0.join("state").join(JOURNAL).exists());

    // A new process: the record is read back and validated.
    let restarted = case.store();
    let saved = restarted.latest_change().unwrap().unwrap();
    assert_eq!(saved, latest);
    assert_eq!(saved.owner, Owner::Profile);
    assert_eq!(saved.label, "Profile “Glass”");
    let mode = fs::metadata(case.fixture.0.join("state").join(UNDO))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);

    // A stale id never undoes the current record.
    assert!(matches!(
        restarted.undo_latest("20200101T000000.000-1-1"),
        Err(DesktopError::UndoUnavailable(_))
    ));
    restarted.undo_latest(&saved.id).unwrap();
    assert_eq!(case.text(), before_config, "original bytes restored");
    let mut tree = case.fixture.tree();
    tree.retain(|path, _| path.starts_with(case.fixture.0.join("config")));
    assert_eq!(tree, before_tree);
    assert_eq!(restarted.latest_change().unwrap(), None, "record cleared");
    assert!(matches!(
        restarted.undo_latest(&saved.id),
        Err(DesktopError::UndoUnavailable(_))
    ));
}

#[test]
fn undo_keeps_newer_unrelated_settings_and_refuses_touched_ones() {
    let case = Case::new("owned-external");
    let (patch, widget) = case.patch();
    let store = case.store();
    let latest = store
        .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
        .unwrap();

    // Someone changes an unrelated member (here through the shared writer).
    let mut value = case.json();
    value["location"]["name"] = json!("NEWER-CITY");
    config::write_config_text(&case.config, Some(&config::config_text(&value).unwrap())).unwrap();
    store.undo_latest(&latest.id).unwrap();
    let restored = case.json();
    assert_eq!(restored["theme"], "teal");
    assert_eq!(restored["appearance"]["opacity"], 90);
    assert!(
        restored["glance"].get("enabled").is_none(),
        "absence restored"
    );
    assert_eq!(
        restored["location"]["name"], "NEWER-CITY",
        "newer unrelated edit kept"
    );
    assert_eq!(case.fixture.read(DOCK, "anchor").as_deref(), Some("Bottom"));

    // A touched member changed since: nothing at all is written.
    let latest = store
        .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
        .unwrap();
    let mut value = case.json();
    value["theme"] = json!("amber");
    config::write_config_text(&case.config, Some(&config::config_text(&value).unwrap())).unwrap();
    let before = (case.text(), case.fixture.read(DOCK, "anchor"));
    match store.undo_latest(&latest.id) {
        Err(DesktopError::FieldConflict { fields, .. }) => {
            assert_eq!(fields, vec!["Card palette".to_string()]);
        }
        other => panic!("expected a field conflict, got {other:?}"),
    }
    assert_eq!((case.text(), case.fixture.read(DOCK, "anchor")), before);
    assert_eq!(store.latest_change().unwrap(), Some(latest), "record kept");
}

#[test]
fn failed_apply_or_record_write_rolls_back_both_parts_and_keeps_previous_undo() {
    for fault in [Fault::FailWidget, Fault::FailRecord] {
        let case = Case::new("owned-fail");
        let store = case.store();
        let mut first = DesktopPatch::new();
        first.set(key("opacity"), Some(raw("0.8")), Some(raw("0.6")));
        let previous = store
            .apply_owned(&first, None, Owner::DockPanel, "Dock & Panel")
            .unwrap();
        let config_before = case.text();
        let mut patch = DesktopPatch::new();
        patch.set(key("anchor"), Some(raw("Bottom")), Some(raw("Left")));
        let mut widget = WidgetPatch::new();
        widget
            .set("/theme", Some(json!("teal")), Some(json!("rose")))
            .unwrap();
        store.inject(Some(fault.clone()));
        let error = store
            .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
            .unwrap_err();
        assert!(
            matches!(error, DesktopError::RolledBack(_)),
            "{fault:?}: {error:?}"
        );
        store.inject(None);
        assert_eq!(case.fixture.read(DOCK, "anchor").as_deref(), Some("Bottom"));
        assert_eq!(
            case.text(),
            config_before,
            "{fault:?}: widget file restored"
        );
        assert!(!case.fixture.0.join("state").join(JOURNAL).exists());
        assert_eq!(
            store.latest_change().unwrap(),
            Some(previous),
            "{fault:?}: a failed Apply keeps the previous Undo"
        );
    }
}

#[test]
fn interrupted_combined_change_recovers_after_restart_at_every_boundary() {
    let boundaries = [
        ("journal written, nothing else", Fault::InterruptAfter(0)),
        ("first key written", Fault::InterruptAfter(1)),
        ("keys written, widget not yet", Fault::InterruptBeforeWidget),
        ("both written, no Undo record", Fault::InterruptBeforeRecord),
        (
            "Undo record written, journal kept",
            Fault::InterruptAfterRecord,
        ),
    ];
    for (name, fault) in boundaries {
        let case = Case::new("owned-interrupt");
        let config_before = case.text();
        let (patch, widget) = case.patch();
        let store = case.store();
        store.inject(Some(fault));
        assert!(
            store
                .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
                .is_err()
        );
        drop(store);

        let restarted = case.store();
        let journal = restarted.pending_recovery().unwrap().expect(name);
        assert_eq!(journal.version, 3, "{name}");
        assert!(journal.widget.is_some(), "{name}");
        assert_eq!(
            journal.undo,
            Some(UndoBefore { record: None }),
            "{name}: there was no record before"
        );
        assert_eq!(
            restarted.apply_owned(&patch, Some(&widget), Owner::Profile, "Profile"),
            Err(DesktopError::RecoveryPending),
            "{name}"
        );
        let report = restarted.recover().unwrap();
        assert_eq!(
            case.fixture.read(DOCK, "anchor").as_deref(),
            Some("Bottom"),
            "{name}"
        );
        assert_eq!(
            case.fixture.read(DOCK, "opacity").as_deref(),
            Some("0.8"),
            "{name}"
        );
        assert_eq!(case.text(), config_before, "{name}: original bytes");
        assert!(
            report.kept_external.is_empty() && report.kept_fields.is_empty(),
            "{name}"
        );
        assert_eq!(restarted.pending_recovery().unwrap(), None, "{name}");
        assert_eq!(restarted.latest_change().unwrap(), None, "{name}");
    }
}

/// A prior successful owned change A (Dock opacity and card opacity), with
/// its exact Undo record bytes and the configuration text it left.
fn apply_a(case: &Case) -> (LatestChange, Vec<u8>, String) {
    let mut patch = DesktopPatch::new();
    patch.set(key("opacity"), Some(raw("0.8")), Some(raw("0.6")));
    let mut widget = WidgetPatch::new();
    widget
        .set("/appearance/opacity", Some(json!(90)), Some(json!(80)))
        .unwrap();
    let a = case
        .store()
        .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile “A”")
        .unwrap();
    (a, fs::read(case.undo()).unwrap(), case.text())
}

/// A later owned change B on other keys and members than A.
fn change_b() -> (DesktopPatch, WidgetPatch) {
    let mut patch = DesktopPatch::new();
    patch.set(key("anchor"), Some(raw("Bottom")), Some(raw("Left")));
    let mut widget = WidgetPatch::new();
    widget
        .set("/theme", Some(json!("teal")), Some(json!("rose")))
        .unwrap();
    (patch, widget)
}

/// Desktop A is back exactly and A's Undo still works after a restart.
fn assert_a_restored(case: &Case, a: &LatestChange, a_record: &[u8], a_config: &str, name: &str) {
    assert_eq!(
        case.fixture.read(DOCK, "anchor").as_deref(),
        Some("Bottom"),
        "{name}"
    );
    assert_eq!(
        case.fixture.read(DOCK, "opacity").as_deref(),
        Some("0.6"),
        "{name}"
    );
    assert_eq!(case.text(), a_config, "{name}: A's configuration bytes");
    assert_eq!(
        fs::read(case.undo()).ok().as_deref(),
        Some(a_record),
        "{name}: A's Undo record bytes"
    );
    let restarted = case.store();
    assert_eq!(restarted.pending_recovery().unwrap(), None, "{name}");
    assert_eq!(
        restarted.latest_change().unwrap().as_ref(),
        Some(a),
        "{name}"
    );
    restarted.undo_latest(&a.id).unwrap();
    assert_eq!(
        case.fixture.read(DOCK, "opacity").as_deref(),
        Some("0.8"),
        "{name}"
    );
    assert_eq!(restarted.latest_change().unwrap(), None, "{name}");
}

#[test]
fn interrupted_replacement_of_a_previous_undo_restores_it_on_recovery() {
    for (name, fault) in [
        ("B stopped before its record", Fault::InterruptBeforeRecord),
        (
            "B stopped after replacing A's record",
            Fault::InterruptAfterRecord,
        ),
    ] {
        let case = Case::new("owned-replace-interrupt");
        let original = case.text();
        let (a, a_record, a_config) = apply_a(&case);
        let (patch, widget) = change_b();
        let store = case.store();
        store.inject(Some(fault));
        assert!(
            store
                .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile “B”")
                .is_err(),
            "{name}"
        );
        drop(store);
        case.store().recover().unwrap();
        assert_a_restored(&case, &a, &a_record, &a_config, name);
        assert_eq!(case.text(), original, "{name}: A undone exactly");
    }
}

#[test]
fn interrupted_undo_after_clearing_its_record_restores_it_on_recovery() {
    let case = Case::new("owned-undo-interrupt");
    let original = case.text();
    let (a, a_record, a_config) = apply_a(&case);
    let store = case.store();
    store.inject(Some(Fault::InterruptAfterRecord));
    assert!(store.undo_latest(&a.id).is_err());
    drop(store);
    // Everything of the Undo was written before the interruption.
    assert_eq!(case.fixture.read(DOCK, "opacity").as_deref(), Some("0.8"));
    assert_eq!(case.text(), original);
    assert!(case.store().latest_change().unwrap().is_none());

    case.store().recover().unwrap();
    assert_a_restored(&case, &a, &a_record, &a_config, "Undo of A");
    assert_eq!(case.text(), original, "A undone exactly");
}

#[test]
fn keep_current_retains_the_record_the_interrupted_change_left() {
    // Apply B stopped after replacing A's record: B stays, with its Undo.
    let case = Case::new("owned-keep-apply");
    let (_, a_record, _) = apply_a(&case);
    let (patch, widget) = change_b();
    let store = case.store();
    store.inject(Some(Fault::InterruptAfterRecord));
    assert!(
        store
            .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile “B”")
            .is_err()
    );
    let b_record = fs::read(case.undo()).unwrap();
    assert_ne!(b_record, a_record);
    case.store().discard_recovery().unwrap();
    assert_eq!(fs::read(case.undo()).unwrap(), b_record, "B's record kept");
    let restarted = case.store();
    let b = restarted.latest_change().unwrap().unwrap();
    assert_eq!(
        (b.label.as_str(), case.json()["theme"].clone()),
        ("Profile “B”", json!("rose"))
    );
    restarted.undo_latest(&b.id).unwrap();
    assert_eq!(case.fixture.read(DOCK, "anchor").as_deref(), Some("Bottom"));
    assert_eq!(case.json()["theme"], "teal");

    // Undo of A stopped after clearing A's record: A stays undone, no record.
    let case = Case::new("owned-keep-undo");
    let original = case.text();
    let (a, _, _) = apply_a(&case);
    let store = case.store();
    store.inject(Some(Fault::InterruptAfterRecord));
    assert!(store.undo_latest(&a.id).is_err());
    case.store().discard_recovery().unwrap();
    assert!(
        case.store().latest_change().unwrap().is_none(),
        "the cleared action is not brought back"
    );
    assert_eq!(case.text(), original);
    assert_eq!(case.store().latest_change().unwrap(), None);
}

#[test]
fn recovery_puts_back_damaged_records_as_they_were_and_keeps_newer_ones() {
    // A damaged or future previous record is kept as inert bytes and put
    // back exactly; it is still never offered.
    for (name, bytes) in [
        ("damaged", b"{not json".to_vec()),
        (
            "future",
            br#"{"format":"desktop-cards-undo","version":9,"id":"x"}"#.to_vec(),
        ),
    ] {
        let case = Case::new("owned-damaged-previous");
        fs::create_dir_all(case.undo().parent().unwrap()).unwrap();
        fs::write(case.undo(), &bytes).unwrap();
        let (patch, widget) = change_b();
        let store = case.store();
        store.inject(Some(Fault::InterruptAfterRecord));
        assert!(
            store
                .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile “B”")
                .is_err()
        );
        let report = case.store().recover().unwrap();
        assert!(!report.kept_undo, "{name}");
        assert_eq!(fs::read(case.undo()).unwrap(), bytes, "{name}");
        assert!(case.store().latest_change().is_err(), "{name}: not offered");
        assert_eq!(case.fixture.read(DOCK, "anchor").as_deref(), Some("Bottom"));
    }

    // A record that is neither what the change found nor what it left is
    // newer: recovery reverts the settings but keeps that record.
    for operation in ["apply", "undo"] {
        let case = Case::new("owned-newer-record");
        let (a, _, a_config) = apply_a(&case);
        let store = case.store();
        store.inject(Some(Fault::InterruptAfterRecord));
        if operation == "apply" {
            let (patch, widget) = change_b();
            assert!(
                store
                    .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile “B”")
                    .is_err()
            );
        } else {
            assert!(store.undo_latest(&a.id).is_err());
        }
        let newer = br#"{"format":"desktop-cards-undo","version":1,"id":"newer"}"#;
        fs::write(case.undo(), newer).unwrap();
        let report = case.store().recover().unwrap();
        assert!(report.kept_undo, "{operation}");
        assert_eq!(fs::read(case.undo()).unwrap(), newer, "{operation}");
        assert_eq!(case.text(), a_config, "{operation}: settings reverted");
        assert_eq!(case.store().pending_recovery().unwrap(), None);
    }
}

#[test]
fn a_record_that_cannot_be_kept_safely_refuses_the_change_before_any_write() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("not text", vec![0xff, 0xfe, b'{']),
        // Every quote doubles when nested in the journal: the planned
        // journal would exceed what recovery reads back.
        ("too large to journal", vec![b'"'; 5 * 1024 * 1024]),
    ];
    for (name, bytes) in cases {
        let case = Case::new("owned-unsafe-record");
        fs::create_dir_all(case.undo().parent().unwrap()).unwrap();
        fs::write(case.undo(), &bytes).unwrap();
        let before = case.fixture.tree();
        let (patch, widget) = change_b();
        let error = case
            .store()
            .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile “B”")
            .unwrap_err();
        let explained = match name {
            "not text" => matches!(error, DesktopError::UndoUnavailable(_)),
            _ => matches!(error, DesktopError::InvalidValue { .. }),
        };
        assert!(explained, "{name}: {error:?}");
        let mut after = case.fixture.tree();
        // Only the lock files may have been created.
        after.retain(|path, _| !path.ends_with(LOCK) && !path.ends_with("config.json.lock"));
        assert_eq!(after, before, "{name}: nothing written");
    }
}

#[test]
fn recovery_restores_only_our_widget_members_when_others_changed_since() {
    let case = Case::new("owned-recover-external");
    let (patch, widget) = case.patch();
    let store = case.store();
    store.inject(Some(Fault::InterruptBeforeRecord));
    assert!(
        store
            .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
            .is_err()
    );
    // After the interruption: an unrelated member and one touched member
    // change elsewhere.
    let mut value = case.json();
    value["location"]["name"] = json!("NEWER-CITY");
    value["appearance"]["opacity"] = json!(80);
    config::write_config_text(&case.config, Some(&config::config_text(&value).unwrap())).unwrap();
    let report = case.store().recover().unwrap();
    let now = case.json();
    assert_eq!(now["theme"], "teal", "ours: restored");
    assert!(
        now["glance"].get("enabled").is_none(),
        "ours: absence restored"
    );
    assert_eq!(now["appearance"]["opacity"], 80, "newer touched value kept");
    assert_eq!(now["location"]["name"], "NEWER-CITY", "unrelated kept");
    assert_eq!(report.kept_fields, vec!["Card opacity".to_string()]);
    assert_eq!(report.restored_fields.len(), 2);
}

#[test]
fn an_older_v2_journal_whose_record_was_written_still_clears_that_record() {
    // Version 2 journals (from before the record joined the transaction)
    // cannot name the record they replaced; they recover as before.
    let case = Case::new("owned-record-then-crash");
    let (patch, widget) = case.patch();
    let store = case.store();
    let latest = store
        .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
        .unwrap();
    // Simulate the crash between the record and journal removal by putting
    // the journal of that very transaction back.
    let journal = Journal {
        format: JOURNAL_FORMAT.into(),
        version: 2,
        id: latest.id.clone(),
        operation: "apply".into(),
        created: latest.created.clone(),
        entries: latest
            .change
            .entries()
            .iter()
            .map(JournalEntry::from)
            .collect(),
        widget: latest.widget.clone(),
        undo: None,
    };
    fs::write(
        case.fixture.0.join("state").join(JOURNAL),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    store.recover().unwrap();
    assert_eq!(case.fixture.read(DOCK, "anchor").as_deref(), Some("Bottom"));
    assert_eq!(store.latest_change().unwrap(), None);
}

#[test]
fn damaged_future_or_forged_records_are_explained_and_never_used() {
    let case = Case::new("owned-records");
    let store = case.store();
    let state = case.fixture.0.join("state");
    fs::create_dir_all(&state).unwrap();
    let record = |entries: serde_json::Value, widget: serde_json::Value, version: u32| {
        json!({
            "format": "desktop-cards-undo", "version": version, "id": "20260101T000000.000-1-1",
            "owner": "profile", "label": "Profile", "created": "now",
            "entries": entries, "widget": widget,
        })
    };
    let dock = json!([{"namespace": DOCK, "version": 1, "key": "anchor", "before": "Top", "after": "Bottom"}]);
    let forged_key = json!([{"namespace": "com.system76.CosmicComp", "version": 1, "key": "cursor_shake_to_find", "before": "true", "after": "false"}]);
    let forged_field = json!({"before": null, "after": "{}", "fields": [{"pointer": "/credentials", "before": null, "after": ["/"]}]});
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("garbage", b"{not json".to_vec()),
        (
            "future version",
            serde_json::to_vec(&record(dock.clone(), json!(null), 2)).unwrap(),
        ),
        (
            "forged key",
            serde_json::to_vec(&record(forged_key, json!(null), 1)).unwrap(),
        ),
        (
            "forged widget member",
            serde_json::to_vec(&record(dock.clone(), forged_field, 1)).unwrap(),
        ),
        (
            "profile bytes",
            br#"{"format":"desktop-cards-profile","version":1,"name":"x"}"#.to_vec(),
        ),
    ];
    for (name, bytes) in cases {
        fs::write(state.join(UNDO), &bytes).unwrap();
        assert!(store.latest_change().is_err(), "{name}");
        assert!(
            store.undo_latest("20260101T000000.000-1-1").is_err(),
            "{name}"
        );
        assert_eq!(
            fs::read(state.join(UNDO)).unwrap(),
            bytes,
            "{name}: kept as is"
        );
        assert_eq!(
            case.fixture.read(DOCK, "anchor").as_deref(),
            Some("Bottom"),
            "{name}"
        );
    }
    // The next successful owned change replaces the unusable record.
    let mut patch = DesktopPatch::new();
    patch.set(key("opacity"), Some(raw("0.8")), Some(raw("0.7")));
    let latest = store
        .apply_owned(&patch, None, Owner::DockPanel, "Dock & Panel")
        .unwrap();
    assert_eq!(store.latest_change().unwrap(), Some(latest));
}

#[test]
fn journals_naming_other_members_or_versions_are_refused_without_writes() {
    let case = Case::new("owned-journals");
    let store = case.store();
    let state = case.fixture.0.join("state");
    fs::create_dir_all(&state).unwrap();
    let before = case.text();
    let journal = |version: u32, widget: serde_json::Value| {
        serde_json::to_vec(&json!({
            "format": JOURNAL_FORMAT, "version": version, "id": "x", "operation": "apply",
            "created": "", "entries": [], "widget": widget,
        }))
        .unwrap()
    };
    let forged = json!({"before": "{}", "after": "{\"project_roots\":[\"/\"]}", "fields": [{"pointer": "/credentials", "before": null, "after": ["/"]}]});
    let valid_widget = json!({"before": null, "after": "{}", "fields": [{"pointer": "/theme", "before": null, "after": "rose"}]});
    let owned = |operation: &str, undo: serde_json::Value| {
        serde_json::to_vec(&json!({
            "format": JOURNAL_FORMAT, "version": 3, "id": "x", "operation": operation,
            "created": "", "entries": [{"namespace": DOCK, "version": 1, "key": "anchor", "before": "Top", "after": "Bottom"}],
            "undo": undo,
        }))
        .unwrap()
    };
    // A record the recovery must not touch while refusing.
    let record = br#"{"format":"desktop-cards-undo","version":1}"#;
    fs::write(state.join(UNDO), record).unwrap();
    for (name, bytes) in [
        ("forged member", journal(2, forged)),
        ("widget in a v1 journal", journal(1, valid_widget.clone())),
        ("future version", journal(4, valid_widget.clone())),
        ("owned journal without its record", journal(3, valid_widget)),
        ("record in a v1 journal", {
            let mut value: serde_json::Value =
                serde_json::from_slice(&journal(1, json!(null))).unwrap();
            value["undo"] = json!({"record": null});
            serde_json::to_vec(&value).unwrap()
        }),
        ("malformed record", owned("apply", json!({"record": 7}))),
        (
            "unknown operation",
            owned("replace", json!({"record": null})),
        ),
        (
            "undo that found no record",
            owned("undo", json!({"record": null})),
        ),
    ] {
        fs::write(state.join(JOURNAL), bytes).unwrap();
        assert!(store.pending_recovery().is_err(), "{name}");
        assert!(store.recover().is_err(), "{name}");
        assert_eq!(case.text(), before, "{name}");
        assert_eq!(
            case.fixture.read(DOCK, "anchor").as_deref(),
            Some("Bottom"),
            "{name}"
        );
        assert_eq!(
            fs::read(state.join(UNDO)).unwrap(),
            record,
            "{name}: record untouched"
        );
        assert!(
            state.join(JOURNAL).exists(),
            "{name}: kept for the user to decide"
        );
    }
    // A widget journal without a trusted configuration path is refused.
    let widget = json!({"before": null, "after": "{}", "fields": [{"pointer": "/theme", "before": null, "after": "rose"}]});
    fs::write(state.join(JOURNAL), journal(2, widget)).unwrap();
    assert!(case.fixture.store().recover().is_err());
}

#[test]
fn widget_changes_need_a_trusted_configuration_and_plain_apply_keeps_the_record() {
    let case = Case::new("owned-context");
    let (patch, widget) = case.patch();
    let before = case.fixture.tree();
    let bare = case.fixture.store();
    assert!(matches!(
        bare.apply_owned(&patch, Some(&widget), Owner::Profile, "Profile"),
        Err(DesktopError::ReadOnly(_))
    ));
    assert_eq!(case.fixture.tree(), before, "nothing created");
    assert!(matches!(
        case.store().read_only("Preview").apply_owned(
            &patch,
            Some(&widget),
            Owner::Profile,
            "Profile"
        ),
        Err(DesktopError::ReadOnly(_))
    ));
    assert_eq!(case.fixture.tree(), before);

    let store = case.store();
    let latest = store
        .apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
        .unwrap();
    // The palette helper's plain apply neither replaces nor clears it.
    let mut palette = DesktopPatch::new();
    palette.set(key("margin"), None, Some(raw("4")));
    store.apply(&palette).unwrap();
    assert_eq!(store.latest_change().unwrap(), Some(latest.clone()));
    // Nor does it take part in the record when interrupted: its journal
    // stays version 1 and recovery leaves the record alone.
    let mut palette = DesktopPatch::new();
    palette.set(key("margin"), Some(raw("4")), Some(raw("8")));
    store.inject(Some(Fault::InterruptAfter(0)));
    assert!(store.apply(&palette).is_err());
    store.inject(None);
    let journal = store.pending_recovery().unwrap().unwrap();
    assert_eq!((journal.version, journal.undo), (1, None));
    store.recover().unwrap();
    assert_eq!(case.fixture.read(DOCK, "margin").as_deref(), Some("4"));
    assert_eq!(store.latest_change().unwrap(), Some(latest));
}

#[test]
fn an_owned_widget_change_waits_for_the_shared_configuration_lock() {
    let case = Case::new("owned-lock");
    let (patch, widget) = case.patch();
    let held = config::lock_config(&case.config).unwrap();
    let store = case.store();
    let config_path = case.config.clone();
    let worker = std::thread::spawn(move || {
        store.apply_owned(&patch, Some(&widget), Owner::Profile, "Profile")
    });
    std::thread::sleep(std::time::Duration::from_millis(300));
    let journal = case.fixture.0.join("state").join(JOURNAL);
    assert!(
        !journal.exists(),
        "nothing journaled while the controller holds the lock"
    );
    assert!(!fs::read_to_string(&config_path).unwrap().contains("rose"));
    assert_eq!(case.fixture.read(DOCK, "anchor").as_deref(), Some("Bottom"));
    drop(held);
    worker.join().unwrap().unwrap();
    assert!(fs::read_to_string(&config_path).unwrap().contains("rose"));
}
