use crate::config::{DevboxPreferences, DevboxSection, Palette};
use crate::data::Metrics;
use cosmic::iced::{Color, Length, Point, Rectangle, mouse};
use cosmic::widget::canvas;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(300);
const MAX_SAMPLES: usize = 301;
pub(super) const HEIGHT: f32 = 14.;

#[derive(Debug, Default)]
pub(super) struct MetricHistories {
    cpu: Series,
    memory: Series,
    disk: Series,
    network: NetworkHistory,
    gpus: BTreeMap<String, Series>,
    gpu_reset_at: Option<Instant>,
    at: Option<Instant>,
}

impl MetricHistories {
    /// A bounded, frozen history for Settings examples. It is built once from
    /// illustrative readings, never from a hardware poll or persisted data.
    pub(super) fn settings_sample(metrics: &Metrics) -> Self {
        let now = Instant::now();
        let mut histories = Self {
            at: Some(now),
            ..Self::default()
        };
        for index in 0..=30 {
            let age = Duration::from_secs((30 - index) * 10);
            let at = now.checked_sub(age).unwrap_or(now);
            let variation = if index == 30 {
                1.
            } else {
                [0.8, 0.9, 0.7, 1.1, 1.3, 1.2, 0.9, 1.][index as usize % 8]
            };
            histories.cpu.push(
                at,
                Some((f64::from(metrics.cpu) * variation).clamp(0., 100.)),
            );
            histories.memory.push(
                at,
                used_percent(metrics.memory_used as f64, metrics.memory_total as f64)
                    .map(|value| (value + (variation - 1.) * 3.).clamp(0., 100.)),
            );
            histories.disk.push(
                at,
                metrics
                    .disk
                    .and_then(|(used, total)| used_percent(used, total)),
            );
            if let Some(network) = &metrics.network {
                histories.network.push(
                    at,
                    network.download.map(|value| value * variation),
                    network.upload.map(|value| value * variation),
                );
            }
            for gpu in &metrics.gpus {
                histories.gpus.entry(gpu.id.clone()).or_default().push(
                    at,
                    gpu.utilization
                        .map(|value| (f64::from(value) * variation).clamp(0., 100.)),
                );
            }
        }
        histories
    }

    pub(super) fn enabled(preferences: &DevboxPreferences, section: DevboxSection) -> bool {
        section.enabled(preferences)
            && match section {
                DevboxSection::Cpu => preferences.history.cpu,
                DevboxSection::Memory => preferences.history.memory,
                DevboxSection::Disk => preferences.history.disk,
                DevboxSection::Network => preferences.history.network,
                DevboxSection::Gpu => preferences.history.gpu,
                DevboxSection::Uptime | DevboxSection::Services => false,
            }
    }

    pub(super) fn any_enabled(preferences: &DevboxPreferences) -> bool {
        DevboxSection::ALL
            .into_iter()
            .any(|section| Self::enabled(preferences, section))
    }

    pub(super) fn record(
        &mut self,
        metrics: &Metrics,
        preferences: &DevboxPreferences,
        visible: bool,
    ) {
        self.reconcile(preferences, visible);
        if !visible {
            return;
        }
        let at = Instant::now();
        let previous_at = self.at.replace(at);
        if Self::enabled(preferences, DevboxSection::Cpu) {
            self.cpu.push(
                at,
                metrics
                    .cpu_sampled
                    .then_some(metrics.cpu as f64)
                    .and_then(percentage),
            );
        }
        if Self::enabled(preferences, DevboxSection::Memory) {
            self.memory.push(
                at,
                metrics
                    .memory_sampled
                    .then(|| used_percent(metrics.memory_used as f64, metrics.memory_total as f64))
                    .flatten(),
            );
        }
        if Self::enabled(preferences, DevboxSection::Disk) {
            self.disk.push(
                at,
                metrics
                    .disk
                    .and_then(|(used, total)| used_percent(used, total)),
            );
        }
        if Self::enabled(preferences, DevboxSection::Network) {
            let network = metrics
                .network
                .as_ref()
                .filter(|network| network.interfaces > 0);
            self.network.push(
                at,
                network.and_then(|network| network.download),
                network.and_then(|network| network.upload),
            );
        }
        if Self::enabled(preferences, DevboxSection::Gpu) {
            let mut identities = BTreeMap::<&str, usize>::new();
            for gpu in &metrics.gpus {
                if !gpu.id.is_empty() {
                    *identities.entry(gpu.id.as_str()).or_default() += 1;
                }
            }
            self.gpus
                .retain(|id, _| identities.get(id.as_str()) == Some(&1));
            for gpu in &metrics.gpus {
                if identities.get(gpu.id.as_str()) != Some(&1) {
                    continue;
                }
                // A newly enabled or rediscovered device must not bring back
                // a cached point from before its history was reset.
                let since = self.gpu_reset_at.max(
                    (!self.gpus.contains_key(&gpu.id))
                        .then_some(previous_at)
                        .flatten(),
                );
                let series = self.gpus.entry(gpu.id.clone()).or_default();
                let sample = gpu
                    .utilization_at
                    .filter(|sample_at| {
                        *sample_at <= at && since.is_none_or(|since| *sample_at > since)
                    })
                    .zip(gpu.utilization.map(f64::from).and_then(percentage))
                    .filter(|_| !gpu.asleep);
                match sample {
                    Some((sample_at, value)) => series.push(sample_at, Some(value)),
                    None => series.push(at, None),
                }
                series.prune(at);
            }
        }
    }

    pub(super) fn reconcile(&mut self, preferences: &DevboxPreferences, visible: bool) {
        if !visible || !Self::enabled(preferences, DevboxSection::Cpu) {
            self.cpu = Series::default();
        }
        if !visible || !Self::enabled(preferences, DevboxSection::Memory) {
            self.memory = Series::default();
        }
        if !visible || !Self::enabled(preferences, DevboxSection::Disk) {
            self.disk = Series::default();
        }
        if !visible || !Self::enabled(preferences, DevboxSection::Network) {
            self.network = NetworkHistory::default();
        }
        if !visible || !Self::enabled(preferences, DevboxSection::Gpu) {
            self.gpus.clear();
            self.gpu_reset_at = Some(Instant::now());
        }
        if !visible {
            self.at = None;
        }
    }

    pub(super) fn unavailable(&mut self, preferences: &DevboxPreferences, visible: bool) {
        self.reconcile(preferences, visible);
        if !visible {
            return;
        }
        let at = Instant::now();
        self.at = Some(at);
        if Self::enabled(preferences, DevboxSection::Cpu) {
            self.cpu.push(at, None);
        }
        if Self::enabled(preferences, DevboxSection::Memory) {
            self.memory.push(at, None);
        }
        if Self::enabled(preferences, DevboxSection::Disk) {
            self.disk.push(at, None);
        }
        if Self::enabled(preferences, DevboxSection::Network) {
            self.network.push(at, None, None);
        }
        for series in self.gpus.values_mut() {
            series.push(at, None);
        }
    }

    pub(super) fn view<'a, Message: 'a>(
        &'a self,
        section: DevboxSection,
        gpu_id: Option<&str>,
        palette: Palette,
    ) -> cosmic::Element<'a, Message> {
        let mut accent = palette.accent;
        accent.a *= 0.62;
        let mut secondary = palette.muted;
        secondary.a *= 0.65;
        let percent = |series| Trace {
            series,
            scale: 100.,
            color: accent,
        };
        let traces = match section {
            DevboxSection::Cpu => vec![percent(&self.cpu)],
            DevboxSection::Memory => vec![percent(&self.memory)],
            DevboxSection::Disk => vec![percent(&self.disk)],
            DevboxSection::Network => vec![
                Trace {
                    series: &self.network.download,
                    scale: self.network.scale,
                    color: accent,
                },
                Trace {
                    series: &self.network.upload,
                    scale: self.network.scale,
                    color: secondary,
                },
            ],
            DevboxSection::Gpu => gpu_id
                .and_then(|id| self.gpus.get(id))
                .map(percent)
                .into_iter()
                .collect(),
            DevboxSection::Uptime | DevboxSection::Services => Vec::new(),
        };
        canvas(Sparkline {
            traces,
            at: self.at,
        })
        .width(Length::Fill)
        .height(Length::Fixed(HEIGHT))
        .into()
    }
}

struct Trace<'a> {
    series: &'a Series,
    scale: f64,
    color: Color,
}

struct Sparkline<'a> {
    traces: Vec<Trace<'a>>,
    at: Option<Instant>,
}

impl<Message> canvas::Program<Message, cosmic::Theme> for Sparkline<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &cosmic::Renderer,
        _: &cosmic::Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        if let Some(at) = self.at {
            let point = |(x, y): (f32, f32)| {
                Point::new(
                    1. + x * (bounds.width - 2.).max(0.),
                    1. + y * (bounds.height - 2.).max(0.),
                )
            };
            for trace in &self.traces {
                for segment in trace.series.segments(at, trace.scale) {
                    if segment.len() == 1 {
                        frame.fill(&canvas::Path::circle(point(segment[0]), 0.65), trace.color);
                    } else {
                        let path = canvas::Path::new(|path| {
                            path.move_to(point(segment[0]));
                            for sample in &segment[1..] {
                                path.line_to(point(*sample));
                            }
                        });
                        frame.stroke(
                            &path,
                            canvas::Stroke::default()
                                .with_color(trace.color)
                                .with_width(1.)
                                .with_line_join(canvas::LineJoin::Round),
                        );
                    }
                }
            }
        }
        vec![frame.into_geometry()]
    }
}

fn percentage(value: f64) -> Option<f64> {
    (value.is_finite() && (0. ..=100.).contains(&value)).then_some(value)
}

fn used_percent(used: f64, total: f64) -> Option<f64> {
    (used.is_finite() && total.is_finite() && total > 0. && used >= 0. && used <= total)
        .then(|| used / total * 100.)
}

#[derive(Debug, Default)]
struct Series {
    samples: VecDeque<(Instant, Option<f64>)>,
}

impl Series {
    fn push(&mut self, at: Instant, value: Option<f64>) {
        // Cached device readings retain their measurement timestamp. Do not
        // turn them into fresh points, or reconnect a gap to an older reading.
        if self.samples.back().is_some_and(|(last, _)| *last >= at) {
            return;
        }
        self.samples
            .push_back((at, value.filter(|value| value.is_finite())));
        self.prune(at);
    }

    fn prune(&mut self, now: Instant) {
        while self.samples.front().is_some_and(|(at, _)| {
            now.saturating_duration_since(*at) > WINDOW || self.samples.len() > MAX_SAMPLES
        }) {
            self.samples.pop_front();
        }
    }

    fn segments(&self, now: Instant, scale: f64) -> Vec<Vec<(f32, f32)>> {
        let mut segments = Vec::new();
        let mut current = Vec::new();
        for (at, value) in &self.samples {
            let age = now.saturating_duration_since(*at);
            if age > WINDOW {
                continue;
            }
            match value {
                Some(value) => current.push((
                    1. - age.as_secs_f32() / WINDOW.as_secs_f32(),
                    (1. - value / scale).clamp(0., 1.) as f32,
                )),
                None if !current.is_empty() => segments.push(std::mem::take(&mut current)),
                None => {}
            }
        }
        if !current.is_empty() {
            segments.push(current);
        }
        segments
    }
}

#[derive(Debug)]
struct NetworkHistory {
    download: Series,
    upload: Series,
    scale: f64,
}

impl Default for NetworkHistory {
    fn default() -> Self {
        Self {
            download: Series::default(),
            upload: Series::default(),
            scale: 1024.,
        }
    }
}

impl NetworkHistory {
    fn push(&mut self, at: Instant, download: Option<f64>, upload: Option<f64>) {
        self.download
            .push(at, download.filter(|value| *value >= 0.));
        self.upload.push(at, upload.filter(|value| *value >= 0.));
        let peak = self
            .download
            .samples
            .iter()
            .chain(&self.upload.samples)
            .filter_map(|(_, value)| *value)
            .fold(0., f64::max);
        // Both directions share a power-of-two ceiling. Keep it until traffic
        // exceeds it, or the entire recent window fits below a quarter of it.
        // This makes ordinary variation move the line, not the graph's scale.
        if peak > self.scale || peak < self.scale / 4. {
            self.scale = 1024.;
            while self.scale < peak {
                self.scale = (self.scale * 2.).min(f64::MAX);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> DevboxPreferences {
        DevboxPreferences {
            history: crate::config::DevboxHistoryPreferences {
                cpu: true,
                memory: true,
                disk: true,
                network: true,
                gpu: true,
            },
            ..Default::default()
        }
    }

    fn metrics() -> Metrics {
        let mut metrics = Metrics {
            cpu: 25.,
            cpu_sampled: true,
            memory_used: 8.,
            memory_total: 32.,
            memory_sampled: true,
            disk: Some((50., 200.)),
            network: Some(Default::default()),
            gpus: vec![Default::default(), Default::default()],
            ..Default::default()
        };
        let network = metrics.network.as_mut().unwrap();
        network.interfaces = 1;
        network.download = Some(1024.);
        network.upload = Some(512.);
        for (index, gpu) in metrics.gpus.iter_mut().enumerate() {
            gpu.id = format!("0000:0{index}:00.0");
            gpu.name = "Same model".into();
            gpu.utilization = Some((index + 1) as f32 * 20.);
            gpu.utilization_at = Some(Instant::now());
        }
        metrics
    }

    #[test]
    fn only_valid_new_percentage_readings_enter_history() {
        let mut histories = MetricHistories::default();
        let mut metrics = metrics();
        histories.record(&metrics, &enabled(), true);
        assert_eq!(histories.cpu.samples.back().unwrap().1, Some(25.));
        assert_eq!(histories.memory.samples.back().unwrap().1, Some(25.));
        assert_eq!(histories.disk.samples.back().unwrap().1, Some(25.));
        metrics.cpu_sampled = false;
        metrics.memory_sampled = false;
        metrics.disk = None;
        histories.record(&metrics, &enabled(), true);
        assert_eq!(histories.cpu.samples.back().unwrap().1, None);
        assert_eq!(histories.memory.samples.back().unwrap().1, None);
        assert_eq!(histories.disk.samples.back().unwrap().1, None);
        metrics.cpu_sampled = true;
        metrics.cpu = f32::NAN;
        metrics.memory_sampled = true;
        metrics.memory_total = 0.;
        metrics.disk = Some((3., 2.));
        histories.record(&metrics, &enabled(), true);
        assert_eq!(histories.cpu.samples.back().unwrap().1, None);
        assert_eq!(histories.memory.samples.back().unwrap().1, None);
        assert_eq!(histories.disk.samples.back().unwrap().1, None);
    }

    #[test]
    fn disabling_a_history_or_section_discards_its_old_samples() {
        let mut histories = MetricHistories::default();
        let mut preferences = enabled();
        histories.record(&metrics(), &preferences, true);
        preferences.history.cpu = false;
        preferences.memory = false;
        preferences.history.network = false;
        preferences.gpu = false;
        histories.reconcile(&preferences, true);
        assert!(histories.cpu.samples.is_empty());
        assert!(histories.memory.samples.is_empty());
        assert!(histories.network.download.samples.is_empty());
        assert!(histories.gpus.is_empty());
        assert_eq!(histories.disk.samples.len(), 1);
        histories.record(&metrics(), &preferences, true);
        assert!(histories.cpu.samples.is_empty());
        assert!(histories.memory.samples.is_empty());
        assert!(histories.network.download.samples.is_empty());
        assert!(histories.gpus.is_empty());
    }

    #[test]
    fn hidden_cards_stop_collecting_and_reset_before_becoming_visible() {
        let mut histories = MetricHistories::default();
        histories.record(&metrics(), &enabled(), true);
        histories.reconcile(&enabled(), false);
        histories.record(&metrics(), &enabled(), false);
        assert!(histories.cpu.samples.is_empty());
        assert!(histories.memory.samples.is_empty());
        assert!(histories.disk.samples.is_empty());
        assert!(histories.network.download.samples.is_empty());
        assert!(histories.gpus.is_empty());
        histories.record(&metrics(), &enabled(), true);
        assert_eq!(histories.cpu.samples.len(), 1);
    }

    #[test]
    fn sample_failure_adds_gaps_to_existing_visible_histories() {
        let mut histories = MetricHistories::default();
        histories.record(&metrics(), &enabled(), true);
        histories.unavailable(&enabled(), true);
        assert_eq!(histories.cpu.samples.back().unwrap().1, None);
        assert_eq!(histories.memory.samples.back().unwrap().1, None);
        assert_eq!(histories.disk.samples.back().unwrap().1, None);
        assert_eq!(histories.network.download.samples.back().unwrap().1, None);
        assert_eq!(histories.network.upload.samples.back().unwrap().1, None);
        for series in histories.gpus.values() {
            assert_eq!(series.samples.back().unwrap().1, None);
        }
    }

    #[test]
    fn gpu_histories_follow_device_identity_across_reorder_and_disappearance() {
        let mut histories = MetricHistories::default();
        let mut metrics = metrics();
        histories.record(&metrics, &enabled(), true);
        metrics.gpus.reverse();
        for gpu in &mut metrics.gpus {
            gpu.utilization_at = Some(Instant::now());
            gpu.utilization = gpu.utilization.map(|value| value + 5.);
        }
        histories.record(&metrics, &enabled(), true);
        assert_eq!(
            histories.gpus["0000:00:00.0"]
                .samples
                .iter()
                .map(|(_, value)| *value)
                .collect::<Vec<_>>(),
            [Some(20.), Some(25.)]
        );
        assert_eq!(
            histories.gpus["0000:01:00.0"]
                .samples
                .iter()
                .map(|(_, value)| *value)
                .collect::<Vec<_>>(),
            [Some(40.), Some(45.)]
        );
        let missing = metrics.gpus.pop().unwrap();
        histories.record(&metrics, &enabled(), true);
        assert!(!histories.gpus.contains_key(&missing.id));
        metrics.gpus.push(missing);
        histories.record(&metrics, &enabled(), true);
        assert_eq!(histories.gpus["0000:00:00.0"].samples.len(), 1);
    }

    #[test]
    fn gpu_cache_is_not_repeated_and_sleeping_devices_produce_gaps() {
        let mut histories = MetricHistories::default();
        let mut metrics = metrics();
        histories.record(&metrics, &enabled(), true);
        histories.record(&metrics, &enabled(), true);
        assert_eq!(histories.gpus["0000:00:00.0"].samples.len(), 1);
        metrics.gpus[0].asleep = true;
        histories.record(&metrics, &enabled(), true);
        assert_eq!(
            histories.gpus["0000:00:00.0"].samples.back().unwrap().1,
            None
        );
    }

    #[test]
    fn reenabled_gpu_history_waits_for_a_measurement_after_reset() {
        let mut histories = MetricHistories::default();
        let mut metrics = metrics();
        let mut preferences = enabled();
        histories.record(&metrics, &preferences, true);
        preferences.history.gpu = false;
        histories.reconcile(&preferences, true);
        preferences.history.gpu = true;
        histories.record(&metrics, &preferences, true);
        assert_eq!(
            histories.gpus["0000:00:00.0"].samples.back().unwrap().1,
            None
        );
        metrics.gpus[0].utilization_at = Some(Instant::now());
        histories.record(&metrics, &preferences, true);
        assert_eq!(
            histories.gpus["0000:00:00.0"].samples.back().unwrap().1,
            Some(20.)
        );
    }

    #[test]
    fn rediscovered_gpu_cannot_restore_a_cached_point_from_before_disappearance() {
        let mut histories = MetricHistories::default();
        let mut metrics = metrics();
        histories.record(&metrics, &enabled(), true);
        let gpu = metrics.gpus.pop().unwrap();
        histories.record(&metrics, &enabled(), true);
        metrics.gpus.push(gpu);
        histories.record(&metrics, &enabled(), true);
        assert_eq!(
            histories.gpus["0000:01:00.0"].samples.back().unwrap().1,
            None
        );
    }

    #[test]
    fn ambiguous_gpu_identifiers_cannot_merge_two_device_histories() {
        let mut histories = MetricHistories::default();
        let mut metrics = metrics();
        metrics.gpus[1].id = metrics.gpus[0].id.clone();
        histories.record(&metrics, &enabled(), true);
        assert!(histories.gpus.is_empty());
        metrics.gpus[0].id.clear();
        metrics.gpus[1].id.clear();
        histories.record(&metrics, &enabled(), true);
        assert!(histories.gpus.is_empty());
    }

    #[test]
    fn history_keeps_a_bounded_five_minute_window() {
        let start = Instant::now();
        let mut series = Series::default();
        for second in 0..600 {
            series.push(start + Duration::from_secs(second), Some(second as f64));
        }
        assert_eq!(series.samples.len(), 301);
        assert_eq!(series.samples.front().unwrap().1, Some(299.));
        assert_eq!(series.samples.back().unwrap().1, Some(599.));
    }

    #[test]
    fn unexpectedly_fast_samples_still_have_a_hard_memory_bound() {
        let start = Instant::now();
        let mut series = Series::default();
        for millis in 0..10_000 {
            series.push(start + Duration::from_millis(millis), Some(42.));
        }
        assert_eq!(series.samples.len(), 301);
    }

    #[test]
    fn missing_and_invalid_samples_break_paths_without_becoming_zero() {
        let start = Instant::now();
        let mut series = Series::default();
        for (second, value) in [Some(25.), None, Some(75.), Some(f64::NAN), Some(50.)]
            .into_iter()
            .enumerate()
        {
            series.push(start + Duration::from_secs(second as u64), value);
        }
        let segments = series.segments(start + Duration::from_secs(4), 100.);
        assert_eq!(segments.iter().map(Vec::len).collect::<Vec<_>>(), [1, 1, 1]);
        assert_eq!(segments[0][0].1, 0.75);
        assert_eq!(segments[1][0].1, 0.25);
        assert_eq!(segments[2][0], (1., 0.5));
    }

    #[test]
    fn plot_coordinates_do_not_stretch_while_history_fills() {
        let start = Instant::now();
        let mut series = Series::default();
        series.push(start, Some(50.));
        series.push(start + Duration::from_secs(150), Some(100.));
        assert_eq!(
            series.segments(start + Duration::from_secs(150), 100.),
            [vec![(0.5, 0.5), (1., 0.)]]
        );
    }

    #[test]
    fn repeated_cached_samples_do_not_create_new_history_points() {
        let start = Instant::now();
        let mut series = Series::default();
        series.push(start, Some(50.));
        series.push(start, Some(50.));
        series.push(start - Duration::from_secs(1), Some(40.));
        assert_eq!(series.samples.len(), 1);
    }

    #[test]
    fn network_uses_one_stable_scale_for_both_directions() {
        let start = Instant::now();
        let mut network = NetworkHistory::default();
        network.push(start, Some(1200.), Some(6000.));
        assert_eq!(network.scale, 8192.);
        for second in 1..=300 {
            network.push(start + Duration::from_secs(second), Some(1100.), Some(50.));
            assert_eq!(network.scale, 8192.);
        }
        network.push(start + Duration::from_secs(301), Some(1100.), Some(50.));
        assert_eq!(network.scale, 2048.);
    }

    #[test]
    fn invalid_network_rates_are_gaps_and_zero_is_a_real_measurement() {
        let start = Instant::now();
        let mut network = NetworkHistory::default();
        network.push(start, Some(-1.), Some(f64::INFINITY));
        network.push(start + Duration::from_secs(1), Some(0.), Some(0.));
        assert_eq!(network.download.samples[0].1, None);
        assert_eq!(network.upload.samples[0].1, None);
        assert_eq!(network.download.samples[1].1, Some(0.));
        assert_eq!(network.scale, 1024.);
    }
}
