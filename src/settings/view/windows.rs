//! Native inspector with direct completed controls and local form validation.
use super::desktop::{ERROR, WARN, banner, conflict_banner, kept};
use super::system_appearance::fill;
use super::*;
use crate::desktop::DesktopStore;
use crate::desktop::appearance::ThemeVariant;
use crate::desktop::panels::Setting;
use crate::desktop::windows::{Behavior, Gap, TileBehavior, WindowsDraft, is_transition};
use crate::settings::windows::Message as Edit;

fn edit(message: Edit) -> Message {
    Message::Windows(message)
}

/// The largest gap the sliders offer; larger stored values are shown as
/// they are and kept until edited.
const GAP_RANGE: u32 = 48;

/// Preview or fixture mode, when the store is not the live desktop.
pub(super) fn store_pill<'a>(store: Option<&DesktopStore>, accent: Color) -> Element<'a, Message> {
    match store {
        Some(store) if !store.writable() => pill("Preview · Read only", WARN),
        Some(store) => match store.fixture_root() {
            Some(root) => pill(
                format!(
                    "Fixture · {}",
                    root.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default()
                ),
                accent,
            ),
            None => text::body("").into(),
        },
        None => text::body("").into(),
    }
}

/// The recovery banner shared by the desktop subpages.
pub(super) fn recovery_banner<'a>(
    recovery: Option<&Result<crate::desktop::store::Journal, String>>,
    writable: bool,
    recover: Message,
    keep: Message,
) -> Option<Element<'a, Message>> {
    match recovery? {
        Ok(_) => Some(banner(
            "An earlier change was interrupted.",
            WARN,
            vec![
                button::text("Keep current")
                    .on_press_maybe(writable.then_some(keep))
                    .into(),
                button::standard("Restore previous")
                    .on_press_maybe(writable.then_some(recover))
                    .into(),
            ],
        )),
        Err(error) => Some(banner(
            error.clone(),
            ERROR,
            vec![
                button::text("Keep current")
                    .on_press_maybe(writable.then_some(keep))
                    .into(),
            ],
        )),
    }
}

/// A row: title and a short consequence on the left, the control right.
fn row_of<'a>(
    title: &'static str,
    note: Option<&'static str>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut left = vec![label(title, 13., FG)];
    if let Some(note) = note {
        left.push(label(note, 11., MUTED));
    }
    split(column::with_children(left).spacing(2).into(), control)
}

fn flag_row<'a>(
    draft: &WindowsDraft,
    behavior: Behavior,
    title: &'static str,
    note: Option<&'static str>,
) -> Element<'a, Message> {
    let control = match draft.flag(behavior) {
        Setting::Known(on) => toggler(on)
            .on_toggle(move |on| edit(Edit::Flag(behavior, on)))
            .into(),
        Setting::Unsupported(raw) => kept(&raw),
    };
    row_of(title, note, control)
}

impl Settings {
    fn windows_accent(&self) -> Color {
        self.settings_accent()
    }

    pub(super) fn windows_page(&self) -> Element<'_, Message> {
        let accent = self.windows_accent();
        let editor = &self.windows;
        let mut items: Vec<Element<'_, Message>> = Vec::new();
        if let Some(error) = editor.unavailable() {
            items.push(split(
                heading("Windows & workspaces", ""),
                store_pill(self.windows.store(), accent),
            ));
            items.push(banner(
                format!("Window settings are unavailable. {error}"),
                ERROR,
                Vec::new(),
            ));
            return column::with_children(items).spacing(14).into();
        }
        let Some(draft) = editor.draft() else {
            items.push(split(
                heading("Windows & workspaces", ""),
                store_pill(self.windows.store(), accent),
            ));
            items.push(label("Reading window settings…", 12., MUTED));
            return column::with_children(items).spacing(14).into();
        };
        items.push(split(
            heading("Windows & workspaces", ""),
            store_pill(self.windows.store(), accent),
        ));
        if let Some(banner) = recovery_banner(
            editor.recovery(),
            editor.writable(),
            edit(Edit::Recover),
            edit(Edit::KeepRecovery),
        ) {
            items.push(banner);
        }
        let conflicts = draft.conflicts();
        if !conflicts.is_empty() {
            items.push(conflict_banner(
                crate::settings::system_appearance::describe(&conflicts),
                edit(Edit::UseCurrent),
            ));
        }
        let blocked = editor.blocked();
        if let Some(transition) = blocked.iter().find(|b| is_transition(b)) {
            let first = match editor.first_step() {
                Some(Behavior::Autotile) => "Apply tiling first",
                _ => "Apply scope first",
            };
            items.push(banner(
                transition.reason.clone(),
                WARN,
                vec![
                    button::standard(first)
                        .on_press_maybe(editor.can_apply_first().then_some(edit(Edit::ApplyFirst)))
                        .into(),
                ],
            ));
        }
        let other: Vec<_> = blocked
            .iter()
            .filter(|b| !is_transition(b))
            .map(|b| b.key.clone())
            .collect();
        if !other.is_empty() {
            items.push(banner(
                format!(
                    "Can’t apply: this version doesn’t recognize the current {}. It is kept as it is; discard these edits to continue.",
                    crate::settings::system_appearance::describe(&other)
                ),
                ERROR,
                vec![button::text("Discard").on_press(edit(Edit::Discard)).into()],
            ));
        }
        items.push(
            self.inspector_layout(
                self.windows_header(draft),
                column::with_children(vec![
                    self.windows_tiling(draft),
                    self.windows_gaps(draft),
                    windows_controls(draft),
                    self.windows_focus(draft),
                ])
                .spacing(16)
                .into(),
                "windows",
            ),
        );
        column::with_children(items).spacing(16).into()
    }

    /// Title, a one-line summary and the schematic, which follows the draft.
    fn windows_header(&self, draft: &WindowsDraft) -> Element<'_, Message> {
        let accent = self.windows_accent();
        let variant = self.windows.variant;
        let tiled = draft.flag(Behavior::Autotile).known() == Some(&true);
        let (outer, inner) = draft.gaps(variant).known().copied().unwrap_or((8, 8));
        let hint = draft.flag(Behavior::ActiveHint).known() != Some(&false);
        let outline = draft.outline_width(variant).known().copied().unwrap_or(1);
        let scope = match draft.scope() {
            Setting::Known(TileBehavior::Global) => "all workspaces",
            Setting::Known(TileBehavior::PerWorkspace) => "each workspace",
            Setting::Unsupported(_) => "custom scope",
        };
        let summary = if tiled {
            format!("Tiling on · {scope}")
        } else {
            "Tiling off · windows float".to_string()
        };
        let details = column::with_children(vec![
            label(summary, 12., MUTED),
            label(
                format!("Gaps {outer} / {inner} px · {} theme", variant.title()),
                12.,
                MUTED,
            ),
        ])
        .spacing(4)
        .width(Length::Fill);
        let height = (self.window.height * 0.17).clamp(84., 120.);
        column::with_children(vec![
            details.into(),
            self.scene_frame(
                container(tiling_schematic(
                    tiled, outer, inner, hint, outline, accent, height,
                ))
                .max_width(520)
                .width(Length::Fill)
                .into(),
                "Schematic · tiling, gaps and active window outline",
            ),
        ])
        .spacing(16)
        .into()
    }

    fn windows_tiling(&self, draft: &WindowsDraft) -> Element<'_, Message> {
        let accent = self.windows_accent();
        let mut items = Vec::new();
        items.push(match draft.flag(Behavior::Autotile) {
            Setting::Known(on) => row_of(
                "Automatic tiling",
                None,
                toggler(on).on_toggle(|on| edit(Edit::Autotile(on))).into(),
            ),
            Setting::Unsupported(raw) => row_of("Automatic tiling", None, kept(&raw)),
        });
        let scope = draft.scope();
        let mut choice = vec![segmented(
            [
                ("All workspaces", TileBehavior::Global),
                ("Each workspace", TileBehavior::PerWorkspace),
            ]
            .into_iter()
            .map(|(title, value)| {
                (
                    title.to_string(),
                    scope.known() == Some(&value),
                    Some(edit(Edit::Scope(value))),
                )
            })
            .collect(),
            accent,
        )];
        if let Setting::Unsupported(raw) = &scope {
            choice.push(kept(raw));
        }
        items.push(row_of(
            "Applies to",
            None,
            row::with_children(choice)
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap()
                .into(),
        ));
        items.push(label(
            match scope {
                Setting::Known(TileBehavior::PerWorkspace) => {
                    "New windows and new workspaces start with this; each workspace keeps its own tiling."
                }
                _ => "Turning tiling on or off rearranges windows on every workspace.",
            },
            11.,
            MUTED,
        ));
        items.push(label(
            "Undo restores these settings, not window positions.",
            11.,
            MUTED,
        ));
        items.push(
            button::custom(split(
                column::with_children(vec![
                    label("Floating apps", 13., FG),
                    label("Apps whose windows open floating while tiling", 11., MUTED),
                ])
                .spacing(2)
                .into(),
                row::with_children(vec![
                    if self.window_rules.dirty() {
                        pill("Not applied", accent)
                    } else {
                        text::body("").into()
                    },
                    icon::from_name("go-next-symbolic").size(16).icon().into(),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ))
            .class(nav_style(false, accent))
            .padding([4, 4])
            .width(Length::Fill)
            .on_press(Message::Page(Page::DesktopWindowRules))
            .into(),
        );
        group(Some("Tiling"), items)
    }

    fn windows_gaps(&self, draft: &WindowsDraft) -> Element<'_, Message> {
        let accent = self.windows_accent();
        let editor = &self.windows;
        let variant = editor.variant;
        let active = draft.active_variant();
        let selector = segmented(
            ThemeVariant::ALL
                .into_iter()
                .map(|option| {
                    let mut title = option.title().to_string();
                    if option == active {
                        title.push_str(" · in use");
                    }
                    if draft.gap_edited(option) {
                        title.push_str(" •");
                    }
                    (title, option == variant, Some(edit(Edit::Variant(option))))
                })
                .collect(),
            accent,
        );
        let mut items = vec![split(
            self.help(
                "window-gaps",
                "Window gaps",
                "Each theme keeps its own gaps. Presets include them with Appearance.",
            ),
            selector,
        )];
        match (draft.gaps(variant), draft.base_gaps(variant)) {
            (Setting::Known((outer, inner)), Setting::Known(_)) => {
                let outline = draft.outline_width(variant).known().copied().unwrap_or(0);
                for (gap, title, value) in [
                    (Gap::Outer, "Screen edges", outer),
                    (Gap::Inner, "Between windows", inner),
                ] {
                    let shown = if value > GAP_RANGE {
                        format!("{value} px · above range")
                    } else {
                        format!("{value} px")
                    };
                    items.push(
                        column::with_children(vec![
                            split(label(title, 12., FG), label(shown, 11., accent)),
                            slider(0..=GAP_RANGE, value.min(GAP_RANGE), move |value| {
                                edit(Edit::Gap(gap, value))
                            })
                            .step(1u32)
                            .on_release(edit(Edit::Apply))
                            .width(Length::Fill)
                            .apply(|control| completed_slider(control.into(), edit(Edit::Apply))),
                        ])
                        .spacing(7)
                        .into(),
                    );
                }
                if outline > 0 {
                    items.push(label(
                        format!("When changed, Between windows is at least the {outline} px active outline."),
                        11.,
                        MUTED,
                    ));
                }
            }
            (_, Setting::Unsupported(raw)) | (Setting::Unsupported(raw), _) => {
                items.push(row_of("Gaps", None, kept(&raw)));
            }
        }
        if variant != active {
            items.push(label(
                format!(
                    "Editing the {} theme. COSMIC keeps using {} until the mode changes.",
                    variant.title(),
                    active.title()
                ),
                11.,
                MUTED,
            ));
        }
        group(Some("Window gaps"), items)
    }

    fn windows_focus(&self, draft: &WindowsDraft) -> Element<'_, Message> {
        let accent = self.windows_accent();
        let editor = &self.windows;
        let mut items = Vec::new();
        items.push(flag_row(
            draft,
            Behavior::FocusFollowsCursor,
            "Focus follows pointer",
            Some("Focus a window by moving the pointer over it"),
        ));
        if draft.flag(Behavior::FocusFollowsCursor).known() == Some(&true) {
            let text = editor.delay_text().map(str::to_owned).unwrap_or_else(|| {
                match draft.focus_delay() {
                    Setting::Known(ms) => ms.to_string(),
                    Setting::Unsupported(_) => String::new(),
                }
            });
            let control: Element<'_, Message> = match draft.focus_delay() {
                Setting::Unsupported(raw) if editor.delay_text().is_none() => kept(&raw),
                _ => row::with_children(vec![
                    container(
                        text_input("250", text)
                            .on_input(|text| edit(Edit::DelayInput(text)))
                            .on_submit(|_| edit(Edit::SubmitNumber(true)))
                            .width(Length::Fill),
                    )
                    .width(84)
                    .into(),
                    label("ms", 12., MUTED),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            };
            items.push(row_of("Delay", None, control));
            if editor.delay_text().is_some() {
                items.push(label(
                    format!(
                        "Use a whole number from 0 to {} ms.",
                        crate::desktop::windows::MAX_FOCUS_DELAY
                    ),
                    11.,
                    ERROR,
                ));
            }
        }
        items.push(flag_row(
            draft,
            Behavior::CursorFollowsFocus,
            "Pointer follows focus",
            Some("Move the pointer to a window focused with the keyboard"),
        ));
        let snap = draft.edge_snap();
        items.push(match &snap {
            Setting::Known(px) => row_of(
                "Snap to screen edges",
                Some("Dragged windows stick to the display edges"),
                toggler(*px > 0)
                    .on_toggle(|on| edit(Edit::EdgeSnap(on)))
                    .into(),
            ),
            Setting::Unsupported(raw) => row_of("Snap to screen edges", None, kept(raw)),
        });
        let open = editor.advanced;
        items.push(
            button::custom(
                row::with_children(vec![
                    label("Advanced", 13., FG),
                    container(text::body("")).width(Length::Fill).into(),
                    icon::from_name(if open {
                        "go-up-symbolic"
                    } else {
                        "go-down-symbolic"
                    })
                    .size(16)
                    .icon()
                    .into(),
                ])
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .class(nav_style(false, accent))
            .padding([4, 4])
            .width(Length::Fill)
            .on_press(edit(Edit::Advanced(!open)))
            .into(),
        );
        if open {
            match snap {
                Setting::Known(px) if px > 0 || editor.snap_text().is_some() => {
                    let text = editor
                        .snap_text()
                        .map(str::to_owned)
                        .unwrap_or_else(|| px.to_string());
                    items.push(row_of(
                        "Snap distance",
                        Some("How close a window gets before it snaps"),
                        row::with_children(vec![
                            container(
                                text_input("10", text)
                                    .on_input(|text| edit(Edit::SnapInput(text)))
                                    .on_submit(|_| edit(Edit::SubmitNumber(false)))
                                    .width(Length::Fill),
                            )
                            .width(84)
                            .into(),
                            label("px", 12., MUTED),
                        ])
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .into(),
                    ));
                    if editor.snap_text().is_some() {
                        items.push(label(
                            format!(
                                "Use a whole number from 1 to {} px.",
                                crate::desktop::windows::MAX_EDGE_SNAP
                            ),
                            11.,
                            ERROR,
                        ));
                    }
                }
                Setting::Known(_) => items.push(label(
                    "Turn on edge snapping to choose its distance.",
                    11.,
                    MUTED,
                )),
                Setting::Unsupported(_) => {}
            }
        }
        group(Some("Focus & pointer"), items)
    }
}

fn windows_controls<'a>(draft: &WindowsDraft) -> Element<'a, Message> {
    group(
        Some("Window controls"),
        vec![
            flag_row(
                draft,
                Behavior::ActiveHint,
                "Active window outline",
                Some("Outline the focused window in the accent color"),
            ),
            flag_row(draft, Behavior::ShowMinimize, "Minimize button", None),
            flag_row(draft, Behavior::ShowMaximize, "Maximize button", None),
        ],
    )
}

/// A screen with windows: tiled side by side within the gaps, or two
/// floating windows. Gaps are drawn at half scale and capped, so large
/// values stay legible; the focused window wears the outline when shown.
#[allow(clippy::too_many_arguments)]
fn tiling_schematic<'a>(
    tiled: bool,
    outer: u32,
    inner: u32,
    hint: bool,
    outline: u32,
    accent: Color,
    height: f32,
) -> Element<'a, Message> {
    let scale = |px: u32| (px as f32 * 0.5).clamp(0., 12.);
    let window = |active: bool| -> Element<'a, Message> {
        let border = if active && hint {
            (accent, (outline as f32).clamp(1., 3.))
        } else {
            (LINE, 1.)
        };
        fill(
            column::with_children(vec![
                fill(
                    cosmic::iced::widget::space().height(5).into(),
                    Color { a: 0.18, ..FG },
                    Color::TRANSPARENT,
                    0.,
                    2.,
                )
                .width(Length::Fill)
                .into(),
            ])
            .padding(3)
            .into(),
            SURFACE,
            border.0,
            border.1,
            4.,
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };
    let content: Element<'a, Message> = if tiled {
        let gap = scale(inner);
        row::with_children(vec![
            container(window(true))
                .width(Length::FillPortion(3))
                .height(Length::Fill)
                .into(),
            column::with_children(vec![window(false), window(false)])
                .spacing(gap)
                .width(Length::FillPortion(2))
                .height(Length::Fill)
                .into(),
        ])
        .spacing(gap)
        .height(Length::Fill)
        .into()
    } else {
        let width = height * 1.7;
        cosmic::iced::widget::stack(vec![
            container(window(false))
                .padding(cosmic::iced::Padding {
                    top: height * 0.06,
                    left: width * 0.08,
                    right: width * 0.38,
                    bottom: height * 0.34,
                })
                .into(),
            container(window(true))
                .padding(cosmic::iced::Padding {
                    top: height * 0.30,
                    left: width * 0.36,
                    right: width * 0.08,
                    bottom: height * 0.08,
                })
                .into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };
    fill(content, BASE, LINE, 1., 8.)
        .padding(if tiled { scale(outer).max(2.) } else { 2. })
        .width(Length::Fill)
        .height(height)
        .into()
}
