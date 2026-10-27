use super::*;
use crate::{
    config::{self, Config},
    desktop::actions,
};
use serde_json::Value;
use std::{
    io::{Read, Write},
    path::Path,
};

/// Existing journal recovery runs before either the service or Settings starts.
/// It does not activate migration; activation remains an explicit action.
pub fn startup() -> Result<(), String> {
    let store = actions::history_store().map_err(|e| e.to_string())?;
    if let Some(journal) = store.pending_recovery().map_err(|e| e.to_string())? {
        if journal.version == 4 {
            store.recover().map_err(|e| e.to_string())?;
        }
    }
    let path = Config::path().ok_or("No configuration directory")?;
    if storage::managed(&path) {
        storage::check(&path)?;
    }
    Ok(())
}

/// Export original bytes even when their schema cannot be understood. The
/// caller's chosen destination must be new; it can never replace applied data.
pub fn export(path: &Path, destination: &Path, legacy: bool) -> Result<(), String> {
    let store = actions::preference_store(path).map_err(|e| e.to_string())?;
    if let Some(root) = store.fixture_root() {
        let root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
        let parent = std::fs::canonicalize(destination.parent().ok_or("No export directory")?)
            .map_err(|e| e.to_string())?;
        if !parent.starts_with(root) {
            return Err("The export must stay inside the preview directory".into());
        }
    }
    let _lock = config::lock_config(path)?;
    let bytes = if legacy {
        let text = config::read_config_text(path)?;
        let mut raw = config::parse_config_text(text.as_deref())?;
        if crate::config::check_config_value(&raw)?.recipes != crate::recipes::Applied::default() {
            return Err("Personal widget recipes cannot be exported as legacy settings. Use built-in widgets first.".into());
        }
        raw.as_object_mut()
            .ok_or("Invalid preferences")?
            .remove("widget_design_version");
        // `project` already requires precisely the representable reserved set.
        // This is a private downgrade file, not a public share card/profile.
        config::config_text(&raw)?.into_bytes()
    } else {
        storage::read_bytes(path)?.ok_or("No widget design to export")?
    };
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}

pub fn run(command: &str, mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let path = Config::path().ok_or("No configuration directory")?;
    let destination = if matches!(command, "--export-design" | "--export-legacy-design") {
        Some(args.next().ok_or("Choose a new export file")?)
    } else {
        None
    };
    if args.next().is_some() {
        return Err("Unexpected extra arguments".into());
    }
    match command {
        "--read-widget-config" => {
            let text = config::read_config_text(&path)?.unwrap_or_else(|| "{}".into());
            println!("{text}");
        }
        "--edit-widget-config" => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take((config::MAX_CONFIG_BYTES * 2 + 1024) as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > config::MAX_CONFIG_BYTES * 2 + 1024 {
                return Err("Controller request is too large".into());
            }
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Request {
                before: Value,
                after: Value,
            }
            let request: Request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            config::check_config_value(&request.before)?;
            config::check_config_value(&request.after)?;
            let patch = actions::preference_patch(&request.before, &request.after)?;
            let store = actions::preference_store(&path).map_err(|e| e.to_string())?;
            actions::commit(
                &store,
                &crate::desktop::DesktopPatch::new(),
                Some(&patch),
                crate::desktop::Owner::Preferences,
                "Controller settings",
                crate::desktop::store::Source::Manual,
            )
            .map_err(|e| e.to_string())?;
        }
        "--migrate-design" => {
            actions::preference_store(&path)
                .map_err(|e| e.to_string())?
                .migrate_design()
                .map_err(|e| e.to_string())?;
        }
        "--keep-widget-design" => {
            actions::preference_store(&path)
                .map_err(|e| e.to_string())?
                .repair_design_source()
                .map_err(|e| e.to_string())?;
        }
        "--recover-design" => {
            startup()?;
        }
        "--export-design" | "--export-legacy-design" => {
            let destination = destination.ok_or("Choose a new export file")?;
            export(
                &path,
                Path::new(&destination),
                command == "--export-legacy-design",
            )?;
        }
        _ => return Err("Unknown widget design command".into()),
    }
    Ok(())
}

pub fn pending_problem() -> Option<String> {
    let store = actions::history_store().ok()?;
    match store.pending_recovery() {
        Ok(Some(journal)) if journal.version == 4 => {
            Some("A widget design change was interrupted. Recover it before continuing.".into())
        }
        Err(error) if Config::path().as_deref().is_some_and(storage::managed) => {
            Some(error.to_string())
        }
        _ => None,
    }
}
