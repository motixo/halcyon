//! Geometry shared by the desktop arrangement editor and restored widget positions.
pub type Point = [f32; 2];
pub type Size = [f32; 2];

fn bounds(widget: Size, screen: Size) -> [(f32, f32); 2] {
    [0, 1].map(|axis| {
        let gap = if axis == 0 { 24. } else { 48. };
        let travel = (screen[axis] - widget[axis]).max(0.);
        if travel >= gap * 2. {
            (gap, travel - gap)
        } else {
            (0., travel)
        }
    })
}
pub fn clamp(point: Point, widget: Size, screen: Size) -> Point {
    let limits = bounds(widget, screen);
    [0, 1].map(|axis| {
        let (min, max) = limits[axis];
        if point[axis].is_finite() {
            point[axis].clamp(min, max)
        } else {
            min
        }
    })
}
pub fn encode(point: Point, widget: Size, screen: Size) -> Point {
    let point = clamp(point, widget, screen);
    let limits = bounds(widget, screen);
    [0, 1].map(|axis| {
        let (min, max) = limits[axis];
        if max > min {
            (point[axis] - min) / (max - min)
        } else {
            0.
        }
    })
}
pub fn decode(position: Point, widget: Size, screen: Size) -> Point {
    let limits = bounds(widget, screen);
    [0, 1].map(|axis| {
        let (min, max) = limits[axis];
        let ratio = if position[axis].is_finite() {
            position[axis].clamp(0., 1.)
        } else {
            0.
        };
        min + ratio * (max - min)
    })
}
pub fn drag(pointer: Point, offset: Point, widget: Size, screen: Size) -> Point {
    clamp(
        [pointer[0] - offset[0], pointer[1] - offset[1]],
        widget,
        screen,
    )
}
/// Fine snapping has the same logical-pixel pitch on every display. Keep the
/// legacy argument so saved Spacious/Balanced preferences remain readable;
/// loading their normalized positions does not snap or move them.
pub fn grid_step(_screen: Size, _columns: u8) -> Size {
    [12., 12.]
}

/// Guide coordinates match the common grid origin and include both safe edges.
pub fn grid_lines(screen: Size) -> [Vec<f32>; 2] {
    let limits = bounds([0., 0.], screen);
    let step = grid_step(screen, 24);
    [0, 1].map(|axis| {
        let (min, max) = limits[axis];
        let count = ((max - min) / step[axis]).floor() as u32;
        let mut lines: Vec<_> = (0..=count)
            .map(|index| min + index as f32 * step[axis])
            .collect();
        if lines.last().is_some_and(|last| *last < max) {
            lines.push(max);
        }
        lines
    })
}
/// Align cards to the desktop grid or a nearer safe boundary, keeping them on screen.
pub fn snap(point: Point, widget: Size, screen: Size, columns: u8) -> Point {
    let point = clamp(point, widget, screen);
    let limits = bounds(widget, screen);
    let step = grid_step(screen, columns);
    [0, 1].map(|axis| {
        let (min, max) = limits[axis];
        let last = ((max - min) / step[axis]).floor().max(0.);
        let grid = (min + ((point[axis] - min) / step[axis]).round().clamp(0., last) * step[axis])
            .clamp(min, max);
        // The final grid cell can leave a large gap when the card's size is
        // not a grid multiple. Include the actual inset boundary as a target.
        if max - point[axis] <= (grid - point[axis]).abs() {
            max
        } else {
            grid
        }
    })
}

/// The floating tools share the same automatic anchors in both editors.
pub fn tool_position(
    widget: crate::drawer::Widget,
    size: Size,
    focus_height: f32,
    screen: Size,
) -> Point {
    use crate::drawer::Widget;
    let x = (screen[0] - size[0]) / 2.;
    let y = match widget {
        Widget::Music => screen[1] - size[1] - 96.,
        Widget::Notes => 88. + focus_height + crate::drawer::CARD_GAP,
        _ => 88.,
    };
    clamp([x, y], size, screen)
}

/// Reject malformed persisted geometry before any write.
pub fn valid_size(size: Size) -> bool {
    size.into_iter()
        .all(|v| v.is_finite() && (1.0..=32768.0).contains(&v))
}

/// Smallest usable explicit viewport. Automatic cards retain their natural size.
pub fn minimum_size(widget: crate::drawer::Widget) -> Size {
    use crate::drawer::Widget::*;
    match widget {
        Clock => [160., 100.],
        Calendar => [300., 240.],
        Glance => [260., 140.],
        Music => [300., 140.],
        Notes => [260., 180.],
        Focus => [280., 160.],
        Weather | Devbox | Projects => [300., 180.],
    }
}

pub fn sized(widget: crate::drawer::Widget, requested: Size, screen: Size) -> Size {
    let min = minimum_size(widget);
    [0, 1].map(|axis| {
        let max = (screen[axis] - if axis == 0 { 48. } else { 96. }).max(1.);
        let value = if requested[axis].is_finite() {
            requested[axis]
        } else {
            min[axis]
        };
        value.clamp(min[axis].min(max), max)
    })
}

/// Resize from the lower-right corner, keeping the top-left anchored. Include
/// the exact safe edge as a snap target even when it is not a grid multiple.
pub fn resize(
    widget: crate::drawer::Widget,
    position: Point,
    requested: Size,
    screen: Size,
    snapping: bool,
) -> Size {
    let min = minimum_size(widget);
    [0, 1].map(|axis| {
        let max = (screen[axis] - position[axis] - if axis == 0 { 24. } else { 48. }).max(1.);
        let min = min[axis].min(max);
        let value = if requested[axis].is_finite() {
            requested[axis].clamp(min, max)
        } else {
            min
        };
        if !snapping {
            return value.round().clamp(min, max);
        }
        let grid = (value / 12.).round().mul_add(12., 0.).clamp(min, max);
        if max - value <= (grid - value).abs() {
            max
        } else {
            grid
        }
    })
}

/// Space kept between widgets that would otherwise touch.
pub const WIDGET_GAP: f32 = 16.;

/// Whether two rectangles (`[x, y, width, height]`) come closer than `gap`.
fn crowds(a: [f32; 4], b: [f32; 4], gap: f32) -> bool {
    a[0] < b[0] + b[2] + gap
        && b[0] < a[0] + a[2] + gap
        && a[1] < b[1] + b[3] + gap
        && b[1] < a[1] + a[3] + gap
}

/// The spot nearest `point` where a widget of `size` stays on screen and
/// clear of every rectangle in `placed`. Widgets never stack: one on top of
/// another would take the clicks meant for the one below. Keeps `point`
/// when nothing is in the way; `None` when the screen has no room for it.
pub fn free_spot(point: Point, size: Size, placed: &[[f32; 4]], screen: Size) -> Option<Point> {
    let point = clamp(point, size, screen);
    let clear = |at: Point| {
        placed
            .iter()
            .all(|rect| !crowds([at[0], at[1], size[0], size[1]], *rect, WIDGET_GAP))
    };
    if clear(point) {
        return Some(point);
    }
    let distance = |at: Point| (at[0] - point[0]).powi(2) + (at[1] - point[1]).powi(2);
    // Beside each obstacle first (the natural places), then a fine scan.
    let mut candidates = Vec::new();
    for rect in placed {
        let right = rect[0] + rect[2] + WIDGET_GAP;
        let left = rect[0] - size[0] - WIDGET_GAP;
        let below = rect[1] + rect[3] + WIDGET_GAP;
        let above = rect[1] - size[1] - WIDGET_GAP;
        candidates.extend([
            [right, point[1]],
            [left, point[1]],
            [point[0], below],
            [point[0], above],
            [right, rect[1]],
            [left, rect[1]],
            [rect[0], below],
            [rect[0], above],
        ]);
    }
    let step = 16.;
    let mut y = 0.;
    while y <= screen[1] {
        let mut x = 0.;
        while x <= screen[0] {
            candidates.push([x, y]);
            x += step;
        }
        y += step;
    }
    candidates
        .into_iter()
        .map(|at| clamp(at, size, screen))
        .filter(|at| clear(*at))
        .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn widgets_never_stack_and_move_as_little_as_possible() {
        let screen = [1920., 1080.];
        let notes = [1536., 494., 360., 180.];
        // Focus dropped almost exactly on Notes goes just below or above it.
        let focus = free_spot([1536., 488.], [360., 192.], &[notes], screen).unwrap();
        let rect = [focus[0], focus[1], 360., 192.];
        assert!(!crowds(rect, notes, WIDGET_GAP), "{focus:?}");
        assert!((focus[0] - 1536.).abs() < 1. || (focus[1] - 488.).abs() < 1.);
        // Already clear: unchanged.
        assert_eq!(
            free_spot([100., 100.], [200., 100.], &[notes], screen),
            Some([100., 100.])
        );
        // Stays on screen.
        let low = free_spot(
            [1536., 900.],
            [360., 192.],
            &[[1536., 850., 360., 200.]],
            screen,
        )
        .unwrap();
        assert!(low[1] + 192. <= screen[1] && low[1] >= 0.);
        // No room anywhere: no spot, rather than on top of another.
        let full = [0., 0., screen[0], screen[1]];
        assert_eq!(free_spot([100., 100.], [200., 100.], &[full], screen), None);
    }

    use super::*;
    #[test]
    fn resize_preserves_an_anchor_and_reaches_safe_edges_on_fine_grid() {
        use crate::drawer::Widget;
        let point = [120., 144.];
        assert_eq!(
            resize(Widget::Music, point, [481., 319.], [1920., 1080.], true),
            [480., 324.]
        );
        assert_eq!(
            resize(Widget::Music, point, [9000., 9000.], [1920., 1080.], true),
            [1776., 888.]
        );
        for widget in Widget::ALL {
            let min = minimum_size(widget);
            assert_eq!(
                resize(widget, point, [-10., f32::NAN], [1920., 1080.], false),
                min
            );
            assert_eq!(sized(widget, [9000., 9000.], [820., 600.]), [772., 504.]);
        }
    }

    #[test]
    fn fine_grid_pitch_stays_small_on_every_screen_and_for_legacy_preferences() {
        for screen in [[1280., 720.], [1920., 1080.], [3840., 2160.]] {
            for columns in [12, 16, 24] {
                assert_eq!(
                    snap([260., 270.], [360., 180.], screen, columns),
                    [264., 276.],
                    "screen {screen:?}, legacy columns {columns}"
                );
                assert_eq!(grid_step(screen, columns), [12., 12.]);
            }
        }
    }

    #[test]
    fn fine_grid_places_cards_close_to_shortened_neighbors_without_overlap() {
        for height in [180., 198., 220., 280., 320.] {
            let neighbor_bottom = 252. + height;
            let placed = snap(
                [408., neighbor_bottom + 12.],
                [360., 180.],
                [1920., 1080.],
                24,
            );
            let gap = placed[1] - neighbor_bottom;
            assert!((6.0..=18.).contains(&gap), "height {height}, gap {gap}");
        }
    }

    #[test]
    fn grid_aligns_widgets_to_a_common_origin_and_respects_screen_edges() {
        let a = snap([260., 270.], [360., 180.], [1920., 1080.], 16);
        let b = snap([260., 270.], [490., 200.], [1920., 1080.], 16);
        assert_eq!(a, b);
        assert_eq!(a, [264., 276.]);
        let edge = snap([3000., 3000.], [360., 180.], [1920., 1080.], 16);
        assert_eq!(edge, [1536., 852.]);
        assert_eq!([1920. - edge[0] - 360., 1080. - edge[1] - 180.], [24., 48.]);
    }
    #[test]
    fn every_density_reaches_both_safe_boundaries_for_all_widget_sizes() {
        let sizes = [
            [490., 200.],
            [360., 320.],
            [360., 180.],
            [360., 280.],
            [360., 198.],
        ];
        for columns in [12, 16, 24] {
            for screen in [
                [1920., 1080.],
                [1280., 720.],
                [800., 600.],
                [380., 230.],
                [300., 200.],
                [1., 1.],
            ] {
                for size in sizes {
                    for point in [[f32::MAX; 2], [-f32::MAX; 2]] {
                        let expected = clamp(point, size, screen);
                        let snapped = snap(point, size, screen, columns);
                        assert_eq!(
                            snapped, expected,
                            "density {columns}, screen {screen:?}, widget {size:?}"
                        );
                        assert_eq!(snap(snapped, size, screen, columns), snapped);
                        assert_eq!(decode(encode(snapped, size, screen), size, screen), snapped);
                    }
                }
            }
        }
    }
    #[test]
    fn safe_boundary_competes_with_the_nearest_grid_line_on_each_axis() {
        let size = [365., 198.];
        let screen = [1920., 1080.];
        assert_eq!(snap([1527.4, 830.9], size, screen, 16), [1524., 828.]);
        assert_eq!(snap([1527.6, 831.1], size, screen, 16), [1531., 834.]);
        assert_eq!(snap([1531., 270.], size, screen, 16), [1531., 276.]);
        assert_eq!(snap([260., 834.], size, screen, 16), [264., 834.]);
    }
    #[test]
    fn right_and_bottom_edges_are_reachable_with_the_pointer_inside_the_miniature() {
        let sizes = [
            [490., 200.],
            [360., 320.],
            [360., 180.],
            [360., 280.],
            [360., 198.],
        ];
        for screen in [[1920., 1080.], [800., 600.], [380., 230.]] {
            for size in sizes {
                if size[0] > screen[0] || size[1] > screen[1] {
                    continue;
                }
                let edge = clamp([f32::MAX; 2], size, screen);
                for grab in [0., 0.5, 0.99] {
                    let offset = [size[0] * grab, size[1] * grab];
                    for scale in [0.25, 250. / screen[1]] {
                        let preview_pointer =
                            [(edge[0] + offset[0]) * scale, (edge[1] + offset[1]) * scale];
                        let pointer = [preview_pointer[0] / scale, preview_pointer[1] / scale];
                        assert!(
                            (0..2).all(|axis| pointer[axis] >= 0. && pointer[axis] <= screen[axis]),
                            "edge placement must not require an outside drop"
                        );
                        let point = [pointer[0] - offset[0], pointer[1] - offset[1]];
                        for columns in [12, 16, 24] {
                            assert_eq!(snap(point, size, screen, columns), edge);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn grid_handles_invalid_input_and_small_screens_without_invalid_coordinates() {
        for columns in [0, 12, 16, 24, 255] {
            let point = snap(
                [f32::NAN, f32::INFINITY],
                [490., 200.],
                [300., 200.],
                columns,
            );
            assert_eq!(point, [0., 0.]);
            let point = snap([400., 300.], [360., 180.], [1280., 720.], columns);
            assert!(point.iter().all(|v| v.is_finite() && *v >= 0.));
        }
    }
    #[test]
    fn grid_positions_round_trip_from_the_scaled_preview_to_the_desktop() {
        let screen = [1920., 1080.];
        let size = [360., 180.];
        let preview_scale = 0.3;
        let pointer = [231., 147.];
        let offset = [30., 15.];
        let point = snap(
            [
                (pointer[0] - offset[0]) / preview_scale,
                (pointer[1] - offset[1]) / preview_scale,
            ],
            size,
            screen,
            16,
        );
        let restored = decode(encode(point, size, screen), size, screen);
        assert!((point[0] - restored[0]).abs() < 0.001);
        assert!((point[1] - restored[1]).abs() < 0.001);
        assert_eq!(point, snap(point, size, screen, 16));
    }
    #[test]
    fn dragged_widgets_keep_clear_of_panel_and_screen_edges() {
        assert_eq!(
            clamp([-20., 2000.], [360., 300.], [1920., 1080.]),
            [24., 732.]
        );
    }
    #[test]
    fn placement_survives_resizing_without_losing_the_right_edge() {
        let point = encode([1536., 732.], [360., 300.], [1920., 1080.]);
        assert_eq!(decode(point, [360., 300.], [1280., 800.]), [896., 452.]);
    }
    #[test]
    fn dragging_retains_where_you_grabbed_the_widget() {
        assert_eq!(
            drag([500., 400.], [50., 30.], [360., 300.], [1920., 1080.]),
            [450., 370.]
        );
    }
    #[test]
    fn tiny_displays_and_invalid_coordinates_never_produce_negative_or_nan_positions() {
        assert_eq!(
            clamp([f32::NAN, f32::INFINITY], [490., 540.], [300., 400.]),
            [0., 0.]
        );
        assert_eq!(decode([1., 1.], [490., 540.], [300., 400.]), [0., 0.]);
    }
    #[test]
    fn normalization_round_trips_without_rounding_drift() {
        let point = [651.5, 320.25];
        let size = [360., 180.];
        let screen = [1920., 1080.];
        let restored = decode(encode(point, size, screen), size, screen);
        assert!((restored[0] - point[0]).abs() < 0.001 && (restored[1] - point[1]).abs() < 0.001);
    }
}
