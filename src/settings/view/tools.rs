//! Music, Notes and Focus timer are individual widget settings pages.
//! Each starts with its on/off switch and the real desktop card.
use super::desktop::{ERROR, WARN, banner, line};
use super::pages::section;
use super::*;
use crate::config::{FocusPreferences, NotesSize};
use crate::settings::tools::Message as Edit;
use crate::tools::{cards, focus, music};
use cosmic::widget::text_editor;

fn edit(message: Edit) -> Message {
    Message::Tools(message)
}
fn on_preview(action: cards::Action) -> Message {
    Message::Tools(Edit::Card(action))
}
fn option(widget: Widget, key: &'static str, value: impl Into<serde_json::Value>) -> Message {
    Message::Tools(Edit::Option(widget, key, value.into()))
}

/// A one-of row whose options wrap under a narrow window.
fn choice_row<'a>(
    title: &'static str,
    options: Vec<(String, bool, Option<Message>)>,
    accent: Color,
) -> Element<'a, Message> {
    row::with_children(vec![
        container(label(title, 14., FG)).width(130).into(),
        container(segmented(options, accent))
            .width(Length::Fill)
            .into(),
    ])
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

impl Settings {
    fn tool_preview_size(&self, widget: Widget) -> [f32; 2] {
        let [width, height] = self
            .config
            .widget_sizes
            .get(widget.key())
            .copied()
            .map(|size| crate::layout::sized(widget, size, self.placement.screen()))
            .unwrap_or_else(|| cards::size(widget, &self.config));
        [
            width.min(if self.window.width >= 990. {
                320.
            } else {
                360.
            }),
            height.min(400.),
        ]
    }

    pub(super) fn music_page(&self) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let tools = &self.tools;
        let player = tools.current(&self.config);
        let preview = cards::music(
            cards::MusicView {
                feed: tools.feed.as_ref(),
                player,
                art: tools.art(&self.config),
                now: std::time::Instant::now(),
                manually_selected: tools.pinned.is_some() || self.config.music.player.is_some(),
                problem: tools.music_problem,
            },
            &self.config,
            Some(on_preview),
            Some(self.tool_preview_size(Widget::Music)),
        );
        // Follow names an app, so it survives the app's restarts.
        let preferred = self.config.music.player.as_deref().map(music::app_key);
        let mut rows: Vec<Element<'_, Message>> = vec![self.player_row(
            "Automatic".into(),
            "Current or most recent player".into(),
            preferred.is_none(),
            Some(edit(Edit::Player(None))),
            accent,
        )];
        let mut seen = std::collections::BTreeSet::new();
        for player in tools.player_list().iter().filter(|p| seen.insert(p.app())) {
            let player = music::choose(tools.player_list(), None, Some(player.app()), &[]).unwrap();
            let state = match player.status {
                music::Playback::Playing => "Playing",
                music::Playback::Paused => "Paused",
                music::Playback::Stopped => "Not playing",
            };
            let detail = if player.has_track() {
                format!("{state} · {}", player.title)
            } else {
                state.to_string()
            };
            rows.push(separator());
            rows.push(self.player_row(
                player.name().to_string(),
                detail,
                preferred == Some(player.app()),
                Some(edit(Edit::Player(Some(player.app().to_string())))),
                accent,
            ));
        }
        if let Some(key) = preferred.filter(|key| {
            !tools
                .player_list()
                .iter()
                .any(|player| player.app() == *key)
        }) {
            rows.push(separator());
            rows.push(self.player_row(
                music::fallback_name(key),
                "Not running · used when it starts".into(),
                true,
                None,
                accent,
            ));
        }
        let hint = match &tools.feed {
            None => Some("Looking for players…"),
            Some(music::Feed::Unavailable(_)) => {
                Some("Media controls need the desktop session bus, which isn’t reachable.")
            }
            Some(music::Feed::Players(players)) if players.is_empty() => Some(
                "Play a video or song in your browser to see it here. Your browser chooses which tab to share with desktop controls.",
            ),
            Some(_) => None,
        };
        let mut follow = vec![
            split(
                label("Follow", 16., FG),
                button::text("Refresh players")
                    .on_press(edit(Edit::Refresh))
                    .into(),
            ),
            column::with_children(rows).spacing(2).into(),
        ];
        if let Some(hint) = hint {
            follow.push(label(hint, 12., MUTED));
        }
        let items = vec![
            section(follow),
            section(vec![
                switch("Album art", "", self.config.music.show_artwork, |value| {
                    option(Widget::Music, "show_artwork", value)
                }),
                separator(),
                switch(
                    "Progress",
                    "Click the bar to seek when the player supports it.",
                    self.config.music.show_progress,
                    |value| option(Widget::Music, "show_progress", value),
                ),
            ]),
        ];
        column::with_children(vec![
            self.widget_heading(Widget::Music, "Music"),
            self.editor_with_preview(
                preview,
                column::with_children(items).spacing(16).into(),
                "Live controls",
            ),
        ])
        .spacing(16)
        .into()
    }

    fn player_row(
        &self,
        title: String,
        detail: String,
        selected: bool,
        message: Option<Message>,
        accent: Color,
    ) -> Element<'_, Message> {
        button::custom(
            row::with_children(vec![
                column::with_children(vec![
                    label(title, 14., if selected { FG } else { MUTED }),
                    line(detail, 11., MUTED),
                ])
                .spacing(2)
                .width(Length::Fill)
                .into(),
                label(if selected { "✓" } else { "" }, 14., accent),
            ])
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .class(nav_style(selected, accent))
        .selected(selected)
        .padding([8, 12])
        .width(Length::Fill)
        .on_press_maybe(message)
        .into()
    }

    pub(super) fn notes_page(&self) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let tools = &self.tools;
        let readable = tools.note.is_ok();
        let editor = text_editor(&tools.content)
            .placeholder("Write anything. Start a line with [ ] for a checklist.")
            .height(Length::Fixed(200.))
            .padding(12)
            .size(14.)
            .on_action(|action| edit(Edit::Note(action)))
            .key_binding(|press| {
                use cosmic::iced::keyboard::Key;
                if press.modifiers.control()
                    && matches!(&press.key, Key::Character(c) if c.as_str() == "s")
                {
                    Some(text_editor::Binding::Custom(edit(Edit::NoteSave)))
                } else {
                    text_editor::Binding::from_key_press(press)
                }
            });
        let dirty = tools.dirty();
        let state = if !readable {
            "This note can’t be read; it is left as it is.".to_string()
        } else if dirty {
            format!("Unsaved · {} characters", tools.note_chars())
        } else {
            match tools
                .note
                .as_ref()
                .ok()
                .and_then(|note| note.updated_at.as_deref())
            {
                Some(at) => chrono::DateTime::parse_from_rfc3339(at)
                    .map(|at| {
                        format!(
                            "Saved {}",
                            at.with_timezone(&chrono::Local).format("%-d %b, %H:%M")
                        )
                    })
                    .unwrap_or_else(|_| "Saved".into()),
                None => "Not written yet".into(),
            }
        };
        let mut note = vec![
            split(label("Note", 16., FG), label("Ctrl+S saves", 11., MUTED)),
            container(editor).width(Length::Fill).into(),
            row::with_children(vec![
                container(label(state, 12., if readable { MUTED } else { WARN }))
                    .width(Length::Fill)
                    .into(),
                button::text("Revert")
                    .on_press_maybe(dirty.then(|| edit(Edit::NoteRevert)))
                    .into(),
                button::suggested("Save")
                    .on_press_maybe((dirty && readable).then(|| edit(Edit::NoteSave)))
                    .into(),
            ])
            .spacing(10)
            .align_y(Alignment::Center)
            .into(),
        ];
        if tools.restored && dirty {
            note.push(banner(
                "Your unsaved edits from last time are back. Save puts them on the desktop.",
                accent,
                Vec::new(),
            ));
        }
        if tools.conflict.is_some() {
            note.push(banner(
                "The note changed on the desktop while you were editing here.",
                WARN,
                vec![
                    button::standard("Use the desktop’s")
                        .on_press(edit(Edit::NoteUseSaved))
                        .into(),
                    button::standard("Keep mine")
                        .on_press(edit(Edit::NoteKeepMine))
                        .into(),
                ],
            ));
        }
        if let Some(problem) = &tools.problem {
            note.push(label(format!("Not saved: {problem}"), 12., ERROR));
        }
        let preview = cards::notes(
            cards::NotesView {
                note: tools.note.as_ref().map_err(String::as_str),
                editing: None,
                paused: false,
                problem: None,
            },
            &self.config,
            (!dirty).then_some(on_preview as fn(cards::Action) -> Message),
            Some(self.tool_preview_size(Widget::Notes)),
        );
        let items = vec![
            section(note),
            section(vec![
                choice_row(
                    "Automatic size",
                    NotesSize::ALL
                        .into_iter()
                        .map(|size| {
                            (
                                size.title().to_string(),
                                self.config.notes.size == size,
                                (!self.config.widget_sizes.contains_key(Widget::Notes.key()))
                                    .then(|| option(Widget::Notes, "size", size.key())),
                            )
                        })
                        .collect(),
                    accent,
                ),
                if self.config.widget_sizes.contains_key(Widget::Notes.key()) {
                    split(
                        detail(
                            "Custom size is active",
                            "Return to Automatic size in Layout to use these presets.",
                        ),
                        button::text("Edit size in Layout")
                            .on_press(Message::Page(Page::Overview))
                            .into(),
                    )
                } else {
                    label(
                        "The card grows with its content within this size.",
                        11.,
                        MUTED,
                    )
                },
            ]),
            label(
                "[ ] starts a checklist item you can tick on the desktop; # starts a heading. Notes stay on this computer: presets and exports never include them.",
                12.,
                MUTED,
            ),
        ];
        column::with_children(vec![
            self.widget_heading(Widget::Notes, "Notes"),
            self.editor_with_preview(
                preview,
                column::with_children(items).spacing(16).into(),
                "Your note",
            ),
        ])
        .spacing(16)
        .into()
    }

    pub(super) fn focus_page(&self) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let prefs = &self.config.focus;
        let visible = self.config.widget_visible(Widget::Focus);
        let preview = cards::focus(
            &self.tools.focus,
            &self.config,
            focus::now_ms(),
            &focus::today(),
            visible.then_some(on_preview as fn(cards::Action) -> Message),
            Some(self.tool_preview_size(Widget::Focus)),
        );
        let minutes = |key: &'static str, choices: &[u16], current: u16| {
            let mut choices = choices.to_vec();
            if !choices.contains(&current) {
                choices.push(current);
                choices.sort_unstable();
            }
            choices
                .into_iter()
                .map(|value| {
                    (
                        format!("{value} min"),
                        value == current,
                        Some(option(Widget::Focus, key, value)),
                    )
                })
                .collect::<Vec<_>>()
        };
        let mut every = vec![0u8, 2, 3, 4, 6];
        if !every.contains(&prefs.long_break_every) {
            every.push(prefs.long_break_every);
            every.sort_unstable();
        }
        let items = vec![
            label(
                if visible {
                    "Preview controls run your desktop timer."
                } else {
                    "Show Focus timer to use its controls. Hiding it does not stop a running session."
                },
                12.,
                MUTED,
            ),
            section(vec![
                label("Lengths", 16., FG),
                choice_row(
                    "Focus",
                    minutes(
                        "focus_minutes",
                        &FocusPreferences::FOCUS_CHOICES,
                        prefs.focus_minutes,
                    ),
                    accent,
                ),
                choice_row(
                    "Break",
                    minutes(
                        "break_minutes",
                        &FocusPreferences::BREAK_CHOICES,
                        prefs.break_minutes,
                    ),
                    accent,
                ),
                choice_row(
                    "Long break",
                    minutes(
                        "long_break_minutes",
                        &FocusPreferences::LONG_BREAK_CHOICES,
                        prefs.long_break_minutes,
                    ),
                    accent,
                ),
                choice_row(
                    "Long break after",
                    every
                        .into_iter()
                        .map(|count| {
                            (
                                if count == 0 {
                                    "Never".to_string()
                                } else {
                                    format!("{count} rounds")
                                },
                                prefs.long_break_every == count,
                                Some(option(Widget::Focus, "long_break_every", count)),
                            )
                        })
                        .collect(),
                    accent,
                ),
            ]),
            section(vec![
                switch(
                    "Start breaks automatically",
                    "",
                    prefs.auto_start_breaks,
                    |value| option(Widget::Focus, "auto_start_breaks", value),
                ),
                separator(),
                switch("Notify when time is up", "", prefs.notify, |value| {
                    option(Widget::Focus, "notify", value)
                }),
            ]),
        ];
        column::with_children(vec![
            self.widget_heading(Widget::Focus, "Focus timer"),
            self.editor_with_preview(
                preview,
                column::with_children(items).spacing(16).into(),
                "Live controls",
            ),
        ])
        .spacing(16)
        .into()
    }
}
