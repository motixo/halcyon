//! Output names are persistent connection choices; handles belong to this client only.
pub use cosmic::cctk::sctk::{
    output::OutputInfo, reexports::client::protocol::wl_output::WlOutput,
};
pub use cosmic::iced::event::wayland::OutputEvent;

pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 128 && !name.chars().any(char::is_control)
}

#[derive(Clone, Debug)]
pub struct Output {
    pub handle: WlOutput,
    pub info: OutputInfo,
}
impl Output {
    pub fn name(&self) -> Option<&str> {
        self.info.name.as_deref().filter(|name| valid_name(name))
    }
    pub fn title(&self) -> String {
        let model = self.info.model.trim();
        match (self.name(), model.is_empty()) {
            (Some(name), false) if model != name => format!("{model} · {name}"),
            (Some(name), _) => name.into(),
            (None, false) => format!("{model} (unnamed display)"),
            (None, true) => "Unnamed display".into(),
        }
    }
    /// OutputEvent's library equality deliberately ignores these properties.
    pub fn same_geometry(&self, other: &Self) -> bool {
        self.handle == other.handle
            && self.info.logical_size == other.info.logical_size
            && self.info.scale_factor == other.info.scale_factor
            && self.info.transform == other.info.transform
    }
}

#[derive(Default, Debug)]
pub struct Registry {
    outputs: Vec<Output>,
}
impl Registry {
    pub fn update(&mut self, event: OutputEvent, handle: WlOutput) {
        match event {
            OutputEvent::Removed => self.outputs.retain(|o| o.handle != handle),
            OutputEvent::Created(None) => {} // Wait for this handle's InfoUpdate.
            OutputEvent::Created(Some(info)) | OutputEvent::InfoUpdate(info) => {
                if let Some(output) = self.outputs.iter_mut().find(|o| o.handle == handle) {
                    output.info = info;
                } else {
                    self.outputs.push(Output { handle, info });
                }
            }
        }
    }
    pub fn unique(&self, name: &str) -> Option<&Output> {
        let mut found = self.outputs.iter().filter(|o| o.name() == Some(name));
        let output = found.next()?;
        found.next().is_none().then_some(output)
    }
    pub fn selected(&self, preferred: Option<&str>) -> Option<&Output> {
        preferred.and_then(|name| self.unique(name)).or_else(|| {
            self.outputs
                .iter()
                .filter(|o| o.name().is_some_and(|n| self.unique(n).is_some()))
                .min_by_key(|o| {
                    let name = o.name().unwrap();
                    (!crate::automation::rules::builtin(name), name)
                })
                // Without unique names only session-local placement is possible.
                .or_else(|| self.outputs.first())
        })
    }
    pub fn choices(&self, preferred: Option<&str>) -> Vec<(String, Option<String>)> {
        let mut named: Vec<_> = self
            .outputs
            .iter()
            .filter(|o| o.name().is_some_and(|name| self.unique(name).is_some()))
            .map(|o| (o.title(), Some(o.name().unwrap().to_string())))
            .collect();
        named.sort_by(|a, b| a.1.cmp(&b.1));
        let mut choices = vec![("Automatic".into(), None)];
        choices.extend(named);
        if let Some(name) = preferred.filter(|name| self.unique(name).is_none()) {
            let reason = if self.outputs.iter().any(|o| o.name() == Some(name)) {
                "ambiguous name"
            } else {
                "not connected"
            };
            choices.push((format!("{name} ({reason})"), Some(name.into())));
        }
        choices
    }
    pub fn status(&self, preferred: Option<&str>) -> String {
        let Some(actual) = self.selected(preferred) else {
            return "Waiting for a display. Your selection and layout are kept.".into();
        };
        if self
            .outputs
            .iter()
            .any(|o| o.name().is_none_or(|n| self.unique(n).is_none()))
        {
            return "Some displays have no unique connection name and cannot be pinned. Automatic placement is temporary on those displays.".into();
        }
        if let Some(name) = preferred.filter(|name| self.unique(name).is_none()) {
            format!(
                "{name} is disconnected; showing widgets on {}. They return when {name} reconnects.",
                actual.name().unwrap()
            )
        } else {
            format!("Showing widgets on {}.", actual.title())
        }
    }
}

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod tests {
    use super::*;
    use test_support::{Wire, info};
    #[test]
    fn monitor_pinning_registry_orders_live_handles_and_waits_for_metadata() {
        let wire = Wire::new();
        let a = wire.output(1);
        let b = wire.output(2);
        let c = wire.output(3);
        for order in [
            [
                (a.clone(), "DP-2"),
                (b.clone(), "DP-1"),
                (c.clone(), "eDP-1"),
            ],
            [
                (c.clone(), "eDP-1"),
                (b.clone(), "DP-1"),
                (a.clone(), "DP-2"),
            ],
        ] {
            let mut registry = Registry::default();
            registry.update(OutputEvent::Created(None), a.clone());
            assert!(registry.selected(None).is_none());
            for (handle, name) in order {
                registry.update(
                    OutputEvent::InfoUpdate(info(Some(name), (1920, 1080))),
                    handle,
                );
            }
            assert_eq!(registry.selected(None).unwrap().handle, c);
            assert_eq!(registry.selected(Some("DP-2")).unwrap().handle, a);
            registry.update(OutputEvent::Removed, c.clone());
            assert_eq!(registry.selected(Some("missing")).unwrap().handle, b);
        }
    }
}
