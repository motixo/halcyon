//! Applet regions: drag-and-drop rows, arrow controls and the picker.
use super::*;
use crate::settings::desktop::{AppletControl, control_id};

/// The picker and the installed list offer search from this many applets.
const SEARCH_FROM: usize = 8;

impl Settings {
    pub(super) fn applets_section(&self, draft: &PanelDraft) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let editor = &self.desktop;
        let horizontal = draft
            .anchor()
            .known()
            .is_none_or(|anchor| anchor.horizontal());
        let regions = match draft.applets() {
            Setting::Known(regions) => regions,
            Setting::Unsupported(raw) => {
                return section(vec![
                    label("Applets", 16., FG),
                    banner(
                        "This applet layout uses a format Halcyon does not edit. It is kept as is.",
                        WARN,
                        Vec::new(),
                    ),
                    kept(&raw),
                ]);
            }
        };
        let total = regions.start.len() + regions.center.len() + regions.end.len();
        let mut blocks: Vec<Element<'_, Message>> = vec![split(
            label("Applets", 16., FG),
            label(
                if editor.drag.active() {
                    "Drop on a row or region · Esc cancels".to_string()
                } else {
                    format!("{total} placed")
                },
                11.,
                MUTED,
            ),
        )];
        // Drop targets never appear or disappear when a drag starts, so the
        // layout cannot shift under the pointer: a region's header inserts at
        // its start, a row takes that row's place, an empty region's zone
        // receives the applet.
        let dragging = editor.drag.active();
        for region in Region::ALL {
            let list = regions.list(region);
            let picking = editor.picker == Some(region);
            let start = Slot { region, index: 0 };
            let island_selected = editor.islands.installation.installed()
                && draft.islands().known().is_some_and(|s| s.enabled)
                && editor.islands.selected
                    == Some(match region {
                        Region::Start => dock_islands_core::Section::Start,
                        Region::Center => dock_islands_core::Section::Center,
                        Region::End => dock_islands_core::Section::End,
                    });
            let header_target = island_selected
                || (dragging && editor.drag.target == Some(start) && !list.is_empty());
            let header = container(split(
                label(
                    format!(
                        "{} · {}",
                        region.title(horizontal).to_uppercase(),
                        list.len()
                    ),
                    10.,
                    if header_target { accent } else { MUTED },
                ),
                button::text(if picking { "Close" } else { "Add" })
                    .leading_icon(
                        icon::from_name(if picking {
                            "window-close-symbolic"
                        } else {
                            "list-add-symbolic"
                        })
                        .size(14),
                    )
                    .on_press_maybe(
                        (!dragging).then(|| edit(Edit::Picker((!picking).then_some(region)))),
                    )
                    .into(),
            ))
            .padding([0, 6])
            .class(cosmic::theme::Container::custom(move |_| {
                cosmic::iced::widget::container::Style {
                    border: Border {
                        color: if header_target {
                            accent
                        } else {
                            Color::TRANSPARENT
                        },
                        width: 1.,
                        radius: 10.into(),
                    },
                    ..Default::default()
                }
            }));
            blocks.push(
                mouse_area(header)
                    .on_release(edit(Edit::DropAt(start)))
                    .on_enter(edit(Edit::DragOver(start)))
                    .on_move(move |_| edit(Edit::DragOver(start)))
                    .on_exit(edit(Edit::DragLeave(start)))
                    .into(),
            );
            if picking {
                blocks.push(self.applet_picker(draft, region));
            }
            let mut rows: Vec<Element<'_, Message>> = Vec::new();
            for (index, id) in list.iter().enumerate() {
                rows.push(self.applet_row(id, Slot { region, index }, list.len(), accent));
            }
            if list.is_empty() {
                let target = dragging && editor.drag.target == Some(start);
                let zone = container(label(
                    if dragging {
                        "Drop here"
                    } else {
                        "Empty · drag an applet here or use Add"
                    },
                    11.,
                    if target { accent } else { MUTED },
                ))
                .padding([8, 12])
                .width(Length::Fill)
                .class(cosmic::theme::Container::custom(move |_| {
                    cosmic::iced::widget::container::Style {
                        background: Some(Background::Color(Color {
                            a: if target { 0.12 } else { 0.03 },
                            ..accent
                        })),
                        border: Border {
                            color: if target { accent } else { LINE },
                            width: 1.,
                            radius: 10.into(),
                        },
                        ..Default::default()
                    }
                }));
                rows.push(
                    mouse_area(zone)
                        .on_release(edit(Edit::DropAt(start)))
                        .on_enter(edit(Edit::DragOver(start)))
                        .on_move(move |_| edit(Edit::DragOver(start)))
                        .on_exit(edit(Edit::DragLeave(start)))
                        .into(),
                );
            }
            blocks.push(column::with_children(rows).spacing(2).into());
        }
        blocks.push(label(
            "Drag the handles, or use the arrows to move between regions.",
            11.,
            MUTED,
        ));
        section(blocks)
    }

    fn applet_row(&self, id: &str, slot: Slot, len: usize, accent: Color) -> Element<'_, Message> {
        let editor = &self.desktop;
        let known = editor.catalog.known(id);
        let dragging = editor.drag.source == Some(slot);
        let target = editor.drag.active() && editor.drag.target == Some(slot) && !dragging;
        let focused = editor.focus == Some(slot);
        let first = slot.region == Region::Start && slot.index == 0;
        let last = slot.region == Region::End && slot.index + 1 == len;
        let idle = !editor.drag.active();
        let mut parts: Vec<Element<'_, Message>> = vec![
            mouse_area(
                container(icon::from_name("grip-lines-symbolic").size(16).icon()).padding(6),
            )
            .on_press(edit(Edit::DragStart(slot)))
            .on_release(edit(Edit::DropAt(slot)))
            .interaction(if dragging {
                cosmic::iced::mouse::Interaction::Grabbing
            } else {
                cosmic::iced::mouse::Interaction::Grab
            })
            .into(),
            icon::from_name(editor.catalog.icon(id))
                .size(18)
                .icon()
                .into(),
        ];
        // The label and its optional pill share one child, so the named
        // controls below keep fixed positions in this row. libcosmic matches
        // their state by id but keeps it at the old index; a pill appearing in
        // front of them during a drag would otherwise break the layout.
        let mut title = vec![
            container(line(
                editor.catalog.label(id),
                14.,
                if known { FG } else { MUTED },
            ))
            .width(Length::Fill)
            .into(),
        ];
        if target || dragging || !known {
            title.push(pill(
                if target {
                    "Drop here"
                } else if dragging {
                    "Esc cancels"
                } else {
                    "Not installed"
                },
                if target { accent } else { MUTED },
            ));
        }
        parts.push(
            row::with_children(title)
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .into(),
        );
        // Stable per-occurrence ids let focus follow a keyboard move.
        parts.push(
            button::icon(icon::from_name("go-up-symbolic"))
                .id(control_id(slot, AppletControl::Up))
                .on_press_maybe((!first && idle).then_some(edit(Edit::Step(slot, -1))))
                .into(),
        );
        parts.push(
            button::icon(icon::from_name("go-down-symbolic"))
                .id(control_id(slot, AppletControl::Down))
                .on_press_maybe((!last && idle).then_some(edit(Edit::Step(slot, 1))))
                .into(),
        );
        parts.push(
            button::icon(icon::from_name("list-remove-symbolic"))
                .id(control_id(slot, AppletControl::Remove))
                .on_press_maybe(idle.then_some(edit(Edit::Remove(slot))))
                .into(),
        );
        let content = container(
            row::with_children(parts)
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding([2, 6])
        .height(42)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(move |_| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(Color {
                    a: if target {
                        0.12
                    } else if dragging || focused {
                        0.07
                    } else {
                        0.
                    },
                    ..accent
                })),
                border: Border {
                    color: if target {
                        accent
                    } else if focused {
                        Color { a: 0.4, ..accent }
                    } else {
                        Color::TRANSPARENT
                    },
                    width: 1.,
                    radius: 12.into(),
                },
                ..Default::default()
            }
        }));
        mouse_area(content)
            .on_release(edit(Edit::DropAt(slot)))
            .on_enter(edit(Edit::DragOver(slot)))
            .on_move(move |_| edit(Edit::DragOver(slot)))
            .on_exit(edit(Edit::DragLeave(slot)))
            .into()
    }

    fn applet_search(&self) -> Element<'_, Message> {
        cosmic::widget::search_input("Search applets", &self.desktop.applet_query)
            .on_input(|text| edit(Edit::Search(text)))
            .on_clear(edit(Edit::Search(String::new())))
            .width(Length::Fill)
            .into()
    }

    /// One installed applet: icon, name, description and where it is used.
    fn applet_entry<'a>(
        &'a self,
        info: &'a crate::desktop::applets::AppletInfo,
        action: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let accent = self.settings_accent();
        let mut title: Vec<Element<'_, Message>> = vec![
            container(line(info.name.clone(), 14., FG))
                .width(Length::Shrink)
                .into(),
        ];
        for (kind, region, horizontal) in self.desktop.placements(&info.id) {
            let here = kind == self.desktop.selected;
            title.push(pill(
                format!("{} · {}", kind.title(), region.title(horizontal)),
                if here { accent } else { MUTED },
            ));
        }
        let mut text_parts = vec![
            row::with_children(title)
                .spacing(6)
                .align_y(Alignment::Center)
                .wrap()
                .into(),
        ];
        if !info.comment.is_empty() {
            text_parts.push(line(info.comment.clone(), 11., MUTED));
        }
        row::with_children(vec![
            icon::from_name(self.desktop.catalog.icon(&info.id))
                .size(20)
                .icon()
                .into(),
            column::with_children(text_parts)
                .spacing(2)
                .width(Length::Fill)
                .into(),
            action,
        ])
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    }

    /// Every installed applet, where each is used, and Add for those not in
    /// the selected bar.
    pub(super) fn installed_applets(&self, draft: &PanelDraft) -> Element<'_, Message> {
        let editor = &self.desktop;
        let all = editor.catalog.sorted();
        let used = all
            .iter()
            .filter(|info| !editor.placements(&info.id).is_empty())
            .count();
        let horizontal = draft
            .anchor()
            .known()
            .is_none_or(|anchor| anchor.horizontal());
        let mut blocks: Vec<Element<'_, Message>> = vec![split(
            column::with_children(vec![
                label("Installed applets", 16., FG),
                label(
                    format!("{} installed · {used} in use", all.len()),
                    11.,
                    MUTED,
                ),
            ])
            .spacing(2)
            .into(),
            button::text(if editor.overview { "Hide" } else { "Show" })
                .trailing_icon(
                    icon::from_name(if editor.overview {
                        "go-up-symbolic"
                    } else {
                        "go-down-symbolic"
                    })
                    .size(14),
                )
                .on_press_maybe(
                    (!editor.drag.active()).then(|| edit(Edit::Overview(!editor.overview))),
                )
                .into(),
        )];
        if !editor.overview {
            return section(blocks);
        }
        // Without a search field, a leftover query filters nothing.
        let searchable = all.len() >= SEARCH_FROM;
        if searchable {
            blocks.push(self.applet_search());
        }
        let placed = draft.applets().known().cloned().unwrap_or_default();
        let shown: Vec<_> = all
            .into_iter()
            .filter(|info| !searchable || editor.matches(info))
            .collect();
        if shown.is_empty() {
            blocks.push(label(
                if editor.catalog.sorted().is_empty() {
                    "No COSMIC applets are installed."
                } else {
                    "No installed applet matches."
                },
                12.,
                MUTED,
            ));
        }
        let region = editor.usual_region().title(horizontal);
        for (index, info) in shown.into_iter().enumerate() {
            if index > 0 {
                blocks.push(separator());
            }
            let action: Element<'_, Message> = if placed.contains(&info.id) {
                container(label("In this bar", 11., MUTED))
                    .padding([0, 8])
                    .into()
            } else {
                button::text("Add")
                    .on_press_maybe(
                        draft
                            .applets()
                            .known()
                            .is_some()
                            .then(|| edit(Edit::AddToBar(info.id.clone()))),
                    )
                    .into()
            };
            blocks.push(self.applet_entry(info, action));
        }
        blocks.push(label(
            format!(
                "Add puts an applet at the {} of the {}; move it from there. Nothing is installed or removed from your computer.",
                region.to_lowercase(),
                editor.selected.title()
            ),
            11.,
            MUTED,
        ));
        section(blocks)
    }

    fn applet_picker(&self, draft: &PanelDraft, region: Region) -> Element<'_, Message> {
        let horizontal = draft
            .anchor()
            .known()
            .is_none_or(|anchor| anchor.horizontal());
        let placed = draft.applets().known().cloned().unwrap_or_default();
        let unplaced: Vec<_> = self
            .desktop
            .catalog
            .sorted()
            .into_iter()
            .filter(|info| !placed.contains(&info.id))
            .collect();
        let searchable = unplaced.len() >= SEARCH_FROM;
        let available: Vec<_> = unplaced
            .iter()
            .copied()
            .filter(|info| !searchable || self.desktop.matches(info))
            .collect();
        let mut rows: Vec<Element<'_, Message>> = vec![label(
            format!("Choose an applet for {}", region.title(horizontal)),
            12.,
            MUTED,
        )];
        if searchable {
            rows.push(self.applet_search());
        }
        if unplaced.is_empty() {
            rows.push(label(
                "Every installed applet is already in this bar.",
                12.,
                MUTED,
            ));
        } else if available.is_empty() {
            rows.push(label("No installed applet matches.", 12., MUTED));
        }
        for (index, info) in available.into_iter().enumerate() {
            if index > 0 {
                rows.push(separator());
            }
            rows.push(
                self.applet_entry(
                    info,
                    button::text("Add")
                        .on_press(edit(Edit::Add(region, info.id.clone())))
                        .into(),
                ),
            );
        }
        frame(
            column::with_children(rows)
                .spacing(8)
                .width(Length::Fill)
                .into(),
            BASE,
            LINE,
            12,
        )
    }
}
