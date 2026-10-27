//! Same native sample components, on a public reference canvas.
use super::*;
use crate::desktop::{
    panels::{PanelDraft, PanelKind},
    profiles::{PanelLook, Profile},
    store::{ConfigKey, KeyEntry, RawValue, Snapshot},
};
use crate::settings::sharing::Scene;
use cosmic::iced::{
    Point,
    widget::{pin, stack},
};

fn bar(kind: PanelKind, look: &PanelLook, background: Color) -> PanelDraft {
    let mut snapshot = Snapshot::default();
    let mut put = |key, value| {
        snapshot.insert(
            key,
            KeyEntry {
                user: None,
                system: Some(value),
            },
        );
    };
    put(
        ConfigKey::new(crate::desktop::panels::NAMESPACE, 1, "entries").unwrap(),
        RawValue::encode(&vec![kind.entry()]).unwrap(),
    );
    if kind == PanelKind::Dock {
        put(
            ConfigKey::new(kind.namespace(), 1, "anchor").unwrap(),
            RawValue::encode(&crate::desktop::panels::Anchor::Bottom).unwrap(),
        );
        put(
            ConfigKey::new(kind.namespace(), 1, "expand_to_edges").unwrap(),
            RawValue::encode(&false).unwrap(),
        );
    }
    for (name, value) in look.values().unwrap_or_default() {
        put(ConfigKey::new(kind.namespace(), 1, name).unwrap(), value);
    }
    if look
        .background
        .as_ref()
        .is_none_or(|bg| *bg == crate::desktop::panels::Background::ThemeDefault)
    {
        put(
            ConfigKey::new(kind.namespace(), 1, "background").unwrap(),
            RawValue::encode(&crate::desktop::panels::Background::Color([
                background.r,
                background.g,
                background.b,
            ]))
            .unwrap(),
        );
    }
    if let Some(islands) = &look.islands {
        put(
            crate::desktop::islands::key(kind),
            RawValue::encode(islands).unwrap(),
        );
    }
    // Explicitly illustrative applets. No installed-app discovery or executable.
    let applets = look
        .applets
        .clone()
        .unwrap_or_else(|| crate::desktop::profiles::Applets {
            start: vec!["com.system76.CosmicPanelWorkspacesButton".into()],
            center: vec!["com.system76.CosmicAppletTime".into()],
            end: vec![
                "com.system76.CosmicAppletAudio".into(),
                "com.system76.CosmicAppletPower".into(),
            ],
        });
    put(
        ConfigKey::new(kind.namespace(), 1, "plugins_wings").unwrap(),
        RawValue::encode(&Some((applets.start, applets.end))).unwrap(),
    );
    put(
        ConfigKey::new(kind.namespace(), 1, "plugins_center").unwrap(),
        RawValue::encode(&Some(applets.center)).unwrap(),
    );
    PanelDraft::from_snapshot(kind, snapshot)
}
fn bars(profile: &Profile, background: Color) -> Vec<PanelDraft> {
    [
        (PanelKind::Dock, profile.dock.as_ref()),
        (PanelKind::Panel, profile.panel.as_ref()),
    ]
    .into_iter()
    .filter_map(|(kind, look)| look.map(|look| bar(kind, look, background)))
    .filter(|bar| bar.autohide().known() != Some(&crate::desktop::panels::AutoHide::Always))
    .collect()
}
pub(in crate::settings) fn view<'a>(scene: &Scene, config: &Config) -> Element<'a, Message> {
    use chrono::TimeZone;
    let now = chrono::Local
        .with_ymd_and_hms(2026, 10, 19, 9, 41, 0)
        .earliest()
        .unwrap();
    let layout = crate::layout_state::public_preview(config);
    let scale = (1280. / layout.screen[0]).min(680. / layout.screen[1]);
    let screen = [layout.screen[0] * scale, layout.screen[1] * scale];
    let background: Element<'_, Message> = if let Some(pixels) = scene.background() {
        cosmic::widget::image(cosmic::widget::image::Handle::from_rgba(
            pixels.width,
            pixels.height,
            pixels.rgba,
        ))
        .width(screen[0])
        .height(screen[1])
        .content_fit(cosmic::iced::ContentFit::Cover)
        .into()
    } else {
        container(cosmic::widget::space())
            .width(screen[0])
            .height(screen[1])
            .class(cosmic::theme::Container::custom(|_| {
                cosmic::iced::widget::container::Style {
                    background: Some(Color::from_rgb(0.10, 0.12, 0.14).into()),
                    ..Default::default()
                }
            }))
            .into()
    };
    let mut layers = vec![background];
    for geometry in layout.widgets {
        let Some(widget) =
            crate::drawer::Widget::parse(&geometry.key).filter(|w| config.widget_visible(*w))
        else {
            continue;
        };
        let size = [geometry.size[0] * scale, geometry.size[1] * scale];
        let depth = (widget==crate::drawer::Widget::Clock).then(|| {
            let background=scene.profile.wallpaper.as_ref()?.background.as_ref()?;
            if !matches!(background.source.as_ref(),Some(crate::desktop::profiles::SourceLook::Bundled(id)) if id=="halcyon-ridge") {return None;}
            let mode=background.scaling(None).ok().flatten().unwrap_or(crate::desktop::wallpaper::ScalingMode::Zoom);
            crate::depth::public_clock(config,crate::depth::Frame{screen:layout.screen,origin:geometry.position.map(f32::round),size:geometry.size,scale:1.},mode,now.naive_local())
        }).flatten();
        let card = depth
            .map(|p| {
                cosmic::widget::image(cosmic::widget::image::Handle::from_rgba(
                    p.width, p.height, p.rgba,
                ))
                .width(size[0])
                .height(size[1])
                .content_fit(cosmic::iced::ContentFit::Fill)
                .into()
            })
            .unwrap_or_else(|| super::profiles::card_at(widget, size, config, scale, Some(now)));
        layers.push(
            pin(card)
                .position(Point::new(
                    geometry.position[0] * scale,
                    geometry.position[1] * scale,
                ))
                .into(),
        );
    }
    let bars = bars(&scene.profile, config.palette().background);
    let catalog = crate::desktop::applets::AppletCatalog::public_sample();
    layers.extend(super::desktop::preview::bars(
        true,
        bars.iter().collect(),
        &catalog,
        None,
        screen,
        scale,
        config.palette().accent,
        None,
    ));
    let rendered = container(stack(layers).width(screen[0]).height(screen[1]))
        .width(1280.)
        .height(680.)
        .center_x(1280.)
        .center_y(680.);
    let caption = if scene.omitted_picture {
        "Halcyon · Sample desktop · Personal or unlisted wallpaper omitted"
    } else {
        "Halcyon · Sample desktop · Illustrative content"
    };
    container(column::with_children(vec![
        rendered.into(),
        container(text::caption(caption))
            .height(40.)
            .width(1280.)
            .center_x(1280.)
            .center_y(40.)
            .into(),
    ]))
    .width(1280.)
    .height(720.)
    .class(cosmic::theme::Container::Background)
    .into()
}
