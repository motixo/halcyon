//! Clock hands are graphics; the readable time stays a native text element.
use super::*;
use cosmic::iced::{Rectangle, mouse};
use cosmic::widget::canvas;

struct Dial {
    reading: ClockReading,
    palette: Palette,
}
impl<Message> canvas::Program<Message, cosmic::Theme> for Dial {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &cosmic::Renderer,
        _: &cosmic::Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2., bounds.height / 2.);
        let radius = (bounds.width.min(bounds.height) / 2. - 2.).max(1.);
        let point = |turn: f32, fraction: f32| {
            let angle = turn * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            Point::new(
                center.x + angle.cos() * radius * fraction,
                center.y + angle.sin() * radius * fraction,
            )
        };
        frame.stroke(
            &canvas::Path::circle(center, radius),
            canvas::Stroke::default()
                .with_color(self.palette.muted)
                .with_width(0.8),
        );
        for tick in 0..60 {
            let major = tick % 5 == 0;
            let turn = tick as f32 / 60.;
            frame.stroke(
                &canvas::Path::line(
                    point(turn, if major { 0.82 } else { 0.90 }),
                    point(turn, 0.94),
                ),
                canvas::Stroke::default()
                    .with_color(if major {
                        self.palette.foreground
                    } else {
                        self.palette.muted
                    })
                    .with_width(if major { 1.4 } else { 0.6 }),
            );
        }
        for (turn, length, color, width) in [
            (
                self.reading.hour_turn,
                0.52,
                self.palette.accent,
                (radius * 0.055).clamp(1.5, 6.),
            ),
            (
                self.reading.minute_turn,
                0.77,
                self.palette.foreground,
                (radius * 0.027).clamp(1., 3.5),
            ),
        ] {
            frame.stroke(
                &canvas::Path::line(point(turn, -0.09), point(turn, length)),
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(width)
                    .with_line_cap(canvas::LineCap::Round),
            );
        }
        frame.fill(
            &canvas::Path::circle(center, (radius * 0.035).clamp(2., 4.)),
            self.palette.foreground,
        );
        vec![frame.into_geometry()]
    }
}

pub(super) fn view(
    reading: ClockReading,
    width: f32,
    height: f32,
    p: Palette,
    font: Font,
    alignment: ClockAlignment,
) -> Element<'static, Message> {
    let caption_size = (height * 0.09).clamp(8., 13.);
    let diameter = width.min((height - caption_size * 1.2 - 3.).max(1.));
    let caption = reading.period.as_ref().map_or_else(
        || reading.time.clone(),
        |period| format!("{} {period}", reading.time),
    );
    container(
        column::with_children(vec![
            canvas::Canvas::new(Dial {
                reading,
                palette: p,
            })
            .width(diameter)
            .height(diameter)
            .into(),
            text::body(caption)
                .size(caption_size)
                .font(font)
                .class(cosmic::theme::Text::Color(p.muted))
                .into(),
        ])
        .align_x(Alignment::Center)
        .spacing(3),
    )
    .width(width)
    .height(height)
    .align_x(alignment.layout())
    .into()
}
