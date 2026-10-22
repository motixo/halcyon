//! A click target with the full height of the progress row, no hover tooltip.
use cosmic::iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, renderer, widget::Tree,
};
use cosmic::iced::{Background, Border, Color, Event, Length, Rectangle, Size, mouse};
use cosmic::{Element, Renderer, Theme};

pub fn view<'a, M: 'a>(
    progress: f32,
    color: Color,
    on_seek: impl Fn(f32) -> M + 'a,
) -> Element<'a, M> {
    Element::new(SeekBar {
        progress: progress.clamp(0., 1.),
        color,
        on_seek: Box::new(on_seek),
    })
}
struct SeekBar<'a, M> {
    progress: f32,
    color: Color,
    on_seek: Box<dyn Fn(f32) -> M + 'a>,
}
impl<M> Widget<M, Theme, Renderer> for SeekBar<'_, M> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(14.))
    }
    fn layout(&mut self, _: &mut Tree, _: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, 14., Size::ZERO))
    }
    fn update(
        &mut self,
        _: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) {
            if let Some(point) = cursor
                .position_in(layout.bounds())
                .filter(|_| cursor.is_over(*viewport))
            {
                shell.publish((self.on_seek)(
                    (point.x / layout.bounds().width.max(1.)).clamp(0., 1.),
                ));
                shell.capture_event();
            }
        }
    }
    fn mouse_interaction(
        &self,
        _: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        _: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) && cursor.is_over(*viewport) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
    fn draw(
        &self,
        _: &Tree,
        renderer: &mut Renderer,
        _: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Rectangle,
    ) {
        let b = layout.bounds();
        let rail = Rectangle {
            x: b.x,
            y: b.center_y() - 2.,
            width: b.width,
            height: 4.,
        };
        let mut paint = |bounds: Rectangle, color: Color, radius: f32| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: radius.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Background::Color(color),
            )
        };
        paint(
            rail,
            Color {
                a: 0.16,
                ..self.color
            },
            2.,
        );
        paint(
            Rectangle {
                width: rail.width * self.progress,
                ..rail
            },
            self.color,
            2.,
        );
        if cursor.is_over(b) {
            let x =
                (b.x + b.width * self.progress).clamp(b.x + 3., (b.x + b.width - 3.).max(b.x + 3.));
            paint(
                Rectangle {
                    x: x - 3.,
                    y: b.center_y() - 3.,
                    width: 6.,
                    height: 6.,
                },
                self.color,
                3.,
            );
        }
    }
}
