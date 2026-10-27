//! Shared color policy, explicit coordination scope and completed color edits.
use super::desktop::conflict_banner;
use super::*;
use crate::settings::wallpaper::Message as Edit;

fn edit(message: Edit) -> Message {
    Message::Wallpaper(message)
}

/// Where the shared accent goes. Order matches `Edit::SyncColors(index == 0)`.
static SCOPES: [&str; 2] = ["Whole desktop", "Widgets only"];

impl Settings {
    pub(super) fn colors_scope(&self) -> Element<'_, Message> {
        let preview = self.page == Page::Themes
            && !matches!(
                self.profiles.selection(),
                crate::settings::profiles::Selection::Current
            );
        let sync = if preview {
            self.profiles
                .profile()
                .and_then(|p| p.widget_style.as_ref())
                .and_then(|v| v.pointer("/appearance/sync_colors"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(self.config.appearance.sync_colors)
        } else {
            self.wallpaper.sync_colors(&self.config)
        };
        container(
            cosmic::widget::dropdown(&SCOPES[..], Some(usize::from(!sync)), move |index| {
                if preview {
                    Message::Profiles(crate::settings::profiles::Message::ThemeScope(index == 0))
                } else {
                    edit(Edit::SyncColors(index == 0))
                }
            })
            .width(Length::Fixed(176.)),
        )
        .id(cosmic::widget::Id::new("colors-scope"))
        .into()
    }

    pub(super) fn colors_conflicts(&self) -> Vec<Element<'_, Message>> {
        let mut items = Vec::new();
        if self.wallpaper.colors_conflict() {
            items.push(conflict_banner("App colors".into(), edit(Edit::UseCurrent)));
        }
        if !self.wallpaper.accent_conflicts().is_empty() {
            items.push(conflict_banner(
                crate::settings::system_appearance::describe(self.wallpaper.accent_conflicts()),
                edit(Edit::UseCurrent),
            ));
        }
        if self.wallpaper.colors_conflict() || !self.wallpaper.accent_conflicts().is_empty() {
            items.push(
                button::standard("Keep my color")
                    .on_press(edit(Edit::KeepColors))
                    .into(),
            );
        }
        items
    }

    /// The Color group: follow wallpaper, where colors apply, and the accent.
    pub(super) fn color_choices(&self) -> Element<'_, Message> {
        let automatic = self.wallpaper.automatic_colors(&self.config);
        let mut follow = vec![self.help(
            "wallpaper-colors",
            "Follow wallpaper",
            "New backgrounds update linked colors. Choosing a swatch or entering a color keeps that color until you follow wallpaper again.",
        )];
        // Only a pending search or a reason it can't follow is worth a line;
        // the normal behavior lives in the explanation above.
        if automatic && !self.wallpaper.automatic_ready() {
            follow.push(label(self.wallpaper.automatic_status(), 12., MUTED));
        }
        let mut rows = vec![split(
            column::with_children(follow).spacing(3).into(),
            container(toggler(automatic).on_toggle(|on| edit(Edit::AutomaticColors(on))))
                .id(cosmic::widget::Id::new("colors-automatic"))
                .into(),
        )];
        if self.config.appearance.family.is_some() {
            rows.push(split(
                label("Theme color", 14., FG),
                button::text("Use theme color")
                    .on_press(edit(Edit::ThemeColor))
                    .into(),
            ));
        }
        let several = automatic
            && self
                .wallpaper
                .displays()
                .iter()
                .filter(|view| view.connected)
                .count()
                > 1;
        if self.page != Page::Themes {
            rows.push(group_row(
                "Color applies to",
                if several {
                    format!(
                        "From {} · built-in display first",
                        self.wallpaper
                            .color_source_label()
                            .unwrap_or("Shared background")
                    )
                } else {
                    String::new()
                },
                self.colors_scope(),
            ));
        }
        rows.push(split(
            label(
                if automatic {
                    "Accent"
                } else {
                    "From wallpaper"
                },
                14.,
                FG,
            ),
            self.wallpaper_swatches(),
        ));
        rows.push(self.accent_code_row(automatic));
        let from_wallpaper: Vec<[f32; 3]> = self
            .wallpaper
            .suggestion()
            .map(|value| {
                value
                    .accents
                    .iter()
                    .map(|bytes| crate::settings::color_well::rgb(*bytes))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(panel) =
            self.color_panel(crate::settings::color_well::Target::Accent, &from_wallpaper)
        {
            rows.push(panel);
        }
        if self.wallpaper.accent_input_invalid() {
            rows.push(label(
                "Enter a complete color, such as #336699, then press Enter.",
                12.,
                desktop::ERROR,
            ));
        }
        // COSMIC draws the active window outline with its own color once one
        // is set; a shared accent would otherwise never reach it.
        let linked = self.wallpaper.outline_linked();
        if self.wallpaper.sync_colors(&self.config) && (self.wallpaper.outline_custom() || linked) {
            rows.push(group_row(
                "Window outline",
                if linked {
                    "Follows the accent"
                } else {
                    "Uses its own color"
                },
                container(
                    button::text(if linked {
                        "Keep its color"
                    } else {
                        "Match accent"
                    })
                    .on_press(edit(Edit::LinkOutline(!linked))),
                )
                .id(cosmic::widget::Id::new("colors-outline"))
                .into(),
            ));
        }
        if !automatic {
            rows.push(split(
                label("Fixed palettes", 14., FG),
                container(
                    button::text(if self.wallpaper.palette_choices {
                        "Hide"
                    } else {
                        "Show"
                    })
                    .id(cosmic::widget::Id::new("colors-palettes"))
                    .trailing_icon(
                        icon::from_name(if self.wallpaper.palette_choices {
                            "go-up-symbolic"
                        } else {
                            "go-down-symbolic"
                        })
                        .size(14),
                    )
                    .on_press(edit(Edit::PaletteChoices(!self.wallpaper.palette_choices))),
                )
                .into(),
            ));
            if self.wallpaper.palette_choices {
                rows.push(
                    row::with_children(Palette::CHOICES.iter().map(|&(key, title)| {
                        container(self.theme_choice(key, title))
                            .id(cosmic::widget::Id::new(format!("colors-palette-{key}")))
                            .into()
                    }))
                    .spacing(4)
                    .wrap()
                    .into(),
                );
            }
        }
        rows.insert(
            0,
            label(
                if self.wallpaper.background_dirty() {
                    "Local color preview · included when you apply the wallpaper"
                } else {
                    "Changes apply directly · complete color text with Enter"
                },
                12.,
                MUTED,
            ),
        );
        self.search_target(
            "wallpaper-colors",
            group(Some("Color source & accent"), rows),
        )
    }

    /// The accent's color control and the suggested-color shortcut share one row.
    fn accent_code_row(&self, automatic: bool) -> Element<'_, Message> {
        let current =
            crate::settings::wallpaper::parse_rgb(&self.wallpaper.color_text(&self.config));
        let mut parts = vec![
            container(self.color_button(
                "colors-hex",
                crate::settings::color_well::Target::Accent,
                current,
                true,
            ))
            .width(Length::Fill)
            .into(),
        ];
        if self
            .wallpaper
            .suggestion()
            .is_some_and(|value| !value.swatches.is_empty())
        {
            parts.push(
                container(
                    button::text(if automatic {
                        "Use recommended"
                    } else {
                        "Use best color"
                    })
                    .on_press(edit(Edit::SuggestAccent)),
                )
                .id(cosmic::widget::Id::new("wallpaper-best-color"))
                .into(),
            );
        }
        row::with_children(parts)
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap()
            .into()
    }
}
