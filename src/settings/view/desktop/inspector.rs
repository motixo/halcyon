//! Related groups reflow without replacing their native widget trees.
use super::*;
pub(in crate::settings::view) fn paired<'a>(
    _: bool,
    first: Element<'a, Message>,
    second: Element<'a, Message>,
) -> Element<'a, Message> {
    crate::settings::view::adaptive::columns(first, second, [280., 280.], false)
}
