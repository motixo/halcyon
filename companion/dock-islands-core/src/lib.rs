//! Versioned settings, wire contract and logical geometry shared by the two
//! native applications. This crate has no toolkit or compositor dependencies.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NAMESPACE: &str = "io.github.desktopcards.DockIslands";
pub const VERSION: u32 = 1;
pub const BASIS: u16 = 10_000;
pub const MAX_WIRE_BYTES: usize = 64 * 1024;
pub const UPSTREAM: &str = "3ce6ce1cec97669f91e809962d9446aaefe8a06e";
pub const HANDLE: f64 = 18.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Section {
    Start,
    Center,
    End,
}
impl Section {
    pub const ALL: [Self; 3] = [Self::Start, Self::Center, Self::End];
    pub fn index(self) -> usize {
        match self {
            Self::Start => 0,
            Self::Center => 1,
            Self::End => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Placement {
    #[default]
    Together,
    Separate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Together {
    pub order: [Section; 3],
    /// Fraction of the legal travel interval, in basis points.
    pub position: u16,
}
impl Default for Together {
    fn default() -> Self {
        Self {
            order: Section::ALL,
            position: BASIS / 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Separate {
    pub order: [Section; 3],
    pub positions: [u16; 3],
}
impl Default for Separate {
    fn default() -> Self {
        Self {
            order: Section::ALL,
            positions: [0, BASIS / 2, BASIS],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arrangement {
    pub together: Together,
    pub separate: Separate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub enabled: bool,
    pub placement: Placement,
    pub gap: u16,
    pub default_layout: Arrangement,
    /// Connector names are local. Portable presets use only default_layout.
    pub outputs: BTreeMap<String, Arrangement>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            enabled: false,
            placement: Placement::Together,
            gap: 14,
            default_layout: Arrangement::default(),
            outputs: BTreeMap::new(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err("Unsupported Dock Islands version".into());
        }
        if self.gap > 64 {
            return Err("Island gap must be 0-64 pixels".into());
        }
        if self.outputs.len() > 32 {
            return Err("Too many saved displays".into());
        }
        for (name, layout) in &self.outputs {
            if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
                return Err("Invalid display name".into());
            }
            layout.validate()?;
        }
        self.default_layout.validate()
    }
    pub fn layout(&self, output: &str) -> &Arrangement {
        self.outputs.get(output).unwrap_or(&self.default_layout)
    }
    pub fn layout_mut(&mut self, output: &str) -> &mut Arrangement {
        if output.is_empty() {
            return &mut self.default_layout;
        }
        self.outputs
            .entry(output.into())
            .or_insert_with(|| self.default_layout.clone())
    }
    pub fn portable(&self) -> Self {
        let mut copy = self.clone();
        copy.outputs.clear();
        copy
    }
}
impl Arrangement {
    pub fn validate(&self) -> Result<(), String> {
        for order in [&self.together.order, &self.separate.order] {
            if Section::ALL.iter().any(|id| !order.contains(id)) {
                return Err("Island order must contain each section once".into());
            }
        }
        if self.together.position > BASIS || self.separate.positions.iter().any(|p| *p > BASIS) {
            return Err("Island position is outside its display".into());
        }
        Ok(())
    }
    pub fn order(&self, mode: Placement) -> &[Section; 3] {
        match mode {
            Placement::Together => &self.together.order,
            Placement::Separate => &self.separate.order,
        }
    }
    /// Move by visible islands, retaining the positions of empty sections.
    pub fn reorder_visible(&mut self, mode: Placement, id: Section, present: [bool; 3], steps: i8) {
        let order = match mode {
            Placement::Together => &mut self.together.order,
            Placement::Separate => &mut self.separate.order,
        };
        let slots: Vec<_> = (0..3).filter(|i| present[order[*i].index()]).collect();
        let Some(from) = slots.iter().position(|i| order[*i] == id) else {
            return;
        };
        let to = (from as isize + steps as isize).clamp(0, slots.len() as isize - 1) as usize;
        if from < to {
            for i in from..to {
                order[slots[i]] = order[slots[i + 1]];
            }
        } else {
            for i in (to + 1..=from).rev() {
                order[slots[i]] = order[slots[i - 1]];
            }
        }
        order[slots[to]] = id;
    }
    pub fn reorder(&mut self, mode: Placement, id: Section, at: usize) {
        let order = match mode {
            Placement::Together => &mut self.together.order,
            Placement::Separate => &mut self.separate.order,
        };
        let from = order.iter().position(|s| *s == id).unwrap();
        let at = at.min(2);
        if from < at {
            order[from..=at].rotate_left(1);
        } else {
            order[at..=from].rotate_right(1);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: f64,
    pub length: f64,
}
impl Span {
    pub fn end(self) -> f64 {
        self.start + self.length
    }
    pub fn center(self) -> f64 {
        self.start + self.length / 2.
    }
    pub fn contains(self, p: f64) -> bool {
        p >= self.start && p < self.end()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub spans: [Option<Span>; 3],
    pub gap: f64,
    pub bounds: Span,
    pub group: Span,
    /// Native applet layout must use these budgets for its existing overflow.
    pub budgets: [f64; 3],
    pub natural: [f64; 3],
}

fn sane(n: f64) -> f64 {
    if n.is_finite() { n.max(0.) } else { 0. }
}

/// Resolve without mutating preferences. Water filling makes long applet
/// sections overflow before a short, unrelated section loses its allocation.
pub fn resolve(
    settings: &Settings,
    output: &str,
    bounds: Span,
    natural: [f64; 3],
    minimum: f64,
) -> Layout {
    let bounds = Span {
        start: sane(bounds.start),
        length: sane(bounds.length),
    };
    let a = settings.layout(output);
    let order: Vec<_> = a
        .order(settings.placement)
        .iter()
        .copied()
        .filter(|s| sane(natural[s.index()]) > 0.)
        .collect();
    let n = order.len();
    if n == 0 {
        return Layout {
            bounds,
            ..Layout::default()
        };
    }
    let minimum = sane(minimum).min(bounds.length / n as f64);
    let minimum_sum: f64 = order
        .iter()
        .map(|s| sane(natural[s.index()]).min(minimum))
        .sum();
    let gap = if n > 1 {
        (settings.gap.min(64) as f64).min((bounds.length - minimum_sum).max(0.) / (n - 1) as f64)
    } else {
        0.
    };
    let room = (bounds.length - gap * (n - 1) as f64).max(0.);
    let mut lengths = [0.; 3];
    let mut remaining = room;
    let mut pending = order.clone();
    while !pending.is_empty() {
        let share = remaining / pending.len() as f64;
        let short: Vec<_> = pending
            .iter()
            .copied()
            .filter(|s| sane(natural[s.index()]) <= share)
            .collect();
        if short.is_empty() {
            for s in pending {
                lengths[s.index()] = share;
            }
            break;
        }
        for s in short {
            lengths[s.index()] = sane(natural[s.index()]);
            remaining -= lengths[s.index()];
            pending.retain(|id| *id != s);
        }
    }
    let total = lengths.iter().sum::<f64>() + gap * (n - 1) as f64;
    let mut starts = [0.; 3];
    match settings.placement {
        Placement::Together => {
            let mut at = bounds.start
                + (bounds.length - total).max(0.) * a.together.position.min(BASIS) as f64
                    / BASIS as f64;
            for s in &order {
                starts[s.index()] = at;
                at += lengths[s.index()] + gap;
            }
        }
        Placement::Separate => {
            let mut at = bounds.start;
            for s in &order {
                let i = s.index();
                let wanted = bounds.start
                    + (bounds.length - lengths[i]).max(0.)
                        * a.separate.positions[i].min(BASIS) as f64
                        / BASIS as f64;
                starts[i] = wanted.max(at);
                at = starts[i] + lengths[i] + gap;
            }
            let mut end = bounds.end();
            for s in order.iter().rev() {
                let i = s.index();
                starts[i] = starts[i].min(end - lengths[i]);
                end = starts[i] - gap;
            }
        }
    }
    let mut spans = [None; 3];
    for s in &order {
        let i = s.index();
        spans[i] = Some(Span {
            start: starts[i],
            length: lengths[i],
        });
    }
    let first = spans[order[0].index()].unwrap();
    let last = spans[order[n - 1].index()].unwrap();
    Layout {
        spans,
        gap,
        bounds,
        budgets: lengths,
        natural,
        group: Span {
            start: first.start,
            length: last.end() - first.start,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Handle {
    Group,
    Island(Section),
}

pub fn fraction(start: f64, travel: f64) -> u16 {
    if travel <= 0. {
        BASIS / 2
    } else {
        ((start / travel).clamp(0., 1.) * BASIS as f64).round() as u16
    }
}

/// Move with optional magnetic targets, retaining both placement modes.
pub fn move_handle(
    settings: &mut Settings,
    output: &str,
    layout: &Layout,
    handle: Handle,
    wanted: f64,
    snap: bool,
) {
    let mode = settings.placement;
    if !wanted.is_finite() {
        return;
    }
    let span = match handle {
        Handle::Group => Some(layout.group),
        Handle::Island(s) => layout.spans[s.index()],
    };
    let Some(span) = span else {
        return;
    };
    let travel = (layout.bounds.length - span.length).max(0.);
    let mut target = wanted.clamp(layout.bounds.start, layout.bounds.start + travel);
    if snap {
        let mut targets = vec![
            layout.bounds.start,
            layout.bounds.start + travel / 2.,
            layout.bounds.start + travel,
        ];
        if let Handle::Island(id) = handle {
            for s in Section::ALL {
                if s != id
                    && let Some(other) = layout.spans[s.index()]
                {
                    targets.extend([
                        other.end() + layout.gap,
                        other.start - layout.gap - span.length,
                    ]);
                }
            }
        }
        if let Some(t) = targets
            .into_iter()
            .filter(|t| *t >= layout.bounds.start && *t <= layout.bounds.start + travel)
            .min_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()))
            .filter(|t| (t - target).abs() <= 8.)
        {
            target = t;
        }
    }
    let a = settings.layout_mut(output);
    match (mode, handle) {
        (Placement::Together, Handle::Group) => {
            a.together.position = fraction(target - layout.bounds.start, travel)
        }
        (Placement::Separate, Handle::Island(s)) => {
            a.separate.positions[s.index()] = fraction(target - layout.bounds.start, travel);
            let center = wanted + span.length / 2.;
            let mut order = a.separate.order;
            order.sort_by(|x, y| {
                let pos = |id: Section| {
                    if id == s {
                        center
                    } else {
                        layout.spans[id.index()].map_or(
                            a.separate.positions[id.index()] as f64 / BASIS as f64
                                * layout.bounds.length
                                + layout.bounds.start,
                            Span::center,
                        )
                    }
                };
                pos(*x).total_cmp(&pos(*y))
            });
            a.separate.order = order;
        }
        (Placement::Together, Handle::Island(s)) => {
            let center = wanted + span.length / 2.;
            let mut order = a.together.order;
            order.sort_by(|x, y| {
                let pos = |id: Section| {
                    if id == s {
                        center
                    } else {
                        layout.spans[id.index()].map_or(f64::MAX, Span::center)
                    }
                };
                pos(*x).total_cmp(&pos(*y))
            });
            a.together.order = order;
        }
        _ => {}
    }
}

#[derive(Clone, Debug)]
pub struct Drag {
    pub handle: Handle,
    pub before: Settings,
    pub origin: f64,
    pub span_start: f64,
    pub started: bool,
}
impl Drag {
    pub fn new(settings: &Settings, layout: &Layout, handle: Handle, pointer: f64) -> Option<Self> {
        let span = match handle {
            Handle::Group => Some(layout.group),
            Handle::Island(id) => layout.spans[id.index()],
        }?;
        Some(Self {
            handle,
            before: settings.clone(),
            origin: pointer,
            span_start: span.start,
            started: false,
        })
    }
    pub fn motion(
        &mut self,
        settings: &mut Settings,
        output: &str,
        layout: &Layout,
        pointer: f64,
    ) -> bool {
        if !pointer.is_finite() {
            return false;
        }
        let delta = pointer - self.origin;
        if !self.started && delta.abs() < 4. {
            return false;
        }
        self.started = true;
        move_handle(
            settings,
            output,
            layout,
            self.handle,
            self.span_start + delta,
            true,
        );
        true
    }
    pub fn cancel(self, settings: &mut Settings) {
        *settings = self.before;
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub protocol: u32,
    pub upstream: String,
    pub together: bool,
    pub separate: bool,
    pub rounded_blur: bool,
}
impl Default for Capabilities {
    fn default() -> Self {
        Self {
            protocol: VERSION,
            upstream: UPSTREAM.into(),
            together: true,
            separate: true,
            rounded_blur: false,
        }
    }
}
impl Capabilities {
    pub fn compatible(&self) -> bool {
        self.protocol == VERSION && self.upstream == UPSTREAM && self.together && self.separate
    }
}

/// Each connection owns at most one lease. Disconnect cancels it. Tokens
/// identify that lease and revisions reject delayed or duplicated edits.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Request {
    Capabilities,
    Inspect {
        panel: String,
        output: String,
    },
    Begin {
        panel: String,
        output: String,
        base: Settings,
        draft: Settings,
    },
    Update {
        token: u64,
        revision: u64,
        draft: Settings,
    },
    Read {
        token: u64,
    },
    Select {
        token: u64,
        handle: Handle,
    },
    Done {
        token: u64,
    },
    Cancel {
        token: u64,
    },
    Activate {
        panel: String,
        expected: Settings,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct View {
    #[serde(default)]
    pub output: String,
    pub selected: Option<Handle>,
    pub visible: bool,
    pub popups: usize,
    pub overflow: [usize; 3],
    pub horizontal: bool,
    pub origin: [f64; 2],
    pub scale: f64,
    pub handles: Vec<(Handle, Span, Span)>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Response {
    Capabilities(Capabilities),
    Draft {
        token: u64,
        revision: u64,
        settings: Settings,
        layout: Layout,
        view: View,
    },
    Applied {
        settings: Settings,
    },
    Cancelled,
    Error(String),
}

#[cfg(test)]
mod tests;
