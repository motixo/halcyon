use super::*;

fn settings(mode: Placement, gap: u16) -> Settings {
    Settings {
        enabled: true,
        placement: mode,
        gap,
        ..Settings::default()
    }
}
fn layout(s: &Settings, width: f64, sizes: [f64; 3]) -> Layout {
    resolve(
        s,
        "DP-1",
        Span {
            start: 7.,
            length: width,
        },
        sizes,
        32.,
    )
}

#[test]
fn all_sizes_and_positions_stay_in_bounds_without_overlap() {
    for mode in [Placement::Together, Placement::Separate] {
        for gap in [0, 14, 64] {
            for width in [1., 32., 70., 110., 320., 820., 1920.] {
                for sizes in [
                    [0.; 3],
                    [80., 0., 0.],
                    [0., 90., 55.],
                    [90., 380., 150.],
                    [1500., 80., 90.],
                ] {
                    for pos in [[0, 0, 0], [10000, 10000, 10000], [9000, 500, 4000]] {
                        let mut s = settings(mode, gap);
                        s.default_layout.separate.positions = pos;
                        let original = s.clone();
                        let l = layout(&s, width, sizes);
                        let mut spans: Vec<_> = l.spans.iter().flatten().collect();
                        spans.sort_by(|a, b| a.start.total_cmp(&b.start));
                        for span in &spans {
                            assert!(span.start >= 7. - 1e-6);
                            assert!(span.end() <= 7. + width + 1e-6);
                        }
                        for pair in spans.windows(2) {
                            assert!(pair[0].end() + l.gap <= pair[1].start + 1e-6);
                        }
                        assert_eq!(
                            s, original,
                            "temporary resolution must not change preferences"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn compact_group_has_exact_gap_and_can_move_to_both_ends() {
    let mut s = settings(Placement::Together, 14);
    let l = layout(&s, 820., [60., 140., 80.]);
    assert_eq!(l.group.length, 308.);
    assert_eq!(l.spans[0].unwrap().end() + 14., l.spans[1].unwrap().start);
    move_handle(&mut s, "DP-1", &l, Handle::Group, -500., true);
    assert_eq!(layout(&s, 820., [60., 140., 80.]).group.start, 7.);
    move_handle(&mut s, "DP-1", &l, Handle::Group, 2000., true);
    assert_eq!(layout(&s, 820., [60., 140., 80.]).group.end(), 827.);
}
#[test]
fn empty_sections_occupy_no_gap_or_background() {
    let l = layout(&settings(Placement::Together, 64), 800., [0., 90., 0.]);
    assert_eq!(l.group.length, 90.);
    assert_eq!(l.gap, 0.);
    assert!(l.spans[0].is_none());
}
#[test]
fn content_overflows_before_short_sections_shrink() {
    let l = layout(&settings(Placement::Separate, 14), 500., [60., 900., 70.]);
    assert_eq!(l.budgets, [60., 342., 70.]);
    assert_eq!(l.gap, 14.);
}
#[test]
fn monitor_and_mode_switches_preserve_both_layouts() {
    let mut s = settings(Placement::Separate, 14);
    s.layout_mut("DP-1").separate.positions = [900, 4000, 9500];
    s.layout_mut("HDMI-A-1").together.position = 10000;
    let first = layout(&s, 1920., [60., 180., 70.]);
    s.placement = Placement::Together;
    let _ = layout(&s, 700., [60., 180., 70.]);
    s.placement = Placement::Separate;
    assert_eq!(first, layout(&s, 1920., [60., 180., 70.]));
    assert_eq!(s.layout("HDMI-A-1").together.position, 10000);
    assert!(s.portable().outputs.is_empty());
}
#[test]
fn drag_threshold_cancel_and_separate_reordering() {
    let mut s = settings(Placement::Separate, 14);
    let before = s.clone();
    let l = layout(&s, 800., [80.; 3]);
    let mut drag = Drag::new(&s, &l, Handle::Island(Section::Start), 9.).unwrap();
    assert!(!drag.motion(&mut s, "DP-1", &l, 12.));
    assert_eq!(s, before);
    assert!(drag.motion(&mut s, "DP-1", &l, 799.));
    assert_eq!(
        s.layout("DP-1").separate.order,
        [Section::Center, Section::End, Section::Start]
    );
    drag.cancel(&mut s);
    assert_eq!(s, before);
}
#[test]
fn malformed_or_newer_data_is_rejected() {
    let mut s = Settings::default();
    s.version = 2;
    assert!(s.validate().is_err());
    s.version = 1;
    s.gap = 65;
    assert!(s.validate().is_err());
    s.gap = 14;
    s.default_layout.together.order = [Section::Start; 3];
    assert!(s.validate().is_err());
    let mut json = serde_json::to_value(Settings::default()).unwrap();
    json["future"] = true.into();
    assert!(serde_json::from_value::<Settings>(json).is_err());
}

#[test]
fn keyboard_reordering_skips_empty_sections_and_preserves_their_slot() {
    let mut a = Arrangement::default();
    let present = [true, false, true];
    a.reorder_visible(Placement::Together, Section::Start, present, 1);
    assert_eq!(
        a.together.order,
        [Section::End, Section::Center, Section::Start]
    );
    a.reorder_visible(Placement::Together, Section::Start, present, -1);
    assert_eq!(a.together.order, Section::ALL);
    a.reorder_visible(Placement::Together, Section::End, present, -3);
    assert_eq!(
        a.together.order,
        [Section::End, Section::Center, Section::Start]
    );
    let saved = a.clone();
    a.reorder_visible(Placement::Together, Section::Center, present, 1);
    assert_eq!(a, saved);
}
