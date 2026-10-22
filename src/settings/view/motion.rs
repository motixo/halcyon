//! Accessibility: the supported app-motion preference and its limits.
//! Desktop Cards edge-menu animation belongs in App settings.
use super::desktop::WARN;
use super::pages::section;
use super::*;
use crate::settings::motion::Message as Edit;

fn row_of<'a>(
    title: &'static str,
    note: &'static str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    split(
        column::with_children(vec![label(title, 13., FG), label(note, 11., MUTED)])
            .spacing(2)
            .into(),
        control,
    )
}

impl Settings {
    pub(super) fn motion_page(&self) -> Element<'_, Message> {
        let editor = &self.motion;
        let mode: Element<'_, Message> = match editor.read_only() {
            Some(_) => pill("Preview · changes are off", WARN),
            None => text::body("").into(),
        };
        let apps: Element<'_, Message> = match (editor.current(), editor.shown()) {
            (Some(Err(_)), _) => label("Unavailable", 12., MUTED),
            (_, Some(on)) => toggler(!on)
                .on_toggle_maybe(
                    editor
                        .can_change()
                        .then_some(|reduce: bool| Message::Motion(Edit::Animations(!reduce))),
                )
                .into(),
            _ => label("Reading…", 12., MUTED),
        };
        let mut app_rows = vec![
            label("Apps", 16., FG),
            row_of(
                "Reduce motion",
                "Reduces motion in Desktop Cards settings, GTK apps, Firefox and Chromium-based browsers.",
                apps,
            ),
        ];
        if let Some(Err(reason)) = editor.current() {
            app_rows.push(label(reason.clone(), 11., WARN));
        }
        column::with_children(vec![
            split(
                column::with_children(vec![
                    split(heading("Accessibility", ""), mode),
                ])
                .spacing(4)
                .into(),
                button::text("Open COSMIC Settings ↗")
                    .on_press(Message::OpenDesktop("accessibility"))
                    .into(),
            ),
            section(app_rows),
            section(vec![
                label(
                    "COSMIC’s window, workspace and launcher animations are built in; this version has no setting for them.",
                    11.,
                    MUTED,
                ),
                split(
                    label("Dock and panel slide speed", 13., FG),
                    button::text("Dock & panel ›")
                        .on_press(Message::Page(Page::DesktopDock))
                        .into(),
                ),
            ]),
        ])
        .spacing(16)
        .into()
    }
}
