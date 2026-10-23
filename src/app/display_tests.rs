use super::*;
use crate::displays::{
    OutputEvent,
    test_support::{Wire, info, isolated},
};
use cosmic::iced::futures::StreamExt;
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::Action as LayerAction;
use cosmic::iced::runtime::{Action as RuntimeAction, platform_specific, task};

fn actions(cards: &mut Cards, task: Task<Message>) -> Vec<SctkLayerSurfaceSettings> {
    let mut pending = vec![task];
    let mut created = Vec::new();
    while let Some(task) = pending.pop() {
        let Some(mut stream) = task::into_stream(task) else {
            continue;
        };
        cosmic::iced::futures::executor::block_on(async {
            while let Some(action) = stream.next().await {
                match action {
                    RuntimeAction::Output(cosmic::Action::App(message)) => {
                        pending.push(cards.update(message))
                    }
                    RuntimeAction::PlatformSpecific(platform_specific::Action::Wayland(
                        platform_specific::wayland::Action::LayerSurface(
                            LayerAction::LayerSurface { builder },
                        ),
                    )) => created.push(builder),
                    _ => {}
                }
            }
        });
    }
    created
}
fn send(cards: &mut Cards, message: Message) -> Vec<SctkLayerSurfaceSettings> {
    let task = cards.update(message);
    actions(cards, task)
}
fn ready(cards: &mut Cards, size: [f32; 2]) -> Vec<SctkLayerSurfaceSettings> {
    send(
        cards,
        Message::SurfaceResized(cards.ids[5], cosmic::iced::Size::new(size[0], size[1])),
    )
}
fn bound(builders: &[SctkLayerSurfaceSettings], handle: &crate::displays::WlOutput) {
    for builder in builders {
        assert!(
            matches!(&builder.output, IcedOutput::Output(output) if output==handle),
            "{} not bound",
            builder.namespace
        );
    }
}

#[test]
fn monitor_pinning_lifecycle_actions() {
    if isolated("app::display_tests::monitor_pinning_lifecycle_actions") {
        return;
    }
    let wire = Wire::new();
    let a = wire.output(1);
    let b = wire.output(2);
    let mut config = Config::default();
    config.widget_display = Some("DP-2".into());
    config.widget_positions.insert("weather".into(), [0.8, 0.6]);
    config.widget_sizes.insert("weather".into(), [500., 240.]);
    let saved = serde_json::to_value(&config).unwrap();
    let (mut cards, _) = Cards::init(Core::default(), config);
    assert!(cards.selected_output.is_none());
    assert!(actions(&mut cards, Task::none()).is_empty());
    assert!(
        send(
            &mut cards,
            Message::OutputChanged(OutputEvent::Created(None), a.clone())
        )
        .is_empty()
    );
    let mut created = send(
        &mut cards,
        Message::OutputChanged(
            OutputEvent::InfoUpdate(info(Some("DP-1"), (1920, 1080))),
            a.clone(),
        ),
    );
    assert_eq!(created.len(), 1);
    assert!(!cards.output_known);
    created.extend(ready(&mut cards, [1920., 1080.]));
    assert_eq!(created.len(), 12);
    assert!(cards.tool_geometry_known(Widget::Notes));
    bound(&created, &a);
    cards.tools.draft = Some(tools::NoteDraft {
        content: cosmic::widget::text_editor::Content::with_text("unsaved monitor regression"),
        base: String::new(),
        typing: true,
        conflicted: false,
        saving: false,
    });
    let old_ids = cards.ids;
    let epoch = cards.display_generation;
    let created = send(
        &mut cards,
        Message::OutputChanged(
            OutputEvent::Created(Some(info(Some("DP-2"), (1920, 1080)))),
            b.clone(),
        ),
    );
    assert_eq!(created.len(), 1);
    bound(&created, &b);
    assert!(cards.ids.iter().all(|id| !old_ids.contains(id)));
    assert!(!cards.output_known);
    assert!(!cards.tools.draft.as_ref().unwrap().typing);
    assert!(
        cards
            .tools
            .draft
            .as_ref()
            .unwrap()
            .content
            .text()
            .contains("unsaved monitor regression")
    );
    let late = cards.update(Message::SurfaceResized(
        old_ids[5],
        cosmic::iced::Size::new(400., 400.),
    ));
    let emitted = cosmic::iced::futures::executor::block_on(
        task::into_stream(late).unwrap().collect::<Vec<_>>(),
    );
    assert!(emitted.iter().any(|action| matches!(action,
        RuntimeAction::PlatformSpecific(platform_specific::Action::Wayland(
            platform_specific::wayland::Action::LayerSurface(LayerAction::Destroy(id))
        )) if *id==old_ids[5])));
    assert!(!cards.output_known);
    assert!(send(&mut cards, Message::RebuildSurfaces(epoch)).is_empty());
    let stale = send(
        &mut cards,
        Message::SurfaceMessage(epoch, Box::new(Message::Arrange)),
    );
    assert!(stale.is_empty());
    assert!(!cards.layout_editor.active);
    bound(&ready(&mut cards, [1920., 1080.]), &b);
    // Same-size replacement must move all roles to a fresh object identity.
    send(
        &mut cards,
        Message::OutputChanged(OutputEvent::Removed, b.clone()),
    );
    bound(&ready(&mut cards, [1280., 720.]), &a);
    let replacement = wire.output(3);
    bound(
        &send(
            &mut cards,
            Message::OutputChanged(
                OutputEvent::Created(Some(info(Some("DP-2"), (1920, 1080)))),
                replacement.clone(),
            ),
        ),
        &replacement,
    );
    bound(&ready(&mut cards, [1920., 1080.]), &replacement);
    assert_eq!(
        serde_json::to_value(&cards.config).unwrap()["widget_positions"],
        saved["widget_positions"]
    );
    assert_eq!(
        serde_json::to_value(&cards.config).unwrap()["widget_sizes"],
        saved["widget_sizes"]
    );
    assert_eq!(cards.config.widget_display.as_deref(), Some("DP-2"));
    send(&mut cards, Message::OutputChanged(OutputEvent::Removed, a));
    send(
        &mut cards,
        Message::OutputChanged(OutputEvent::Removed, replacement),
    );
    assert!(!cards.output_known);
    assert!(cards.selected_output.is_none());
    assert!(send(&mut cards, Message::Arrange).is_empty());
    let generation = cards.display_generation;
    assert!(send(&mut cards, Message::RebuildSurfaces(generation)).is_empty());
}

#[test]
fn monitor_pinning_metadata_scaling_and_saved_drafts() {
    if isolated("app::display_tests::monitor_pinning_metadata_scaling_and_saved_drafts") {
        return;
    }
    let wire = Wire::new();
    let a = wire.output(1);
    let b = wire.output(2);
    let mut config = Config::default();
    config.widget_display = Some("DP-2".into());
    config.widget_positions.insert("weather".into(), [0.7, 0.4]);
    config.widget_sizes.insert("weather".into(), [500., 240.]);
    let (mut cards, _) = Cards::init(Core::default(), config);
    send(
        &mut cards,
        Message::OutputChanged(
            OutputEvent::Created(Some(info(Some("DP-2"), (2560, 1440)))),
            b.clone(),
        ),
    );
    ready(&mut cards, [2560., 1440.]);
    let original = cards.config.widget_positions.clone();
    let sizes = cards.config.widget_sizes.clone();
    let arranged = send(&mut cards, Message::Arrange);
    bound(&arranged, &b);
    assert!(cards.layout_editor.active);
    let generation = cards.display_generation;
    // A primary/global-position update is not a placement change.
    let mut updated = info(Some("DP-2"), (2560, 1440));
    updated.logical_position = Some((3000, 0));
    assert!(
        send(
            &mut cards,
            Message::OutputChanged(OutputEvent::InfoUpdate(updated), b.clone())
        )
        .is_empty()
    );
    assert_eq!(cards.display_generation, generation);
    send(
        &mut cards,
        Message::OutputChanged(
            OutputEvent::Created(Some(info(Some("eDP-1"), (1280, 720)))),
            a.clone(),
        ),
    );
    send(
        &mut cards,
        Message::OutputChanged(OutputEvent::Removed, b.clone()),
    );
    assert!(!cards.layout_editor.active);
    assert!(cards.paused_layout.is_some());
    ready(&mut cards, [1280., 720.]);
    bound(&send(&mut cards, Message::Arrange), &a);
    assert!(cards.layout_editor.active);
    for dimensions in [(1536, 864), (1080, 1920), (320, 240), (1920, 1080)] {
        let mut updated = info(Some("eDP-1"), dimensions);
        updated.scale_factor = 2;
        send(
            &mut cards,
            Message::OutputChanged(OutputEvent::InfoUpdate(updated), a.clone()),
        );
        bound(
            &ready(&mut cards, [dimensions.0 as f32, dimensions.1 as f32]),
            &a,
        );
        assert_eq!(
            cards.screen_size,
            [dimensions.0 as f32, dimensions.1 as f32]
        );
        assert_eq!(cards.config.widget_positions, original);
        assert_eq!(cards.config.widget_sizes, sizes);
        let size = cards.widget_size(Widget::Weather);
        assert!(size[0] <= cards.screen_size[0] && size[1] <= cards.screen_size[1]);
    }
    assert_eq!(cards.config.widget_display.as_deref(), Some("DP-2"));
}

#[test]
fn monitor_pinning_restart_and_order() {
    if isolated("app::display_tests::monitor_pinning_restart_and_order") {
        return;
    }
    Config::save_changes(serde_json::json!({"widget_display":"DP-2"})).unwrap();
    let wire = Wire::new();
    let a = wire.output(1);
    let b = wire.output(2);
    for order in [
        [("DP-1", a.clone()), ("DP-2", b.clone())],
        [("DP-2", b.clone()), ("DP-1", a.clone())],
    ] {
        let (mut cards, _) = Cards::init(Core::default(), Config::try_load().unwrap());
        for (name, handle) in order {
            send(
                &mut cards,
                Message::OutputChanged(
                    OutputEvent::Created(Some(info(Some(name), (1920, 1080)))),
                    handle,
                ),
            );
        }
        let created = ready(&mut cards, [1920., 1080.]);
        assert_eq!(created.len(), 11);
        bound(&created, &b);
        let mut cfg = cards.config.clone();
        cfg.widget_display = Some("DP-1".into());
        let task = cards.apply_config(cfg);
        bound(&actions(&mut cards, task), &a);
        bound(&ready(&mut cards, [1920., 1080.]), &a);
    }
}

impl Cards {
    /// Renderer-only regressions explicitly supply their simulated output.
    pub(super) fn test_output_ready(&mut self) -> Wire {
        let wire = Wire::new();
        let output = wire.output(1);
        self.displays.update(
            OutputEvent::Created(Some(info(Some("test-output"), (1920, 1080)))),
            output,
        );
        self.selected_output = self.displays.selected(None).cloned();
        self.output_known = true;
        wire
    }
}

#[test]
fn monitor_migration_cancels_sidebar_preview_before_replacement_geometry() {
    if isolated(
        "app::display_tests::monitor_migration_cancels_sidebar_preview_before_replacement_geometry",
    ) {
        return;
    }
    use cosmic::iced::{Point, mouse};
    let original = vec![Widget::Weather, Widget::Devbox, Widget::Projects];
    // Both removal/fallback and geometry changes retire the old sidebar surface.
    for disconnect in [true, false] {
        Config::save_changes(serde_json::json!({
            "widget_display":"DP-2", "widget_order":["weather","devbox","projects"],
            "future_preference":37
        }))
        .unwrap();
        let config_path = Config::path().unwrap();
        let saved = std::fs::read(&config_path).unwrap();
        let wire = Wire::new();
        let fallback = wire.output(1);
        let selected = wire.output(2);
        let (mut cards, _) = Cards::init(Core::default(), Config::try_load().unwrap());
        send(
            &mut cards,
            Message::OutputChanged(
                OutputEvent::Created(Some(info(Some("DP-2"), (1920, 1080)))),
                selected.clone(),
            ),
        );
        send(
            &mut cards,
            Message::OutputChanged(
                OutputEvent::Created(Some(info(Some("eDP-1"), (1280, 720)))),
                fallback.clone(),
            ),
        );
        ready(&mut cards, [1920., 1080.]);
        assert!(!cards.config.has_custom_positions());
        let old_ids = cards.ids;
        let start = Point::new(70., drawer::FIRST_ROW + 20.);
        send(
            &mut cards,
            Message::Pointer(old_ids[2], mouse::Event::CursorMoved { position: start }),
        );
        send(&mut cards, Message::SidebarPress(Widget::Weather));
        send(
            &mut cards,
            Message::Pointer(
                old_ids[2],
                mouse::Event::CursorMoved {
                    position: Point::new(70., drawer::FIRST_ROW + 2. * drawer::ROW_STEP + 20.),
                },
            ),
        );
        assert_ne!(
            cards.config.ordered_right_cards(),
            original,
            "actual pointer reducer must preview a reorder"
        );
        assert_eq!(
            cards.sidebar.drag.as_ref().unwrap().original_order,
            original
        );
        assert_eq!(
            std::fs::read(&config_path).unwrap(),
            saved,
            "preview never saved"
        );
        let migration = if disconnect {
            OutputEvent::Removed
        } else {
            OutputEvent::InfoUpdate(info(Some("DP-2"), (1280, 720)))
        };
        let created = send(
            &mut cards,
            Message::OutputChanged(migration, selected.clone()),
        );
        assert_eq!(
            cards.config.ordered_right_cards(),
            original,
            "display migration must cancel the sidebar preview before creating replacement geometry"
        );
        assert!(cards.sidebar.drag.is_none());
        assert!(!cards.output_known);
        assert_eq!(
            created.len(),
            1,
            "only the fresh budget probe may be created"
        );
        let expected_output = if disconnect { &fallback } else { &selected };
        bound(&created, expected_output);
        assert_ne!(cards.ids[2], old_ids[2]);
        send(
            &mut cards,
            Message::Pointer(
                old_ids[2],
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ),
        );
        assert_eq!(cards.config.ordered_right_cards(), original);
        assert_eq!(
            std::fs::read(&config_path).unwrap(),
            saved,
            "stale release cannot commit a preview"
        );
        bound(&ready(&mut cards, [1280., 720.]), expected_output);
        assert!(cards.output_known);
        assert_eq!(cards.config.ordered_right_cards(), original);
        assert_eq!(
            std::fs::read(&config_path).unwrap(),
            saved,
            "fresh probe must use the cancellation baseline"
        );
        // A fresh, deliberate drag must capture the original order, not inherit
        // the abandoned preview; its release can then commit exactly that edit.
        let new_sidebar = cards.ids[2];
        send(
            &mut cards,
            Message::Pointer(new_sidebar, mouse::Event::CursorMoved { position: start }),
        );
        send(&mut cards, Message::SidebarPress(Widget::Weather));
        assert_eq!(
            cards.sidebar.drag.as_ref().unwrap().original_order,
            original
        );
        send(
            &mut cards,
            Message::Pointer(
                new_sidebar,
                mouse::Event::CursorMoved {
                    position: Point::new(70., drawer::FIRST_ROW + drawer::ROW_STEP + 20.),
                },
            ),
        );
        send(
            &mut cards,
            Message::Pointer(
                new_sidebar,
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ),
        );
        assert_eq!(
            Config::try_load().unwrap().ordered_right_cards(),
            vec![Widget::Devbox, Widget::Weather, Widget::Projects]
        );
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
        assert_eq!(value["future_preference"], 37);
    }
}
