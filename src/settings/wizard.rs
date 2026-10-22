//! First-run setup: Welcome, Look, Dock & panel, Weather and Ready, one
//! decision per screen in the Settings window without the sidebar.
//!
//! A look and the dock and panel choices are applied as they are made,
//! through the same profile plan as Setups, so they never touch applets,
//! their order or anything else the look doesn't own. Every change is a
//! normal desktop change; Cancel undoes the wizard's own changes, newest
//! first. Choosing a city is a preference and stays.
use crate::desktop::DesktopStore;
use crate::desktop::store::LatestChange;
use crate::desktop::panels::MAX_RADIUS;
use crate::desktop::profiles::{self, Profile};
use dock_islands_core::Placement;
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Welcome,
    Look,
    Bars,
    Weather,
    Ready,
}

impl Step {
    pub const ALL: [Self; 5] = [
        Self::Welcome,
        Self::Look,
        Self::Bars,
        Self::Weather,
        Self::Ready,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|step| *step == self).unwrap_or(0)
    }
    fn next(self) -> Option<Self> {
        Self::ALL.get(self.index() + 1).copied()
    }
    fn back(self) -> Option<Self> {
        self.index().checked_sub(1).map(|index| Self::ALL[index])
    }
    fn key(self) -> &'static str {
        match self {
            Self::Welcome => "welcome",
            Self::Look => "look",
            Self::Bars => "bars",
            Self::Weather => "weather",
            Self::Ready => "ready",
        }
    }
    fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|step| step.key() == key)
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Halcyon",
            Self::Look => "Look",
            Self::Bars => "Dock & panel",
            Self::Weather => "Weather",
            Self::Ready => "Ready",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Welcome => "Desktop widgets and themes for COSMIC.",
            Self::Look => "Wallpaper, colors, windows, dock and widgets in one.",
            Self::Bars => "Shape and placement.",
            Self::Weather => "Your city for the forecast.",
            Self::Ready => "All of this lives in Settings.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Pill,
    Rounded,
    Square,
}

impl Shape {
    pub const ALL: [Self; 3] = [Self::Pill, Self::Rounded, Self::Square];
    pub fn title(self) -> &'static str {
        match self {
            Self::Pill => "Pill",
            Self::Rounded => "Rounded",
            Self::Square => "Square",
        }
    }
    pub fn radius(self) -> u32 {
        match self {
            Self::Pill => MAX_RADIUS,
            Self::Rounded => 12,
            Self::Square => 0,
        }
    }
}

/// The dock and panel choices. The default is what every starter look sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bars {
    /// Islands, kept together; otherwise one bar across the screen.
    pub islands: bool,
    pub panel_shape: Shape,
    pub dock_shape: Shape,
    /// The dock floats off the screen edge.
    pub floating: bool,
    /// The dock hides behind windows.
    pub hides: bool,
}

impl Bars {
    pub fn starter(islands: bool) -> Self {
        Self {
            islands,
            panel_shape: Shape::Pill,
            dock_shape: Shape::Pill,
            floating: true,
            hides: false,
        }
    }

    /// What the dock and panel show now, so an untouched step shows (and
    /// keeps) the person's own setup.
    pub fn current(store: &DesktopStore, islands_available: bool) -> Self {
        use crate::desktop::panels::{AutoHide, PanelDraft, PanelKind};
        let shape = |radius: Option<u32>| match radius {
            Some(0) => Shape::Square,
            Some(radius) if radius < 40 => Shape::Rounded,
            _ => Shape::Pill,
        };
        let dock = PanelDraft::load(store, PanelKind::Dock).ok();
        let panel = PanelDraft::load(store, PanelKind::Panel).ok();
        Self {
            islands: islands_available
                && panel
                    .as_ref()
                    .and_then(|panel| panel.islands().known().map(|islands| islands.enabled))
                    .unwrap_or(false),
            panel_shape: shape(panel.as_ref().and_then(|panel| panel.radius().known().copied())),
            dock_shape: shape(dock.as_ref().and_then(|dock| dock.radius().known().copied())),
            floating: dock
                .as_ref()
                .and_then(|dock| dock.floating().known().copied())
                .unwrap_or(false),
            hides: dock
                .as_ref()
                .and_then(|dock| dock.autohide().known().copied())
                .is_some_and(|mode| mode != AutoHide::Never),
        }
    }

    /// A dock-and-panel-only record with just the `touched` choices, so
    /// everything else stays as the person has it. `None` when nothing was
    /// touched.
    pub fn profile(
        self,
        touched: Touched,
        islands_available: bool,
        gap: u16,
    ) -> Result<Option<Profile>, String> {
        let mut panel = serde_json::Map::new();
        if touched.panel_shape {
            panel.insert("border_radius".into(), json!(self.panel_shape.radius()));
        }
        if touched.islands && islands_available {
            panel.insert("floating".into(), json!(self.islands));
            panel.insert("margin".into(), json!(if self.islands { 6 } else { 0 }));
            let islands = dock_islands_core::Settings {
                enabled: self.islands,
                placement: Placement::Together,
                gap,
                ..Default::default()
            };
            panel.insert(
                "islands".into(),
                serde_json::to_value(islands).expect("islands serialize"),
            );
        }
        let mut dock = serde_json::Map::new();
        if touched.dock_shape {
            dock.insert("border_radius".into(), json!(self.dock_shape.radius()));
        }
        if touched.floating {
            dock.insert("floating".into(), json!(self.floating));
            dock.insert("margin".into(), json!(if self.floating { 6 } else { 0 }));
        }
        if touched.hides {
            dock.insert(
                "autohide".into(),
                json!(if self.hides { "OnOverlap" } else { "Never" }),
            );
        }
        if panel.is_empty() && dock.is_empty() {
            return Ok(None);
        }
        let mut record = json!({
            "format": "desktop-cards-profile", "version": profiles::VERSION, "name": "Dock & panel",
        });
        if !panel.is_empty() {
            record["panel"] = panel.into();
        }
        if !dock.is_empty() {
            record["dock"] = dock.into();
        }
        profiles::import(record.to_string().as_bytes())
            .map(Some)
            .map_err(|error| error.to_string())
    }
}

/// The dock and panel choices someone actually made on that step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Touched {
    pub islands: bool,
    pub panel_shape: bool,
    pub dock_shape: bool,
    pub floating: bool,
    pub hides: bool,
}

impl Touched {
    fn any(self) -> bool {
        self.islands || self.panel_shape || self.dock_shape || self.floating || self.hides
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Start,
    /// Leaves the wizard as it is now (on Welcome: before anything changed).
    Leave,
    Next,
    Back,
    /// Returns to a step from Ready's summary.
    Show(Step),
    Skip,
    Cancel,
    Finish,
    Look(&'static str),
    Islands(bool),
    PanelShape(Shape),
    DockShape(Shape),
    Floating(bool),
    Hides(bool),
    Startup(bool),
}

/// What Settings does after a wizard message.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    /// The desktop changed: reread preferences and editors.
    Changed,
    /// The wizard is over. `finish` starts the cards (and at sign-in when
    /// `startup`); otherwise nothing more happens.
    Done { finish: bool, startup: bool },
}

pub struct Wizard {
    pub step: Step,
    /// The starter look on the desktop, chosen here.
    pub look: Option<&'static str>,
    pub bars: Bars,
    /// The dock and panel as the desktop shows them; Skip returns to these.
    applied_bars: Bars,
    /// Choices made on the dock and panel step since they were last applied.
    touched: Touched,
    pub islands_available: bool,
    /// The wizard's own desktop changes, oldest first, for Cancel. Kept in a
    /// state file too, so Cancel still works after closing and reopening.
    changes: Vec<LatestChange>,
    pub error: Option<String>,
    pub startup: bool,
    /// The person chose Start at sign-in themselves.
    pub startup_touched: bool,
}

/// Setup in progress, kept between Settings sessions.
const STATE_FILE: &str = "setup-wizard.json";
const STATE_FORMAT: &str = "halcyon-setup";

impl Wizard {
    /// `bars` is what the desktop shows now (`Bars::current`); `startup`
    /// whether the cards start at sign-in now (on for a first run).
    pub fn new(islands_available: bool, bars: Bars, startup: bool) -> Self {
        Self {
            step: Step::Welcome,
            look: None,
            bars,
            applied_bars: bars,
            touched: Touched::default(),
            islands_available,
            changes: Vec::new(),
            error: None,
            startup,
            startup_touched: false,
        }
    }

    /// Setup someone left before finishing, with its changes, if any.
    pub fn resume(
        store: &DesktopStore,
        islands_available: bool,
        bars: Bars,
        startup: bool,
    ) -> Option<Self> {
        let path = store.state_file(STATE_FILE).ok()?;
        let text = std::fs::read_to_string(path).ok()?;
        if text.len() > 2 * 1024 * 1024 {
            return None;
        }
        let saved: serde_json::Value = serde_json::from_str(&text).ok()?;
        if saved["format"] != STATE_FORMAT {
            return None;
        }
        let mut wizard = Self::new(islands_available, bars, startup);
        wizard.step = saved["step"].as_str().and_then(Step::parse).unwrap_or(Step::Look);
        wizard.look = saved["look"]
            .as_str()
            .and_then(profiles::preset)
            .map(|preset| preset.id);
        wizard.changes = saved["changes"]
            .as_array()?
            .iter()
            .filter_map(|change| LatestChange::decode(change.as_str()?).ok())
            .collect();
        Some(wizard)
    }

    /// Whether setup was left unfinished (it resumes on the next launch).
    pub fn pending(store: &DesktopStore) -> bool {
        store.state_file(STATE_FILE).is_ok_and(|path| path.exists())
    }

    fn save(&self, store: &DesktopStore) {
        let Ok(path) = store.state_file(STATE_FILE) else {
            return;
        };
        let changes: Vec<String> = self
            .changes
            .iter()
            .filter_map(|change| change.encode().ok())
            .collect();
        let state = json!({
            "format": STATE_FORMAT, "version": 1, "step": self.step.key(),
            "look": self.look, "changes": changes,
        });
        let _ = crate::config::write_config_text(&path, Some(&state.to_string()));
    }

    /// Setup is over: nothing to resume.
    pub fn forget(store: &DesktopStore) {
        if let Ok(path) = store.state_file(STATE_FILE) {
            let _ = crate::config::write_config_text(&path, None);
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    pub fn update(
        &mut self,
        message: Message,
        store: Option<&DesktopStore>,
        config: &crate::config::Config,
    ) -> Outcome {
        self.error = None;
        let outcome = self.handle(message, store, config);
        if !matches!(outcome, Outcome::Done { .. })
            && let Some(store) = store
        {
            self.save(store);
        }
        outcome
    }

    fn handle(
        &mut self,
        message: Message,
        store: Option<&DesktopStore>,
        config: &crate::config::Config,
    ) -> Outcome {
        match message {
            Message::Start => self.step = Step::Look,
            Message::Leave => return Outcome::Done { finish: false, startup: false },
            Message::Back => {
                if let Some(step) = self.step.back() {
                    self.step = step;
                }
            }
            Message::Show(step) => self.step = step,
            Message::Skip => {
                // Skipping the dock and panel keeps them as they are.
                if self.step == Step::Bars {
                    self.bars = self.applied_bars;
                    self.touched = Touched::default();
                }
                self.advance();
            }
            Message::Next => {
                if self.step == Step::Bars && self.touched.any() {
                    let changed = self.apply_bars(store);
                    if self.error.is_some() {
                        return changed;
                    }
                    self.advance();
                    return changed;
                }
                self.advance();
            }
            Message::Look(id) => return self.choose_look(id, store, config),
            Message::Islands(on) => {
                self.bars.islands = on && self.islands_available;
                self.touched.islands = true;
            }
            Message::PanelShape(shape) => {
                self.bars.panel_shape = shape;
                self.touched.panel_shape = true;
            }
            Message::DockShape(shape) => {
                self.bars.dock_shape = shape;
                self.touched.dock_shape = true;
            }
            Message::Floating(on) => {
                self.bars.floating = on;
                self.touched.floating = true;
            }
            Message::Hides(on) => {
                self.bars.hides = on;
                self.touched.hides = true;
            }
            Message::Startup(on) => {
                self.startup = on;
                self.startup_touched = true;
            }
            Message::Cancel => {
                let changed = self.undo_all(store);
                if self.error.is_some() {
                    return changed;
                }
                return Outcome::Done { finish: false, startup: false };
            }
            Message::Finish => {
                return Outcome::Done {
                    finish: true,
                    startup: self.startup,
                };
            }
        }
        Outcome::None
    }

    fn advance(&mut self) {
        if let Some(step) = self.step.next() {
            self.step = step;
        }
    }

    fn choose_look(
        &mut self,
        id: &'static str,
        store: Option<&DesktopStore>,
        config: &crate::config::Config,
    ) -> Outcome {
        if self.look == Some(id) {
            return Outcome::None;
        }
        let Some(store) = store else {
            self.error = Some("Desktop settings are unavailable.".into());
            return Outcome::None;
        };
        let Some(preset) = profiles::preset(id) else {
            return Outcome::None;
        };
        // Another look goes on top of the last one: if it can't be applied,
        // the last one stays, still selected. Cancel undoes both.
        let bundled = crate::desktop::wallpaper::bundled_dir(store);
        let preset = profiles::fitted(preset, config, &bundled, self.islands_available);
        match apply(store, &preset.profile, true) {
            Ok(change) => {
                self.changes.push(change);
                self.look = Some(id);
                self.bars = Bars::current(store, self.islands_available);
                self.applied_bars = self.bars;
                self.touched = Touched::default();
            }
            Err(error) => self.error = Some(error),
        }
        Outcome::Changed
    }

    fn apply_bars(&mut self, store: Option<&DesktopStore>) -> Outcome {
        let Some(store) = store else {
            self.error = Some("Desktop settings are unavailable.".into());
            return Outcome::None;
        };
        let gap = self
            .look
            .and_then(profiles::preset)
            .and_then(|preset| preset.profile.panel?.islands)
            .map_or(14, |islands| islands.gap);
        let applied = self
            .bars
            .profile(self.touched, self.islands_available, gap)
            .and_then(|profile| profile.map(|profile| apply(store, &profile, false)).transpose());
        match applied {
            Ok(change) => {
                self.changes.extend(change);
                self.applied_bars = self.bars;
                self.touched = Touched::default();
            }
            Err(error) => self.error = Some(error),
        }
        Outcome::Changed
    }

    /// Undoes the wizard's own changes, newest first, leaving anything else
    /// (a chosen city, an edit made elsewhere) alone. A change whose settings
    /// were edited since stays, and stays listed for another try.
    fn undo_all(&mut self, store: Option<&DesktopStore>) -> Outcome {
        let Some(store) = store else {
            return Outcome::None;
        };
        let any = !self.changes.is_empty();
        while let Some(change) = self.changes.last() {
            if let Err(error) = store.undo_change(change) {
                self.error = Some(format!(
                    "{error} Some of setup's changes stay; undo them from Setups or the Dock & panel page."
                ));
                break;
            }
            self.changes.pop();
        }
        if self.changes.is_empty() {
            self.look = None;
        }
        self.bars = Bars::current(store, self.islands_available);
        self.applied_bars = self.bars;
        self.touched = Touched::default();
        if any { Outcome::Changed } else { Outcome::None }
    }
}

/// Plans `profile` for all its parts and applies it as one change; the
/// change's ID, for Undo. `keep_colors` keeps a look's colors from being
/// recomputed for the wallpaper it just set, as Setups does.
fn apply(
    store: &DesktopStore,
    profile: &Profile,
    keep_colors: bool,
) -> Result<LatestChange, String> {
    let installed = profiles::Installed::discover(store);
    let selected = profile.domains().into_iter().collect();
    let plan = profiles::plan(store, &installed, profile, &selected);
    if !plan.can_apply() {
        return Err(plan.blocking_reasons().join(" "));
    }
    if let Some(reason) = plan.recheck().into_iter().next() {
        return Err(reason);
    }
    let latest = profiles::apply(store, &plan).map_err(|error| error.to_string())?;
    if keep_colors {
        let _ = crate::desktop::wallpaper::automatic::keep_manual_colors(store);
    }
    Ok(latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dock_and_panel_choice_is_a_valid_record() {
        for islands_available in [true, false] {
            for islands in [true, false] {
                for panel_shape in Shape::ALL {
                    for dock_shape in Shape::ALL {
                        for floating in [true, false] {
                            for hides in [true, false] {
                                let bars = Bars { islands, panel_shape, dock_shape, floating, hides };
                                let all = Touched {
                                    islands: true,
                                    panel_shape: true,
                                    dock_shape: true,
                                    floating: true,
                                    hides: true,
                                };
                                let record = bars.profile(all, islands_available, 12);
                                assert!(record.is_ok(), "{bars:?} {islands_available}: {record:?}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn only_the_choices_made_are_written() {
        let bars = Bars::starter(true);
        assert!(bars.profile(Touched::default(), true, 12).unwrap().is_none());
        let only_hiding = Touched { hides: true, ..Default::default() };
        let profile = bars.profile(only_hiding, true, 12).unwrap().unwrap();
        assert!(profile.panel.is_none(), "the panel stays as it is");
        let dock = profile.dock.unwrap();
        assert!(dock.autohide.is_some());
        assert!(dock.border_radius.is_none() && dock.floating.is_none() && dock.margin.is_none());
    }
}
