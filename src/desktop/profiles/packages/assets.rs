use super::*;
use image::{ImageDecoder, ImageEncoder};
use std::{
    fs,
    io::{Cursor, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
pub(super) fn read(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > max as u64 {
        return Err("Choose a regular picture within the size limit".into());
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > max {
        return Err("The picture is too large".into());
    }
    Ok(bytes)
}
fn decode(bytes: &[u8], png_only: bool) -> Result<image::DynamicImage, String> {
    if bytes.len() > MAX_IMAGE {
        return Err("The picture is too large".into());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) || (png_only && reader.format() != Some(image::ImageFormat::Png))
    {
        return Err("Use a PNG or JPEG picture".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (width, height) = decoder.dimensions();
    if width == 0
        || height == 0
        || width as u64 * height as u64 > MAX_PIXELS
        || decoder.total_bytes() > 64 * 1024 * 1024
    {
        return Err("The picture exceeds the decoded size limit".into());
    }
    image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())
}
#[derive(Serialize, Deserialize)]
enum Request {
    Normalize(PathBuf),
    Validate(String),
}
fn request(request: Request) -> Result<Vec<u8>, String> {
    let input = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    let run = async {
        let (owner, cancel) = tokio::sync::watch::channel(false);
        let result =
            crate::desktop::wallpaper::media::worker::package_image(input, cancel, |_| {}).await;
        drop(owner);
        result.ok_or_else(|| {
            "The picture is invalid, exceeds its limits, or took too long to process.".to_string()
        })
    };
    // Public callers are the existing blocking native jobs (and isolated tests).
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(run),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?
            .block_on(run),
    }
}
pub(super) fn check_png(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_IMAGE {
        return Err("The picture is too large".into());
    }
    request(Request::Validate(STANDARD.encode(bytes))).and_then(|result| {
        if result == b"ok" {
            Ok(())
        } else {
            Err("Invalid picture".into())
        }
    })
}
pub(super) fn normalize(path: &Path) -> Result<Vec<u8>, String> {
    request(Request::Normalize(path.to_owned()))
}
fn normalize_local(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = read(path, MAX_IMAGE)?;
    let rgba = decode(&bytes, false)?.into_rgba8();
    struct Bounded(Vec<u8>);
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_IMAGE.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("The normalized picture is too large"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Bounded(Vec::new());
    image::codecs::png::PngEncoder::new(&mut output)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    Ok(output.0)
}
pub(crate) fn main() -> ! {
    crate::desktop::wallpaper::media::worker::limits();
    let run = || -> Option<()> {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > MAX_BYTES {
            return None;
        }
        let request: Request = serde_json::from_slice(&bytes).ok()?;
        crate::desktop::wallpaper::media::worker::ready()?;
        #[cfg(test)]
        if std::env::var_os("HALCYON_PACKAGE_WORKER_STALL").is_some() {
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        let result = match request {
            Request::Normalize(path) => normalize_local(&path).ok()?,
            Request::Validate(encoded) => {
                let decoded = STANDARD.decode(encoded).ok()?;
                decode(&decoded, true).ok()?;
                b"ok".to_vec()
            }
        };
        if result.len() > MAX_IMAGE {
            return None;
        }
        let mut output = std::io::stdout().lock();
        output
            .write_all(&(result.len() as u32).to_le_bytes())
            .ok()?;
        output.write_all(&result).ok()?;
        output.flush().ok()?;
        Some(())
    };
    std::process::exit(if run().is_some() { 0 } else { 2 })
}
/// Retained immutable objects support old drafts, applied references and Undo.
pub(super) fn stage(
    store: &crate::desktop::DesktopStore,
    hash: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    if !hex(hash, 64) || bytes.len() > MAX_IMAGE || digest(bytes) != hash {
        return Err("Invalid picture identity".into());
    }
    let lock = store
        .state_file("setup-assets.lock")
        .map_err(|e| e.to_string())?;
    let _guard = crate::config::lock_config(&lock)?;
    let dir = store
        .state_subpath(ASSET_DIR, None)
        .map_err(|e| e.to_string())?;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .map_err(|e| e.to_string())?;
    fs::File::open(store.state_dir())
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    let path = store
        .state_subpath(ASSET_DIR, Some(&format!("{hash}.png")))
        .map_err(|e| e.to_string())?;
    let stage = store
        .state_subpath(ASSET_DIR, Some("staged-picture.tmp"))
        .map_err(|e| e.to_string())?;
    // The writer lock excludes a current staging operation. A prior interrupted
    // temporary has never been referenced and is the only object cleanup removes.
    match fs::symlink_metadata(&stage) {
        Ok(meta) if meta.is_file() => fs::remove_file(&stage).map_err(|e| e.to_string())?,
        Ok(_) => return Err("The package staging file needs repair".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    if fs::symlink_metadata(&path).is_ok() {
        if read(&path, MAX_IMAGE)? != bytes {
            return Err("An imported picture has changed. It was preserved.".into());
        }
        fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        fs::File::open(&dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        return Ok(path);
    }
    let mut count = 0;
    let mut total = 0u64;
    for item in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let meta = fs::symlink_metadata(item.path()).map_err(|e| e.to_string())?;
        count += 1;
        total = total.saturating_add(meta.len());
        if !meta.is_file() || count >= 64 || total + bytes.len() as u64 > 256 * 1024 * 1024 {
            return Err("Imported picture storage is full or needs repair. Existing pictures were kept for Undo.".into());
        }
    }
    let publish = || -> Result<(), String> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&stage)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        #[cfg(test)]
        if store
            .fixture_root()
            .is_some_and(|root| root.join("interrupt-package-stage").exists())
        {
            return Err("Fixture interruption before publication".into());
        }
        fs::hard_link(&stage, &path).map_err(|e| e.to_string())?;
        #[cfg(test)]
        if store
            .fixture_root()
            .is_some_and(|root| root.join("interrupt-package-publish").exists())
        {
            return Err("Fixture interruption after publication".into());
        }
        fs::File::open(&dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    };
    let result = publish();
    let _ = fs::remove_file(&stage);
    result?;
    Ok(path)
}
