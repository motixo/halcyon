//! One bounded, cancelable Clock specimen for the visible setup comparison.
use super::*;
use cosmic::widget::image::Handle;

#[derive(Default)]
pub(super) struct Cache {
    key: Option<Vec<u8>>,
    current: bool,
    id: u64,
    cancel: Option<tokio::sync::watch::Sender<bool>>,
    image: Option<Handle>,
}
impl Cache {
    fn clear(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.send_replace(true);
        }
        self.id = self.id.wrapping_add(1);
        self.key = None;
        self.image = None;
    }
    pub fn image(&self, current: bool) -> Option<&Handle> {
        (self.current == current)
            .then_some(self.image.as_ref())
            .flatten()
    }
    pub fn ready(&mut self, id: u64, pixels: Option<crate::desktop::wallpaper::media::Pixels>) {
        if id == self.id {
            self.cancel = None;
            self.image = pixels.map(|p| Handle::from_rgba(p.width, p.height, p.rgba));
        }
    }
}
impl Settings {
    fn depth_preview_request(&self) -> Option<(bool, String, crate::depth::Request)> {
        if self.page != Page::DesktopProfiles || !self.scene_open() {
            return None;
        }
        let current = self.profiles.compare_current;
        let candidate = self.profiles.preview()?;
        let preview = if current {
            view::profiles::comparison_current(candidate, self.profiles.plan())
        } else {
            candidate.clone()
        };
        if preview.bars.iter().any(|bar| {
            bar.listed()
                && bar.autohide().known() != Some(&crate::desktop::panels::AutoHide::Always)
        }) {
            return None;
        }
        let mut config = preview.config.unwrap_or_else(|| self.config.clone());
        if !current
            && self
                .profiles
                .included(crate::desktop::profiles::Domain::Wallpaper)
            && matches!(self.profiles.profile()?.wallpaper.as_ref()?.background.as_ref()?.source.as_ref(),Some(crate::desktop::profiles::SourceLook::Bundled(id)) if id=="halcyon-ridge")
        {
            config.appearance.wallpaper_light = false;
        }
        if !crate::depth::eligible(&config) {
            return None;
        }
        let (stamp, background) = self.profiles.depth_background(current)?;
        // The editor's existing miniature uses a cover image. Fit is only
        // qualified on the actual output until that scene supports letterboxes.
        if background.scaling_mode != crate::desktop::wallpaper::ScalingMode::Zoom {
            return None;
        }
        let layout =
            view::profiles::scene_geometry(self.placement.preview_snapshot().clone(), &config);
        let clock = layout.widgets.iter().find(|w| w.key == "clock")?;
        let frame = crate::depth::Frame {
            screen: layout.screen,
            origin: clock.position.map(f32::round),
            size: clock.size,
            scale: 1.,
        };
        let request = crate::depth::Request::new(
            &config,
            frame,
            background,
            chrono::Local::now().naive_local(),
        )?;
        Some((current, stamp, request))
    }
    pub(super) fn depth_preview_sync(&mut self) -> Task<Message> {
        let Some((current, stamp, request)) = self.depth_preview_request() else {
            self.depth_preview.clear();
            return Task::none();
        };
        let Ok(key) = serde_json::to_vec(&(current, stamp, &request)) else {
            return Task::none();
        };
        if self.depth_preview.key.as_ref() == Some(&key) {
            return Task::none();
        }
        self.depth_preview.clear();
        self.depth_preview.key = Some(key);
        self.depth_preview.current = current;
        let id = self.depth_preview.id;
        let Ok(bytes) = serde_json::to_vec(&request) else {
            return Task::none();
        };
        let (send, cancel) = tokio::sync::watch::channel(false);
        self.depth_preview.cancel = Some(send);
        cosmic::task::future(async move {
            Message::DepthPreviewReady(
                id,
                crate::desktop::wallpaper::media::worker::render_depth(bytes, cancel).await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fb14_preview_cache_rejects_old_completion_after_navigation() {
        let mut cache = Cache::default();
        let id = cache.id;
        let (sender, receiver) = tokio::sync::watch::channel(false);
        cache.cancel = Some(sender);
        cache.clear();
        assert!(*receiver.borrow());
        cache.ready(
            id,
            Some(crate::desktop::wallpaper::media::Pixels {
                width: 1,
                height: 1,
                rgba: vec![255; 4],
            }),
        );
        assert!(cache.image(false).is_none());
        cache.current = true;
        cache.ready(
            cache.id,
            Some(crate::desktop::wallpaper::media::Pixels {
                width: 1,
                height: 1,
                rgba: vec![255; 4],
            }),
        );
        assert!(cache.image(true).is_some() && cache.image(false).is_none());
    }
}
