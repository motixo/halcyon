//! Panel contents are the main editor. Each occurrence has a stable selectable
//! chip, a drag handle and a named button path through the inspector.
use super::*;
use crate::desktop::applets::AppletGroup;
use crate::desktop::panels::Regions;
use crate::settings::desktop::{AppletControl, PanelStyle, control_id};

fn region_title(region: Region, horizontal: bool) -> &'static str {
    if horizontal && region == Region::Center {
        "Centre"
    } else {
        region.title(horizontal)
    }
}

impl Settings {
    pub(super) fn panel_contents(&self, draft: &PanelDraft) -> Element<'_, Message> {
        let editor = &self.desktop;
        let accent = self.settings_accent();
        let horizontal = draft.anchor().known().is_none_or(|a| a.horizontal());
        let Setting::Known(regions) = draft.applets() else {
            return self.applets_section(draft);
        };
        if let Some(region) = editor.picker {
            return section(vec![self.panel_picker(draft, region)]);
        }
        let mut blocks = vec![split(
            label("Panel contents", 16., FG),
            button::text("Add applets")
                .leading_icon(icon::from_name("list-add-symbolic").size(16))
                .on_press_maybe(
                    (!editor.drag.active()).then_some(edit(Edit::Picker(Some(Region::End)))),
                )
                .into(),
        )];
        blocks.push(label(
            "Drag a handle to move an applet, or select one for controls.",
            11.,
            MUTED,
        ));
        let lanes: Vec<Element<'_, Message>> = Region::ALL
            .into_iter()
            .map(|region| {
                let list = regions.list(region);
                let start = Slot { region, index: 0 };
                let end = Slot {
                    region,
                    index: list.len(),
                };
                let target =
                    editor.drag.active() && editor.drag.target.is_some_and(|s| s.region == region);
                let header = split(
                    label(
                        format!("{} · {}", region_title(region, horizontal), list.len()),
                        12.,
                        FG,
                    ),
                    button::icon(icon::from_name("list-add-symbolic"))
                        .tooltip(format!("Add to {}", region_title(region, horizontal)))
                        .on_press_maybe(
                            (!editor.drag.active()).then_some(edit(Edit::Picker(Some(region)))),
                        )
                        .into(),
                );
                let mut chips = vec![self.panel_drop_target(header, start)];
                chips.push(
                    cosmic::widget::flex_row(
                        list.iter()
                            .enumerate()
                            .map(|(index, id)| self.panel_chip(id, Slot { region, index }))
                            .collect::<Vec<_>>(),
                    )
                    .spacing(3)
                    .into(),
                );
                // The trailing target always exists, including during drag, so
                // a drop can append to a populated region without layout shifts.
                chips.push(
                    self.panel_drop_target(
                        container(label(
                            if list.is_empty() {
                                "Empty · drop here"
                            } else {
                                ""
                            },
                            11.,
                            MUTED,
                        ))
                        .height(if list.is_empty() { 38 } else { 14 })
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .into(),
                        end,
                    ),
                );
                container(column::with_children(chips).spacing(3))
                    .padding(6)
                    .width(Length::FillPortion(1))
                    .class(cosmic::theme::Container::custom(move |theme| {
                        cosmic::iced::widget::container::Style {
                            background: Some(Background::Color(surface_color(theme, BASE))),
                            border: Border {
                                color: if target {
                                    accent
                                } else {
                                    surface_color(theme, LINE)
                                },
                                width: 1.,
                                radius: 10.into(),
                            },
                            ..Default::default()
                        }
                    }))
                    .into()
            })
            .collect();
        blocks.push(row::with_children(lanes).spacing(8).into());
        if let Some(slot) = editor
            .focus
            .filter(|s| s.index < regions.list(s.region).len())
        {
            blocks.push(self.panel_inspector(&regions, slot, horizontal));
        }
        if editor.starter_open {
            let starter = editor.catalog.starter(editor.has_battery);
            blocks.push(separator());
            blocks.push(label("A quiet starting point", 14., FG));
            for region in Region::ALL {
                let names = starter
                    .list(region)
                    .iter()
                    .map(|id| editor.catalog.panel_label(id))
                    .collect::<Vec<_>>();
                blocks.push(label(
                    format!(
                        "{}: {}",
                        region_title(region, horizontal),
                        if names.is_empty() {
                            "Empty".into()
                        } else {
                            names.join(", ")
                        }
                    ),
                    12.,
                    MUTED,
                ));
            }
            blocks.push(label("Choose a layout to replace this panel’s contents. Undo restores your previous layout.", 11., MUTED));
            blocks.push(
                row::with_children(vec![
                    button::suggested("Use starter layout")
                        .on_press_maybe(
                            (starter != Regions::default()).then_some(edit(Edit::UseStarter)),
                        )
                        .into(),
                    button::text("Cancel")
                        .on_press(edit(Edit::Starter(false)))
                        .into(),
                ])
                .spacing(8)
                .into(),
            );
        } else {
            blocks.push(split(
                label(
                    if editor.drag.active() {
                        "Drop on an applet or empty space · Esc cancels"
                    } else if regions == Regions::default() {
                        "Start with the essentials, or add your own applets."
                    } else {
                        "Move or remove applets here. Undo restores your last change."
                    },
                    11.,
                    MUTED,
                ),
                button::text("Starter layout…")
                    .on_press_maybe((!editor.drag.active()).then_some(edit(Edit::Starter(true))))
                    .into(),
            ));
        }
        section(blocks)
    }

    fn panel_drop_target<'a>(
        &self,
        content: Element<'a, Message>,
        slot: Slot,
    ) -> Element<'a, Message> {
        mouse_area(content)
            .on_release(edit(Edit::DropAt(slot)))
            .on_enter(edit(Edit::DragOver(slot)))
            .on_move(move |_| edit(Edit::DragOver(slot)))
            .on_exit(edit(Edit::DragLeave(slot)))
            .into()
    }

    fn panel_chip(&self, id: &str, slot: Slot) -> Element<'_, Message> {
        let editor = &self.desktop;
        let accent = self.settings_accent();
        let selected = editor.focus == Some(slot);
        let target = editor.drag.active() && editor.drag.target == Some(slot);
        let dragging = editor.drag.source == Some(slot);
        let name = editor.catalog.panel_label(id);
        let known = editor.catalog.known(id);
        let handle = mouse_area(
            container(icon::from_name("grip-lines-symbolic").size(12).icon()).padding([8, 3]),
        )
        .on_press(edit(Edit::DragStart(slot)))
        .on_release(edit(Edit::DropAt(slot)))
        .interaction(if dragging {
            cosmic::iced::mouse::Interaction::Grabbing
        } else {
            cosmic::iced::mouse::Interaction::Grab
        });
        let compact = self.window.width < 900.;
        let mut content = vec![
            icon::from_name(editor.catalog.icon(id))
                .size(18)
                .icon()
                .into(),
        ];
        if !compact {
            content.push(line(name.clone(), 11., if known { FG } else { MUTED }));
        }
        let chip = button::custom(
            row::with_children(content)
                .spacing(5)
                .align_y(Alignment::Center)
                .width(Length::Fill),
        )
        .id(control_id(slot, AppletControl::Select))
        .class(nav_style(selected || target, accent))
        .selected(selected)
        .padding([5, 4])
        .width(Length::Fill)
        .on_press_maybe((!editor.drag.active()).then_some(edit(Edit::SelectApplet(Some(slot)))));
        let chip = cosmic::widget::tooltip(
            chip,
            text::body(if known {
                name
            } else {
                format!("{name} · Not installed · kept")
            }),
            cosmic::widget::tooltip::Position::Bottom,
        );
        self.panel_drop_target(
            row::with_children(vec![handle.into(), chip.into()])
                .align_y(Alignment::Center)
                .width(if compact { 48 } else { 114 })
                .into(),
            slot,
        )
    }

    fn panel_inspector(
        &self,
        regions: &Regions,
        slot: Slot,
        horizontal: bool,
    ) -> Element<'_, Message> {
        let editor = &self.desktop;
        let id = &regions.list(slot.region)[slot.index];
        let accent = self.settings_accent();
        let description = editor.catalog.get(id).map_or(
            "Not installed. This applet stays in your layout until you remove it.",
            |i| i.description(),
        );
        let idle = !editor.drag.active();
        let mut items = vec![
            separator(),
            split(
                label(editor.catalog.label(id), 14., FG),
                button::text("Done")
                    .on_press(edit(Edit::SelectApplet(None)))
                    .into(),
            ),
            label(description, 12., MUTED),
        ];
        items.push(
            row::with_children(vec![
                label("Move to", 12., MUTED),
                segmented(
                    Region::ALL
                        .into_iter()
                        .map(|region| {
                            (
                                region_title(region, horizontal).into(),
                                slot.region == region,
                                (idle && slot.region != region)
                                    .then_some(edit(Edit::MoveTo(slot, region))),
                            )
                        })
                        .collect(),
                    accent,
                ),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap()
            .into(),
        );
        items.push(
            row::with_children(vec![
                button::standard("Earlier")
                    .id(control_id(slot, AppletControl::Up))
                    .leading_icon(icon::from_name("go-previous-symbolic").size(14))
                    .on_press_maybe(
                        (idle && !(slot.region == Region::Start && slot.index == 0))
                            .then_some(edit(Edit::Step(slot, -1))),
                    )
                    .into(),
                button::standard("Later")
                    .id(control_id(slot, AppletControl::Down))
                    .trailing_icon(icon::from_name("go-next-symbolic").size(14))
                    .on_press_maybe(
                        (idle
                            && !(slot.region == Region::End
                                && slot.index + 1 == regions.end.len()))
                        .then_some(edit(Edit::Step(slot, 1))),
                    )
                    .into(),
                button::text("Remove")
                    .id(control_id(slot, AppletControl::Remove))
                    .on_press_maybe(idle.then_some(edit(Edit::Remove(slot))))
                    .into(),
            ])
            .spacing(8)
            .wrap()
            .into(),
        );
        column::with_children(items).spacing(8).into()
    }

    fn panel_picker(&self, draft: &PanelDraft, region: Region) -> Element<'_, Message> {
        let editor = &self.desktop;
        let accent = self.settings_accent();
        let horizontal = draft.anchor().known().is_none_or(|a| a.horizontal());
        let placed = draft.applets().known().cloned().unwrap_or_default();
        let mut items = vec![
            separator(),
            split(
                label("Add an applet", 14., FG),
                button::text("Close")
                    .on_press(edit(Edit::Picker(None)))
                    .into(),
            ),
        ];
        items.push(field(
            "Add to",
            segmented(
                Region::ALL
                    .into_iter()
                    .map(|r| {
                        (
                            region_title(r, horizontal).into(),
                            r == region,
                            Some(edit(Edit::PickerRegion(r))),
                        )
                    })
                    .collect(),
                accent,
            ),
        ));
        items.push(
            cosmic::widget::search_input("Search installed applets", &editor.applet_query)
                .on_input(|text| edit(Edit::Search(text)))
                .on_clear(edit(Edit::Search(String::new())))
                .into(),
        );
        items.push(segmented(
            AppletGroup::ALL
                .into_iter()
                .map(|g| {
                    (
                        g.title().into(),
                        editor.applet_group == g,
                        Some(edit(Edit::AppletGroup(g))),
                    )
                })
                .collect(),
            accent,
        ));
        let shown: Vec<_> = editor
            .catalog
            .sorted()
            .into_iter()
            .filter(|info| editor.applet_group.includes(info) && editor.matches(info))
            .collect();
        if shown.is_empty() {
            items.push(label(if editor.applet_group == AppletGroup::Readings && editor.applet_query.trim().is_empty() {
                "No system readings applet is installed. Detailed readings are available in the System monitor desktop card."
            } else { "No installed applet matches. Try another search or category." }, 12., MUTED));
        }
        for info in shown {
            let here = placed.contains(&info.id);
            let locations = editor
                .placements(&info.id)
                .into_iter()
                .map(|(kind, r, h)| format!("{} · {}", kind.title(), region_title(r, h)))
                .collect::<Vec<_>>()
                .join(" · ");
            let mut details = vec![label(info.name.clone(), 13., FG)];
            if !info.description().is_empty() {
                details.push(label(info.description(), 11., MUTED));
            }
            if !locations.is_empty() {
                details.push(label(locations, 10., MUTED));
            }
            items.push(
                row::with_children(vec![
                    icon::from_name(editor.catalog.icon(&info.id))
                        .size(20)
                        .icon()
                        .into(),
                    column::with_children(details)
                        .spacing(2)
                        .width(Length::Fill)
                        .into(),
                    button::text(if here { "Add another" } else { "Add" })
                        .on_press(edit(Edit::Add(region, info.id.clone())))
                        .into(),
                ])
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            );
        }
        column::with_children(items).spacing(10).into()
    }

    pub(super) fn panel_appearance(&self, draft: &PanelDraft) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let mut items = vec![label("Appearance & behavior", 16., FG)];
        let mut sizes: Vec<_> = PanelSize::CHOICES
            .into_iter()
            .map(|size| (size.title(), size))
            .collect();
        if let Setting::Known(custom @ PanelSize::Custom(_)) = draft.size() {
            sizes.push((custom.title(), custom));
        }
        items.push(field(
            "Size",
            choice(&draft.size(), sizes, Edit::Size, accent),
        ));
        if let Setting::Known(opacity) = draft.opacity() {
            let percent = (opacity * 100.).round().clamp(0., 100.) as u32;
            items.push(value_slider(
                "Transparency",
                format!("{}%", 100 - percent),
                0..=100,
                100 - percent,
                1,
                accent,
                |v| Edit::Opacity((100 - v) as u8),
            ));
        } else if let Setting::Unsupported(raw) = draft.opacity() {
            items.push(field("Transparency", kept(&raw)));
        }
        items.push(field(
            "Auto-hide",
            choice(
                &draft.autohide(),
                AutoHide::ALL
                    .into_iter()
                    .map(|mode| (mode.title().into(), mode))
                    .collect(),
                Edit::AutoHide,
                accent,
            ),
        ));
        // Native edge and display choices remain available for existing panels.
        items.push(field(
            "Screen edge",
            choice(
                &draft.anchor(),
                Anchor::ALL
                    .into_iter()
                    .map(|a| (a.title().into(), a))
                    .collect(),
                Edit::Anchor,
                accent,
            ),
        ));
        let mut outputs = vec![("All displays".into(), Output::All)];
        outputs.extend(
            self.desktop
                .outputs
                .iter()
                .map(|n| (n.clone(), Output::Name(n.clone()))),
        );
        match draft.output() {
            Setting::Known(Output::Name(n)) if !self.desktop.outputs.contains(&n) => {
                outputs.push((format!("{n} (not connected)"), Output::Name(n)))
            }
            Setting::Known(Output::Active) => {
                outputs.push(("Active display".into(), Output::Active))
            }
            _ => {}
        }
        items.push(field(
            "Display",
            choice(&draft.output(), outputs, Edit::Output, accent),
        ));
        items.push(self.desktop_switch(
            "Floating",
            "Gap from the screen edge",
            draft.floating(),
            Edit::Floating,
        ));
        items.push(field(
            "Corners",
            choice(
                &draft.radius(),
                vec![
                    ("Square".into(), 0),
                    ("Rounded".into(), 12),
                    ("Pill".into(), MAX_RADIUS),
                ],
                Edit::RadiusChoice,
                accent,
            ),
        ));
        if let Setting::Known(radius) = draft.radius()
            && ![0, 12, MAX_RADIUS].contains(&radius)
        {
            items.push(label(
                format!("Custom radius {radius} px · adjust in Advanced"),
                11.,
                MUTED,
            ));
        }
        let mut backgrounds = vec![
            ("Theme".into(), PanelBackground::ThemeDefault),
            ("Dark".into(), PanelBackground::Dark),
            ("Light".into(), PanelBackground::Light),
        ];
        if let Setting::Known(c @ PanelBackground::Color(_)) = draft.background() {
            backgrounds.push(("Custom color".into(), c));
        }
        items.push(field(
            "Background",
            choice(&draft.background(), backgrounds, Edit::Background, accent),
        ));
        section(items)
    }

    pub(super) fn panel_style(&self, draft: &PanelDraft) -> Element<'_, Message> {
        let Setting::Known(islands) = draft.islands() else {
            return section(vec![field("Style", kept("Unsupported island settings"))]);
        };
        let mut options = vec![
            (
                if draft.anchor().known().is_none_or(|a| a.horizontal()) {
                    "Full width"
                } else {
                    "Full height"
                }
                .into(),
                !islands.enabled && draft.extend().known() == Some(&true),
                Some(edit(Edit::PanelStyle(PanelStyle::FullWidth))),
            ),
            (
                "Islands".into(),
                islands.enabled,
                Some(edit(Edit::PanelStyle(PanelStyle::Islands))),
            ),
        ];
        if !self.desktop.islands.installation.installed() {
            options.truncate(1);
        }
        if !islands.enabled && draft.extend().known() != Some(&true) {
            options.push(("Compact · kept".into(), true, None));
        }
        field("Panel style", segmented(options, self.settings_accent()))
    }
}
