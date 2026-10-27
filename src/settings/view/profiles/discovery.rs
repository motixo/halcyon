use super::*;
impl Settings {
    pub(super) fn discovery_gallery(&self) -> Element<'_, Message> {
        let editor = &self.profiles;
        let tiles = crate::desktop::profiles::discovery::CURATED
            .into_iter()
            .filter_map(|id| editor.presets().iter().find(|p| p.id == id))
            .filter(|p| crate::desktop::profiles::discovery::matches(p, &editor.discovery_query))
            .map(|p| self.preset_tile(p))
            .collect::<Vec<_>>();
        let mut items = vec![
            row::with_children(vec![
                text_input("Find a look", &editor.discovery_query)
                    .id(cosmic::widget::Id::new("discovery-search"))
                    .on_input(|s| edit(Edit::DiscoveryQuery(s)))
                    .into(),
                button::text("Open a shared setup…")
                    .id(cosmic::widget::Id::new("discovery-import"))
                    .on_press_maybe(
                        (editor.writable() && !self.native_busy).then_some(edit(Edit::Import)),
                    )
                    .into(),
            ])
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
        ];
        if tiles.is_empty() {
            items.push(label(
                "No featured looks match. Try a name or description.",
                14.,
                MUTED,
            ));
        } else {
            items.push(row::with_children(tiles).spacing(16).wrap().into());
        }
        column::with_children(items).spacing(12).into()
    }
    pub(super) fn setup_sources(&self) -> Element<'_, Message> {
        let sources = self.profiles.discovery_sources();
        if sources.is_empty() {
            return cosmic::widget::space().height(0).into();
        }
        let mut rows = vec![label("Sources & revisions", 16., FG)];
        for source in sources {
            let credits = &source.attribution.credits;
            let name = if credits.author.is_empty() {
                "Uncredited source"
            } else {
                &credits.author
            };
            let mut row_items = vec![
                column::with_children(vec![
                    label(format!("{} · {}", name, credits.license), 14., FG),
                    label(
                        crate::settings::profiles::ProfilesEditor::revision_label(
                            &source.attribution.identity,
                        ),
                        12.,
                        MUTED,
                    ),
                ])
                .spacing(4)
                .width(Length::Fill)
                .into(),
            ];
            if let Some(selection) = source.exact {
                row_items.push(
                    button::text(if matches!(selection, Selection::Preset(_)) {
                        "View source"
                    } else {
                        "Open local copy"
                    })
                    .on_press(edit(Edit::Select(selection)))
                    .into(),
                );
            } else {
                row_items.push(label("Source not in library", 12., MUTED));
            }
            if let Some(selection) = source.latest {
                row_items.push(
                    button::text("Review update")
                        .on_press(edit(Edit::Select(selection)))
                        .into(),
                );
            }
            rows.push(
                row::with_children(row_items)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .wrap()
                    .into(),
            );
            if !credits.source.is_empty() {
                rows.push(label(credits.source.clone(), 12., MUTED));
            }
        }
        rows.push(label(
            "Opening a source leaves your saved copy unchanged.",
            12.,
            MUTED,
        ));
        self.disclosure("setup-sources", "Sources & revisions", section(rows))
    }
}
