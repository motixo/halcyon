//! Two stable child trees reflow with available space; resizing never replaces
//! the editor tree, focus or scroll position. All dimensions are logical pixels.
use super::*;
use cosmic::iced::advanced::widget::{Operation, Tree};
use cosmic::iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer};
use cosmic::iced::{Event, Rectangle, Size, Vector, mouse};
use cosmic::{Renderer, Theme};

pub(super) fn columns<'a>(
    left: Element<'a, Message>,
    right: Element<'a, Message>,
    minimum: [f32; 2],
    inspector: bool,
) -> Element<'a, Message> {
    Element::new(Columns {
        children: vec![left, right],
        minimum,
        inspector,
        field: false,
    })
}
pub(super) fn field<'a>(
    left: Element<'a, Message>,
    right: Element<'a, Message>,
) -> Element<'a, Message> {
    Element::new(Columns {
        children: vec![left, right],
        minimum: [100., 0.],
        inspector: false,
        field: true,
    })
}
struct Columns<'a> {
    children: Vec<Element<'a, Message>>,
    minimum: [f32; 2],
    inspector: bool,
    field: bool,
}
impl Widget<Message, Theme, Renderer> for Columns<'_> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }
    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children);
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        use cosmic::iced::advanced::text::Renderer as _;
        let scale = (renderer.default_size().0 / 14.).max(1.);
        let width = limits.max().width;
        let gap = if self.field { 8. } else { 16. } * scale;
        let right = if self.field {
            self.children[1]
                .as_widget_mut()
                .layout(
                    &mut tree.children[1],
                    renderer,
                    &layout::Limits::new(Size::ZERO, limits.max()),
                )
                .size()
                .width
        } else if self.inspector {
            (width * 0.38).clamp(
                self.minimum[1] * scale,
                312_f32.max(self.minimum[1]) * scale,
            )
        } else {
            (width - gap) / 2.
        };
        let side = if self.field {
            width >= self.minimum[0] * scale + right + gap
        } else {
            width >= (self.minimum[0] + self.minimum[1]) * scale + gap
        };
        let widths = if side {
            [width - right - gap, right]
        } else {
            [width, width]
        };
        let mut nodes = Vec::new();
        for (i, (child, state)) in self.children.iter_mut().zip(&mut tree.children).enumerate() {
            let child_limits = layout::Limits::new(
                Size::new(widths[i], 0.),
                Size::new(widths[i], limits.max().height),
            );
            nodes.push(child.as_widget_mut().layout(state, renderer, &child_limits));
        }
        let height = if side {
            nodes[0].size().height.max(nodes[1].size().height)
        } else {
            nodes[0].size().height + gap + nodes[1].size().height
        };
        let offset = if side {
            Vector::new(widths[0] + gap, 0.)
        } else {
            Vector::new(0., nodes[0].size().height + gap)
        };
        nodes[1] = nodes[1].clone().translate(offset);
        layout::Node::with_children(Size::new(width, height), nodes)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, state), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(state, layout, renderer, operation);
            }
        });
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
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }
    fn drag_destinations(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        dnd_rectangles: &mut cosmic::iced::advanced::clipboard::DndDestinationRectangles,
    ) {
        for ((child, child_layout), state) in self
            .children
            .iter()
            .zip(layout.children())
            .zip(state.children.iter())
        {
            child.as_widget().drag_destinations(
                state,
                child_layout.with_virtual_offset(layout.virtual_offset()),
                renderer,
                dnd_rectangles,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structural_columns_reflow_for_enlarged_text_and_long_controls() {
        use cosmic::iced::advanced::renderer::Headless;
        for (font_size, stacked) in [(14., false), (22., true)] {
            let renderer = cosmic::iced::futures::executor::block_on(<Renderer as Headless>::new(
                cosmic::font::default(),
                cosmic::iced::Pixels(font_size),
                Some("tiny-skia"),
            ))
            .unwrap();
            let mut content = view();
            let mut tree = Tree::new(&content);
            let layout = content.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, Size::new(616., 1000.)),
            );
            let left = layout.children()[0].bounds();
            let right = layout.children()[1].bounds();
            assert_eq!(right.y >= left.y + left.height, stacked);
            assert!(right.x + right.width <= 616.);
        }
        let mut native = crate::native_test::Native::new(Size::new(320., 600.));
        let content = || {
            field(
                container(label("Location", 14., FG))
                    .id("field-label")
                    .into(),
                button::standard("Move the selected widgets to this display")
                    .id("field-action".into())
                    .on_press(Message::ToggleHelp("test"))
                    .into(),
            )
        };
        let label = native.bounds(content(), "field-label");
        let action = native.bounds(content(), "field-action");
        assert!(action.y >= label.y + label.height);
        assert!(action.x + action.width <= 320.);
    }
    fn view() -> Element<'static, Message> {
        columns(
            container(label("Preview", 14., FG))
                .id("stage")
                .width(Length::Fill)
                .height(180)
                .into(),
            container(
                text_input("Name", "Unsaved name")
                    .id("draft-name".into())
                    .on_input(|_| Message::ToggleHelp("test")),
            )
            .id("inspector")
            .width(Length::Fill)
            .into(),
            [320., 248.],
            true,
        )
    }
    #[test]
    fn structural_columns_reflow_without_replacing_focused_draft() {
        let mut native = crate::native_test::Native::new(Size::new(616., 450.));
        let stage = native.bounds(view(), "stage");
        let controls = native.bounds(view(), "inspector");
        assert!(controls.x >= stage.x + stage.width + 15. && controls.width >= 248.);
        native.operate(
            view(),
            Box::new(cosmic::iced::advanced::widget::operation::focusable::focus(
                cosmic::widget::Id::new("draft-name"),
            )),
        );
        native.size = Size::new(480., 450.);
        let stage = native.bounds(view(), "stage");
        let controls = native.bounds(view(), "inspector");
        assert!(controls.y >= stage.y + stage.height + 15.);
        // The same focused text input survives the orientation change.
        let (_, status) = native.event(
            view(),
            cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                key: cosmic::iced::keyboard::Key::Character("x".into()),
                modified_key: cosmic::iced::keyboard::Key::Character("x".into()),
                physical_key: cosmic::iced::keyboard::key::Physical::Code(
                    cosmic::iced::keyboard::key::Code::KeyX,
                ),
                location: cosmic::iced::keyboard::Location::Standard,
                modifiers: cosmic::iced::keyboard::Modifiers::empty(),
                text: Some("x".into()),
                repeat: false,
            }),
            cosmic::iced::Point::ORIGIN,
        );
        assert_eq!(status, cosmic::iced::event::Status::Captured);
    }
}
