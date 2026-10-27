//! The applied document lives beside its trusted Config path. This module
//! reads/projects only; the desktop transaction is the sole writer.
use super::*;
use crate::config;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn path(config: &Path) -> PathBuf {
    if config.file_name().is_some_and(|s| s == "config.json") {
        config.with_file_name("design-v1.json")
    } else {
        let mut name = config.as_os_str().to_os_string();
        name.push(".design-v1.json");
        PathBuf::from(name)
    }
}
/// Not-found is the only legacy state. Invalid, newer and conflicting data
/// must remain errors all the way to the caller.
pub fn read(config: &Path) -> Result<Option<String>, String> {
    read_bytes(config)?
        .map(|bytes| String::from_utf8(bytes).map_err(|_| "Widget design is not UTF-8".into()))
        .transpose()
}
pub fn read_bytes(config: &Path) -> Result<Option<Vec<u8>>, String> {
    let path = path(config);
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Could not read widget design: {e}")),
    };
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 {
        return Err("Widget design is not a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("Widget design is too large".into());
    }
    Ok(Some(bytes))
}

pub fn projected_text(config_path: &Path) -> Result<Option<String>, String> {
    let raw = config::read_raw_config_text(config_path)?;
    let marker = config::parse_config_text(raw.as_deref())?
        .get("widget_design_version")
        .cloned();
    if marker.as_ref().is_some_and(|v| v.as_u64() != Some(1)) {
        return Err("Unsupported widget design activation version".into());
    }
    let Some(text) = read(config_path)? else {
        if marker.is_some() {
            return Err(
                "The widget design is missing. Restore its saved file before continuing.".into(),
            );
        }
        return Ok(raw);
    };
    let document = Document::parse(text.as_bytes())?;
    let value = document.project(&config::parse_config_text(raw.as_deref())?)?;
    config::config_text(&value).map(Some)
}

pub fn check(config_path: &Path) -> Result<(), String> {
    projected_text(config_path).map(|_| ())
}

/// Used only to select recovery behavior; it never treats an unreadable
/// design as legacy settings.
pub fn managed(config_path: &Path) -> bool {
    match read(config_path) {
        Ok(Some(_)) | Err(_) => true,
        Ok(None) => config::read_raw_config_text(config_path)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .is_some_and(|v| v.get("widget_design_version").is_some()),
    }
}

pub fn source_conflict(config: &Path) -> bool {
    let check = || -> Result<bool, String> {
        let text = read(config)?.ok_or("No design")?;
        let document = Document::parse(text.as_bytes())?;
        let raw = crate::config::parse_config_text(
            crate::config::read_raw_config_text(config)?.as_deref(),
        )?;
        document.restore_frozen_source(&raw)?;
        Ok(document.check_frozen(&raw).is_err())
    };
    check().unwrap_or(false)
}
