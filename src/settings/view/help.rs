//! Supplementary, noninteractive help. Hover, keyboard focus and pinned touch
//! help share one overlay; dismissal never changes the form's geometry.
use super::*;
use cosmic::iced::advanced::widget::{Operation, Tree, operation::Focusable, tree};
use cosmic::iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer};
use cosmic::iced::{Event, Point, Rectangle, Size, Vector, keyboard, mouse, window};
use cosmic::{Renderer, Theme};
use std::cell::Cell;
use std::time::{Duration, Instant};

pub(super) fn explanation<'a>(
    key: &'static str,
    title: &'static str,
    explanation: &'static str,
    pinned: bool,
) -> Element<'a, Message> {
    let content = button::custom(container(label("?", 13., MUTED)).center_x(28).center_y(28))
        .id(cosmic::widget::Id::new(format!("help-{key}")))
        .width(28)
        .height(28)
        .padding(0)
        .on_press(Message::ToggleHelp(key))
        .into();
    let popup = container(
        column::with_children(vec![label(title, 13., FG), label(explanation, 13., MUTED)])
            .spacing(6),
    )
    .id(cosmic::widget::Id::new(format!("help-text-{key}")))
    .padding(12)
    .max_width(320)
    .class(cosmic::theme::Container::Secondary)
    .into();
    Element::new(Help {
        content,
        popup,
        key,
        pinned,
    })
}
struct Help<'a> {
    content: Element<'a, Message>,
    popup: Element<'a, Message>,
    key: &'static str,
    pinned: bool,
}
#[derive(Default)]
struct State {
    focused: bool,
    dismissed: bool,
    hovered: Option<Instant>,
    open: bool,
    pinned: bool,
    bounds: Cell<Rectangle>,
}
#[derive(Default)]
struct Focus(bool);
impl Operation for Focus {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn focusable(
        &mut self,
        _: Option<&cosmic::widget::Id>,
        _: Rectangle,
        state: &mut dyn Focusable,
    ) {
        self.0 |= state.is_focused();
    }
}
impl Widget<Message, Theme, Renderer> for Help<'_> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.popup)]
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut [&mut self.content, &mut self.popup]);
        let state = tree.state.downcast_mut::<State>();
        if self.pinned != state.pinned {
            state.pinned = self.pinned;
            state.dismissed = !self.pinned;
        }
    }
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
        let mut focus = Focus::default();
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, &mut focus);
        let state = tree.state.downcast_mut::<State>();
        if state.focused != focus.0 {
            state.focused = focus.0;
            state.dismissed = false;
        }
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let mut focus = Focus::default();
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, &mut focus);
        let state = tree.state.downcast_mut::<State>();
        if state.focused != focus.0 {
            state.focused = focus.0;
            state.dismissed = false;
        }
        let visible = !state.dismissed && (self.pinned || state.focused || state.open);
        if visible
            && matches!(
                event,
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                })
            )
        {
            state.dismissed = true;
            state.open = false;
            state.hovered = None;
            if self.pinned {
                shell.publish(Message::ToggleHelp(self.key));
            }
            shell.capture_event();
            shell.request_redraw();
            return;
        }
        let over =
            cursor.is_over(layout.bounds()) || (visible && cursor.is_over(state.bounds.get()));
        if matches!(
            event,
            Event::Mouse(_) | Event::Window(window::Event::RedrawRequested(_))
        ) {
            if over {
                if state.hovered.is_none() {
                    state.hovered = Some(Instant::now());
                }
                let due = state.hovered.unwrap() + Duration::from_millis(250);
                if match event {
                    Event::Window(window::Event::RedrawRequested(now)) => *now,
                    _ => Instant::now(),
                } >= due
                {
                    if !state.open {
                        state.open = true;
                        shell.request_redraw();
                    }
                } else {
                    shell.request_redraw_at(due);
                }
            } else {
                if state.open {
                    shell.request_redraw();
                }
                state.hovered = None;
                state.open = false;
                if !state.focused && !self.pinned {
                    state.dismissed = false;
                }
            }
        }
        if self.pinned
            && !over
            && matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(_))
                    | Event::Touch(cosmic::iced::touch::Event::FingerPressed { .. })
            )
        {
            shell.publish(Message::ToggleHelp(self.key));
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
    fn drag_destinations(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        rectangles: &mut cosmic::iced::advanced::clipboard::DndDestinationRectangles,
    ) {
        self.content
            .as_widget()
            .drag_destinations(&tree.children[0], layout, renderer, rectangles);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_ref::<State>();
        if state.dismissed
            || !(self.pinned || state.focused || state.open)
            || !layout.bounds().intersects(viewport)
        {
            return None;
        }
        Some(overlay::Element::new(Box::new(Popup {
            content: &mut self.popup,
            tree: &mut tree.children[1],
            anchor: layout.bounds() + translation,
            bounds: &state.bounds,
        })))
    }
}
struct Popup<'a, 'b> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    anchor: Rectangle,
    bounds: &'b Cell<Rectangle>,
}
impl overlay::Overlay<Message, Theme, Renderer> for Popup<'_, '_> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let node = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(bounds.width.min(320.), bounds.height)),
        );
        let x = self
            .anchor
            .x
            .min((bounds.width - node.size().width).max(0.))
            .max(0.);
        let y = if self.anchor.y + self.anchor.height + node.size().height <= bounds.height {
            self.anchor.y + self.anchor.height
        } else {
            (self.anchor.y - node.size().height).max(0.)
        };
        let node = node.move_to(Point::new(x, y));
        self.bounds.set(node.bounds());
        node
    }
    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }
    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view(pinned: bool) -> Element<'static, Message> {
        column::with_children(vec![
            explanation(
                "test",
                "Preview",
                "This changes the preview only. Your desktop stays as it is.",
                pinned,
            ),
            container(label("Next control", 14., FG))
                .id("following-row")
                .into(),
        ])
        .into()
    }
    fn escape() -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }
    #[test]
    fn structural_help_focus_hover_pin_and_escape_leave_rows_still() {
        let mut native = crate::native_test::Native::new(Size::new(420., 240.));
        let before = native.bounds(view(false), "following-row");
        native.operate(
            view(false),
            Box::new(cosmic::iced::advanced::widget::operation::focusable::focus(
                cosmic::widget::Id::new("help-test"),
            )),
        );
        let (_, status) = native.event(view(false), escape(), Point::new(400., 200.));
        assert_eq!(
            status,
            cosmic::iced::event::Status::Captured,
            "focused explanation is dismissible"
        );
        assert_eq!(native.bounds(view(false), "following-row"), before);
        let (messages, status) = native.event(view(true), escape(), Point::new(400., 200.));
        assert_eq!(status, cosmic::iced::event::Status::Captured);
        assert!(matches!(messages.as_slice(), [Message::ToggleHelp("test")]));
        let mut native = crate::native_test::Native::new(Size::new(420., 240.));
        let point = native.bounds(view(false), "help-test").center();
        native.event(
            view(false),
            Event::Mouse(mouse::Event::CursorMoved { position: point }),
            point,
        );
        native.event(
            view(false),
            Event::Window(window::Event::RedrawRequested(
                Instant::now() + Duration::from_secs(1),
            )),
            point,
        );
        let popup_point = Point::new(100., 44.);
        native.event(
            view(false),
            Event::Mouse(mouse::Event::CursorMoved {
                position: popup_point,
            }),
            popup_point,
        );
        let (_, status) = native.event(view(false), escape(), popup_point);
        assert_eq!(
            status,
            cosmic::iced::event::Status::Captured,
            "pointer can enter the explanation"
        );
        assert_eq!(native.bounds(view(false), "following-row"), before);
    }
}
