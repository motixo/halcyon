//! Desktop icons subpage: a schematic of the drafted icon size, spacing and
//! what the desktop shows, the controls, and the deferred-refresh note.
use super::desktop::{ERROR, WARN, banner, conflict_banner, kept};
use super::windows::{recovery_banner, store_pill};
use super::*;
use crate::desktop::desktop_icons::{DesktopIcons, SCALE_MAX, SCALE_MIN, SCALE_STEP, Show};
use crate::desktop::panels::Setting;
use crate::settings::desktop_icons::{Message as Edit, REFRESH_NOTE, UNAVAILABLE};
use cosmic::iced::{Point, Rectangle, Size, mouse};
use cosmic::widget::canvas;

fn edit(message: Edit) -> Message {
    Message::DesktopIcons(message)
}

/// Preview size (16:9).
const PREVIEW: (f32, f32) = (288., 162.);

/// What the miniature desktop draws, in desktop (1920×1080) units.
struct Miniature {
    icons: DesktopIcons,
    accent: Color,
}

impl Miniature {
    /// One entry per item the desktop shows: (kind, colour).
    fn items(&self) -> Vec<Color> {
        let mut items = Vec::new();
        if self.icons.show_content {
            items.extend([self.accent; 4]);
        }
        if self.icons.show_mounted_drives {
            items.push(Color::from_rgb(0.62, 0.66, 0.74));
        }
        if self.icons.show_trash {
            items.push(MUTED);
        }
        items
    }
}

impl<Message> canvas::Program<Message, cosmic::Theme> for Miniature {
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
        // cosmic-files lays items out in columns from the top left; a cell
        // grows with the icon size and the spacing scales it. The miniature
        // shows that corner (a quarter of a 1920×1080 desktop) at twice the
        // scale, so size and spacing changes stay visible.
        let (desk_w, desk_h) = (960f32, 540f32);
        let size = f32::from(self.icons.icon_size.get()) / 100.;
        let spacing = f32::from(self.icons.grid_spacing.get()) / 100.;
        let icon = 64. * size;
        let cell = Size::new((icon * 1.5).max(96.) * spacing, (icon + 40.) * spacing);
        let margin = 24.;
        let items = self.items();
        let rows = (((desk_h - 2. * margin) / cell.height).floor() as usize).max(1);
        let columns = items.len().div_ceil(rows).max(1);
        // Very large icons are drawn smaller so every item stays in view.
        let need_w = margin * 2. + columns as f32 * cell.width;
        let need_h = margin * 2. + rows.min(items.len().max(1)) as f32 * cell.height;
        let scale = (bounds.width / desk_w)
            .min(bounds.width / need_w)
            .min(bounds.height / need_h);
        for (index, color) in items.iter().enumerate() {
            let (column, row) = (index / rows, index % rows);
            let x = (margin + column as f32 * cell.width) * scale;
            let y = (margin + row as f32 * cell.height) * scale;
            let cell_w = cell.width * scale;
            let side = icon * scale;
            let left = x + (cell_w - side) / 2.;
            frame.fill(
                &canvas::Path::rounded_rectangle(
                    Point::new(left, y),
                    Size::new(side, side),
                    (side * 0.2).into(),
                ),
                Color { a: 0.85, ..*color },
            );
            // The label, as a bar under the icon.
            let bar_w = (cell_w * 0.8).min(side * 1.4);
            let bar_h = (10. * size * scale).max(2.);
            frame.fill(
                &canvas::Path::rounded_rectangle(
                    Point::new(x + (cell_w - bar_w) / 2., y + side + 6. * scale),
                    Size::new(bar_w, bar_h),
                    (bar_h / 2.).into(),
                ),
                Color { a: 0.55, ..FG },
            );
        }
        vec![frame.into_geometry()]
    }
}

impl Settings {
    pub(super) fn desktop_icons_page(&self) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let editor = &self.desktop_icons;
        let mut items: Vec<Element<'_, Message>> = Vec::new();
        let heading = || {
            column::with_children(vec![split(
                heading("Desktop icons", ""),
                store_pill(editor.store(), accent),
            )])
            .spacing(4)
            .width(Length::Fill)
        };
        if let Some(error) = editor.unavailable() {
            items.push(heading().into());
            items.push(banner(
                format!("Desktop icon settings are unavailable. {error}"),
                ERROR,
                Vec::new(),
            ));
            return column::with_children(items).spacing(14).into();
        }
        if editor.opened() && !editor.available() {
            items.push(heading().into());
            items.push(banner(UNAVAILABLE, WARN, Vec::new()));
            return column::with_children(items).spacing(14).into();
        }
        let Some(draft) = editor.draft() else {
            items.push(heading().into());
            items.push(label("Reading desktop icon settings…", 12., MUTED));
            return column::with_children(items).spacing(14).into();
        };
        let config = draft.config();
        let known = config.known().copied();
        items.push(heading().into());
        items.push(
            self.scene_frame(
                canvas(Miniature {
                    icons: known.unwrap_or_default(),
                    accent,
                })
                .width(PREVIEW.0)
                .height(PREVIEW.1)
                .into(),
                "Schematic · desktop items, icon size and spacing",
            ),
        );
        if let Some(banner) = recovery_banner(
            editor.recovery(),
            editor.writable(),
            edit(Edit::Recover),
            edit(Edit::KeepRecovery),
        ) {
            items.push(banner);
        }
        if !draft.conflicts().is_empty() {
            items.push(conflict_banner(
                crate::settings::system_appearance::describe(draft.conflicts()),
                edit(Edit::UseCurrent),
            ));
        }
        if let Setting::Unsupported(raw) = &config {
            items.push(banner(
                "These desktop icon settings include values this version doesn’t recognize. They are kept as they are and can’t be changed here.",
                WARN,
                vec![kept(raw)],
            ));
        }
        let show = |title: &'static str, what: Show, value: Option<bool>| {
            let mut toggle = toggler(value.unwrap_or(false));
            if value.is_some() {
                toggle = toggle.on_toggle(move |on| edit(Edit::Show(what, on)));
            }
            split(label(title, 13., FG), toggle.into())
        };
        items.push(group(
            Some("Desktop items"),
            vec![
                show(
                    "Desktop folder",
                    Show::Folder,
                    known.map(|icons| icons.show_content),
                ),
                show(
                    "Mounted drives",
                    Show::Drives,
                    known.map(|icons| icons.show_mounted_drives),
                ),
                show("Trash", Show::Trash, known.map(|icons| icons.show_trash)),
            ],
        ));
        let scale = |title: &'static str, value: Option<u16>, message: fn(u16) -> Edit| {
            let value_label = match value {
                Some(value)
                    if value % SCALE_STEP != 0 || !(SCALE_MIN..=SCALE_MAX).contains(&value) =>
                {
                    pill(format!("{value} % · kept"), WARN)
                }
                Some(value) => label(format!("{value} %"), 12., accent),
                None => label("—", 12., MUTED),
            };
            let mut parts = vec![split(label(title, 13., FG), value_label)];
            // Kept settings: no slider to drag (the toggles are inert too).
            if let Some(value) = value {
                parts.push(
                    slider(
                        SCALE_MIN..=SCALE_MAX,
                        value.clamp(SCALE_MIN, SCALE_MAX),
                        move |value| edit(message(value)),
                    )
                    .step(SCALE_STEP)
                    .on_release(edit(Edit::Apply))
                    .width(Length::Fill)
                    .apply(|control| completed_slider(control.into(), edit(Edit::Apply))),
                );
            }
            column::with_children(parts).spacing(6).into()
        };
        items.push(group(
            Some("Size & spacing"),
            vec![
                scale(
                    "Icon size",
                    known.map(|icons| icons.icon_size.get()),
                    Edit::Size,
                ),
                scale(
                    "Spacing",
                    known.map(|icons| icons.grid_spacing.get()),
                    Edit::Spacing,
                ),
            ],
        ));
        items.push(label(REFRESH_NOTE, 12., MUTED));
        column::with_children(items).spacing(16).into()
    }
}
