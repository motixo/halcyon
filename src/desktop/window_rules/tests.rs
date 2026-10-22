//! Floating-app rules: upstream's exact-pair merge, the compositor's regex
//! semantics, default overrides and what is never rewritten.
use super::*;
use crate::desktop::Owner;
use crate::desktop::store::rule_for;
use crate::desktop::store::tests::Fixture;

/// cosmic-comp's `has_floating_exception`: one regex set per field, and a
/// rule applies when the same index matches both.
fn floats(rows: &[RuleRow], app: &str, title: &str) -> bool {
    let rules: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.enabled && Regex::new(&row.appid).is_ok() && Regex::new(&row.title).is_ok()
        })
        .collect();
    let apps = RegexSet::new(rules.iter().map(|row| &row.appid)).unwrap();
    let titles = RegexSet::new(rules.iter().map(|row| &row.title)).unwrap();
    let titled = titles.matches(title);
    apps.matches(app).into_iter().any(|at| titled.matched(at))
}

/// Defaults expand in order and custom entries override exact pairs; new
/// literal rules match only what they say; patterns stay patterns; the
/// defaults are never written and changes to them are noticed.
#[test]
fn floating_rules_match_and_preserve_defaults() {
    let fixture = Fixture::new("window-rules");
    fixture.system(
        NAMESPACE,
        "tiling_exception_defaults",
        r#"[
    // Any app, by title
    (appid: ".*", titles: ["Discord Updater"]),
    (appid: "Authy Desktop", titles: [".*"]),
    (appid: "Gjs", titles: ["Settings", "Settings"]),
]"#,
    );
    let custom = r#"[(appid: "Authy Desktop", title: ".*", enabled: false), (appid: "(sentinel|private)-.*", title: "Private Title Sentinel", enabled: true)]"#;
    fixture.user(NAMESPACE, "tiling_exception_custom", custom);
    let store = fixture.store();
    let mut draft = WindowRulesDraft::load(&store).unwrap();
    let rows = draft.rows();
    let pairs: Vec<_> = rows
        .iter()
        .map(|row| (row.appid.as_str(), row.title.as_str(), row.enabled))
        .collect();
    assert_eq!(
        pairs,
        [
            (".*", "Discord Updater", true),
            ("Authy Desktop", ".*", false),
            ("Gjs", "Settings", true),
            ("Gjs", "Settings", true),
            ("(sentinel|private)-.*", "Private Title Sentinel", true),
        ]
    );
    assert!(rows[1].overridden && rows[1].default);
    assert!(!rows[4].default && rows[4].copies == 1);
    assert_eq!(rows[2].default_copies, 2);
    // Unanchored regexes on both fields, as the compositor matches them.
    assert!(floats(&rows, "anything", "Discord Updater - v2"));
    assert!(!floats(&rows, "Authy Desktop", "Main"), "overridden off");
    assert!(floats(&rows, "x-sentinel-y", "a Private Title Sentinel b"));
    assert!(!floats(&rows, "sentinel-y", "Other"));

    // Reading never guesses: only exact escaped literals read as text.
    assert_eq!(classify(".*"), Match::Any);
    assert_eq!(classify(""), Match::Any);
    assert_eq!(
        classify("^org\\.mozilla\\.firefox$"),
        Match::Exact("org.mozilla.firefox".into())
    );
    assert_eq!(
        classify("Authy Desktop"),
        Match::Contains("Authy Desktop".into())
    );
    assert_eq!(classify("(a|b)"), Match::Pattern("(a|b)".into()));
    assert_eq!(
        classify("wl-clipboard"),
        Match::Contains("wl-clipboard".into())
    );
    assert_eq!(
        classify("wl\\-clipboard"),
        Match::Contains("wl-clipboard".into())
    );
    assert_eq!(classify("a.b"), Match::Pattern("a.b".into()));
    assert_eq!(classify("\\bword"), Match::Pattern("\\bword".into()));
    assert_eq!(classify("^fire.*$"), Match::Pattern("^fire.*$".into()));

    // A literal app ID matches exactly that ID; a title literal is "contains".
    draft
        .save(
            None,
            CustomRule {
                appid: exact_app("org.example.App"),
                title: title_contains("Picture-in-Picture (1)"),
                enabled: true,
            },
        )
        .unwrap();
    let rows = draft.rows();
    assert!(floats(
        &rows,
        "org.example.App",
        "Video Picture-in-Picture (1)"
    ));
    assert!(!floats(
        &rows,
        "org.example.App-beta",
        "Picture-in-Picture (1)"
    ));
    assert!(!floats(&rows, "orgXexample.App", "Picture-in-Picture (1)"));
    assert!(!floats(&rows, "org.example.App", "Picture-in-Picture 1"));

    // Defaults: off writes an override, on removes it again.
    draft.set_enabled(".*", "Discord Updater", false).unwrap();
    assert!(
        draft
            .custom()
            .unwrap()
            .iter()
            .any(|rule| rule.title == "Discord Updater" && !rule.enabled)
    );
    draft.set_enabled(".*", "Discord Updater", true).unwrap();
    assert!(
        !draft
            .custom()
            .unwrap()
            .iter()
            .any(|rule| rule.title == "Discord Updater")
    );
    // A repeated default can't be switched off by one override.
    assert!(draft.set_enabled("Gjs", "Settings", false).is_err());
    // Restore a default by removing its override.
    draft.remove("Authy Desktop", ".*").unwrap();
    assert!(draft.rows()[1].enabled && !draft.rows()[1].overridden);

    // Duplicates, invalid patterns and match-everything are refused.
    let rule = |appid: &str, title: &str| CustomRule {
        appid: appid.into(),
        title: title.into(),
        enabled: true,
    };
    assert!(draft.save(None, rule("Gjs", "Settings")).is_err());
    assert!(
        draft
            .save(
                None,
                rule(
                    &exact_app("org.example.App"),
                    &title_contains("Picture-in-Picture (1)")
                )
            )
            .is_err()
    );
    assert!(
        draft
            .save(None, rule("(", ""))
            .unwrap_err()
            .contains("pattern")
    );
    assert!(draft.save(None, rule("", ".*")).is_err());
    // Editing a pattern rule keeps whatever pattern the form holds.
    draft
        .save(
            Some(("(sentinel|private)-.*", "Private Title Sentinel")),
            rule("(sentinel|private)-.*", "Other Private Title"),
        )
        .unwrap();

    let plan = draft.plan();
    let entry = plan.patch.get(&custom_key()).unwrap();
    let written = entry.desired.clone().unwrap();
    assert!((rule_for(&custom_key()).unwrap().validate)(&written).is_ok());
    let rules: Vec<CustomRule> = written.decode().unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(
        rules[0].appid, "(sentinel|private)-.*",
        "order and pattern kept"
    );
    assert_eq!(rules[1].appid, "^org\\.example\\.App$");
    assert!(
        plan.patch.get(&defaults_key()).is_none(),
        "defaults are never written"
    );
    assert!(plan.patch.guards().any(|(key, _)| *key == defaults_key()));
    assert!(rule_for(&defaults_key()).is_none());

    // A changed system default is noticed although the user layer didn't change.
    fixture.system(
        NAMESPACE,
        "tiling_exception_defaults",
        r#"[(appid: "Gjs", titles: ["Settings"])]"#,
    );
    let fresh = store.load(&WindowRulesDraft::keys()).unwrap();
    assert_eq!(draft.stale(&fresh), [defaults_key()]);
    let mut refreshed = draft.clone();
    refreshed.refresh(fresh);
    assert_eq!(refreshed.conflicts(), [defaults_key()]);
    assert!(refreshed.dirty(), "edits wait for a decision");

    // Apply and Undo restore the exact custom text.
    let draft = refreshed;
    let change = store
        .apply_owned(
            &draft.plan().patch,
            None,
            Owner::WindowRules,
            "Floating apps",
        )
        .unwrap();
    store.undo_latest(&change.id).unwrap();
    assert_eq!(
        fixture
            .read(NAMESPACE, "tiling_exception_custom")
            .as_deref(),
        Some(custom)
    );

    // Members this version doesn't know: COSMIC's reader skips them and
    // applies the rules, so they are listed as COSMIC applies them, and the
    // list is never rewritten (that would drop the unknown members).
    fixture.user(
        NAMESPACE,
        "tiling_exception_custom",
        r#"[(appid: "a", title: "", enabled: false, priority: 1)]"#,
    );
    let mut draft = WindowRulesDraft::load(&store).unwrap();
    let pairs: Vec<_> = draft
        .rows()
        .into_iter()
        .map(|row| (row.appid, row.title, row.enabled, row.default))
        .collect();
    assert_eq!(
        pairs,
        [
            ("Gjs".into(), "Settings".into(), true, true),
            ("a".into(), String::new(), false, false),
        ]
    );
    assert!(draft.save(None, rule("^b$", "")).is_err());
    assert!(draft.set_enabled("Gjs", "Settings", false).is_err());
    assert!(draft.set_enabled("a", "", true).is_err());
    assert!(draft.remove("a", "").is_err());
    assert!(!draft.dirty() && draft.plan().patch.is_empty());
    assert_eq!(draft.custom_state(), ListState::Unknown);
    assert!(!draft.editable());

    // Unreadable text: COSMIC ignores the list, so only the defaults apply.
    fixture.user(
        NAMESPACE,
        "tiling_exception_custom",
        r#"[(appid: "a", title: "", enabled: maybe)]"#,
    );
    let mut draft = WindowRulesDraft::load(&store).unwrap();
    assert!(draft.base_custom().is_err());
    assert!(matches!(draft.custom_state(), ListState::Unreadable(_)));
    assert!(draft.save(None, rule("^b$", "")).is_err());
    assert!(!draft.dirty());
    assert_eq!(draft.rows().len(), 1, "only the defaults apply");
    assert!(
        (rule_for(&custom_key()).unwrap().validate)(
            &RawValue::parse(r#"[(appid: "a", title: "", enabled: true, priority: 1)]"#).unwrap()
        )
        .is_err(),
        "this app never writes members it doesn't know"
    );

    // Defaults with unknown members are read as COSMIC reads them; custom
    // edits work and never write the defaults.
    fixture.user(NAMESPACE, "tiling_exception_custom", "[]");
    fixture.system(
        NAMESPACE,
        "tiling_exception_defaults",
        r#"[(appid: "Gjs", titles: ["Settings"], source: "vendor")]"#,
    );
    let mut draft = WindowRulesDraft::load(&store).unwrap();
    assert_eq!(draft.rows().len(), 1, "{:?}", draft.rows());
    assert_eq!(draft.defaults_state(), ListState::Unknown);
    assert!(draft.editable());
    draft.set_enabled("Gjs", "Settings", false).unwrap();
    let plan = draft.plan();
    assert!(plan.blocked.is_empty(), "{:?}", plan.blocked);
    assert!(plan.patch.get(&defaults_key()).is_none());
    assert!(plan.patch.get(&custom_key()).is_some());
}
