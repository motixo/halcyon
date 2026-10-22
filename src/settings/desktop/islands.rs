//! Arrange holds a companion lease. Only Apply writes the sidecar.
use super::*;
use crate::desktop::islands::Client;
use dock_islands_core::{Handle, Layout, Placement, Request, Response, Section, Settings, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    Enabled(bool),
    Placement(Placement),
    Gap(u16),
    GapText(String),
    Select(Section),
    Arrange,
    Done,
    Cancel,
    Check,
    Sync,
    Move(i32),
    End(bool),
    Group(bool),
}
#[derive(Default)]
pub struct State {
    pub compatible: bool,
    pub status: Option<String>,
    pub selected: Option<Section>,
    pub group: bool,
    pub gap_text: Option<String>,
    pub lease: Option<Lease>,
    pub geometry: Option<Layout>,
    pub measured_output: Option<(PanelKind, Output, String)>,
}
pub struct Lease {
    client: Client,
    token: u64,
    revision: u64,
    pub(super) before: Settings,
    pub(super) kind: PanelKind,
}
impl DesktopEditor {
    pub fn arranging(&self) -> bool {
        self.islands.lease.is_some()
    }
    fn requested_island_output(&self) -> String {
        match self.current().map(PanelDraft::output) {
            Some(Setting::Known(Output::Name(name))) => name,
            _ => String::new(),
        }
    }
    pub fn island_output(&self) -> String {
        if let Some((kind, configured, actual)) = &self.islands.measured_output
            && *kind == self.selected
            && self
                .current()
                .and_then(|d| d.output().known().cloned())
                .as_ref()
                == Some(configured)
        {
            return actual.clone();
        }
        self.requested_island_output()
    }
    fn remember_island_output(&mut self, actual: String) {
        self.islands.measured_output = self
            .current()
            .and_then(|d| d.output().known().cloned())
            .map(|configured| (self.selected, configured, actual));
    }
    pub fn check_islands(&mut self) {
        let result = self
            .store()
            .ok_or_else(|| "Desktop settings are unavailable".to_string())
            .and_then(Client::connect)
            .and_then(|mut c| c.capabilities());
        self.islands.compatible = result.is_ok();
        self.islands.status = result.err();
        if !self.arranging() {
            self.islands.geometry = None;
            self.islands.measured_output = None;
            if self.islands.compatible
                && let Some(store) = self.store()
                && let Ok(mut client) = Client::connect(store)
                && let Ok(Response::Draft { layout, view, .. }) =
                    client.request(&Request::Inspect {
                        panel: self.selected.entry().into(),
                        output: self.requested_island_output(),
                    })
            {
                self.islands.geometry = Some(layout);
                self.remember_island_output(view.output);
            }
        }
    }
    pub fn islands_can_apply(&self) -> bool {
        !self.arranging()
            && self.drafts.values().all(|d| {
                !d.edited().any(|name| name == "islands")
                    || d.islands().known().is_none_or(|s| !s.enabled)
                    || self.islands.compatible
            })
    }
    fn sync_island_lease(&mut self) -> bool {
        let Some(lease) = &mut self.islands.lease else {
            return true;
        };
        let result = lease.client.request(&Request::Read { token: lease.token });
        match result {
            Ok(Response::Draft {
                revision,
                settings,
                layout,
                view,
                ..
            }) => {
                lease.revision = revision;
                if let Some(handle) = view.selected {
                    self.islands.group = handle == Handle::Group;
                    self.islands.selected = match handle {
                        Handle::Island(id) => Some(id),
                        Handle::Group => None,
                    };
                }
                self.islands.geometry = Some(layout);
                if let Some(draft) = self.drafts.get_mut(&lease.kind) {
                    draft.set_islands(settings);
                }
                true
            }
            response => {
                let lease = self.islands.lease.take().unwrap();
                if let Some(draft) = self.drafts.get_mut(&lease.kind) {
                    draft.set_islands(lease.before);
                }
                let error = response
                    .err()
                    .unwrap_or("The arrangement session ended".into());
                self.notice = Some(if error == "The arrangement session ended" {
                    Notice::Done("Arrangement cancelled".into())
                } else {
                    Notice::Error(error)
                });
                self.check_islands();
                false
            }
        }
    }
    fn select_island_handle(&mut self, handle: Handle) {
        if let Some(lease) = &mut self.islands.lease {
            if let Err(error) = lease.client.request(&Request::Select {
                token: lease.token,
                handle,
            }) {
                self.notice = Some(Notice::Error(error));
            }
        }
    }
    fn set_island_draft(&mut self, change: impl FnOnce(&mut Settings)) {
        if !self.sync_island_lease() {
            return;
        }
        let Some(Setting::Known(mut s)) = self.current().map(PanelDraft::islands) else {
            return;
        };
        change(&mut s);
        if let Err(e) = s.validate() {
            self.notice = Some(Notice::Error(e));
            return;
        }
        if let Some(lease) = &mut self.islands.lease {
            let response = lease.client.request(&Request::Update {
                token: lease.token,
                revision: lease.revision,
                draft: s.clone(),
            });
            match response {
                Ok(Response::Draft {
                    revision, layout, ..
                }) => {
                    lease.revision = revision;
                    self.islands.geometry = Some(layout);
                }
                Ok(_) => {
                    self.notice = Some(Notice::Error("Unexpected panel reply".into()));
                    return;
                }
                Err(e) => {
                    self.notice = Some(Notice::Error(e));
                    return;
                }
            }
        }
        self.edit(|d| d.set_islands(s));
    }
    pub fn island_edit(&mut self, edit: Edit) {
        match edit {
            Edit::Enabled(enabled) => {
                if self.arranging() {
                    self.island_edit(Edit::Cancel);
                }
                self.set_island_draft(|s| s.enabled = enabled);
                self.check_islands();
            }
            Edit::Placement(mode) => {
                self.set_island_draft(|s| s.placement = mode);
                self.islands.group = false;
            }
            Edit::Gap(gap) => {
                self.set_island_draft(|s| s.gap = gap.min(64));
                self.islands.gap_text = None;
            }
            Edit::GapText(text) => {
                let text: String = text.chars().filter(char::is_ascii_digit).take(2).collect();
                if let Ok(gap) = text.parse::<u16>()
                    && gap <= 64
                {
                    self.set_island_draft(|s| s.gap = gap);
                }
                self.islands.gap_text = Some(text);
            }
            Edit::Select(s) => {
                self.select_island_handle(Handle::Island(s));
                self.islands.selected = Some(s);
                self.islands.group = false;
                self.focus = Some(Slot {
                    region: match s {
                        Section::Start => Region::Start,
                        Section::Center => Region::Center,
                        Section::End => Region::End,
                    },
                    index: 0,
                });
            }
            Edit::Group(group) => {
                self.islands.group = group;
                if group {
                    self.islands.selected = None;
                    self.select_island_handle(Handle::Group);
                }
            }
            Edit::Check => self.check_islands(),
            Edit::Sync => {
                self.sync_island_lease();
            }
            Edit::Arrange => {
                if self.arranging() {
                    return;
                }
                let result = (|| {
                    let draft = self.current().ok_or("No panel selected")?;
                    if draft.edited().any(|key| key != "islands") {
                        return Err(
                            "Apply screen edge and applet changes before arranging on screen"
                                .into(),
                        );
                    }
                    let settings = draft
                        .islands()
                        .known()
                        .cloned()
                        .ok_or("Island settings cannot be read")?;
                    let base = draft
                        .islands_base()
                        .ok_or("Island settings cannot be read")?;
                    let mut client = Client::connect(self.store().ok_or("Desktop unavailable")?)?;
                    client.capabilities()?;
                    let response = client.request(&Request::Begin {
                        panel: self.selected.entry().into(),
                        output: self.requested_island_output(),
                        base,
                        draft: settings.clone(),
                    })?;
                    match response {
                        Response::Draft {
                            token,
                            revision,
                            layout,
                            view,
                            ..
                        } => Ok((
                            Lease {
                                client,
                                token,
                                revision,
                                before: settings,
                                kind: self.selected,
                            },
                            layout,
                            view.output,
                        )),
                        _ => Err("The panel did not start Arrange".into()),
                    }
                })();
                match result {
                    Ok((lease, layout, output)) => {
                        self.remember_island_output(output);
                        self.islands.lease = Some(lease);
                        self.islands.geometry = Some(layout);
                        self.notice = None;
                    }
                    Err(e) => self.notice = Some(Notice::Error(e)),
                }
            }
            Edit::Done | Edit::Cancel => {
                let Some(mut lease) = self.islands.lease.take() else {
                    return;
                };
                let cancel = edit == Edit::Cancel;
                let response = lease.client.request(&if cancel {
                    Request::Cancel { token: lease.token }
                } else {
                    Request::Done { token: lease.token }
                });
                match response {
                    Ok(Response::Draft { settings, .. }) if !cancel => {
                        if let Some(d) = self.drafts.get_mut(&lease.kind) {
                            d.set_islands(settings);
                        }
                        self.notice = None;
                    }
                    Ok(Response::Cancelled) if cancel => {
                        if let Some(d) = self.drafts.get_mut(&lease.kind) {
                            d.set_islands(lease.before);
                        }
                        self.notice = None;
                    }
                    result => {
                        if let Some(d) = self.drafts.get_mut(&lease.kind) {
                            d.set_islands(lease.before);
                        }
                        self.notice = Some(Notice::Error(
                            result
                                .err()
                                .unwrap_or("Arrange ended without a result".into()),
                        ));
                    }
                }
                self.check_islands();
            }
            Edit::Move(_) | Edit::End(_) => {
                if !self.sync_island_lease() {
                    return;
                }
                let output = self.island_output();
                let handle = if self.islands.group {
                    Handle::Group
                } else {
                    Handle::Island(self.islands.selected.unwrap_or(Section::Center))
                };
                let Some(d) = self.current() else {
                    return;
                };
                let Some(mut s) = d.islands().known().cloned() else {
                    return;
                };
                let mode = s.placement;
                if mode == Placement::Together
                    && let Handle::Island(id) = handle
                {
                    let present = preview_lengths(d).map(|length| length > 0.);
                    let steps = match edit {
                        Edit::End(false) => -3,
                        Edit::End(true) => 3,
                        Edit::Move(n) => n.signum() as i8,
                        _ => unreachable!(),
                    };
                    s.layout_mut(&output)
                        .reorder_visible(mode, id, present, steps);
                } else {
                    let measured = self.islands.geometry.as_ref();
                    let natural = measured.map_or_else(
                        || preview_lengths(d),
                        |l| l.spans.map(|s| s.map_or(0., |s| s.length)),
                    );
                    let layout = dock_islands_core::resolve(
                        &s,
                        &output,
                        measured.map_or(
                            Span {
                                start: 0.,
                                length: 1024.,
                            },
                            |l| l.bounds,
                        ),
                        natural,
                        32.,
                    );
                    let span = match handle {
                        Handle::Group => Some(layout.group),
                        Handle::Island(id) => layout.spans[id.index()],
                    };
                    if let Some(span) = span {
                        let wanted = match edit {
                            Edit::End(false) => layout.bounds.start,
                            Edit::End(true) => layout.bounds.end() - span.length,
                            Edit::Move(n) => span.start + 8. * n as f64,
                            _ => unreachable!(),
                        };
                        dock_islands_core::move_handle(
                            &mut s, &output, &layout, handle, wanted, false,
                        );
                    }
                }
                self.set_island_draft(|current| *current = s);
            }
        }
    }
    pub fn acknowledge_islands(&mut self) -> Result<(), String> {
        let store = self.store().ok_or("Desktop unavailable")?;
        let mut client = Client::connect(store)?;
        for d in self.drafts.values() {
            let Some(settings) = d.islands().known().cloned() else {
                continue;
            };
            if !d.listed() {
                continue;
            }
            if !matches!(client.request(&Request::Activate{panel:d.kind().entry().into(),expected:settings.clone()})?,Response::Applied{settings:ack} if ack==settings)
            {
                return Err("The panel did not acknowledge the saved layout".into());
            }
        }
        Ok(())
    }
}
pub fn preview_lengths(draft: &PanelDraft) -> [f64; 3] {
    let regions = draft.applets().known().cloned().unwrap_or_default();
    let size = draft
        .size()
        .known()
        .cloned()
        .unwrap_or(PanelSize::M)
        .thickness() as f64;
    let padding = draft.padding().known().copied().unwrap_or(4) as f64;
    let spacing = draft.spacing().known().copied().unwrap_or(4) as f64;
    Region::ALL.map(|r| {
        let n = regions.list(r).len();
        if n == 0 {
            0.
        } else {
            n as f64 * size + (n - 1) as f64 * spacing + padding * 2.
        }
    })
}
