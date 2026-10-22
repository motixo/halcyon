use super::pages::section;
use super::*;

impl Settings {
    pub(in crate::settings) fn needs_setup(&self) -> bool {
        // First-run setup replaces the checklist for anyone who went through it.
        self.setup_requested
            || !self.config.setup_dismissed
                && !self.config.onboarded
                && (!self.calendar.connection.connected()
                    || self.config.location.is_none()
                    || (self.service_known && !self.service.running))
    }
    pub(super) fn setup_card(&self) -> Element<'_, Message> {
        let accent = self.settings_accent();
        let mut startup = toggler(self.service.startup);
        if self.service_known && !self.busy && !preview() {
            startup = startup.on_toggle(Message::Startup);
        }
        let step = |number: &str,
                    title: &str,
                    description: String,
                    ready: bool,
                    button_text: &str,
                    action: Option<Message>| {
            split(
                row::with_children(vec![
                    pill(
                        if ready { "✓" } else { number },
                        if ready { accent } else { MUTED },
                    ),
                    detail(title.to_string(), description),
                ])
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
                button::standard(button_text.to_string())
                    .id(cosmic::widget::Id::new(format!("setup-step-{number}")))
                    .on_press_maybe(action)
                    .into(),
            )
        };
        section(vec![
            split(
                detail("Set up Desktop Cards", "All steps are optional."),
                button::text("Skip setup")
                    .id(cosmic::widget::Id::new("setup-skip"))
                    .on_press(Message::SetupDismissed(true))
                    .into(),
            ),
            step(
                "1",
                "Desktop widgets",
                if self.service.running {
                    "Running".into()
                } else {
                    "Choose which widgets to show.".into()
                },
                self.service.running,
                if self.service.running {
                    "Choose widgets"
                } else if !self.service_known {
                    "Checking…"
                } else {
                    "Show widgets"
                },
                if self.service.running {
                    Some(Message::Page(Page::Widgets))
                } else {
                    (self.service_known && !self.busy && !preview())
                        .then_some(Message::Desktop(true))
                },
            ),
            separator(),
            split(detail("Start when you sign in", ""), startup.into()),
            separator(),
            step(
                "2",
                "Local weather",
                self.config
                    .location
                    .as_ref()
                    .map(|location| location.name.clone())
                    .unwrap_or_else(|| "Choose a city. No account or API key needed.".into()),
                self.config.location.is_some(),
                if self.config.location.is_some() {
                    "Change city"
                } else {
                    "Choose city"
                },
                Some(Message::Page(Page::Weather)),
            ),
            separator(),
            step(
                "3",
                "Look",
                match self.config.appearance.family {
                    Some(family) if self.config.appearance.wallpaper_colors => {
                        format!("{} · colors from your wallpaper", family.title())
                    }
                    Some(family) => format!("{} theme", family.title()),
                    None => {
                        "Style widgets, dock and panel together, in your wallpaper’s colors.".into()
                    }
                },
                self.config.appearance.family.is_some(),
                if self.config.appearance.family.is_some() {
                    "Change look"
                } else {
                    "Choose a look"
                },
                // A first look comes from the starter looks; later changes
                // are made in Colors & theme.
                Some(Message::Page(if self.config.appearance.family.is_some() {
                    Page::Themes
                } else {
                    Page::DesktopProfiles
                })),
            ),
            separator(),
            step(
                "4",
                "Google Calendar",
                if self.calendar.connection.connected() {
                    self.calendar.connection.account.clone()
                } else {
                    "Connect Google to see your agenda and meeting links.".into()
                },
                self.calendar.connection.connected(),
                if self.calendar.connection.connected() {
                    "Manage"
                } else {
                    "Set up calendar"
                },
                Some(Message::Page(Page::Calendar)),
            ),
            separator(),
            row::with_children(vec![
                button::text("Add project folders")
                    .on_press(Message::Page(Page::Projects))
                    .into(),
                button::text("More widgets")
                    .on_press(Message::Page(Page::Widgets))
                    .into(),
            ])
            .spacing(8)
            .wrap()
            .into(),
            label(
                "Arrange cards from Layout. Hold the pointer at the far-right screen edge for one second to open widget controls.",
                11.,
                MUTED,
            ),
        ])
    }

    pub(super) fn setup_prompt(&self) -> Element<'_, Message> {
        frame(
            split(
                detail("Finish setup", "Weather city and calendar account"),
                row::with_children(vec![
                    button::standard("Set up")
                        .id(cosmic::widget::Id::new("setup-open"))
                        .on_press(Message::SetupOpen(true))
                        .into(),
                    button::icon(icon::from_name("window-close-symbolic").size(14))
                        .id(cosmic::widget::Id::new("setup-dismiss"))
                        .tooltip("Dismiss setup")
                        .on_press(Message::SetupDismissed(true))
                        .into(),
                ])
                .spacing(6)
                .align_y(Alignment::Center)
                .into(),
            ),
            SURFACE,
            LINE,
            12,
        )
    }
}
