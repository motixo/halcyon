// Native layer-shell setup adapted from longer54/pop-os-cosmic-widgets (GPL-3.0-or-later).
use crate::config::{Appearance, Config, Location, Palette};
use crate::data::{
    self, CalendarData, CalendarEvent, MeetingState, Metrics, ProjectsData, Sampler, Weather,
    WeatherSnapshot,
};
use crate::drawer::{self, DragResult, ReorderDrag, Sidebar, Widget};
mod autoswitch;
mod clock;
mod devbox;
mod glance;

pub(crate) fn settings_clock_preview(
    config: &Config,
    size: [f32; 2],
) -> cosmic::Element<'static, Message> {
    clock::view(config, chrono::Local::now().naive_local(), size)
}

mod history;
mod layout;
mod panel;
mod projects;
mod settings_preview;
pub(crate) use settings_preview::{
    SettingsSamples, settings_devbox_preview, settings_weather_preview,
};
#[cfg(test)]
mod theme_desktop_tests;
#[cfg(test)]
mod theme_tests;
#[cfg(test)]
mod look_render_tests;
mod tools;
#[cfg(test)]
mod utility_tests;
mod wallpaper_colors;
use chrono::{Datelike, Duration as Days, Local, NaiveDate};
use cosmic::Application as _;
use cosmic::Element;
use cosmic::app::{Core, Task};
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{
    IcedMargin, SctkLayerSurfaceSettings,
};
use cosmic::iced::platform_specific::shell::commands::layer_surface::{
    Anchor, KeyboardInteractivity, Layer, get_layer_surface, set_margin,
};
use cosmic::iced::{Alignment, Background, Border, Color, Font, Length, Subscription, window};
use cosmic::widget::{column, container, mouse_area, row, scrollable, text};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    /// The wake for this deadline fired, this much earlier than the
    /// monotonic clock reached it (time lost in suspend).
    Woke(Instant, Duration),
    FilesChanged,
    DesktopPaletteChanged,
    WallpaperColorsChanged,
    WallpaperColorsReady(
        u64,
        Result<Option<crate::desktop::wallpaper::automatic::Prepared>, String>,
    ),
    WallpaperColorsApplied(Result<(), String>),
    Metrics(Result<(Sampler, Metrics), String>),
    Animate,
    Hover(bool, bool),
    ToggleWidget(Widget),
    FocusWidget(Widget),
    ShowControls,
    Arrange,
    ResetLayout,
    LayoutPress(Widget),
    LayoutResizePress(Widget),
    LayoutSelect(Widget),
    LayoutResize([i8; 2]),
    LayoutResizeMode,
    LayoutAutomatic,
    LayoutUndo,
    LayoutFinish(bool),
    LayoutDefaults,
    LayoutSnap(bool),
    LayoutCancelDrag,
    LayoutKey(cosmic::iced::keyboard::Key, bool),
    HideControls,
    SidebarPress(Widget),
    Pointer(window::Id, cosmic::iced::mouse::Event),
    SurfaceResized(window::Id, cosmic::iced::Size),
    ToggleProjects,
    ToggleProject(String),
    ToggleEarlier,
    RefreshProjects,
    Projects(Vec<String>, Result<ProjectsData, String>),
    Weather(Location, Result<WeatherSnapshot, String>),
    Service(bool),
    Calendar(String, String, Result<CalendarData, String>),
    CalendarToday(String, Result<CalendarData, String>),
    PreviewCalendar(CalendarData),
    RequestCapture,
    Week(i32),
    Day(NaiveDate),
    Today,
    RefreshCalendar,
    Open(String),
    Settings,
    WeatherSettings,
    CalendarSettings,
    ProjectSettings,
    ControlFinished(Result<(), String>),
    CaptureScreens,
    Captured(usize, window::Screenshot),
    /// A Music, Notes or Focus card control.
    Tool(crate::tools::cards::Action),
    MusicFeed(crate::tools::music::Feed),
    MusicArt(String, Result<crate::tools::music::Art, String>),
    ToolDone(Result<(), String>),
    MusicControlDone(Result<Vec<crate::tools::music::Player>, String>),
    NoteCopyDone(Result<(), String>),
    /// A note save finished: the text it saved, and the result. (Tests
    /// save inline.)
    #[cfg_attr(test, allow(dead_code))]
    NoteSaved(String, Result<crate::tools::notes::Note, crate::tools::notes::SaveError>),
    /// `desktop-cards focus start|pause|toggle|reset|skip`.
    FocusCommand(String),
    /// The compositor added or removed a display.
    DisplaysChanged,
    /// Displays settled after a change: build every surface again.
    RebuildSurfaces(u64),
    /// Automatic profile switches finished.
    Switched(Vec<(crate::automation::Due, crate::automation::run::Ran)>),
}
pub struct Cards {
    core: Core,
    ids: [window::Id; 12],
    layout_editor: layout::LayoutEditor,
    surface_geometry: Option<layout::SurfaceGeometry>,
    /// When the cards started (display events at startup aren't changes).
    started: Instant,
    /// Display changes seen; only the latest one rebuilds the surfaces.
    display_generation: u64,
    /// Where each card surface takes clicks (`Widget::BASE` order).
    surface_inputs: Option<[Vec<[u32; 4]>; 6]>,
    /// Inside `update` (which also runs nested for some messages).
    updating: bool,
    /// The card surfaces need rebuilding at the end of this update.
    rebuild_pending: bool,
    screen_size: [f32; 2],
    output_known: bool,
    sidebar: Sidebar,
    desktop_height: f32,
    calendar_height: f32,
    projects: Option<ProjectsData>,
    projects_fetching: bool,
    projects_error: bool,
    next_projects: Instant,
    all_projects: bool,
    expanded_projects: std::collections::BTreeSet<String>,
    config: Config,
    sampler: Sampler,
    metrics: Metrics,
    histories: history::MetricHistories,
    metrics_fetching: bool,
    next_metrics: Instant,
    service_fetching: bool,
    next_service: Instant,
    next_clock: Instant,
    next_calendar_view: Instant,
    weather: Option<Weather>,
    weather_error: bool,
    weather_at: Option<String>,
    next_weather: Instant,
    fetching: bool,
    capture_key: Option<String>,
    calendar: Option<CalendarData>,
    calendar_connection: crate::calendar::Connection,
    calendar_today: Option<CalendarData>,
    today_fetching: bool,
    calendar_today_error: bool,
    calendar_preview: bool,
    next_today_calendar: Instant,
    calendar_error: bool,
    calendar_fetching: bool,
    next_calendar: Instant,
    month: NaiveDate,
    selected_day: NaiveDate,
    calendar_home: bool,
    last_today: NaiveDate,
    show_earlier: bool,
    tools: tools::Tools,
    automator: crate::automation::runtime::Automator,
    /// Both clocks at the last wake, to see time lost in suspend.
    woke_at: (Instant, SystemTime),
    /// Part of the wake's identity, so a fired wake can be armed again for
    /// the same deadline.
    wake_generation: u64,
    wallpaper_color_generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
}
impl cosmic::Application for Cards {
    type Executor = cosmic::executor::Default;
    type Flags = Config;
    type Message = Message;
    const APP_ID: &'static str = "io.github.workstation.DesktopCards";
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn init(mut core: Core, config: Config) -> (Self, Task<Message>) {
        // Styles name their fonts; learn which are installed before drawing.
        crate::desktop_theme::load_installed_fonts();
        core.window.show_headerbar = false;
        // Desktop surfaces draw their own shapes. Automatic compositor blur
        // covers the entire surface rectangle, including transparent clock
        // pixels and the space outside rounded card corners.
        core.set_auto_blur(Default::default());
        let ids = std::array::from_fn(|_| window::Id::unique());
        let sidebar = Sidebar::new();
        let mut tasks = vec![
            get_layer_surface(SctkLayerSurfaceSettings {
                id: ids[0],
                layer: Layer::Bottom,
                keyboard_interactivity: KeyboardInteractivity::None,
                anchor: Anchor::TOP | Anchor::LEFT,
                margin: IcedMargin {
                    top: 88,
                    left: config.clock_margin(),
                    ..Default::default()
                },
                size: Some((Some(490), Some(250))),
                // The clock is decorative. Its transparent area must never
                // intercept clicks intended for Calendar or the desktop.
                input_zone: Some(Vec::new()),
                exclusive_zone: 0,
                namespace: "desktop-cards-clock".into(),
                ..Default::default()
            }),
            get_layer_surface(SctkLayerSurfaceSettings {
                id: ids[1],
                layer: Layer::Bottom,
                keyboard_interactivity: KeyboardInteractivity::None,
                anchor: Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM,
                margin: IcedMargin {
                    top: 48,
                    right: 42,
                    bottom: 48,
                    left: 0,
                },
                size: Some((Some(360), Some(0))),
                exclusive_zone: 0,
                namespace: "desktop-cards-widgets".into(),
                ..Default::default()
            }),
            Self::controls_surface(ids[2], sidebar.margin()),
            Self::edge_surface(ids[3]),
            get_layer_surface(SctkLayerSurfaceSettings {
                id: ids[4],
                layer: Layer::Bottom,
                keyboard_interactivity: KeyboardInteractivity::None,
                anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT,
                margin: IcedMargin {
                    top: 48,
                    left: config.calendar_margin(),
                    bottom: 48,
                    right: 0,
                },
                size: Some((Some(360), Some(0))),
                exclusive_zone: 0,
                namespace: "desktop-cards-calendar".into(),
                ..Default::default()
            }),
        ];
        // A passive, click-through surface reports the active output dimensions.
        // It is created before the visible widgets so its backdrop stays behind them.
        let probe = Self::layout_probe(ids[5], false);
        for index in [6, 7, 8, 9, 10, 11] {
            tasks.push(get_layer_surface(SctkLayerSurfaceSettings {
                id: ids[index],
                layer: Layer::Bottom,
                keyboard_interactivity: KeyboardInteractivity::None,
                anchor: Anchor::TOP | Anchor::LEFT,
                margin: IcedMargin {
                    left: -10000,
                    ..Default::default()
                },
                size: Some((Some(16), Some(16))),
                exclusive_zone: 0,
                namespace: format!("desktop-cards-extra-{index}"),
                ..Default::default()
            }));
        }
        // COSMIC's backdrop for the full-height Calendar surface can obscure
        // an earlier Clock, including the empty space above the agenda. Create
        // the decorative, click-through clock last within the same bottom layer.
        let clock = tasks.remove(0);
        let mut tasks = vec![probe.chain(Task::batch(tasks)).chain(clock)];
        let today = Local::now().date_naive();
        let month = today.with_day(1).unwrap();
        let calendar = CalendarData::cached(&month.format("%Y-%m").to_string());
        let weather = config.location.as_ref().and_then(WeatherSnapshot::cached);
        let calendar_refresh = config.refresh_intervals.duration(Widget::Calendar);
        let mut app = Self {
            core,
            ids,
            layout_editor: layout::LayoutEditor::default(),
            surface_geometry: None,
            started: Instant::now(),
            display_generation: 0,
            surface_inputs: None,
            updating: false,
            rebuild_pending: false,
            screen_size: [1920., 1080.],
            output_known: false,
            sidebar,
            desktop_height: 984.,
            calendar_height: 984.,
            projects: config.project_roots.is_empty().then(ProjectsData::default),
            projects_fetching: false,
            projects_error: false,
            next_projects: Instant::now(),
            all_projects: false,
            expanded_projects: Default::default(),
            config,
            sampler: Sampler::default(),
            metrics: Metrics::default(),
            histories: history::MetricHistories::default(),
            metrics_fetching: false,
            next_metrics: Instant::now(),
            service_fetching: false,
            next_service: Instant::now(),
            next_clock: Instant::now(),
            next_calendar_view: Instant::now(),
            weather: weather.as_ref().map(|snapshot| snapshot.weather.clone()),
            weather_error: false,
            weather_at: weather.map(|snapshot| snapshot.updated_at),
            next_weather: Instant::now(),
            fetching: false,
            capture_key: None,
            calendar_today: calendar.clone(),
            today_fetching: false,
            calendar_today_error: false,
            calendar_preview: false,
            next_today_calendar: Instant::now() + calendar_refresh,
            calendar,
            calendar_connection: crate::calendar::load_connection(),
            calendar_error: false,
            calendar_fetching: false,
            next_calendar: Instant::now(),
            month,
            selected_day: today,
            calendar_home: true,
            last_today: today,
            show_earlier: false,
            tools: tools::Tools::load(),
            automator: crate::automation::runtime::Automator::from_env(),
            woke_at: (Instant::now(), SystemTime::now()),
            wake_generation: 0,
            wallpaper_color_generation: Default::default(),
        };
        let creation = tasks.remove(0);
        tasks.insert(0, creation.chain(app.sync_widget_surfaces()));
        tasks.extend(app.refresh_visible_data());
        tasks.push(app.wallpaper_colors_changed());
        (app, Task::batch(tasks))
    }
    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = Vec::new();
        if let Some(deadline) = self.wake_deadline() {
            subscriptions.push(
                crate::schedule::wake_at(deadline)
                    .with((deadline, self.wake_generation))
                    .map(|((deadline, _), early)| Message::Woke(deadline, early)),
            );
        }
        // Players are followed only while the Music card is shown.
        if self.config.widget_visible(Widget::Music) && !self.layout_editor.active {
            subscriptions.push(crate::tools::music::watch().map(Message::MusicFeed));
        }
        let paths = Config::path()
            .into_iter()
            .chain(Self::command_path())
            .chain(crate::tools::notes::path())
            .chain(crate::calendar::account_path())
            .chain(self.automator.paths())
            .collect();
        subscriptions.push(crate::watch::files(paths).map(|_| Message::FilesChanged));
        subscriptions.push(
            self.config
                .color_subscription()
                .map(|_| Message::DesktopPaletteChanged),
        );
        if (self.config.appearance.wallpaper_colors || self.config.appearance.family.is_some())
            && let Ok(store) = crate::desktop::DesktopStore::from_env()
            && store.writable()
        {
            subscriptions.push(
                crate::watch::read_only_files(crate::desktop::wallpaper::automatic::watch_paths(
                    &store,
                ))
                .map(|_| Message::WallpaperColorsChanged),
            );
        }
        subscriptions.push(
            cosmic::iced::event::listen_with(|event, status, id| match event {
                cosmic::iced::Event::Mouse(event) => Some(Message::Pointer(id, event)),
                cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) if status == cosmic::iced::event::Status::Ignored => {
                    Some(Message::LayoutKey(key, modifiers.shift()))
                }
                cosmic::iced::Event::Window(window::Event::Unfocused) => {
                    Some(Message::LayoutCancelDrag)
                }
                cosmic::iced::Event::Window(window::Event::Opened { size, .. })
                | cosmic::iced::Event::Window(window::Event::Resized(size)) => {
                    Some(Message::SurfaceResized(id, size))
                }
                cosmic::iced::Event::PlatformSpecific(
                    cosmic::iced::event::PlatformSpecific::Wayland(
                        cosmic::iced::event::wayland::Event::Output(..),
                    ),
                ) => Some(Message::DisplaysChanged),
                _ => None,
            })
            .with((
                self.ids,
                self.sidebar.drag.is_some(),
                self.layout_editor.active,
            ))
            .filter_map(|((ids, dragging, arranging), message)| {
                match &message {
                    Message::Pointer(id, event) => {
                        // Motion over Calendar/Weather must not rebuild all five surfaces.
                        // Outside releases still cancel an active sidebar drag.
                        ((arranging
                            && (*id == ids[5]
                                || matches!(
                                    event,
                                    cosmic::iced::mouse::Event::ButtonReleased(
                                        cosmic::iced::mouse::Button::Left
                                    )
                                )))
                            || *id == ids[2]
                            || *id == ids[3]
                            || (dragging
                                && matches!(
                                    event,
                                    cosmic::iced::mouse::Event::ButtonReleased(
                                        cosmic::iced::mouse::Button::Left
                                    )
                                )))
                        .then_some(message)
                    }
                    Message::LayoutKey(..) | Message::LayoutCancelDrag => {
                        arranging.then_some(message)
                    }
                    _ => Some(message),
                }
            }),
        );
        if self.sidebar.animating() {
            subscriptions.push(
                cosmic::iced::time::every(Duration::from_millis(16)).map(|_| Message::Animate),
            );
        }
        if let Some(deadline) = self.sidebar.next_transition() {
            subscriptions.push(crate::schedule::wake_at(deadline).map(|_| Message::Animate));
        }
        Subscription::batch(subscriptions)
    }
    fn view(&self) -> Element<'_, Message> {
        text::body("").into()
    }
    fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if id == self.ids[5] {
            return self.layout_view();
        }
        if id == self.ids[2] {
            return self.control_rail();
        }
        if id == self.ids[3] {
            return self.edge_handle();
        }
        if self.layout_editor.active {
            return empty_surface();
        }
        if id == self.ids[8] {
            return self.individual_widget(Widget::Glance);
        }
        for tool in Widget::TOOLS {
            if id == self.widget_id(tool) {
                return self.individual_widget(tool);
            }
        }
        if self.custom_layout() {
            let widget = if id == self.ids[0] {
                Widget::Clock
            } else if id == self.ids[1] {
                Widget::Weather
            } else if id == self.ids[4] {
                Widget::Calendar
            } else if id == self.ids[6] {
                Widget::Devbox
            } else {
                Widget::Projects
            };
            return self.individual_widget(widget);
        }
        if id == self.ids[0] {
            self.individual_widget(Widget::Clock)
        } else if id == self.ids[4] {
            self.calendar_widget()
        } else if id == self.ids[1] {
            self.desktop_widgets()
        } else {
            empty_surface()
        }
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        // Card surfaces are rebuilt at most once per update, for its final
        // geometry: two rebuilds at once race, and the second creation is
        // refused, leaving a surface with a stale size and input region.
        // Surface work runs beside the update's other work (an artwork or
        // weather download), never queued behind it.
        let outer = !self.updating;
        self.updating = true;
        let task = self.handle_update(msg);
        if !outer {
            return task;
        }
        self.updating = false;
        let surfaces = if std::mem::take(&mut self.rebuild_pending)
            && self.surface_geometry.as_ref() != Some(&self.surface_geometry())
        {
            self.rebuild_cards()
        } else {
            self.sync_music_shown()
        };
        Task::batch([task, surfaces])
    }
    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: self.config.palette().foreground,
            icon_color: self.config.palette().foreground,
        })
    }
}

impl Cards {
    /// The edge menu's surface.
    fn controls_surface(id: window::Id, margin: i32) -> Task<Message> {
        get_layer_surface(SctkLayerSurfaceSettings {
            id,
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::None,
            anchor: Anchor::RIGHT,
            margin: IcedMargin {
                right: margin,
                ..Default::default()
            },
            size: Some((Some(drawer::SIDEBAR_WIDTH), Some(drawer::SIDEBAR_HEIGHT))),
            exclusive_zone: 0,
            namespace: "desktop-cards-controls".into(),
            ..Default::default()
        })
    }
    /// The thin screen-edge strip that opens the edge menu.
    fn edge_surface(id: window::Id) -> Task<Message> {
        get_layer_surface(SctkLayerSurfaceSettings {
            id,
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::None,
            anchor: Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM,
            size: Some((Some(2), Some(0))),
            exclusive_zone: 0,
            namespace: "desktop-cards-edge".into(),
            ..Default::default()
        })
    }
    /// Every surface again, as at startup: the output probe first (behind
    /// the cards), the edge menu, then the cards and tools.
    fn rebuild_all_surfaces(&mut self) -> Task<Message> {
        let destroy = Task::batch(self.ids.map(
            cosmic::iced::platform_specific::shell::commands::layer_surface::destroy_layer_surface,
        ));
        self.surface_geometry = None;
        self.surface_inputs = None;
        self.forget_tool_surfaces();
        let probe = Self::layout_probe(self.ids[5], self.layout_editor.active);
        let controls = Task::batch([
            Self::controls_surface(self.ids[2], self.sidebar.margin()),
            Self::edge_surface(self.ids[3]),
        ]);
        // Right after the old surfaces go, never deferred: a later rebuild
        // would race this one's destroy.
        let cards = self.rebuild_cards();
        destroy.chain(probe).chain(controls).chain(cards)
    }
    fn handle_update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Woke(deadline, early) => {
                self.resumed(early, Instant::now(), SystemTime::now());
                let task = self.update(Message::Tick);
                // Handling a wake normally moves its deadline. One that came
                // early and is still next (a deadline computed again from the
                // same moment) is armed again: a finished wake with the same
                // identity never fires a second time.
                if self.wake_deadline() == Some(deadline) && deadline > Instant::now() {
                    self.wake_generation += 1;
                }
                task
            }
            Message::Tick => {
                let mut tasks = vec![];
                let today = Local::now().date_naive();
                if today != self.last_today {
                    self.show_earlier = false;
                    if self.calendar_home || self.selected_day == self.last_today {
                        tasks.push(self.calendar_overview());
                    }
                    self.last_today = today;
                    self.next_today_calendar = Instant::now();
                }
                tasks.extend(self.refresh_visible_data());
                tasks.push(self.tools_tick());
                tasks.push(self.automation_tick());
                tasks.push(self.capture_if_changed());
                Task::batch(tasks)
            }
            Message::FilesChanged => {
                let mut tasks = vec![];
                self.reload_note();
                tasks.push(self.reload_calendar_connection());
                match Config::try_load() {
                    Ok(config) => tasks.push(self.apply_config(config)),
                    Err(error) => eprintln!("Keeping last valid widget settings: {error}"),
                }
                if let Some(message) = Self::read_command() {
                    tasks.push(self.update(message));
                }
                tasks.push(self.automation_reload());
                tasks.extend(self.refresh_visible_data());
                tasks.push(self.capture_if_changed());
                Task::batch(tasks)
            }
            Message::DesktopPaletteChanged => {
                if let Err(error) = self.config.refresh_desktop_palette() {
                    eprintln!("Keeping last valid desktop colors: {error}");
                }
                self.capture_if_changed()
            }
            Message::WallpaperColorsChanged => self.wallpaper_colors_changed(),
            Message::WallpaperColorsReady(id, result) => self.wallpaper_colors_ready(id, result),
            Message::WallpaperColorsApplied(result) => {
                if let Err(error) = result {
                    eprintln!("Wallpaper colors: {error}");
                }
                self.update(Message::FilesChanged)
            }
            Message::Metrics(result) => {
                let height_before = self.devbox_natural_height();
                self.metrics_fetching = false;
                match result {
                    Ok((sampler, mut metrics)) => {
                        // Service status can finish while the sample is in flight.
                        metrics.t3 = self.metrics.t3;
                        self.histories.record(
                            &metrics,
                            &self.config.devbox,
                            self.config.widget_visible(Widget::Devbox),
                        );
                        self.sampler = sampler;
                        self.metrics = metrics;
                    }
                    Err(error) => {
                        self.histories.unavailable(
                            &self.config.devbox,
                            self.config.widget_visible(Widget::Devbox),
                        );
                        eprintln!("System sample: {error}");
                    }
                }
                if self.devbox_natural_height() != height_before {
                    self.sync_widget_surfaces()
                } else {
                    Task::none()
                }
            }
            Message::Animate => {
                self.sidebar.tick(Instant::now(), self.config.sidebar_slide);
                set_margin(self.ids[2], 0, self.sidebar.margin(), 0, 0)
            }
            Message::Hover(edge, entered) => {
                self.sidebar.hover(edge, entered, Instant::now());
                Task::none()
            }
            Message::SurfaceResized(id, size) => {
                // libcosmic's layer Size action emits a synthetic square resize
                // using width twice. The output probe is compositor-sized, so
                // only its dimensions are authoritative for the desktop budget.
                if id == self.ids[5] && size.width > 0. && size.height > 0. {
                    let screen = [size.width, size.height];
                    self.desktop_height = (size.height - 96.).max(1.);
                    self.calendar_height = self.desktop_height;
                    if self.screen_size != screen || !self.output_known {
                        self.output_known = true;
                        self.screen_size = screen;
                        return self.sync_widget_surfaces();
                    }
                }
                Task::none()
            }
            Message::SidebarPress(widget) => {
                self.sidebar.drag = Some(ReorderDrag::new(
                    widget,
                    self.config.ordered_right_cards(),
                    self.sidebar.pointer,
                ));
                self.sidebar.show(Instant::now());
                Task::none()
            }
            Message::Pointer(id, event) => {
                if self.layout_editor.active
                    && (id == self.ids[5]
                        || matches!(
                            event,
                            cosmic::iced::mouse::Event::ButtonReleased(
                                cosmic::iced::mouse::Button::Left
                            )
                        ))
                {
                    self.layout_pointer(event)
                } else {
                    self.sidebar_pointer(id, event)
                }
            }
            Message::Arrange => self.arrange(),
            Message::ResetLayout => self.reset_layout(),
            Message::LayoutPress(widget) => {
                self.layout_press(widget);
                Task::none()
            }
            Message::LayoutResizePress(widget) => {
                self.layout_resize_press(widget);
                Task::none()
            }
            Message::LayoutSelect(widget) => {
                self.layout_editor.selected = Some(widget);
                Task::none()
            }
            Message::LayoutResize(direction) => {
                self.layout_resize(direction, false);
                self.commit_arrange();
                Task::none()
            }
            Message::LayoutResizeMode => {
                self.layout_resize_mode();
                Task::none()
            }
            Message::LayoutAutomatic => {
                self.layout_automatic();
                self.commit_arrange();
                Task::none()
            }
            Message::LayoutUndo => {
                self.layout_undo();
                self.commit_arrange();
                Task::none()
            }
            Message::LayoutFinish(save) => self.finish_arrange(save),
            Message::LayoutDefaults => {
                self.layout_defaults();
                self.commit_arrange();
                Task::none()
            }
            Message::LayoutSnap(value) => {
                self.layout_editor.grid.snap = value;
                self.commit_arrange();
                Task::none()
            }
            Message::LayoutKey(key, shift) => self.layout_key(key, shift),
            Message::LayoutCancelDrag => {
                self.cancel_layout_drag();
                Task::none()
            }
            Message::ToggleWidget(widget) => {
                self.set_widget_visible(widget, !self.config.data_visible(widget))
            }
            Message::ShowControls => {
                self.sidebar.show(Instant::now());
                Task::none()
            }
            Message::HideControls => {
                if let Some(drag) = self.sidebar.drag.take() {
                    self.config.widget_order =
                        drag.original_order.iter().map(|w| w.key().into()).collect();
                }
                self.sidebar.hide();
                Task::none()
            }
            Message::FocusWidget(widget) => {
                if !self.config.data_visible(widget) {
                    self.set_widget_visible(widget, true)
                } else {
                    Task::none()
                }
            }
            Message::ToggleProjects => {
                self.all_projects = !self.all_projects;
                self.sync_widget_surfaces()
            }
            Message::ToggleProject(id) => {
                if self
                    .projects
                    .as_ref()
                    .is_some_and(|data| projects::has_group(&data.projects, &id))
                {
                    if !self.expanded_projects.remove(&id) {
                        self.expanded_projects.insert(id);
                    }
                }
                self.sync_widget_surfaces()
            }
            Message::ToggleEarlier => {
                self.show_earlier = !self.show_earlier;
                self.sync_widget_surfaces()
            }
            Message::RefreshProjects => {
                self.next_projects = Instant::now();
                Task::none()
            }
            Message::Projects(roots, result) => {
                self.projects_fetching = false;
                if roots != self.config.project_roots {
                    self.next_projects = Instant::now();
                    return Task::none();
                }
                self.projects_error = result.is_err();
                if let Ok(projects) = result {
                    self.expanded_projects
                        .retain(|id| projects::has_group(&projects.projects, id));
                    self.projects = Some(projects);
                }
                self.next_projects =
                    Instant::now() + self.config.refresh_intervals.duration(Widget::Projects);
                self.sync_widget_surfaces()
            }
            Message::Weather(loc, result) => {
                self.fetching = false;
                if self.config.location.as_ref() != Some(&loc) {
                    return Task::none();
                }
                match result {
                    Ok(snapshot) => {
                        self.weather = Some(snapshot.weather);
                        self.weather_error = false;
                        self.weather_at = Some(snapshot.updated_at);
                        self.next_weather = Instant::now()
                            + self.config.refresh_intervals.duration(Widget::Weather);
                    }
                    Err(e) => {
                        eprintln!("Weather: {e}");
                        self.weather_error = true;
                        self.next_weather = Instant::now() + Duration::from_secs(120);
                    }
                }
                self.sync_widget_surfaces()
            }
            Message::Calendar(revision, month, result) => {
                if revision != self.calendar_connection.revision {
                    return Task::none();
                }
                if self.calendar_preview {
                    return Task::none();
                }
                self.calendar_fetching = false;
                if month == Local::now().format("%Y-%m").to_string() {
                    self.calendar_today_error = result.is_err();
                    if let Ok(calendar) = &result {
                        self.calendar_today = Some(calendar.clone());
                    }
                    self.next_today_calendar =
                        Instant::now() + self.config.refresh_intervals.duration(Widget::Calendar);
                }
                if month != self.month.format("%Y-%m").to_string() {
                    return self.sync_widget_surfaces();
                }
                match result {
                    Ok(calendar) => {
                        self.calendar = Some(calendar);
                        self.calendar_error = false;
                        self.next_calendar = Instant::now()
                            + self.config.refresh_intervals.duration(Widget::Calendar);
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        self.calendar_error = true;
                        self.next_calendar = Instant::now() + Duration::from_secs(120);
                    }
                }
                self.sync_widget_surfaces()
            }
            Message::CalendarToday(revision, result) => {
                if revision != self.calendar_connection.revision {
                    return Task::none();
                }
                if self.calendar_preview {
                    return Task::none();
                }
                self.today_fetching = false;
                if result.as_ref().is_ok_and(|calendar| {
                    calendar.month != Local::now().format("%Y-%m").to_string()
                }) {
                    self.next_today_calendar = Instant::now();
                    return Task::none();
                }
                self.next_today_calendar = Instant::now()
                    + if result.is_ok() {
                        self.config.refresh_intervals.duration(Widget::Calendar)
                    } else {
                        Duration::from_secs(120)
                    };
                self.calendar_today_error = result.is_err();
                if let Ok(calendar) = result {
                    self.calendar_today = Some(calendar);
                }
                self.sync_widget_surfaces()
            }
            Message::PreviewCalendar(calendar) => {
                if std::env::var_os("DESKTOP_CARDS_CAPTURE_DIR").is_none()
                    || std::env::var_os("DESKTOP_CARDS_CONFIG").is_none()
                {
                    return Task::none();
                }
                self.calendar_preview = true;
                self.calendar_today = Some(calendar.clone());
                self.calendar = Some(calendar);
                self.calendar_home = true;
                self.selected_day = Local::now().date_naive();
                self.month = self.selected_day.with_day(1).unwrap();
                self.calendar_fetching = false;
                self.today_fetching = false;
                self.calendar_error = false;
                self.calendar_today_error = false;
                self.next_calendar = Instant::now() + Duration::from_secs(3600);
                self.next_today_calendar = self.next_calendar;
                self.sync_widget_surfaces()
            }
            Message::Week(delta) => {
                let anchor = self
                    .calendar_days(Local::now().date_naive())
                    .first()
                    .copied()
                    .unwrap_or(self.selected_day);
                if let Some(day) = anchor.checked_add_signed(Days::weeks(delta.into())) {
                    return self.select_day(day);
                }
                Task::none()
            }
            Message::Day(day) => self.select_day(day),
            Message::Today => self.calendar_overview(),
            Message::RefreshCalendar => {
                self.next_today_calendar = Instant::now();
                self.next_calendar = Instant::now();
                Task::none()
            }
            Message::Service(active) => {
                self.service_fetching = false;
                self.metrics.t3 = active;
                Task::none()
            }
            Message::Settings => self.launch("desktop-cards", vec!["settings".into()]),
            Message::WeatherSettings => self.launch("desktop-cards", vec!["weather-picker".into()]),
            Message::CalendarSettings => self.launch(
                "desktop-cards",
                vec!["settings".into(), "--page".into(), "calendar".into()],
            ),
            Message::ProjectSettings => self.launch(
                "desktop-cards",
                vec!["settings".into(), "--page".into(), "projects".into()],
            ),
            Message::Open(url) => self.launch("xdg-open", vec![url]),
            Message::ControlFinished(result) => {
                if let Err(e) = result {
                    eprintln!("Desktop Cards: {e}");
                }
                Task::none()
            }
            Message::RequestCapture => cosmic::task::future(async {
                tokio::time::sleep(Duration::from_millis(250)).await;
                Message::CaptureScreens
            }),
            Message::CaptureScreens => {
                Task::batch(self.ids.into_iter().enumerate().map(|(i, id)| {
                    window::screenshot(id)
                        .map(move |s| cosmic::Action::App(Message::Captured(i, s)))
                }))
            }
            Message::Tool(action) => {
                let task = self.update_tool(action);
                Task::batch([task, self.capture_if_changed()])
            }
            Message::MusicFeed(feed) => {
                let task = self.music_feed(feed);
                Task::batch([task, self.capture_if_changed()])
            }
            Message::MusicArt(url, result) => {
                let task = self.music_art(url, result);
                Task::batch([task, self.capture_if_changed()])
            }
            Message::FocusCommand(command) => self.focus_command(&command),
            Message::DisplaysChanged => {
                // A removed display takes its surfaces with it, and a new one
                // needs them: once the burst of display events settles,
                // every surface is built again. The startup burst is skipped.
                let automation = self.automation_displays();
                if self.started.elapsed() < Duration::from_secs(5) {
                    return automation;
                }
                self.display_generation += 1;
                let generation = self.display_generation;
                Task::batch([
                    automation,
                    cosmic::task::future(async move {
                        tokio::time::sleep(Duration::from_millis(1500)).await;
                        Message::RebuildSurfaces(generation)
                    }),
                ])
            }
            Message::RebuildSurfaces(generation) => {
                if generation != self.display_generation {
                    return Task::none();
                }
                self.rebuild_all_surfaces()
            }
            Message::Switched(results) => self.automation_done(results),
            Message::MusicControlDone(result) => match result {
                Ok(players) => {
                    self.tools.music_problem = false;
                    self.music_feed(crate::tools::music::Feed::Players(players))
                }
                Err(error) => {
                    eprintln!("Music: {error}");
                    self.tools.music_problem = true;
                    Task::none()
                }
            },
            Message::ToolDone(result) => {
                if let Err(error) = result {
                    eprintln!("Desktop Cards: {error}");
                }
                Task::none()
            }
            Message::NoteSaved(text, result) => self.note_saved(text, result),
            Message::NoteCopyDone(result) => {
                self.tools.note_problem = result.err();
                Task::none()
            }
            Message::Captured(i, shot) => {
                if let Some(path) = std::env::var_os("DESKTOP_CARDS_CAPTURE_DIR") {
                    let path = std::path::PathBuf::from(path);
                    let stem = format!("{}-{i}", self.config.theme);
                    let _ = std::fs::create_dir_all(&path);
                    let _ = std::fs::write(path.join(format!("{stem}.rgba")), &shot.rgba);
                    let _ = std::fs::write(
                        path.join(format!("{stem}.json")),
                        format!("[{}, {}]", shot.size.width, shot.size.height),
                    );
                }
                Task::none()
            }
        }
    }
}
fn empty_surface<'a>() -> Element<'a, Message> {
    container(cosmic::iced::widget::space())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
fn label<'a>(value: impl Into<String>, size: f32, color: Color) -> Element<'a, Message> {
    text::body(value.into())
        .size(size.max(crate::widget_style::META))
        .line_height(1.25)
        .font(crate::widget_style::font())
        .class(cosmic::theme::Text::Color(crate::widget_style::semantic(
            color,
        )))
        .into()
}
fn timestamp_text(value: Option<&str>, failed: bool, now: chrono::DateTime<Local>) -> String {
    let date = value
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|date| date.with_timezone(&Local));
    let time = date
        .as_ref()
        .map(|date| {
            if date.date_naive() == now.date_naive() {
                date.format("%H:%M").to_string()
            } else if date.date_naive() == now.date_naive() - Days::days(1) {
                format!("Yesterday · {}", date.format("%H:%M"))
            } else if date.year() == now.year() {
                date.format("%-d %b · %H:%M").to_string()
            } else {
                date.format("%-d %b %Y · %H:%M").to_string()
            }
        })
        .unwrap_or_else(|| "—".into());
    if failed {
        format!("Saved · {time}")
    } else {
        time
    }
}
fn timestamp<'a>(
    value: Option<&str>,
    failed: bool,
    _source: &str,
    p: Palette,
) -> Element<'a, Message> {
    label(
        timestamp_text(value, failed, Local::now()),
        9.,
        if failed {
            Color::from_rgb8(233, 192, 124)
        } else {
            p.muted
        },
    )
}

fn forecast_temperature(value: Option<&f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.0}°"))
        .unwrap_or_else(|| "—".into())
}

fn action<'a>(value: impl Into<String>, message: Message, color: Color) -> Element<'a, Message> {
    mouse_area(
        container(label(value, 12., color))
            .padding([2, 8])
            .align_x(Alignment::Center),
    )
    .on_press(message)
    .interaction(cosmic::iced::mouse::Interaction::Pointer)
    .into()
}
fn spread<'a>(left: Element<'a, Message>, right: Element<'a, Message>) -> Element<'a, Message> {
    row::with_children(vec![container(left).width(Length::Fill).into(), right])
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
}
fn card<'a>(
    children: Vec<Element<'a, Message>>,
    p: Palette,
    appearance: Appearance,
    widget: Widget,
    height: f32,
) -> Element<'a, Message> {
    let spacing = match widget {
        Widget::Weather => 6,
        Widget::Devbox => 8,
        _ => 10,
    };
    card_frame(
        scrollable(
            container(
                column::with_children(children)
                    .spacing(spacing)
                    .width(Length::Fill),
            )
            .width(Length::Fill)
            .padding([0, 6, 0, 0]),
        )
        .id(cosmic::widget::Id::new(widget.key()))
        .height(Length::Fill)
        .width(Length::Fill)
        .scroller_width(3.)
        .scrollbar_width(3.)
        .scrollbar_padding(2.)
        .into(),
        p,
        appearance,
        Length::Fixed(height),
    )
}
fn card_frame<'a>(
    content: Element<'a, Message>,
    p: Palette,
    appearance: Appearance,
    height: Length,
) -> Element<'a, Message> {
    // The pinned keyed column diffs card roots directly. Keep their root types
    // identical so switching between an agenda and a scroller reconciles state
    // through Tree::diff instead of reusing a different widget's state.
    let edge = crate::widget_style::edge(p);
    container(
        column::with_children(vec![content])
            .width(Length::Fill)
            .height(if height == Length::Shrink {
                Length::Shrink
            } else {
                Length::Fill
            }),
    )
    .height(height)
    .padding(crate::widget_style::PADDING)
    .width(Length::Fill)
    .class(cosmic::theme::Container::custom(move |_| {
        cosmic::iced::widget::container::Style {
            text_color: Some(p.foreground),
            icon_color: Some(p.foreground),
            background: Some(Background::Color(p.background)),
            border: Border {
                color: Color {
                    a: if p.background.a == 0. {
                        0.
                    } else {
                        f32::from(appearance.border_opacity.min(50)) / 100.
                    },
                    ..p.accent
                },
                width: edge.0,
                radius: f32::from(appearance.corner_radius.min(32)).into(),
            },
            shadow: edge.1,
            ..Default::default()
        }
    }))
    .into()
}
fn mix(a: Color, b: Color, amount: f32) -> Color {
    crate::widget_style::tint(a, b, amount)
}
fn weather_dew_point_degrees(config: &Config, current: &data::Current) -> Option<String> {
    let value = current
        .dew_point_2m
        .filter(|value| config.weather.show_dew_point && value.is_finite())?;
    let rounded = value.round();
    // Avoid a misleading "-0" when the reading is just below freezing.
    let rounded = if rounded == 0. { 0. } else { rounded };
    Some(format!("{rounded:.0}°"))
}
fn weather_dew_point(config: &Config, current: &data::Current) -> Option<String> {
    weather_dew_point_degrees(config, current).map(|degrees| format!("Dew point {degrees}C"))
}

/// Shared, read-only rendering for desktop weather and its Settings preview.
fn weather_view(
    config: &Config,
    weather: Option<&Weather>,
    _updated_at: Option<&str>,
    failed: bool,
    height: f32,
) -> Element<'static, Message> {
    let _style = crate::widget_style::scope(config);
    let p = config.palette();
    let mut rows = vec![label(
        config
            .location
            .as_ref()
            .map(|l| l.name.as_str())
            .unwrap_or("Weather"),
        crate::widget_style::TITLE,
        p.foreground,
    )];
    match (&config.location, weather) {
        (_, Some(w)) => {
            let wind = format!("Wind\u{a0}{:.0}\u{a0}km/h", w.current.wind_speed_10m);
            let details = weather_dew_point(config, &w.current)
                .map(|dew| format!("{dew} · {wind}"))
                .unwrap_or(wind);
            rows.push(
                row::with_children(vec![
                    label(
                        data::weather_icon(w.current.weather_code, w.current.is_day != 0),
                        32.,
                        p.accent,
                    ),
                    label(
                        format!("{:.0}°", w.current.temperature_2m),
                        44.,
                        p.foreground,
                    ),
                    container(
                        column::with_children(vec![
                            label(
                                data::weather_description(w.current.weather_code),
                                12.,
                                p.foreground,
                            ),
                            label(
                                format!(
                                    "Feels {:.0}°C · Humidity {:.0}%",
                                    w.current.apparent_temperature, w.current.relative_humidity_2m
                                ),
                                10.,
                                p.muted,
                            ),
                            container(label(details, 10., p.muted))
                                .id(cosmic::widget::Id::new("weather-details"))
                                .into(),
                        ])
                        .spacing(2),
                    )
                    .width(Length::Fill)
                    .into(),
                ])
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            );
            let forecast: Vec<Element<'_, Message>> = w
                .daily
                .time
                .iter()
                .enumerate()
                .take(3)
                .map(|(i, day)| {
                    let today = Local::now().date_naive();
                    let title = NaiveDate::parse_from_str(day, "%Y-%m-%d")
                        .map(|date| {
                            if date == today {
                                "Today".into()
                            } else if date == today + Days::days(1) {
                                "Tomorrow".into()
                            } else {
                                date.format("%a %-d").to_string()
                            }
                        })
                        .unwrap_or_default();
                    let high = forecast_temperature(w.daily.temperature_2m_max.get(i));
                    let low = forecast_temperature(w.daily.temperature_2m_min.get(i));
                    let rain = w
                        .daily
                        .precipitation_probability_max
                        .get(i)
                        .and_then(|v| *v)
                        .map(|n| format!("Rain {n:.0}%"))
                        .unwrap_or_else(|| "Rain —".into());
                    container(
                        column::with_children(vec![
                            spread(
                                label(title, 10., p.muted),
                                label(
                                    data::weather_icon(
                                        *w.daily.weather_code.get(i).unwrap_or(&3),
                                        true,
                                    ),
                                    16.,
                                    p.accent,
                                ),
                            ),
                            label(format!("{high} / {low}"), 13., p.foreground),
                            label(rain, 9., p.muted),
                        ])
                        .spacing(2),
                    )
                    .width(Length::Fill)
                    .into()
                })
                .collect();
            rows.push(row::with_children(forecast).spacing(12).into());
        }
        (Some(_), None) => rows.push(label(
            if failed {
                "Weather unavailable — retrying soon"
            } else {
                "Checking the skies…"
            },
            14.,
            p.muted,
        )),
        (None, None) => rows.push(action(
            "Choose your city →",
            Message::WeatherSettings,
            p.accent,
        )),
    }
    card(rows, p, config.appearance, Widget::Weather, height)
}

fn clock_labels(now: chrono::NaiveTime, format_24h: bool) -> (String, Option<String>) {
    if format_24h {
        (now.format("%H:%M").to_string(), None)
    } else {
        (
            now.format("%-I:%M").to_string(),
            Some(now.format("%p").to_string()),
        )
    }
}
fn week_start(day: NaiveDate) -> NaiveDate {
    day - Days::days(day.weekday().num_days_from_monday().into())
}
fn week_label(day: NaiveDate) -> String {
    let start = week_start(day);
    let end = start + Days::days(6);
    if start.year() != end.year() {
        format!(
            "{} – {}",
            start.format("%-d %b %Y"),
            end.format("%-d %b %Y")
        )
    } else if start.month() != end.month() {
        format!("{} – {}", start.format("%-d %b"), end.format("%-d %b %Y"))
    } else {
        format!("{}–{}", start.day(), end.format("%-d %b %Y"))
    }
}
fn bar<'a>(segments: &[(f32, Color)], p: Palette) -> Element<'a, Message> {
    let mut used = 0u16;
    let mut cells = vec![];
    for (fraction, color) in segments {
        let portion = ((fraction.clamp(0., 1.) * 1000.).round() as u16).min(1000 - used);
        if portion == 0 {
            continue;
        }
        used += portion;
        cells.push(bar_piece(*color, portion));
    }
    if used < 1000 {
        cells.push(bar_piece(mix(p.accent, p.background, 0.09), 1000 - used));
    }
    row::with_children(cells).height(Length::Fixed(5.)).into()
}
fn bar_piece<'a>(color: Color, portion: u16) -> Element<'a, Message> {
    container(text::body(""))
        .height(Length::Fixed(5.))
        .width(Length::FillPortion(portion))
        .class(cosmic::theme::Container::custom(move |_| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 2.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}
impl Cards {
    fn select_day(&mut self, day: NaiveDate) -> Task<Message> {
        let scroll = self.show_day(day);
        Task::batch([scroll, self.sync_widget_surfaces()])
    }
    /// Shows `day` alone; the caller syncs the surfaces.
    fn show_day(&mut self, day: NaiveDate) -> Task<Message> {
        self.calendar_home = false;
        let month = day.with_day(1).unwrap();
        if self.month != month {
            self.month = month;
            self.calendar = CalendarData::cached(&month.format("%Y-%m").to_string());
            self.next_calendar = Instant::now();
            self.calendar_error = false;
        }
        self.selected_day = day;
        // Each day starts at its first event when navigating, even if the old list was scrolled.
        let scroll = Task::batch(["calendar"].map(|id| {
            cosmic::iced::widget::scrollable::snap_to(
                cosmic::widget::Id::new(id),
                cosmic::iced::widget::scrollable::RelativeOffset {
                    x: None,
                    y: Some(0.),
                },
            )
        }));
        scroll
    }
    fn calendar_overview(&mut self) -> Task<Message> {
        // One surface sync for the overview's final height, not one for the
        // single day and another for the overview.
        let scroll = self.show_day(Local::now().date_naive());
        self.calendar_home = true;
        Task::batch([scroll, self.sync_widget_surfaces()])
    }
    fn calendar_days(&self, today: NaiveDate) -> Vec<NaiveDate> {
        if !self.calendar_home {
            return vec![self.selected_day];
        }
        let mut days = Vec::new();
        if self.config.calendar_show_today {
            days.push(today);
        }
        if self.config.calendar_show_tomorrow {
            days.push(today + Days::days(1));
        }
        days
    }
    fn launch(&self, command: &str, args: Vec<String>) -> Task<Message> {
        let command = if command == "desktop-cards" {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".local/bin/desktop-cards")
        } else {
            std::path::PathBuf::from("/usr/bin/xdg-open")
        };
        cosmic::task::future(async move {
            let result = tokio::task::spawn_blocking(move || {
                let status = std::process::Command::new(command)
                    .args(args)
                    .status()
                    .map_err(|e| e.to_string())?;
                if status.success() {
                    Ok(())
                } else {
                    Err(format!("Command exited with {status}"))
                }
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
            Message::ControlFinished(result)
        })
    }
    fn weather_task(&mut self) -> Task<Message> {
        let Some(loc) = self.config.location.clone() else {
            return Task::none();
        };
        self.fetching = true;
        cosmic::task::future(async move {
            let result = data::fetch_weather(&loc).await;
            Message::Weather(loc, result)
        })
    }
    fn reload_calendar_connection(&mut self) -> Task<Message> {
        if self.calendar_preview {
            return Task::none();
        }
        let connection = crate::calendar::load_connection();
        if connection == self.calendar_connection {
            return Task::none();
        }
        self.calendar_connection = connection;
        self.calendar = None;
        self.calendar_today = None;
        self.calendar_error = false;
        self.calendar_today_error = false;
        self.calendar_fetching = false;
        self.today_fetching = false;
        self.show_earlier = false;
        self.next_calendar = Instant::now();
        self.next_today_calendar = Instant::now();
        self.sync_widget_surfaces()
    }
    fn calendar_task(&mut self) -> Task<Message> {
        if !self.calendar_connection.connected() {
            return Task::none();
        }
        let connection = self.calendar_connection.clone();
        let month = self.month.format("%Y-%m").to_string();
        self.calendar_fetching = true;
        cosmic::task::future(async move {
            let revision = connection.revision.clone();
            let result = crate::calendar::fetch(connection, month.clone()).await;
            Message::Calendar(revision, month, result)
        })
    }
    fn clock(&self) -> Element<'_, Message> {
        clock::view(
            &self.config,
            Local::now().naive_local(),
            self.widget_size(Widget::Clock),
        )
    }
    fn weather_card(&self) -> Element<'_, Message> {
        weather_view(
            &self.config,
            self.weather.as_ref(),
            self.weather_at.as_deref(),
            self.weather_error,
            self.card_height(Widget::Weather),
        )
    }
    #[cfg(test)]
    fn agenda_events(&self, day: NaiveDate) -> Vec<&CalendarEvent> {
        self.calendar_agenda(day).events
    }
    fn calendar_agenda(&self, day: NaiveDate) -> data::CalendarAgenda<'_> {
        self.calendar
            .as_ref()
            .map(|calendar| calendar.agenda_at(day, Local::now(), self.show_earlier))
            .unwrap_or_default()
    }
    fn earlier_control(&self, count: usize, p: Palette) -> Element<'_, Message> {
        container(action(
            if self.show_earlier {
                "Hide earlier ↑".into()
            } else {
                format!("Earlier · {count} ↓")
            },
            Message::ToggleEarlier,
            p.muted,
        ))
        .id(cosmic::widget::Id::new("calendar-earlier"))
        .height(22.)
        .align_y(Alignment::Center)
        .into()
    }
    /// The same toggle, sized to sit beside a day heading.
    fn earlier_inline(&self, count: usize, p: Palette) -> Element<'_, Message> {
        container(
            mouse_area(label(
                if self.show_earlier {
                    "· Hide earlier ↑".into()
                } else {
                    format!("· Earlier {count} ↓")
                },
                10.,
                p.muted,
            ))
            .on_press(Message::ToggleEarlier)
            .interaction(cosmic::iced::mouse::Interaction::Pointer),
        )
        .id(cosmic::widget::Id::new("calendar-earlier"))
        .into()
    }
    fn calendar_event_height(&self, event: &CalendarEvent, width: f32) -> f32 {
        let title = glance::agenda_text_height(event, 15., (width - 115.).max(20.));
        let status = matches!(
            event.meeting_state_at(Local::now(), self.config.meeting_notice_minutes),
            MeetingState::Soon(_) | MeetingState::Live
        );
        (title + if status { 15. } else { 0. }).max(30.) + 12.
    }
    fn calendar_natural_height(&self) -> f32 {
        let now = Local::now();
        let days = self.calendar_days(now.date_naive());
        let width = self
            .explicit_size(Widget::Calendar)
            .map(|size| size[0])
            .unwrap_or_else(|| (self.screen_size[0] - 48.).min(360.));
        let mut rows = Vec::new();
        if days.is_empty() {
            rows.push(18.);
        }
        for day in &days {
            if days.len() > 1 {
                rows.push(26.);
            }
            let agenda = self.calendar_agenda(*day);
            let events = agenda.events;
            if self.calendar.is_none() {
                rows.push(36.);
            } else if events.is_empty() {
                rows.push(38.);
            } else {
                for event in events {
                    rows.push(self.calendar_event_height(event, width));
                }
            }
            if agenda.earlier > 0 && days.len() == 1 {
                rows.push(22.);
            }
        }
        (176. + rows.iter().sum::<f32>() + rows.len().saturating_sub(1) as f32 * 3.).ceil()
    }
    fn calendar_card(&self) -> Element<'_, Message> {
        let _style = crate::widget_style::scope(&self.config);
        let p = self.config.palette();
        let now = Local::now();
        let today = now.date_naive();
        let days = self.calendar_days(today);
        let display_day = days.first().copied().unwrap_or(self.selected_day);
        let heading = if self.calendar_home && days.len() != 1 {
            "Calendar".into()
        } else if self.calendar_home && days.first() == Some(&(today + Days::days(1))) {
            "Tomorrow".into()
        } else if self.selected_day == today {
            "Today".into()
        } else {
            self.selected_day.format("%A").to_string()
        };
        let mut rows = vec![spread(
            label(heading, 23., p.foreground),
            label(display_day.format("%a, %-d %b").to_string(), 11., p.muted),
        )];
        rows.push(spread(
            label(week_label(display_day), 11., p.muted),
            row::with_children(vec![
                action(
                    if self.config.calendar_show_today && !self.config.calendar_show_tomorrow {
                        "Today"
                    } else {
                        "Overview"
                    },
                    Message::Today,
                    p.accent,
                ),
                action("↑", Message::Week(-1), p.accent),
                action("↓", Message::Week(1), p.accent),
            ])
            .spacing(6)
            .into(),
        ));
        let start = week_start(display_day);
        rows.push(
            row::with_children((0..7).map(|offset| {
                let day = start + Days::days(offset);
                let selected = if self.calendar_home {
                    days.contains(&day)
                } else {
                    day == self.selected_day
                };
                let color = if selected {
                    p.foreground
                } else if day == today {
                    p.accent
                } else {
                    p.muted
                };
                let cell = container(
                    column::with_children(vec![
                        label(day.format("%a").to_string(), 9., color),
                        label(day.day().to_string(), 13., color),
                    ])
                    .spacing(2)
                    .align_x(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fixed(36.))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .class(cosmic::theme::Container::custom(move |_| {
                    cosmic::iced::widget::container::Style {
                        background: selected.then_some(Background::Color(mix(
                            p.muted,
                            p.background,
                            0.16,
                        ))),
                        border: Border {
                            color: if day == today {
                                mix(p.muted, p.background, 0.48)
                            } else if selected {
                                mix(p.muted, p.background, 0.24)
                            } else {
                                Color::TRANSPARENT
                            },
                            width: 1.,
                            radius: 10.into(),
                        },
                        ..Default::default()
                    }
                }));
                mouse_area(cell)
                    .on_press(Message::Day(day))
                    .interaction(cosmic::iced::mouse::Interaction::Pointer)
                    .into()
            }))
            .spacing(3)
            .into(),
        );
        let header = container(column::with_children(rows).spacing(8).width(Length::Fill))
            .height(Length::Fixed(100.));
        let mut agenda = Vec::new();
        if days.is_empty() {
            agenda.push(label("Choose a date above", 14., p.muted));
        }
        for day in &days {
            if days.len() > 1 {
                agenda.push(
                    container(spread(
                        {
                            let name = label(
                                if *day == today { "TODAY" } else { "TOMORROW" },
                                11.,
                                p.accent,
                            );
                            let earlier = self.calendar_agenda(*day).earlier;
                            if earlier > 0 {
                                row::with_children(vec![name, self.earlier_inline(earlier, p)])
                                    .spacing(6)
                                    .align_y(Alignment::Center)
                                    .into()
                            } else {
                                name
                            }
                        },
                        label(day.format("%a, %-d %b").to_string(), 10., p.muted),
                    ))
                    .padding([8, 2, 4, 2])
                    .into(),
                );
            }
            let day_agenda = self.calendar_agenda(*day);
            let events = day_agenda.events;
            if self.calendar.is_none() {
                agenda.push(if !self.calendar_connection.connected() {
                    action(
                        "Connect your calendar →",
                        Message::CalendarSettings,
                        p.accent,
                    )
                } else if self.calendar_error {
                    action(
                        "Check calendar connection →",
                        Message::CalendarSettings,
                        p.accent,
                    )
                } else {
                    label("Loading your events…", 14., p.muted)
                });
            } else if events.is_empty() {
                agenda.push(
                    container(label(
                        if day_agenda.earlier > 0 {
                            "All done for today"
                        } else {
                            "No events"
                        },
                        14.,
                        p.muted,
                    ))
                    .padding([10, 6])
                    .into(),
                );
            } else {
                for event in events {
                    let state = event.meeting_state_at(now, self.config.meeting_notice_minutes);
                    let highlighted = matches!(state, MeetingState::Soon(_) | MeetingState::Live);
                    let color = if matches!(state, MeetingState::Soon(_)) {
                        Color::from_rgb8(245, 190, 107)
                    } else {
                        p.accent
                    };
                    let status = match state {
                        MeetingState::Soon(minutes) => format!("IN {minutes} MIN"),
                        MeetingState::Live => "LIVE NOW".into(),
                        _ => String::new(),
                    };
                    let end = if event.all_day {
                        String::new()
                    } else {
                        chrono::DateTime::parse_from_rfc3339(&event.end)
                            .map(|d| d.with_timezone(&Local).format("%H:%M").to_string())
                            .unwrap_or_default()
                    };
                    let timing = container(
                        column::with_children(vec![
                            label(event.time_label(), 12., color),
                            label(end, 10., p.muted),
                        ])
                        .spacing(2),
                    )
                    .width(Length::Fixed(43.));
                    let mut detail: Vec<Element<'_, Message>> = vec![
                        cosmic::iced::widget::rich_text(glance::agenda_spans(
                            event,
                            if event.finished() {
                                p.muted
                            } else {
                                p.foreground
                            },
                            color,
                        ))
                        .size(15.)
                        .line_height(1.25)
                        .font(crate::widget_style::font())
                        .wrapping(cosmic::iced::advanced::text::Wrapping::WordOrGlyph)
                        .width(Length::Fill)
                        .on_link_click(Message::Open)
                        .into(),
                    ];
                    if !status.is_empty() {
                        detail.push(label(status, 9., color));
                    }
                    let marker = container(text::body(""))
                        .width(Length::Fixed(2.))
                        .height(Length::Fixed(26.))
                        .class(cosmic::theme::Container::custom(move |_| {
                            cosmic::iced::widget::container::Style {
                                background: Some(Background::Color(Color {
                                    a: if highlighted { 0.9 } else { 0.30 },
                                    ..color
                                })),
                                border: Border {
                                    radius: 1.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        }));
                    let entry = vec![
                        timing.into(),
                        marker.into(),
                        container(column::with_children(detail).spacing(3))
                            .width(Length::Fill)
                            .into(),
                    ];
                    agenda.push(
                        container(
                            row::with_children(entry)
                                .spacing(8)
                                .align_y(Alignment::Center),
                        )
                        .padding([6, 6])
                        .width(Length::Fill)
                        .class(cosmic::theme::Container::custom(move |_| {
                            cosmic::iced::widget::container::Style {
                                background: Some(Background::Color(mix(
                                    color,
                                    p.background,
                                    if highlighted { 0.10 } else { 0.025 },
                                ))),
                                border: Border {
                                    color: Color {
                                        a: if highlighted { 0.65 } else { 0.0 },
                                        ..color
                                    },
                                    width: 1.,
                                    radius: 10.into(),
                                },
                                ..Default::default()
                            }
                        }))
                        .into(),
                    );
                }
            }
            // With day headings, the toggle sits beside the day name.
            if day_agenda.earlier > 0 && days.len() == 1 {
                agenda.push(self.earlier_control(day_agenda.earlier, p));
            }
        }
        let account = self
            .calendar
            .as_ref()
            .map(|calendar| calendar.account.as_str())
            .filter(|account| !account.is_empty());
        let mut calendar_url =
            reqwest::Url::parse("https://calendar.google.com/calendar/u/0/r").unwrap();
        if let Some(account) = account {
            calendar_url
                .query_pairs_mut()
                .append_pair("authuser", account);
        }
        let footer = container(spread(
            mouse_area(
                text::body(if self.calendar_error {
                    "Check connection →".into()
                } else {
                    format!("{} ↗", account.unwrap_or("Google Calendar"))
                })
                .size(9.)
                .font(crate::widget_style::font())
                .class(cosmic::theme::Text::Color(p.muted))
                .width(Length::Fill)
                .wrapping(cosmic::iced::advanced::text::Wrapping::None)
                .ellipsize(cosmic::iced::advanced::text::Ellipsize::End(
                    cosmic::iced::advanced::text::EllipsizeHeightLimit::Lines(1),
                )),
            )
            .on_press(if self.calendar_error {
                Message::CalendarSettings
            } else {
                Message::Open(calendar_url.into())
            })
            .interaction(cosmic::iced::mouse::Interaction::Pointer)
            .into(),
            row::with_children(vec![
                timestamp(
                    self.calendar
                        .as_ref()
                        .map(|calendar| calendar.updated_at.as_str()),
                    self.calendar_error,
                    "Google Calendar",
                    p,
                ),
                action(
                    if self.calendar_fetching { "◌" } else { "↻" },
                    Message::RefreshCalendar,
                    p.muted,
                ),
            ])
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
        ))
        .height(Length::Fixed(20.));
        let agenda = column::with_children(agenda).spacing(3).width(Length::Fill);
        let explicit = self.explicit_size(Widget::Calendar).is_some();
        let limit = self.card_height(Widget::Calendar);
        // 100px header + 20px footer + two 10px gaps + 36px frame padding.
        // Shrink to the actual rendered rows, including wrapped titles. Only
        // the agenda is capped, so busy days keep the date and sync controls.
        const CHROME_HEIGHT: f32 = 176.;
        if limit < CHROME_HEIGHT + 44. {
            // On very short displays there isn't room for fixed controls and
            // an event; keep everything reachable in one compact scroller.
            return card(
                vec![header.into(), agenda.into(), footer.into()],
                p,
                self.config.appearance,
                Widget::Calendar,
                limit,
            );
        }
        let agenda = container(
            scrollable(container(agenda).padding([0, 6, 0, 0]).width(Length::Fill))
                .id(cosmic::widget::Id::new(Widget::Calendar.key()))
                .height(if explicit {
                    Length::Fill
                } else {
                    Length::Shrink
                })
                .width(Length::Fill)
                .scroller_width(2.)
                .scrollbar_width(2.)
                .scrollbar_padding(2.),
        )
        .max_height(limit - CHROME_HEIGHT)
        .height(if explicit {
            Length::Fill
        } else {
            Length::Shrink
        })
        .width(Length::Fill);
        card_frame(
            column::with_children(vec![header.into(), agenda.into(), footer.into()])
                .spacing(10)
                .height(if explicit {
                    Length::Fill
                } else {
                    Length::Shrink
                })
                .width(Length::Fill)
                .into(),
            p,
            self.config.appearance,
            if explicit {
                Length::Fixed(limit)
            } else {
                Length::Shrink
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_surfaces_do_not_blur_transparent_clock_or_rounded_corners() {
        use cosmic::Application;
        let (cards, _) = Cards::init(Core::default(), Config::default());
        assert!(cards.core.auto_blur().is_empty());
    }

    #[test]
    fn output_probe_owns_desktop_budget_despite_synthetic_widget_resizes() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(Core::default(), Config::default());
        let _ = cards.update(Message::SurfaceResized(
            cards.ids[5],
            cosmic::iced::Size::new(1920., 1080.),
        ));
        for id in [cards.ids[1], cards.ids[4], cards.ids[6], cards.ids[7]] {
            let _ = cards.update(Message::SurfaceResized(
                id,
                cosmic::iced::Size::new(360., 360.),
            ));
        }
        assert_eq!(cards.desktop_height, 984.);
        assert_eq!(cards.calendar_height, 984.);
        let _ = cards.update(Message::SurfaceResized(
            cards.ids[5],
            cosmic::iced::Size::new(1280., 720.),
        ));
        assert_eq!(cards.screen_size, [1280., 720.]);
        assert_eq!(cards.desktop_height, 624.);
        assert_eq!(cards.calendar_height, 624.);
    }
    #[test]
    fn clock_formats_midnight_noon_and_evening_without_leading_spaces() {
        let midnight = chrono::NaiveTime::from_hms_opt(0, 5, 0).unwrap();
        let noon = chrono::NaiveTime::from_hms_opt(12, 5, 0).unwrap();
        let evening = chrono::NaiveTime::from_hms_opt(21, 7, 0).unwrap();
        assert_eq!(
            clock_labels(midnight, false),
            ("12:05".into(), Some("AM".into()))
        );
        assert_eq!(
            clock_labels(noon, false),
            ("12:05".into(), Some("PM".into()))
        );
        assert_eq!(
            clock_labels(evening, false),
            ("9:07".into(), Some("PM".into()))
        );
        assert_eq!(clock_labels(midnight, true), ("00:05".into(), None));
        assert_eq!(clock_labels(evening, true), ("21:07".into(), None));
    }
    #[test]
    fn appearance_and_clock_preferences_redraw_without_scheduling_data_refreshes() {
        use cosmic::Application;
        use cosmic::iced::advanced::widget::Tree;
        let (mut cards, _) = Cards::init(
            Core::default(),
            Config {
                enabled_widgets: vec!["clock".into()],
                ..Config::default()
            },
        );
        cards.next_clock = Instant::now() + Duration::from_secs(60);
        let deadline = cards.next_wake();
        let mut clock = Tree::new(cards.clock().as_widget());
        let mut devbox = Tree::new(cards.devbox().as_widget());
        let mut calendar = Tree::new(cards.calendar_card().as_widget());
        for (radius, border, format_24h, show_date) in [
            (0, 0, false, false),
            (32, 50, true, true),
            (16, 20, false, true),
        ] {
            let mut config = cards.config.clone();
            config.appearance.corner_radius = radius;
            config.appearance.border_opacity = border;
            config.clock.format_24h = format_24h;
            config.clock.show_date = show_date;
            let _ = cards.apply_config(config);
            clock.diff(cards.clock().as_widget_mut());
            devbox.diff(cards.devbox().as_widget_mut());
            calendar.diff(cards.calendar_card().as_widget_mut());
            assert_eq!(cards.next_wake(), deadline);
            assert!(cards.refresh_visible_data().is_empty());
        }
    }
    #[test]
    fn independent_deadlines_pause_hidden_widgets_and_do_not_duplicate_samples() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(
            Core::default(),
            Config {
                enabled_widgets: vec![],
                ..Config::default()
            },
        );
        assert!(cards.next_wake().is_none());
        let mut config = cards.config.clone();
        config.enabled_widgets = vec!["devbox".into()];
        config.devbox.services = true;
        config.refresh_intervals.devbox = 10;
        let _ = cards.apply_config(config);
        let start = Instant::now();
        assert_eq!(cards.refresh_visible_data().len(), 2);
        assert!(cards.metrics_fetching && cards.service_fetching);
        assert!(cards.next_wake().is_none());
        assert!(cards.refresh_visible_data().is_empty());
        assert!(cards.next_metrics >= start + Duration::from_secs(10));
        let _ = cards.update(Message::Service(true));
        let _ = cards.update(Message::Metrics(Ok((
            Sampler::default(),
            Metrics::default(),
        ))));
        assert!(cards.metrics.t3);
        assert!(cards.refresh_visible_data().is_empty());
        let mut config = cards.config.clone();
        config.refresh_intervals.devbox = 1;
        let _ = cards.apply_config(config);
        assert_eq!(cards.refresh_visible_data().len(), 2);
        cards.config.enabled_widgets.clear();
        assert!(cards.next_wake().is_none());
        assert!(cards.refresh_visible_data().is_empty());
    }
    /// After a suspend the wake comes by the wall clock while the monotonic
    /// deadlines are still ahead. It must be handled (the clock moves on,
    /// automation looks again, displays included) and a wake must stay
    /// armed: iced keeps a finished wake under its identity and never runs
    /// it again, so an unhandled early wake used to stop every timer.
    #[test]
    fn a_wake_after_suspend_is_handled_and_the_next_wake_stays_armed() {
        use crate::automation::{self, Change, DisplayWhen, ProfileRef, Rule, Trigger};
        use crate::desktop::store::tests::Fixture;
        use cosmic::Application;
        let fixture = Fixture::new("app-resume");
        std::fs::write(fixture.0.join("outputs"), "eDP-1\n").unwrap();
        automation::update(&fixture.store(), |settings| {
            settings.enabled = true;
            settings.rules.push(Rule {
                id: "r1".into(),
                enabled: true,
                since: 0,
                profile: ProfileRef::Preset("presentation".into()),
                name: "Presentation".into(),
                when: Trigger::Display(DisplayWhen {
                    change: Change::Connect,
                    display: None,
                    label: None,
                }),
            });
            Ok(())
        })
        .unwrap();
        let (mut cards, _) = Cards::init(
            Core::default(),
            Config {
                enabled_widgets: vec!["clock".into()],
                ..Config::default()
            },
        );
        cards.automator = automation::runtime::Automator::with_store(Some(fixture.store()));
        let _ = cards.update(Message::Tick);
        // Before the suspend: the clock and automation's next look are ahead.
        let start = Instant::now();
        cards.next_clock = start + Duration::from_secs(40);
        let looked = cards.automation_wake().unwrap();
        assert!(looked > start);
        let armed = cards.wake_deadline().unwrap();
        // Nine hours asleep: the wall clock moved on, the monotonic one
        // didn't. A dock was plugged in meanwhile.
        cards.woke_at = (start, SystemTime::now() - Duration::from_secs(9 * 3600));
        std::fs::write(fixture.0.join("outputs"), "DP-2\neDP-1\n").unwrap();
        let woke = Instant::now();
        let early = armed.saturating_duration_since(woke);
        assert!(!early.is_zero());
        let _ = cards.update(Message::Woke(armed, early));
        // Handled: the clock went on to its next minute, and automation
        // read the displays again (the new set now settles from here).
        assert!(cards.next_clock > woke);
        let next = cards.automation_wake().unwrap();
        assert!(next >= woke + crate::automation::runtime::SETTLE);
        assert_ne!(cards.wake_deadline(), Some(armed));
        // An early wake with nothing due that leaves the same deadline next
        // (the settle time is counted from a fixed moment) is armed again.
        cards.next_clock = Instant::now() + Duration::from_secs(50);
        let armed = cards.wake_deadline().unwrap();
        assert_eq!(Some(armed), cards.automation_wake());
        let generation = cards.wake_generation;
        let early = armed.saturating_duration_since(Instant::now());
        let _ = cards.update(Message::Woke(armed, early));
        assert_eq!(cards.wake_deadline(), Some(armed));
        assert_eq!(cards.wake_generation, generation + 1);
        // A wake that arrives on time doesn't change the identity.
        cards.next_clock = Instant::now();
        let _ = cards.update(Message::Woke(cards.next_clock, Duration::ZERO));
        assert_eq!(cards.wake_generation, generation + 1);
    }
    #[test]
    fn timer_does_not_reload_preferences_or_revert_the_in_memory_selection() {
        use cosmic::Application;
        let config = Config {
            enabled_widgets: vec![],
            theme: "teal".into(),
            ..Config::default()
        };
        let (mut cards, _) = Cards::init(Core::default(), config);
        let _ = cards.update(Message::Tick);
        assert!(cards.config.enabled_widgets.is_empty());
        assert_eq!(cards.config.theme, "teal");
    }
    #[test]
    fn old_project_results_do_not_replace_a_new_folder_selection() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(
            Core::default(),
            Config {
                enabled_widgets: vec![],
                ..Config::default()
            },
        );
        cards.projects = Some(ProjectsData::default());
        cards.projects_error = true;
        cards.all_projects = true;
        let mut config = cards.config.clone();
        config.project_roots = vec!["/new/project".into()];
        let _ = cards.apply_config(config);
        assert!(cards.projects.is_none());
        assert!(!cards.projects_error && !cards.all_projects);
        let _ = cards.update(Message::Projects(
            vec!["/old/project".into()],
            Ok(ProjectsData::default()),
        ));
        assert!(cards.projects.is_none());
        assert!(cards.next_projects <= Instant::now());
    }
    #[test]
    fn disabled_widgets_do_not_start_jobs_and_reenable_resumes_due_work() {
        use cosmic::Application;
        let config = Config {
            enabled_widgets: Vec::new(),
            location: Some(Location {
                name: "Test".into(),
                latitude: 0.,
                longitude: 0.,
            }),
            ..Config::default()
        };
        let (mut cards, _) = Cards::init(Core::default(), config);
        cards.calendar_connection = crate::calendar::Connection {
            backend: crate::calendar::Backend::Google,
            account: "sample@example.com".into(),
            revision: "test".into(),
            ..Default::default()
        };
        cards.config.project_roots = vec!["/fixture/projects".into()];
        assert!(!cards.fetching && !cards.calendar_fetching && !cards.projects_fetching);
        assert!(cards.refresh_visible_data().is_empty());
        cards.config.enabled_widgets = vec!["weather".into(), "calendar".into(), "projects".into()];
        // Refresh may also reconcile calendar geometry; the fetching flags below
        // track the three data jobs independently of those local UI tasks.
        assert!(!cards.refresh_visible_data().is_empty());
        assert!(cards.fetching && cards.calendar_fetching && cards.projects_fetching);
        // A second refresh must not queue duplicate jobs while these are in flight.
        assert!(cards.refresh_visible_data().is_empty());
    }
    #[test]
    fn fresh_calendar_and_projects_wait_for_setup_and_old_account_results_are_ignored() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(
            Core::default(),
            Config {
                enabled_widgets: vec!["calendar".into(), "projects".into()],
                ..Default::default()
            },
        );
        // `init` reads the machine's own calendar account; a test machine with
        // Calendar connected has already started a sync. Start from a fresh,
        // unconnected state whatever this machine has.
        cards.calendar_connection = crate::calendar::Connection::default();
        cards.calendar = None;
        cards.calendar_today = None;
        cards.calendar_fetching = false;
        cards.today_fetching = false;
        cards.projects_fetching = false;
        let _ = cards.refresh_visible_data();
        assert!(!cards.calendar_fetching && !cards.projects_fetching);
        // A visible calendar still updates its date/highlights locally. Only
        // remote sync and project scanning must wait for setup.
        assert_eq!(cards.next_wake(), Some(cards.next_calendar_view));
        assert!(cards.next_calendar_view > Instant::now());
        assert!(cards.refresh_visible_data().is_empty());
        let data = CalendarData {
            account: "former@example.com".into(),
            ..Default::default()
        };
        let _ = cards.update(Message::Calendar(
            "former-revision".into(),
            cards.selected_day.format("%Y-%m").to_string(),
            Ok(data.clone()),
        ));
        let _ = cards.update(Message::CalendarToday("former-revision".into(), Ok(data)));
        assert!(cards.calendar.is_none() && cards.calendar_today.is_none());
    }
    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }
    #[test]
    fn calendar_and_scrollable_cards_reconcile_when_toggled_or_reordered() {
        use cosmic::Application;
        use cosmic::iced::advanced::widget::Tree;

        let config = Config {
            enabled_widgets: vec!["weather".into(), "devbox".into(), "calendar".into()],
            ..Config::default()
        };
        let (mut cards, _) = Cards::init(Core::default(), config);
        let mut tree = Tree::new(cards.desktop_widgets().as_widget());
        let mut calendar_tree = Tree::new(cards.calendar_widget().as_widget());
        // Appending a scrollable card used to pass Calendar's stateless root
        // directly to the scroller, panicking on a state downcast.
        for selection in [
            vec!["weather", "devbox", "calendar", "projects"],
            vec!["calendar", "projects"],
            vec!["clock"],
            vec!["clock", "calendar"],
            vec!["weather", "devbox", "calendar", "projects"],
            vec![],
            vec!["weather", "devbox", "calendar"],
        ] {
            cards.config.enabled_widgets = selection.into_iter().map(String::from).collect();
            tree.diff(cards.desktop_widgets().as_widget_mut());
            calendar_tree.diff(cards.calendar_widget().as_widget_mut());
        }
        cards.config.enabled_widgets.push("projects".into());
        for order in [
            ["calendar", "weather", "projects", "devbox"],
            ["devbox", "projects", "weather", "calendar"],
            ["weather", "devbox", "calendar", "projects"],
        ] {
            cards.config.widget_order = order.map(String::from).into();
            // This also switches between the content-sized agenda and the
            // whole-card scroll fallback for very short displays.
            for height in [384., 624., 984.] {
                cards.desktop_height = height;
                cards.calendar_height = height;
                tree.diff(cards.desktop_widgets().as_widget_mut());
                calendar_tree.diff(cards.calendar_widget().as_widget_mut());
            }
        }
    }
    #[test]
    fn calendar_space_is_independent_of_clock_and_the_right_column() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(Core::default(), Config::default());
        cards.calendar_height = 624.;
        let with_clock = cards.card_height(Widget::Calendar);
        cards
            .config
            .enabled_widgets
            .retain(|name| name == "calendar" || name == "clock");
        assert_eq!(cards.card_height(Widget::Calendar), with_clock);
        cards.config.enabled_widgets.retain(|name| name != "clock");
        assert_eq!(cards.card_height(Widget::Calendar), with_clock);
        assert!(cards.card_height(Widget::Calendar) <= cards.calendar_height);
        cards.calendar_height = 384.;
        assert_eq!(cards.card_height(Widget::Calendar), 384.);
        assert!(
            !cards
                .config
                .ordered_right_cards()
                .contains(&Widget::Calendar)
        );
        assert!(!cards.config.ordered_right_cards().contains(&Widget::Clock));
    }
    #[test]
    fn calendar_grows_past_the_old_cap_and_only_stops_at_the_output_budget() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(Core::default(), Config::default());
        let today = Local::now().date_naive();
        cards.calendar = Some(CalendarData {
            events: (0..24)
                .map(|index| CalendarEvent {
                    title: format!("Event {index}"),
                    start: today.to_string(),
                    end: (today + Days::days(1)).to_string(),
                    all_day: true,
                    url: String::new(),
                    meeting_url: None,
                })
                .collect(),
            ..CalendarData::default()
        });
        assert_eq!(cards.agenda_events(today).len(), 24);
        assert!(cards.calendar_natural_height() > 540.);
        assert_eq!(cards.card_height(Widget::Calendar), cards.calendar_height);
        cards
            .config
            .widget_positions
            .insert("calendar".into(), [0., 0.]);
        assert_eq!(cards.widget_size(Widget::Calendar)[1], 984.);
        cards.screen_size = [1920., 1440.];
        assert!(cards.widget_size(Widget::Calendar)[1] > 984.);
    }
    #[test]
    fn calendar_overview_preferences_and_date_browsing_are_independent() {
        use cosmic::Application;
        use cosmic::iced::advanced::widget::Tree;
        let (mut cards, _) = Cards::init(Core::default(), Config::default());
        let last_day = date(2026, 12, 31);
        let mut tree = Tree::new(cards.calendar_widget().as_widget());
        for (today, tomorrow, expected) in [
            (true, false, vec![last_day]),
            (true, true, vec![last_day, date(2027, 1, 1)]),
            (false, true, vec![date(2027, 1, 1)]),
            (false, false, vec![]),
        ] {
            cards.config.calendar_show_today = today;
            cards.config.calendar_show_tomorrow = tomorrow;
            cards.calendar_home = true;
            assert_eq!(cards.calendar_days(last_day), expected);
            tree.diff(cards.calendar_widget().as_widget_mut());
            let _ = cards.update(Message::Day(date(2026, 10, 20)));
            assert_eq!(cards.calendar_days(last_day), vec![date(2026, 10, 20)]);
            let _ = cards.update(Message::Today);
            assert!(cards.calendar_home);
            let now = Local::now().date_naive();
            assert_eq!(cards.selected_day, now);
            assert_eq!(
                cards.calendar_days(now).len(),
                usize::from(today) + usize::from(tomorrow)
            );
            tree.diff(cards.calendar_widget().as_widget_mut());
        }
    }
    #[test]
    fn week_navigation_from_tomorrow_overview_keeps_its_weekday() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(
            Core::default(),
            Config {
                calendar_show_today: false,
                calendar_show_tomorrow: true,
                ..Config::default()
            },
        );
        let tomorrow = Local::now().date_naive() + Days::days(1);
        let _ = cards.update(Message::Week(1));
        assert_eq!(cards.selected_day, tomorrow + Days::weeks(1));
        assert!(!cards.calendar_home);
        let _ = cards.update(Message::Today);
        assert_eq!(
            cards.calendar_days(Local::now().date_naive()),
            vec![tomorrow]
        );
    }
    #[test]
    fn monday_to_sunday_week_crosses_year_boundary() {
        let new_year = date(2027, 1, 1);
        assert_eq!(week_start(new_year), date(2026, 12, 28));
        assert_eq!(week_start(date(2027, 1, 3)), date(2026, 12, 28));
        assert_eq!(week_start(date(2027, 1, 4)), date(2027, 1, 4));
        assert_eq!(week_label(new_year), "28 Dec 2026 – 3 Jan 2027");
    }
    #[test]
    fn week_navigation_preserves_the_selected_weekday() {
        use cosmic::Application;
        let sunday = date(2026, 5, 31);
        let (mut cards, _) = Cards::init(Core::default(), Config::default());
        let _ = cards.update(Message::Day(sunday));
        let _ = cards.update(Message::Week(1));
        assert_eq!(cards.selected_day, date(2026, 6, 7));
        assert_eq!(cards.selected_day.weekday(), sunday.weekday());
        assert_eq!(cards.month, date(2026, 6, 1));
        let _ = cards.update(Message::Week(-1));
        assert_eq!(cards.selected_day, sunday);
        assert_eq!(cards.month, date(2026, 5, 1));
        assert_eq!(week_label(sunday), "25–31 May 2026");
        assert_eq!(week_label(date(2026, 9, 30)), "28 Sep – 4 Oct 2026");
    }
    #[test]
    fn navigation_ignores_an_old_fetch_and_today_restores_the_current_week() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(Core::default(), Config::default());
        let _ = cards.update(Message::Day(date(2026, 12, 31)));
        let _ = cards.update(Message::Week(1));
        cards.calendar = None;
        let _ = cards.update(Message::Calendar(
            cards.calendar_connection.revision.clone(),
            "2026-12".into(),
            Ok(CalendarData::default()),
        ));
        assert_eq!(cards.selected_day, date(2027, 1, 7));
        assert_eq!(cards.month, date(2027, 1, 1));
        assert!(cards.calendar.is_none());
        let _ = cards.update(Message::Today);
        let today = Local::now().date_naive();
        assert_eq!(cards.selected_day, today);
        assert_eq!(cards.month, today.with_day(1).unwrap());
    }
    #[test]
    fn sidebar_visibility_does_not_change_the_desktop_selection() {
        use cosmic::Application;
        let config = Config {
            enabled_widgets: vec!["projects".into(), "clock".into()],
            ..Config::default()
        };
        let (mut cards, _) = Cards::init(Core::default(), config);
        let _ = cards.update(Message::ShowControls);
        assert!(cards.sidebar.visible);
        let _ = cards.update(Message::HideControls);
        assert!(!cards.sidebar.visible);
        assert_eq!(cards.config.enabled_widgets, ["projects", "clock"]);
        assert_eq!(cards.desktop_height, 984.);
        cards.config.enabled_widgets.clear();
        let _ = cards.update(Message::ShowControls);
        assert!(cards.config.enabled_widgets.is_empty());
    }
    #[test]
    fn meeting_alert_keeps_today_while_browsing_another_month() {
        use cosmic::Application;
        let (mut cards, _) = Cards::init(Core::default(), Config::default());
        let now = Local::now();
        let event = CalendarEvent {
            title: "Test meeting".into(),
            start: (now + Days::minutes(5)).to_rfc3339(),
            end: (now + Days::minutes(35)).to_rfc3339(),
            all_day: false,
            url: String::new(),
            meeting_url: Some("https://meet.google.com/example".into()),
        };
        let _ = cards.update(Message::CalendarToday(
            cards.calendar_connection.revision.clone(),
            Ok(CalendarData {
                events: vec![event],
                month: now.format("%Y-%m").to_string(),
                ..CalendarData::default()
            }),
        ));
        let _ = cards.update(Message::Day(now.date_naive() + Days::days(90)));
        assert!(cards.meeting_alert());
        cards
            .config
            .enabled_widgets
            .retain(|widget| widget != "calendar");
        assert!(!cards.meeting_alert());
    }
}
