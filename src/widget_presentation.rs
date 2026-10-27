//! Shared logical sizes and interaction policy. No providers or persistence.
use crate::drawer::Widget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Live,
    LocalPreview,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub natural: [f32; 2],
    pub minimum: [f32; 2],
}

pub const CLOCK_SIZE: [f32; 2] = [490., 200.];
pub const WEATHER_SIZE: [f32; 2] = [360., 180.];

pub fn metrics(widget: Widget) -> Metrics {
    Metrics {
        natural: match widget {
            Widget::Clock => CLOCK_SIZE,
            Widget::Weather => WEATHER_SIZE,
            _ => unreachable!("Clock/Weather presentation metrics"),
        },
        minimum: crate::layout::minimum_size(widget),
    }
}

/// Preview bounds may be smaller than the editor minimum. Keep them finite;
/// the placement owner still enforces user-facing resize limits separately.
pub fn viewport(requested: [f32; 2], natural: [f32; 2]) -> [f32; 2] {
    [0, 1].map(|axis| {
        if requested[axis].is_finite() {
            requested[axis].clamp(1., 32768.)
        } else {
            natural[axis]
        }
    })
}

/// Wayland input uses integer logical coordinates, independently of output
/// scale. Clip to the surface and round inward so decoration and empty space
/// never acquire input merely because the renderer uses fractional geometry.
pub fn input_rect(origin: [f32; 2], size: [f32; 2], surface: [f32; 2]) -> Option<[u32; 4]> {
    if origin
        .into_iter()
        .chain(size)
        .chain(surface)
        .any(|v| !v.is_finite())
        || size
            .into_iter()
            .chain(surface)
            .any(|v| v <= 0. || v > 32768.)
    {
        return None;
    }
    let start = [0, 1].map(|i| origin[i].max(0.).ceil());
    let end = [0, 1].map(|i| (origin[i] + size[i]).min(surface[i]).floor());
    (0..2).all(|i| end[i] > start[i]).then(|| {
        [
            start[0] as u32,
            start[1] as u32,
            (end[0] - start[0]) as u32,
            (end[1] - start[1]) as u32,
        ]
    })
}
