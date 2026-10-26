//! Layout only: no selection or commit state is duplicated by the inspector.
use super::*;

/// Related native groups sit together when each has enough room; otherwise
/// they stack in reading order, preserving the controls and their widget IDs.
pub(in crate::settings::view) fn paired<'a>(
    wide: bool,
    first: Element<'a, Message>,
    second: Element<'a, Message>,
) -> Element<'a, Message> {
    if wide {
        row::with_children(vec![
            container(first).width(Length::FillPortion(1)).into(),
            container(second).width(Length::FillPortion(1)).into(),
        ])
        .spacing(16)
        .align_y(Alignment::Start)
        .into()
    } else {
        column::with_children(vec![first, second])
            .spacing(16)
            .into()
    }
}
