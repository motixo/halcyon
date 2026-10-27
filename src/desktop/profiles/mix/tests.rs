use super::*;
use crate::desktop::profiles::{export_local, import, import_local};
fn sources() -> [Source; 3] {
    let mut p = Profile::new("Source").unwrap();
    p.widget_style = Some(
        json!({"clock":{"face":"analog", "format_24h":false,"show_date":false},"appearance":{"contrast_assist":false},"devbox":{"cpu":false},"material_motion":true}),
    );
    p.widget_layout = Some(json!({"widget_order":["clock","weather"]}));
    p.wallpaper = Some(
        serde_json::from_value(json!({"background":{"source":{"color":[0.2,0.3,0.4]}}})).unwrap(),
    );
    Component::ALL
        .map(|c| Source::freeze(&p, c, SourceKind::Library, Some("source-1".into())).unwrap())
}
#[test]
fn fb12_components_preserve_personal_choices_and_freeze_sources() {
    let s = sources();
    let p = compose("Mix", &s).unwrap();
    let style = p.widget_style.unwrap();
    assert_eq!(style.pointer("/clock/face"), Some(&json!("analog")));
    for key in [
        "/clock/format_24h",
        "/clock/show_date",
        "/appearance/contrast_assist",
        "/material_motion",
        "/devbox/cpu",
    ] {
        assert!(style.pointer(key).is_none(), "{key}");
    }
    assert!(p.windows.is_none() && p.workspaces.is_none());
    assert_eq!(p.widget_layout, s[2].profile.widget_layout);
    assert_eq!(p.version, 10);
}
#[test]
fn fb12_provenance_is_bounded_and_portable_with_legacy_versions_preserved() {
    let p = compose("Mix", &sources()).unwrap();
    let local = export_local(&p).unwrap();
    assert_eq!(import_local(&local).unwrap(), p);
    assert!(import(&local).is_err());
    let portable = export(&p).unwrap();
    let restored = import(&portable.bytes).unwrap();
    assert!(!restored.composition.as_ref().unwrap().has_local_ids());
    assert_eq!(
        restored.composition.as_ref().unwrap().picture.name,
        "Source"
    );
    let mut broken = p.clone();
    broken.composition.as_mut().unwrap().layout.fingerprint = "bad".into();
    assert!(broken.validate().is_err());
    broken = p.clone();
    broken.version = 9;
    assert!(broken.validate().is_err());
    let mut clock = Profile::new("Clock").unwrap();
    clock.widget_style = Some(json!({"clock":{"face":"analog"}}));
    clock.version = 9;
    assert_eq!(clock.written_version(), 9);
    assert!(clock.validate().is_ok());
    assert_eq!(import_local(&export_local(&clock).unwrap()).unwrap(), clock);
}
#[test]
fn fb12_bar_style_cannot_override_layout_orientation_or_islands() {
    use crate::desktop::panels::Anchor;
    let mut s = sources();
    s[1].profile.panel = Some(PanelLook {
        anchor: Some(Anchor::Top),
        opacity: Some(0.8),
        ..Default::default()
    });
    s[2].profile.panel = Some(PanelLook {
        anchor: Some(Anchor::Left),
        padding: Some(8),
        ..Default::default()
    });
    let p = compose("Spine mix", &s).unwrap().panel.unwrap();
    assert_eq!(p.anchor, Some(Anchor::Left));
    assert_eq!(p.padding, Some(8));
    assert_eq!(p.opacity, Some(0.8));
}
