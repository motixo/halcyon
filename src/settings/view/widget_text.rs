//! Widget style › Text: the widget font and the clock's own font, each an
//! expanding list where every family is shown in itself.
use super::pages::section;
use super::*;
use crate::settings::FontTarget;

/// Common, readable families offered first when they are installed.
const SUGGESTED: [&str; 15] = [
    "Inter",
    "Fira Sans",
    "Open Sans",
    "Noto Sans",
    "Ubuntu",
    "Roboto",
    "Lato",
    "Source Sans 3",
    "Noto Serif",
    "Merriweather",
    "Lora",
    "Fira Mono",
    "JetBrains Mono",
    "Noto Sans Mono",
    "Hack",
];
/// Enough to browse without a search; searching reaches the rest.
const SHOWN: usize = 80;

fn in_font<'a>(name: &str, size: f32, color: Color) -> Element<'a, Message> {
    text::body(name.to_string())
        .size(size)
        .font(cosmic::iced::Font {
            family: cosmic::iced::font::Family::Name(crate::widget_style::intern(name)),
            ..cosmic::iced::Font::DEFAULT
        })
        .class(cosmic::theme::Text::Color(color))
        .into()
}

impl Settings {
    /// Installed families, suggested ones first, matching the search.
    pub(in crate::settings) fn widget_font_families(&self) -> Vec<String> {
        let Some(fonts) = &self.font_catalog else {
            return Vec::new();
        };
        let mut all: Vec<String> = fonts
            .families(false)
            .iter()
            .chain(fonts.families(true))
            .map(|family| family.name.clone())
            .collect();
        all.sort_by_key(|name| name.to_lowercase());
        all.dedup();
        let query = self.font_search.trim().to_lowercase();
        let mut ordered: Vec<String> = SUGGESTED
            .iter()
            .filter(|name| all.iter().any(|family| family == *name))
            .map(|name| name.to_string())
            .collect();
        ordered.extend(all.into_iter().filter(|name| !SUGGESTED.contains(&name.as_str())));
        ordered.retain(|name| query.is_empty() || name.to_lowercase().contains(&query));
        ordered
    }

    pub(super) fn widget_text_section(&self) -> Element<'_, Message> {
        section(vec![
            label("Text", 14., FG),
            self.widget_font_row(
                FontTarget::Text,
                "Font",
                self.config.widget_font.as_deref(),
                "Style default",
            ),
            separator(),
            self.widget_font_row(
                FontTarget::Clock,
                "Clock font",
                self.config.clock_font.as_deref(),
                "Same as text",
            ),
        ])
    }

    /// At a glance's own fonts; the defaults are the widget fonts.
    pub(super) fn glance_text_section(&self) -> Element<'_, Message> {
        let g = &self.config.glance;
        section(vec![
            label("Text", 14., FG),
            self.widget_font_row(
                FontTarget::GlanceText,
                "Font",
                g.font.as_deref(),
                "Same as widgets",
            ),
            separator(),
            self.widget_font_row(
                FontTarget::GlanceClock,
                "Clock font",
                g.clock_font.as_deref(),
                "Same as text",
            ),
        ])
    }

    fn widget_font_row(
        &self,
        target: FontTarget,
        title: &'static str,
        current: Option<&str>,
        default: &'static str,
    ) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let open = self.font_menu == Some(target);
        let key = match target {
            FontTarget::Text => "text",
            FontTarget::Clock => "clock",
            FontTarget::GlanceText => "glance-text",
            FontTarget::GlanceClock => "glance-clock",
        };
        let shown: Element<'_, Message> = match current {
            Some(name) => in_font(name, 14., FG),
            None => label(default, 14., FG),
        };
        let mut rows = vec![split(
            label(title, 14., FG),
            button::custom(
                row::with_children(vec![
                    shown,
                    icon::from_name(if open {
                        "go-up-symbolic"
                    } else {
                        "go-down-symbolic"
                    })
                    .size(14)
                    .icon()
                    .into(),
                ])
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .id(cosmic::widget::Id::new(format!("font-{key}")))
            .class(chip_style(open, accent))
            .padding([6, 12])
            .on_press(Message::FontMenu(Some(target)))
            .into(),
        )];
        if !open {
            return column::with_children(rows).into();
        }
        let choice = |value: Option<String>, face: Element<'static, Message>, selected: bool| {
            button::custom(container(face).width(Length::Fill))
                .class(chip_style(selected, accent))
                .padding([6, 10])
                .width(Length::Fill)
                .on_press(Message::WidgetFont(target, value))
                .into()
        };
        let mut list: Vec<Element<'_, Message>> = vec![choice(
            None,
            label(default, 14., MUTED),
            current.is_none(),
        )];
        let families = self.widget_font_families();
        let total = families.len();
        for name in families.into_iter().take(SHOWN) {
            let selected = current == Some(name.as_str());
            let face = in_font(&name, 15., FG);
            list.push(choice(Some(name), face, selected));
        }
        rows.push(
            container(
                text_input("Search fonts", &self.font_search)
                    .on_input(Message::FontSearch)
                    .width(Length::Fill),
            )
            .id(cosmic::widget::Id::new(format!("font-search-{key}")))
            .into(),
        );
        rows.push(
            container(scrollable(column::with_children(list).spacing(2)).height(260))
                .id(cosmic::widget::Id::new(format!("font-list-{key}")))
                .into(),
        );
        if total > SHOWN {
            rows.push(label(
                format!("Showing {SHOWN} of {total}. Search to find the rest."),
                11.,
                MUTED,
            ));
        } else if total == 0 {
            rows.push(label(
                if self.font_search.trim().is_empty() {
                    "No installed fonts were found."
                } else {
                    "No fonts match."
                },
                12.,
                MUTED,
            ));
        }
        column::with_children(rows).spacing(8).into()
    }
}
