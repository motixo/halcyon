use super::*;
use crate::displays::Output;
use std::collections::BTreeMap;

pub(super) struct Probe {
    pub output: Output,
    pub id: window::Id,
    pub size: Option<[f32; 2]>,
}
impl Cards {
    pub(super) fn arrange_on_display(&mut self, display: String) -> Task<Message> {
        if self.displays.unique(&display).is_none()
            || self.layout_editor.active
            || self
                .paused_layout
                .as_ref()
                .is_some_and(|e| e.exit_failed || e.pending_resume.is_some())
        {
            return Task::none();
        }
        self.paused_layout = None;
        self.arrange_display = Some(display);
        let migrate = self.reconcile_output();
        if self.output_known {
            Task::batch([migrate, self.arrange()])
        } else {
            self.pending_arrange = true;
            migrate
        }
    }

    pub(super) fn widget_output(&self, widget: Widget) -> Option<&Output> {
        self.displays.widget(&self.config, widget)
    }
    pub(super) fn widget_layer_output(&self, widget: Widget) -> IcedOutput {
        self.widget_output(widget)
            .map(|o| IcedOutput::Output(o.handle.clone()))
            .unwrap_or_else(|| self.layer_output())
    }
    pub(super) fn widget_screen(&self, widget: Widget) -> [f32; 2] {
        let Some(output) = self.widget_output(widget) else {
            return self.screen_size;
        };
        self.extra_probes
            .iter()
            .find(|p| p.output.handle == output.handle)
            .and_then(|p| p.size)
            .unwrap_or(self.screen_size)
    }
    pub(super) fn widget_available(&self, widget: Widget) -> bool {
        let Some(output) = self.widget_output(widget) else {
            return self.displays.outputs().is_empty();
        };
        self.extra_probes
            .iter()
            .find(|p| p.output.handle == output.handle)
            .is_none_or(|p| p.size.is_some())
    }
    pub(super) fn same_widget_display(&self, a: Widget, b: Widget) -> bool {
        self.widget_output(a).map(|o| &o.handle) == self.widget_output(b).map(|o| &o.handle)
    }
    pub(super) fn layout_contexts(&self) -> crate::config::monitors::Contexts {
        self.displays.contexts(&self.config)
    }
    pub(super) fn layout_output_handles(&self) -> BTreeMap<String, crate::displays::WlOutput> {
        Widget::ALL
            .into_iter()
            .filter_map(|widget| {
                Some((
                    widget.key().into(),
                    self.widget_output(widget)?.handle.clone(),
                ))
            })
            .collect()
    }
    pub(super) fn effective_config(&self) -> Config {
        self.displays.effective_config(&self.config)
    }
    pub(super) fn refresh_display_layouts(&mut self) {
        let config = self.effective_config();
        self.display_positions = config.widget_positions;
        self.display_sizes = config.widget_sizes;
    }
    pub(super) fn effective_positions(&self) -> &BTreeMap<String, [f32; 2]> {
        if self.config.multi_monitor() {
            &self.display_positions
        } else {
            &self.config.widget_positions
        }
    }
    pub(super) fn effective_sizes(&self) -> &BTreeMap<String, [f32; 2]> {
        if self.config.multi_monitor() {
            &self.display_sizes
        } else {
            &self.config.widget_sizes
        }
    }
    pub(super) fn arranging_widget(&self, widget: Widget) -> bool {
        self.config.widget_visible(widget)
            && self.widget_available(widget)
            && self.widget_output(widget).map(|o| &o.handle)
                == self.selected_output.as_ref().map(|o| &o.handle)
    }
    pub(super) fn extra_probe_tasks(&self) -> Task<Message> {
        Task::batch(self.extra_probes.iter().map(|p| {
            get_layer_surface(SctkLayerSurfaceSettings {
                id: p.id,
                output: IcedOutput::Output(p.output.handle.clone()),
                layer: Layer::Bottom,
                keyboard_interactivity: KeyboardInteractivity::None,
                anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                size: Some((Some(0), Some(0))),
                input_zone: Some(Vec::new()),
                exclusive_zone: 0,
                namespace: "halcyon-display-probe".into(),
                ..Default::default()
            })
        }))
    }
    pub(super) fn identity_scan(&self) -> Task<Message> {
        if std::env::var_os("DESKTOP_CARDS_DESKTOP_ROOT").is_some() {
            return Task::none();
        }
        let generation = self.displays.generation;
        cosmic::task::future(async move {
            let ids = tokio::task::spawn_blocking(crate::displays::physical_identities)
                .await
                .unwrap_or_default();
            Message::DisplayIdentities(generation, ids)
        })
    }
}
