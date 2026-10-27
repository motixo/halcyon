//! Public, bounded scene export. Inputs are profile data, never live desktop state.
use crate::config::Config;
use crate::desktop::profiles::{self, Profile, SourceLook};
use crate::desktop::store::widget::{self, WidgetGroup};
use crate::desktop::wallpaper::media::{self, Pixels};
use serde::{Deserialize, Serialize};
use std::io::Read;
use tokio::sync::watch;

pub(crate) const MAX_INPUT: usize = profiles::MAX_PROFILE_BYTES + 8192;
pub(crate) const MAX_PNG: usize = 4 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scene {
    pub profile: Profile,
    pub omitted_picture: bool,
}
impl Scene {
    pub fn new(profile: &Profile) -> Result<Self, String> {
        profile.validate().map_err(|e| e.to_string())?;
        let eligible = profile
            .wallpaper
            .as_ref()
            .and_then(|w| w.background.as_ref())
            .and_then(|b| b.source.as_ref())
            .is_some_and(|s| {
                matches!(
                    s,
                    SourceLook::Bundled(_) | SourceLook::Color(_) | SourceLook::Gradient(_)
                )
            });
        let (mut profile, _) = profile.portable();
        profile.name = "Sample desktop".into();
        profile.composition = None;
        profile.windows = None;
        profile.workspaces = None;
        profile.desktop_icons = None;
        if !eligible {
            profile.wallpaper = None;
        }
        // Never permit a caller to turn the public renderer into a file loader.
        Ok(Self {
            profile,
            omitted_picture: !eligible,
        })
    }
    pub fn config(&self) -> Result<Config, String> {
        let mut value = serde_json::to_value(Config::default()).map_err(|e| e.to_string())?;
        for group in [WidgetGroup::Style, WidgetGroup::Layout] {
            if let Some(section) = self.profile.widget_section(group) {
                for field in widget::fields(group) {
                    if let Some(member) = widget::get(section, field.pointer) {
                        widget::put(&mut value, field.pointer, Some(member))?;
                    }
                }
            }
        }
        let mut config =
            Config::from_bytes(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)?;
        config.location = None;
        config.appearance.wallpaper_light = matches!(self.profile.wallpaper.as_ref().and_then(|w| w.background.as_ref()).and_then(|b| b.source.as_ref()), Some(SourceLook::Bundled(id)) if id == "halcyon-paper");
        Ok(config)
    }
    pub fn background(&self) -> Option<Pixels> {
        let background = self.profile.wallpaper.as_ref()?.background.as_ref()?;
        let source = background.source.as_ref()?;
        let pixels = match source {
            SourceLook::Bundled(id) => media::bundled_scene(id)?,
            SourceLook::Color(_) | SourceLook::Gradient(_) => {
                media::render_color(&source.color().ok()??, 1280, 720)
            }
            _ => return None,
        };
        let mode = background
            .scaling(None)
            .ok()
            .flatten()
            .unwrap_or(crate::desktop::wallpaper::ScalingMode::Zoom);
        Some(media::fit_preview(&pixels, &mode, 1280, 720))
    }
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: u64,
    pub scene: Scene,
    pub cancel: watch::Receiver<bool>,
}
impl Job {
    pub async fn run(self) -> Result<Vec<u8>, String> {
        let input = serde_json::to_vec(&self.scene).map_err(|e| e.to_string())?;
        let pixels = media::worker::render_share(input, self.cancel)
            .await
            .ok_or("Could not render the preview. Try again.")?;
        let (send, receive) = tokio::sync::oneshot::channel();
        media::worker::prepare_static(move || {
            let result = (|| -> Result<Vec<u8>, String> {
                use image::ImageEncoder;
                let mut bytes = Vec::new();
                image::codecs::png::PngEncoder::new(&mut bytes)
                    .write_image(
                        &pixels.rgba,
                        pixels.width,
                        pixels.height,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|e| e.to_string())?;
                if bytes.len() > MAX_PNG {
                    return Err("Preview exceeds the image limit.".into());
                }
                Ok(bytes)
            })();
            let _ = send.send(result);
        })
        .await;
        receive.await.map_err(|e| e.to_string())?
    }
}

pub(crate) fn render(scene: &Scene) -> Option<Pixels> {
    use cosmic::iced::advanced::renderer::{Headless, Style};
    use cosmic::iced::runtime::user_interface::{Cache, UserInterface};
    use cosmic::iced::{Color, Pixels as TextPixels, Size, mouse};
    let scene = Scene::new(&scene.profile).ok().map(|mut safe| {
        safe.omitted_picture |= scene.omitted_picture;
        safe
    })?;
    let config = scene.config().ok()?;
    let mut renderer =
        cosmic::iced::futures::executor::block_on(<cosmic::Renderer as Headless>::new(
            cosmic::font::default(),
            TextPixels(14.),
            Some("tiny-skia"),
        ))?;
    let theme = if config.appearance.wallpaper_light {
        cosmic::Theme::light()
    } else {
        cosmic::Theme::dark()
    };
    let view = super::view::sharing::view(&scene, &config);
    let mut ui = UserInterface::build(
        view,
        Size::new(1280., 720.),
        Cache::default(),
        &mut renderer,
    );
    ui.draw(
        &mut renderer,
        &theme,
        &Style::default(),
        mouse::Cursor::Unavailable,
    );
    Some(Pixels {
        width: 1280,
        height: 720,
        rgba: renderer.screenshot(Size::new(1280, 720), 1., Color::TRANSPARENT),
    })
}

pub(crate) fn main() -> ! {
    media::worker::limits();
    let result = (|| -> Option<()> {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_INPUT as u64 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > MAX_INPUT {
            return None;
        }
        let scene: Scene = serde_json::from_slice(&bytes).ok()?;
        media::worker::ready()?;
        media::worker::write_pixels(&render(&scene)?)
    })();
    std::process::exit(if result.is_some() { 0 } else { 2 });
}

#[cfg(test)]
mod tests;
