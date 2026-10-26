use super::*;
use cosmic::Application;
use cosmic::iced::futures::StreamExt;
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::Action as LayerAction;
use cosmic::iced::runtime::{Action as RuntimeAction, platform_specific, task};

fn emitted(cards: &mut Cards, task: Task<Message>) -> Vec<LayerAction> {
    let mut pending = vec![task];
    let mut actions = Vec::new();
    while let Some(task) = pending.pop() {
        if let Some(mut stream) = task::into_stream(task) {
            cosmic::iced::futures::executor::block_on(async {
                while let Some(action) = stream.next().await {
                    match action {
                        RuntimeAction::Output(cosmic::Action::App(message)) => {
                            pending.push(cards.update(message));
                        }
                        RuntimeAction::PlatformSpecific(platform_specific::Action::Wayland(
                            platform_specific::wayland::Action::LayerSurface(action),
                        )) => actions.push(action),
                        _ => {}
                    }
                }
            });
        }
    }
    actions
}

#[test]
fn auditfix_run01_offscreen_notes_release_before_move_or_recreation_and_keep_draft() {
    if crate::displays::test_support::isolated(
        "app::tools::auditfix_tests::auditfix_run01_offscreen_notes_release_before_move_or_recreation_and_keep_draft",
    ) {
        return;
    }
    // Stable margins, direct recreation, and the actual config-update path.
    for mode in 0..3 {
        let mut config = Config::default();
        config.enabled_widgets.clear();
        config.notes.enabled = true;
        config.glance.initialized = true;
        config.widget_sizes = [
            ("glance".into(), [752., 504.]),
            ("notes".into(), [300., 180.]),
        ]
        .into();
        config.widget_positions = [("glance".into(), [0., 0.])].into();
        let (mut cards, _) = Cards::init(Core::default(), config);
        let _wire = cards.test_output_ready();
        cards.screen_size = [800., 600.];
        let task = cards.rebuild_cards();
        emitted(&mut cards, task);
        let task = cards.update(Message::Tool(Action::NoteEdit));
        emitted(&mut cards, task);
        cards.tools.draft.as_mut().unwrap().content =
            text_editor::Content::with_text("unsaved offscreen draft");
        assert!(cards.note_lifted());
        let id = cards.widget_id(Widget::Notes);
        let mut shown = cards.config.clone();
        shown.glance.enabled = true;
        let task = if mode == 2 {
            cards.apply_config(shown)
        } else {
            cards.config = shown;
            if mode == 0 {
                Task::batch(cards.sync_tool_surfaces())
            } else {
                cards.rebuild_cards()
            }
        };
        let actions = emitted(&mut cards, task);
        assert!(cards.config.widget_visible(Widget::Notes));
        assert!(cards.placed_position(Widget::Notes)[0] < -9000.);
        let release = actions.iter().position(|action| matches!(action,
            LayerAction::KeyboardInteractivity { id: target, keyboard_interactivity: KeyboardInteractivity::None }
            if *target == id)).expect("release keyboard for the retained Notes surface");
        let hidden = actions
            .iter()
            .position(|action| match action {
                LayerAction::Margin { id: target, margin } => *target == id && margin.left < -9000,
                LayerAction::Destroy(target) => *target == id,
                _ => false,
            })
            .expect("move or destroy Notes after releasing input");
        assert!(release < hidden);
        assert!(actions.iter().all(|action| match action {
            LayerAction::LayerSurface { builder } if builder.id == id =>
                builder.keyboard_interactivity == KeyboardInteractivity::None
                    && builder.layer == Layer::Bottom,
            LayerAction::KeyboardInteractivity {
                id: target,
                keyboard_interactivity,
            } if *target == id => *keyboard_interactivity == KeyboardInteractivity::None,
            _ => true,
        }));
        let draft = cards.tools.draft.as_ref().unwrap();
        assert!(!draft.typing && draft.content.text().contains("unsaved offscreen draft"));
        cards.config.glance.enabled = false;
        let task = Task::batch(cards.sync_tool_surfaces());
        emitted(&mut cards, task);
        assert!(cards.placed_position(Widget::Notes)[0] >= 0.);
        assert!(
            !cards.note_lifted(),
            "room returning must not implicitly resume typing"
        );
        let task = cards.update(Message::Tool(Action::NoteEdit));
        emitted(&mut cards, task);
        assert!(cards.note_lifted());
        assert!(
            cards
                .tools
                .draft
                .as_ref()
                .unwrap()
                .content
                .text()
                .contains("unsaved offscreen draft")
        );
    }
}
