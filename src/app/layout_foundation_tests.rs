use super::*;
use crate::native_test::Native;
use cosmic::Application;
use cosmic::iced::{Rectangle, Size, advanced::widget::Operation};
use std::sync::{Arc, Mutex};

struct Frames(Arc<Mutex<Vec<Rectangle>>>);
impl Operation for Frames {
    fn traverse(&mut self, f: &mut dyn FnMut(&mut dyn Operation)) {
        f(self)
    }
    fn container(&mut self, _: Option<&cosmic::widget::Id>, bounds: Rectangle) {
        self.0.lock().unwrap().push(bounds);
    }
}

#[test]
fn fb08_arrange_hides_shared_column_even_when_weather_is_disabled() {
    let mut config = Config::default();
    config.enabled_widgets = vec!["projects".into(), "devbox".into()];
    let (mut cards, _) = Cards::init(Core::default(), config);
    let _wire = cards.test_output_ready();
    assert!(!cards.custom_layout());
    assert!(!cards.card_input_rects(Widget::Weather).is_empty());
    let mut native = Native::new(Size::new(360., 984.));
    let shown = native.render(
        cards.surface_view(cards.widget_id(Widget::Weather)),
        &cosmic::Theme::dark(),
    );
    assert!(shown.chunks_exact(4).any(|pixel| pixel[3] != 0));
    cards.layout_editor.active = true;
    assert!(cards.card_input_rects(Widget::Weather).is_empty());
    let hidden = native.render(
        cards.surface_view(cards.widget_id(Widget::Weather)),
        &cosmic::Theme::dark(),
    );
    assert!(
        hidden.chunks_exact(4).all(|pixel| pixel[3] == 0),
        "shared column must not render a second copy under Arrange"
    );
}

#[test]
fn fb08_column_hit_regions_match_native_frames_and_leave_gaps_and_shadows_clear() {
    for family in [
        None,
        Some(crate::desktop_theme::Family::Soft),
        Some(crate::desktop_theme::Family::Paper),
    ] {
        for order in [
            ["weather", "projects", "devbox"],
            ["projects", "devbox", "weather"],
        ] {
            let mut config = Config::default();
            config.appearance.family = family;
            config.widget_order = order.map(str::to_owned).into();
            let (mut cards, _) = Cards::init(Core::default(), config);
            let _wire = cards.test_output_ready();
            cards.screen_size = [1920., 1080.];
            cards.desktop_height = 984.;
            let (_, margin) = cards.shadow_px();
            let mut native = Native::new(Size::new(360. + margin as f32, 984. + margin as f32));
            let frames = Arc::new(Mutex::new(Vec::new()));
            native.operate(cards.desktop_widgets(), Box::new(Frames(frames.clone())));
            let regions = cards.card_input_rects(Widget::Weather);
            assert_eq!(regions.len(), 3);
            for (widget, region) in cards.visible_cards().into_iter().zip(&regions) {
                let size = cards.widget_size(widget);
                let matching = frames.lock().unwrap().iter().any(|r| {
                    (r.width - size[0]).abs() < 0.01
                        && (r.height - size[1]).abs() < 0.01
                        && region[0] as f32 >= r.x
                        && region[1] as f32 >= r.y
                        && (region[0] + region[2]) as f32 <= r.x + r.width + 0.01
                        && (region[1] + region[3]) as f32 <= r.y + r.height + 0.01
                        && (r.x - region[0] as f32).abs() < 1.01
                        && (r.y - region[1] as f32).abs() < 1.01
                });
                assert!(
                    matching,
                    "{family:?} {order:?} {widget:?} {region:?}: {:?}",
                    frames.lock().unwrap()
                );
            }
            for pair in regions.windows(2) {
                assert!(
                    pair[0][1] + pair[0][3] < pair[1][1],
                    "gap must pass clicks through"
                );
            }
            assert!(regions.iter().all(|r| r[0] == margin / 2 && r[2] == 360));
            cards.layout_editor.active = true;
            assert!(
                cards.card_input_rects(Widget::Weather).is_empty(),
                "Arrange owns input; hidden originals do not"
            );
            assert!(cards.card_input_rects(Widget::Clock).is_empty());
        }
    }
}

#[test]
fn fb08_logical_input_clips_fractional_and_invalid_frames() {
    use crate::widget_presentation::input_rect;
    assert_eq!(
        input_rect([12.5, -3.5], [360.25, 180.25], [400., 200.]),
        Some([13, 0, 359, 176])
    );
    assert_eq!(
        input_rect([390., 190.], [360., 180.], [400., 200.]),
        Some([390, 190, 10, 10])
    );
    for origin in [
        [f32::NAN, 0.],
        [0., f32::INFINITY],
        [400., 0.],
        [-1000., 0.],
    ] {
        assert!(input_rect(origin, [360., 180.], [400., 200.]).is_none());
    }
    assert!(input_rect([0., 0.], [f32::NAN, 180.], [400., 200.]).is_none());
    assert!(input_rect([0., 0.], [360., 180.], [f32::INFINITY, 200.]).is_none());
}
