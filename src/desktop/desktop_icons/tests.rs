use super::*;
use crate::desktop::store::tests::Fixture;

fn stored(fixture: &Fixture) -> DesktopIcons {
    RawValue::parse(fixture.read(NAMESPACE, "desktop").unwrap())
        .unwrap()
        .decode()
        .unwrap()
}

#[test]
fn icons_rewrite_only_edited_members_and_refuse_unknown() {
    let fixture = Fixture::new("icons-partial");
    fixture.user(NAMESPACE, "desktop", "(icon_size: 150)");
    let store = fixture.store();
    let mut draft = DesktopIconsDraft::load(&store).unwrap();
    draft.set_show(Show::Trash, true);
    let plan = draft.plan();
    assert!(plan.blocked.is_empty());
    store.apply(&plan.patch).unwrap();
    assert_eq!(
        stored(&fixture),
        DesktopIcons {
            icon_size: NonZeroU16::new(150).unwrap(),
            show_trash: true,
            ..DesktopIcons::default()
        }
    );

    for (case, text) in [
        ("member", "(icon_size: 150, sort_by_name: true)"),
        ("bounds", "(icon_size: 900)"),
    ] {
        let fixture = Fixture::new(&format!("icons-unknown-{case}"));
        fixture.user(NAMESPACE, "desktop", text);
        let mut draft = DesktopIconsDraft::load(&fixture.store()).unwrap();
        assert!(matches!(draft.config(), Setting::Unsupported(_)), "{case}");
        draft.set_show(Show::Drives, true);
        let plan = draft.plan();
        assert!(plan.patch.is_empty(), "{case}");
        assert_eq!(plan.blocked.len(), 1, "{case}");
        assert!(
            fixture
                .store()
                .apply(&{
                    let mut patch = DesktopPatch::new();
                    patch.set(key(), None, Some(RawValue::parse(text).unwrap()));
                    patch
                })
                .is_err()
        );
    }
}

#[test]
fn icon_sizes_outside_native_steps_are_kept_until_moved() {
    let fixture = Fixture::new("icons-off-step");
    fixture.user(NAMESPACE, "desktop", "(icon_size: 110, grid_spacing: 30)");
    let mut draft = DesktopIconsDraft::load(&fixture.store()).unwrap();
    let config = *draft.config().known().unwrap();
    assert_eq!(config.icon_size.get(), 110);
    assert_eq!(config.grid_spacing.get(), 30);
    assert!(!draft.dirty());
    assert!(draft.plan().patch.is_empty());
    // A value outside the slider range is ignored, so nothing moves.
    draft.set_icon_size(40);
    draft.set_spacing(30);
    assert!(!draft.dirty());
    draft.set_icon_size(125);
    let plan = draft.plan();
    let value: DesktopIcons = plan
        .patch
        .get(&key())
        .unwrap()
        .desired
        .as_ref()
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(value.icon_size.get(), 125);
    assert_eq!(value.grid_spacing.get(), 30, "the kept spacing stays");
}

#[test]
fn icons_defaults_when_absent() {
    let fixture = Fixture::new("icons-absent");
    let store = fixture.store();
    let mut draft = DesktopIconsDraft::load(&store).unwrap();
    assert_eq!(draft.config(), Setting::Known(DesktopIcons::default()));
    assert_eq!(fixture.tree().len(), 0, "loading writes nothing");
    draft.set_icon_size(200);
    let plan = draft.plan();
    assert_eq!(plan.patch.get(&key()).unwrap().original, None);
    store.apply(&plan.patch).unwrap();
    assert_eq!(
        stored(&fixture),
        DesktopIcons {
            icon_size: NonZeroU16::new(200).unwrap(),
            ..DesktopIcons::default()
        }
    );
}
