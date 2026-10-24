use super::*;

impl Settings {
    pub(super) fn search_target<'a>(
        &'a self,
        id: &str,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let selected = self.search_selected.is_some_and(|hit| hit.anchor() == id);
        let accent = self.settings_accent();
        container(content)
            .id(cosmic::widget::Id::new(id.to_owned()))
            .width(Length::Fill)
            .class(cosmic::theme::Container::custom(move |_| {
                cosmic::iced::widget::container::Style {
                    border: Border {
                        color: if selected { accent } else { Color::TRANSPARENT },
                        width: if selected { 1. } else { 0. },
                        radius: 12.into(),
                    },
                    ..Default::default()
                }
            }))
            .into()
    }
    pub(super) fn page_settings_advanced<'a>(
        &'a self,
        page: Page,
        children: Vec<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let open = self.advanced_open(page);
        let mut rows = vec![
            button::custom(split(
                label("Advanced", 14., FG),
                icon::from_name(if open {
                    "go-up-symbolic"
                } else {
                    "go-down-symbolic"
                })
                .size(16)
                .icon()
                .into(),
            ))
            .class(nav_style(false, self.settings_accent()))
            .width(Length::Fill)
            .padding([4, 2])
            .on_press(Message::ToggleAdvanced(page))
            .into(),
        ];
        let mut body = vec![separator()];
        body.extend(children);
        rows.push(super::reveal::reveal(
            container(column::with_children(body).spacing(10))
                .padding([10, 0, 0, 0])
                .into(),
            open,
            self.config.settings_animations && self.system_animations,
        ));
        self.search_target(
            "settings-advanced",
            pages::section(vec![column::with_children(rows).into()]),
        )
    }

    pub(super) fn navigation_header(&self) -> Element<'_, Message> {
        // Breadcrumb on the left, a native search field on the right.
        let mut search_row: Vec<Element<'_, Message>> = Vec::new();
        search_row.push(match self.page.parent() {
            Some(parent) => container(
                button::text(parent.nav_title())
                    .id(cosmic::widget::Id::new("settings-back"))
                    .leading_icon(icon::from_name("go-previous-symbolic").size(16))
                    .tooltip("Back (Alt+←)")
                    .on_press(Message::Back),
            )
            .width(Length::Fill)
            .into(),
            None => cosmic::iced::widget::space().width(Length::Fill).into(),
        });
        let mut search =
            cosmic::widget::text_input::search_input("Search settings", &self.search_query)
                .id(super::super::navigation::search_id())
                .on_input(Message::SearchSettings)
                .on_submit(|_| Message::SearchFirst)
                .width(Length::Fill);
        if !self.search_query.is_empty() {
            search = search.on_clear(Message::SearchSettings(String::new()));
        }
        search_row.push(
            container(search)
                .width(if self.window.width >= 990. { 340 } else { 260 })
                .into(),
        );
        let mut items = vec![
            row::with_children(search_row)
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
        ];
        if let Some(problem) = &self.session_drafts.error {
            items.push(label(
                format!("Draft recovery: {problem}"),
                11.,
                Color::from_rgb8(245, 149, 138),
            ));
        }
        container(column::with_children(items).spacing(4))
            .padding([10, 22, 10, 22])
            .width(Length::Fill)
            .into()
    }

    pub(super) fn search_results_page(&self) -> Element<'_, Message> {
        let matches = super::super::navigation::search(&self.search_query);
        let mut rows = vec![heading("Find a setting", "")];
        if matches.is_empty() {
            rows.push(label(
                "No settings found. Try city, GPU, font, tiling or startup.",
                14.,
                MUTED,
            ));
        }
        for hit in matches {
            rows.push(
                button::custom(split(
                    row::with_children(vec![
                        dot(
                            area_tint(
                                hit.page.parent().unwrap_or(hit.page).area(),
                                self.settings_accent(),
                            ),
                            8.,
                        ),
                        detail(
                            hit.label,
                            format!(
                                "{} › {}{}",
                                hit.page.parent().unwrap_or(hit.page).area().label(),
                                hit.page.title(),
                                if hit.advanced { " › Advanced" } else { "" }
                            ),
                        ),
                    ])
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .into(),
                    icon::from_name("go-next-symbolic").size(16).icon().into(),
                ))
                .class(nav_style(false, self.settings_accent()))
                .padding(14)
                .width(Length::Fill)
                .on_press(Message::SearchResult(hit))
                .into(),
            );
        }
        column::with_children(rows).spacing(10).into()
    }

    pub(super) fn pending_changes_page(&self) -> Element<'_, Message> {
        let mut rows = vec![heading(
            "Recover unfinished work",
            "These choices have no effect until you use them. Originals are retained.",
        )];
        for (index, proposal) in self.recovery_sheet.rows.iter().enumerate() {
            let mut parts = vec![split(
                detail(proposal.kind.title(), &proposal.summary),
                toggler(proposal.selected)
                    .on_toggle(move |on| Message::RecoverySelect(index, on))
                    .into(),
            )];
            if let Some(problem) = &proposal.problem {
                parts.push(label(problem, 12., desktop::ERROR));
            }
            for (field, (name, value)) in proposal.fields().into_iter().enumerate() {
                parts.push(split(
                    label(name, 13., FG),
                    cosmic::widget::text_input("#RRGGBB", value)
                        .on_input(move |text| Message::RecoveryText(index, field, text))
                        .width(180)
                        .into(),
                ));
            }
            let mut actions = vec![
                button::text("Keep current")
                    .on_press(Message::RecoveryDiscard(index))
                    .into(),
                button::text("Use this choice")
                    .on_press(Message::RecoveryApplyOne(index))
                    .into(),
            ];
            if proposal.problem.is_some() {
                if proposal.fields().is_empty() {
                    actions.push(
                        button::text("Open form")
                            .on_press(Message::RecoveryOpen(index))
                            .into(),
                    );
                }
                actions.push(
                    button::text("Use recovered values")
                        .on_press(Message::RecoveryUseProposed(index))
                        .into(),
                );
            }
            parts.push(row::with_children(actions).spacing(8).wrap().into());
            rows.push(pages::section(parts));
        }
        if self.recovery_sheet.rows.is_empty() {
            rows.push(label(
                "No unfinished work from the previous version.",
                14.,
                MUTED,
            ));
        } else {
            rows.push(
                button::suggested("Use selected choices")
                    .on_press(Message::RecoveryApply)
                    .into(),
            );
        }
        column::with_children(rows).spacing(12).into()
    }

    pub(super) fn simple_status(&self) -> Element<'_, Message> {
        if let Some(review) = &self.undo_review {
            let mut lines = vec![label(
                &format!(
                    "Undo {} · some values changed elsewhere",
                    review.label.to_lowercase()
                ),
                13.,
                desktop::ERROR,
            )];
            lines.extend(
                review
                    .conflicts
                    .iter()
                    .take(3)
                    .map(|conflict| label(conflict, 11., Color::from_rgb8(170, 175, 184))),
            );
            lines.push(
                row::with_children(vec![
                    button::text("Keep all current values")
                        .id("undo-keep-current".into())
                        .on_press(Message::ResolveUndoShown(review.clone(), false))
                        .into(),
                    button::suggested("Undo other changes")
                        .id("undo-reviewed".into())
                        .on_press(Message::ResolveUndoShown(review.clone(), true))
                        .into(),
                ])
                .spacing(8)
                .into(),
            );
            return container(column::with_children(lines).spacing(6))
                .padding([8, 22])
                .width(Length::Fill)
                .into();
        }
        use crate::settings::desktop::Notice;
        let notice = match self.page {
            Page::DesktopDock => self.desktop.notice.as_ref(),
            Page::DesktopAppearance => self.system_appearance.notice.as_ref(),
            Page::DesktopWindows => self.windows.notice().or(self.workspaces.notice()),
            Page::DesktopWorkspaces => self.workspaces.notice(),
            Page::DesktopIcons => self.desktop_icons.notice(),
            Page::DesktopWindowRules => self.window_rules.notice(),
            Page::DesktopShortcuts => self.shortcuts.notice(),
            Page::DesktopProfiles => self.profiles.notice.as_ref(),
            Page::DesktopAutomation => self.automation.notice.as_ref(),
            Page::Themes if matches!(self.profiles.notice, Some(Notice::Error(_))) => {
                self.profiles.notice.as_ref()
            }
            Page::DesktopWallpaper | Page::Themes | Page::Appearance => self.wallpaper.notice(),
            _ => None,
        };
        let error = match notice {
            Some(Notice::Error(error)) => Some(error.as_str()),
            _ if self.error => Some(self.status.as_str()),
            _ => None,
        };
        let mut items = vec![
            container(label(error.unwrap_or(""), 12., desktop::ERROR))
                .width(Length::Fill)
                .into(),
        ];
        let mut empty = error.is_none();
        if let Some((_, label, id)) = self.last_desktop_change.as_ref().filter(|(owner, _, id)| {
            owns(*owner, self.page) && self.undo_baseline.as_ref() != Some(id)
        }) {
            empty = false;
            items.push(
                button::text(format!("Undo {}", label.to_lowercase()))
                    .id(cosmic::widget::Id::new("settings-undo"))
                    .leading_icon(icon::from_name("edit-undo-symbolic").size(14))
                    .on_press(Message::UndoDesktopShown(Some(id.clone())))
                    .into(),
            );
        }
        // Nothing to say: no empty bar at the bottom of the page.
        if empty {
            return cosmic::iced::widget::space().height(0).into();
        }
        container(
            row::with_children(items)
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding([8, 22])
        .width(Length::Fill)
        .into()
    }
}

/// Undo shows only where its change was made, not on every page.
fn owns(owner: crate::desktop::Owner, page: Page) -> bool {
    use crate::desktop::Owner;
    match owner {
        Owner::Recovery => false,
        Owner::DockPanel => page == Page::DesktopDock,
        Owner::Preferences => {
            page.sidebar() == Page::Widgets
                || matches!(
                    page,
                    Page::Glance
                        | Page::Overview
                        | Page::WidgetLayout
                        | Page::Appearance
                        | Page::Themes
                )
        }
        Owner::Appearance => matches!(page, Page::Themes | Page::DesktopAppearance),
        Owner::Profile => matches!(
            page,
            Page::Themes | Page::DesktopProfiles | Page::DesktopAutomation
        ),
        Owner::Windows | Owner::Workspaces | Owner::WindowRules => matches!(
            page,
            Page::DesktopWindows | Page::DesktopWorkspaces | Page::DesktopWindowRules
        ),
        Owner::Shortcuts => page == Page::DesktopShortcuts,
        Owner::Wallpaper => matches!(page, Page::DesktopWallpaper | Page::Themes),
        Owner::DesktopIcons => page == Page::DesktopIcons,
    }
}
