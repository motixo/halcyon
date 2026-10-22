use super::*;
use crate::desktop::store::Owner;
use crate::desktop::store::tests::Fixture;
use crate::desktop::windows::{Behavior, Choice, WindowsDraft};

const COMP: &str = "com.system76.CosmicComp";
/// The shape COSMIC writes today (`r#None` is RON's raw identifier).
const CURRENT: &str = "(workspace_mode:OutputBound,workspace_layout:Horizontal,action_on_typing:r#None,workspace_wraparound:true)";

/// The struct is decoded exactly and rewritten whole from that value:
/// unknown variants, extra members and malformed text are shown as they are
/// and never written; a partial legacy struct takes the serde defaults; the
/// pinned default is used only when no layer defines the key. Windows edits
/// stay applicable meanwhile.
#[test]
fn workspace_future_members_are_preserved() {
    let fixture = Fixture::new("workspaces-current");
    fixture.user(COMP, "workspaces", CURRENT);
    fixture.user(
        COMP,
        "pinned_workspaces",
        "[(output:(name:\"DP-1\",edid:None),tiling_enabled:true,id:Some(\"w1\"),name:None)]",
    );
    let store = fixture.store();
    let mut draft = WorkspacesDraft::load(&store).unwrap();
    assert_eq!(
        draft.config(),
        Setting::Known(WorkspaceConfig {
            workspace_mode: WorkspaceMode::OutputBound,
            workspace_layout: WorkspaceLayout::Horizontal,
            action_on_typing: TypingAction::None,
            workspace_wraparound: true,
        })
    );
    draft.set_wraparound(false);
    draft.set_layout(WorkspaceLayout::Horizontal);
    assert_eq!(
        draft.edits().workspace_layout,
        None,
        "the current value is no edit"
    );
    store
        .apply_owned(&draft.plan().patch, None, Owner::Workspaces, "Workspaces")
        .unwrap();
    let written = RawValue::parse(fixture.read(COMP, "workspaces").unwrap()).unwrap();
    assert_eq!(
        decode_exact::<WorkspaceConfig>(&written).unwrap(),
        WorkspaceConfig {
            workspace_mode: WorkspaceMode::OutputBound,
            workspace_layout: WorkspaceLayout::Horizontal,
            action_on_typing: TypingAction::None,
            workspace_wraparound: false,
        },
        "the other three members keep their values"
    );
    assert!(
        fixture
            .read(COMP, "pinned_workspaces")
            .unwrap()
            .contains("w1")
    );

    // Absent from every layer: the pinned default.
    let empty = Fixture::new("workspaces-default");
    assert_eq!(
        WorkspacesDraft::load(&empty.store()).unwrap().config(),
        Setting::Known(WorkspaceConfig::default())
    );
    let default = WorkspaceConfig::default();
    assert_eq!(
        (
            default.workspace_mode,
            default.workspace_layout,
            default.action_on_typing,
            default.workspace_wraparound
        ),
        (
            WorkspaceMode::OutputBound,
            WorkspaceLayout::Vertical,
            TypingAction::None,
            true
        )
    );

    // A partial legacy struct: the missing members take serde's defaults.
    let legacy = Fixture::new("workspaces-legacy");
    legacy.user(COMP, "workspaces", "(workspace_mode:Global)");
    let mut draft = WorkspacesDraft::load(&legacy.store()).unwrap();
    assert_eq!(
        draft.config(),
        Setting::Known(WorkspaceConfig {
            workspace_mode: WorkspaceMode::Global,
            ..WorkspaceConfig::default()
        })
    );
    draft.set_typing(TypingAction::OpenLauncher);
    let plan = draft.plan();
    assert!(plan.blocked.is_empty());
    let value = plan.patch.get(&key()).unwrap().desired.clone().unwrap();
    assert_eq!(
        decode_exact::<WorkspaceConfig>(&value)
            .unwrap()
            .workspace_mode,
        WorkspaceMode::Global
    );

    // Unknown variant, extra member, malformed text, and a system default
    // with an unknown member: kept, never rewritten; Windows still applies.
    for (name, text, system) in [
        (
            "variant",
            "(workspace_mode:Diagonal,workspace_layout:Vertical)",
            false,
        ),
        (
            "member",
            "(workspace_mode:OutputBound,workspace_layout:Vertical,action_on_typing:OpenLauncher,workspace_wraparound:true,future_member:(depth:2))",
            false,
        ),
        ("malformed", "(workspace_mode: OutputBound,", false),
        (
            "system",
            "(workspace_mode:OutputBound,future_member:1)",
            true,
        ),
    ] {
        let fixture = Fixture::new(&format!("workspaces-{name}"));
        if system {
            fixture.system(COMP, "workspaces", text);
        } else {
            fixture.user(COMP, "workspaces", text);
        }
        let store = fixture.store();
        let before = fixture.tree();
        let mut draft = WorkspacesDraft::load(&store).unwrap();
        assert!(
            matches!(draft.config(), Setting::Unsupported(ref raw) if raw == text.trim()),
            "{name}"
        );
        draft.set_layout(WorkspaceLayout::Horizontal);
        draft.set_wraparound(false);
        let plan = draft.plan();
        assert!(plan.patch.is_empty(), "{name}: nothing to write");
        assert_eq!(plan.blocked.len(), 1, "{name}");
        assert_eq!(fixture.tree(), before, "{name}");
        // Its validator refuses the same text for any writer.
        if let Ok(raw) = RawValue::parse(text) {
            assert!((RULES[0].validate)(&raw).is_err(), "{name}");
        }
        let mut windows = WindowsDraft::load(&store).unwrap();
        windows.set(Behavior::FocusFollowsCursor, Choice::Flag(true));
        store
            .apply_owned(&windows.plan().patch, None, Owner::Windows, "Windows")
            .unwrap();
        assert_eq!(
            fixture.read(COMP, "focus_follows_cursor").as_deref(),
            Some("true")
        );
        if !system {
            assert_eq!(fixture.read(COMP, "workspaces").as_deref(), Some(text));
        }
    }
}

/// Written the way COSMIC writes it: the `None` action as RON's raw
/// identifier, so the compositor's own reader takes it as the variant.
#[test]
fn workspaces_are_encoded_like_cosmic() {
    let text = RawValue::encode(&WorkspaceConfig::default()).unwrap();
    assert!(
        text.as_str().contains("action_on_typing: r#None"),
        "{}",
        text.as_str()
    );
    let expected = WorkspaceConfig {
        workspace_layout: WorkspaceLayout::Horizontal,
        ..WorkspaceConfig::default()
    };
    assert_eq!(
        decode_exact::<WorkspaceConfig>(&RawValue::parse(CURRENT).unwrap()).unwrap(),
        expected
    );
}
