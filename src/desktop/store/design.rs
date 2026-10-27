//! The versioned widget document is a participant of the existing journal.
//! Paths come from DesktopStore, never from a persisted record.
use super::*;
use crate::design::{Document, legacy, storage};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignWrite {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    source_repair: bool,
    config_before: Option<String>,
    config_after: Option<String>,
    design_before: Option<String>,
    design_after: Option<String>,
}
impl DesignWrite {
    pub(super) fn validate(&self) -> Result<(), String> {
        for (index, (config, design)) in [
            (&self.config_before, &self.design_before),
            (&self.config_after, &self.design_after),
        ]
        .into_iter()
        .enumerate()
        {
            if config
                .as_ref()
                .is_some_and(|s| s.len() > config::MAX_CONFIG_BYTES)
            {
                return Err("Recorded preferences are too large".into());
            }
            let raw = config::parse_config_text(config.as_deref())?;
            if let Some(text) = design {
                let document = Document::parse(text.as_bytes())?;
                if self.source_repair && index == 0 {
                    config::check_config_value(&raw)?;
                    if document.check_frozen(&raw).is_ok() {
                        return Err("There is no frozen-source conflict".into());
                    }
                } else {
                    document.project(&raw)?;
                }
            } else {
                config::check_config_value(&raw)?;
            }
        }
        if self.source_repair {
            if self.design_before.is_none() || self.design_before != self.design_after {
                return Err("Source recovery cannot replace the applied design".into());
            }
            let document = Document::parse(self.design_before.as_deref().unwrap().as_bytes())?;
            let raw = config::parse_config_text(self.config_before.as_deref())?;
            if document.restore_frozen_source(&raw)?
                != config::parse_config_text(self.config_after.as_deref())?
            {
                return Err("Source recovery changes unrelated preferences".into());
            }
        }
        if let (Some(before), Some(after)) = (&self.design_before, &self.design_after) {
            if Document::parse(before.as_bytes())?.migration
                != Document::parse(after.as_bytes())?.migration
            {
                return Err("The frozen migration source cannot be changed".into());
            }
        }
        if self.design_after.is_none() {
            return Err("Missing applied widget design".into());
        }
        Ok(())
    }
    fn write(&self, store: &DesktopStore, path: &Path) -> Result<(), String> {
        // The locks protect cooperating writers; compare again before publish
        // so a late old client cannot be silently overwritten.
        if config::read_raw_config_text(path)? != self.config_before
            || storage::read(path)? != self.design_before
        {
            return Err("Widget settings changed before publication".into());
        }
        if self.config_after != self.config_before {
            config::write_config_text(path, self.config_after.as_deref())?;
            sync_dir(path.parent().ok_or("No configuration directory")?)
                .map_err(|e| e.to_string())?;
        }
        #[cfg(test)]
        if matches!(
            *store.fault.borrow(),
            Some(Fault::InterruptBetweenDesignFiles)
        ) {
            return Err("Simulated interruption between design files".into());
        }
        let _ = store;
        if self.design_after != self.design_before {
            write_document(path, self.design_after.as_deref())?;
        }
        Ok(())
    }
    fn rollback(
        &self,
        path: &Path,
        change: Option<&WidgetChange>,
        report: &mut RecoveryReport,
    ) -> Result<(), String> {
        let design_now = storage::read(path)?;
        if design_now == self.design_after {
            if self.design_before != self.design_after {
                write_document(path, self.design_before.as_deref())?;
            }
        } else if design_now != self.design_before {
            // A new valid document may have unrelated edits. Only reverse
            // this transaction's own leaves, retaining all other identities.
            let Some(change) = change else {
                return Err(
                    "The migrated design changed again; keep it or export it before recovery"
                        .into(),
                );
            };
            let mut document = Document::parse(
                design_now
                    .as_deref()
                    .ok_or("The widget design was removed")?
                    .as_bytes(),
            )?;
            let after = Document::parse(
                self.design_after
                    .as_deref()
                    .ok_or("Missing recorded design")?
                    .as_bytes(),
            )?;
            if document.migration != after.migration {
                return Err("The migration source changed".into());
            }
            let raw = config::parse_config_text(self.config_after.as_deref())?;
            let current = document.project(&raw)?;
            for field in change.fields.iter().filter(|f| legacy::owned(&f.pointer)) {
                let now = widget::get(&current, &field.pointer);
                if now == field.after {
                    document.put_legacy(&field.pointer, field.before.clone())?;
                    report.restored_fields.push(widget::label(&field.pointer));
                } else if now != field.before {
                    report.kept_fields.push(widget::label(&field.pointer));
                }
            }
            write_document(
                path,
                Some(&String::from_utf8(document.bytes()?).map_err(|e| e.to_string())?),
            )?;
        }
        // Recover local preferences separately. The raw widget members are
        // immutable source provenance and are never restored by a new edit.
        let current = config::read_raw_config_text(path)?;
        if current == self.config_after {
            if self.config_before != self.config_after {
                config::write_config_text(path, self.config_before.as_deref())?;
            }
        } else if current != self.config_before {
            if let Some(change) = change {
                let mut local = change.clone();
                local.before = self.config_before.clone();
                local.after = self.config_after.clone();
                local.fields.retain(|f| !legacy::owned(&f.pointer));
                recover_widget(&local, path, report).map_err(|e| e.to_string())?;
            } else if self.config_before != self.config_after {
                return Err("Preferences changed during migration".into());
            }
        }
        sync_dir(path.parent().ok_or("No configuration directory")?).map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn write_document(path: &Path, text: Option<&str>) -> Result<(), String> {
    let path = storage::path(path);
    match text {
        Some(text) => atomic_write(&path, text.as_bytes(), Some(0o600)),
        None => match fs::remove_file(&path) {
            Ok(()) => sync_dir(path.parent().ok_or("No design directory")?),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        },
    }
    .map_err(|e| format!("Could not write widget design: {e}"))
}

impl DesktopStore {
    pub(super) fn prepare_design(
        &self,
        patch: &WidgetPatch,
        path: &Path,
        conflicts: &mut Vec<String>,
        text: String,
    ) -> Result<Option<WidgetWrite>, DesktopError> {
        self.confine(&storage::path(path))?;
        let invalid = DesktopError::Io;
        let raw_text = config::read_raw_config_text(path).map_err(invalid)?;
        let mut raw = config::parse_config_text(raw_text.as_deref()).map_err(invalid)?;
        let mut document = Document::parse(text.as_bytes()).map_err(invalid)?;
        let before = document.project(&raw).map_err(invalid)?;
        for (pointer, entry) in patch.iter() {
            if widget::get(&before, pointer) != entry.original {
                conflicts.push(widget::label(pointer));
            }
        }
        if !conflicts.is_empty() || patch.is_empty() {
            return Ok(None);
        }
        let mut after = before.clone();
        for (pointer, entry) in patch.changes() {
            widget::put(&mut after, pointer, entry.desired.clone()).map_err(invalid)?;
            if legacy::owned(pointer) {
                document
                    .put_legacy(pointer, entry.desired.clone())
                    .map_err(invalid)?;
            } else {
                widget::put(&mut raw, pointer, entry.desired.clone()).map_err(invalid)?;
            }
        }
        config::check_config_value(&after).map_err(invalid)?;
        let raw_after = if config::parse_config_text(raw_text.as_deref()).map_err(invalid)? == raw {
            raw_text.clone()
        } else {
            Some(config::config_text(&raw).map_err(invalid)?)
        };
        let updated = String::from_utf8(document.bytes().map_err(invalid)?)
            .map_err(|e| invalid(e.to_string()))?;
        let physical = DesignWrite {
            source_repair: false,
            config_before: raw_text,
            config_after: raw_after,
            design_before: Some(text),
            design_after: Some(updated),
        };
        physical.validate().map_err(invalid)?;
        Ok(Some(WidgetWrite {
            path: path.into(),
            design: Some(physical),
            change: WidgetChange {
                before: Some(config::config_text(&before).map_err(invalid)?),
                after: Some(config::config_text(&after).map_err(invalid)?),
                design_v1: true,
                fields: patch
                    .changes()
                    .map(|(pointer, entry)| WidgetFieldChange {
                        pointer: pointer.into(),
                        before: entry.original.clone(),
                        after: entry.desired.clone(),
                    })
                    .collect(),
            },
        }))
    }
    pub(super) fn write_design(&self, path: &Path, change: &DesignWrite) -> Result<(), String> {
        change.write(self, path)
    }
    pub(super) fn recover_design(
        &self,
        path: &Path,
        design: &DesignWrite,
        change: Option<&WidgetChange>,
        report: &mut RecoveryReport,
    ) -> Result<(), DesktopError> {
        self.confine(&storage::path(path))?;
        design
            .rollback(path, change, report)
            .map_err(DesktopError::Io)
    }

    /// Explicit activation. The unchanged legacy widget domain becomes frozen;
    /// old history stays addressable through the semantic field adapter.
    pub fn migrate_design(&self) -> Result<(), DesktopError> {
        self.ensure_writable()?;
        let path = self
            .app_config
            .as_deref()
            .ok_or_else(|| DesktopError::Io("No widget configuration".into()))?;
        self.confine(path)?;
        self.confine(&storage::path(path))?;
        let _config_lock = config::lock_config(path).map_err(DesktopError::Io)?;
        let _lock = self.lock()?;
        if self.journal_path().exists() {
            return Err(DesktopError::RecoveryPending);
        }
        if storage::read(path).map_err(DesktopError::Io)?.is_some() {
            storage::check(path).map_err(DesktopError::Io)?;
            return Ok(());
        }
        // A missing document with an activation marker is damaged state,
        // never permission to remigrate its frozen snapshot.
        storage::check(path).map_err(DesktopError::Io)?;
        let raw = config::read_raw_config_text(path).map_err(DesktopError::Io)?;
        let value = config::parse_config_text(raw.as_deref()).map_err(DesktopError::Io)?;
        config::check_config_value(&value).map_err(DesktopError::Io)?;
        let document = Document::from_legacy(&value).map_err(DesktopError::Io)?;
        document.project(&value).map_err(DesktopError::Io)?;
        let text = String::from_utf8(document.bytes().map_err(DesktopError::Io)?)
            .map_err(|e| DesktopError::Io(e.to_string()))?;
        let mut activated = value.clone();
        activated["widget_design_version"] = 1.into();
        let physical = DesignWrite {
            source_repair: false,
            config_before: raw.clone(),
            config_after: Some(config::config_text(&activated).map_err(DesktopError::Io)?),
            design_before: None,
            design_after: Some(text),
        };
        self.publish_design_boundary(path, physical, "migrate")
    }

    pub fn repair_design_source(&self) -> Result<(), DesktopError> {
        self.ensure_writable()?;
        let path = self
            .app_config
            .as_deref()
            .ok_or_else(|| DesktopError::Io("No widget configuration".into()))?;
        self.confine(path)?;
        self.confine(&storage::path(path))?;
        let _config_lock = config::lock_config(path).map_err(DesktopError::Io)?;
        let _lock = self.lock()?;
        if self.journal_path().exists() {
            return Err(DesktopError::RecoveryPending);
        }
        let text = storage::read(path)
            .map_err(DesktopError::Io)?
            .ok_or_else(|| DesktopError::Io("No applied design".into()))?;
        let document = Document::parse(text.as_bytes()).map_err(DesktopError::Io)?;
        let raw = config::read_raw_config_text(path).map_err(DesktopError::Io)?;
        let value = config::parse_config_text(raw.as_deref()).map_err(DesktopError::Io)?;
        let restored = document
            .restore_frozen_source(&value)
            .map_err(DesktopError::Io)?;
        let physical = DesignWrite {
            source_repair: true,
            config_before: raw,
            config_after: Some(config::config_text(&restored).map_err(DesktopError::Io)?),
            design_before: Some(text.clone()),
            design_after: Some(text),
        };
        self.publish_design_boundary(path, physical, "repair_design")
    }

    fn publish_design_boundary(
        &self,
        path: &Path,
        physical: DesignWrite,
        operation: &str,
    ) -> Result<(), DesktopError> {
        physical.validate().map_err(DesktopError::Io)?;
        let id = format!(
            "design-{}-{}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        );
        let found = self.snapshot_record()?;
        let record = self
            .history_file()?
            .map(|mut file| {
                if physical.source_repair && file.version == 1 {
                    // An old client has replaced the migrated history. Preserve
                    // it in the conflict archive, but never claim those writes
                    // as operations on the current applied document.
                    file = LatestChange {
                        id: id.clone(),
                        owner: Owner::Recovery,
                        label: "Kept widget design".into(),
                        created: chrono::Local::now().to_rfc3339(),
                        change: AppliedChange {
                            id: id.clone(),
                            entries: vec![],
                        },
                        widget: None,
                    }
                    .to_file();
                    file.source = Source::Derived;
                }
                file.version = 2;
                file.transaction_id = Some(id.clone());
                serde_json::to_vec_pretty(&file).map_err(|e| DesktopError::Io(e.to_string()))
            })
            .transpose()?;
        let journal = Journal {
            format: JOURNAL_FORMAT.into(),
            version: 4,
            id,
            operation: operation.into(),
            created: chrono::Local::now().to_rfc3339(),
            entries: vec![],
            widget: None,
            design: Some(physical.clone()),
            undo: Some(UndoBefore { record: found }),
        };
        let journal_bytes =
            serde_json::to_vec_pretty(&journal).map_err(|e| DesktopError::Io(e.to_string()))?;
        if journal_bytes.len() as u64 > MAX_RECORD_BYTES
            || record
                .as_ref()
                .is_some_and(|b| b.len() as u64 > MAX_RECORD_BYTES)
        {
            return Err(DesktopError::Io(
                "Migration is too large to journal; nothing was changed".into(),
            ));
        }
        self.confine(&self.journal_path())?;
        self.confine(&self.undo_path())?;
        super::super::recovery::ensure_locked(
            self,
            super::super::recovery::BaselineKind::OriginalDesktop,
        )?;
        if physical.source_repair {
            let archive = self
                .state_dir
                .join(format!("widget-design-conflict-{}.json", journal.id));
            self.confine(&archive)?;
            let bytes = serde_json::to_vec_pretty(&serde_json::json!({
                "format":"halcyon-widget-source-conflict", "version":1,
                "preferences_original":physical.config_before,
                "undo_original":journal.undo.as_ref().and_then(|u| u.record.as_ref()),
            }))
            .map_err(|e| DesktopError::Io(e.to_string()))?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&archive)
                .map_err(|e| io_error("Could not preserve the old-client changes", e))?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .and_then(|()| sync_dir(&self.state_dir))
                .map_err(|e| io_error("Could not preserve the old-client changes", e))?;
        }
        atomic_write(&self.journal_path(), &journal_bytes, Some(0o600))
            .map_err(|e| io_error("Could not journal migration", e))?;
        #[cfg(test)]
        if matches!(*self.fault.borrow(), Some(Fault::InterruptBeforeWidget)) {
            return Err(DesktopError::Io("Simulated migration interruption".into()));
        }
        // Keep the journal on any interruption. No host desktop runs until
        // recovery has restored or finished this activation.
        physical.write(self, path).map_err(DesktopError::Io)?;
        #[cfg(test)]
        if matches!(*self.fault.borrow(), Some(Fault::InterruptBeforeRecord)) {
            return Err(DesktopError::Io("Simulated migration interruption".into()));
        }
        if let Some(record) = record {
            atomic_write(&self.undo_path(), &record, Some(0o600))
                .map_err(|e| io_error("Could not preserve migration history", e))?;
        }
        #[cfg(test)]
        if matches!(*self.fault.borrow(), Some(Fault::InterruptAfterRecord)) {
            return Err(DesktopError::Io("Simulated migration interruption".into()));
        }
        fs::remove_file(self.journal_path())
            .map_err(|e| io_error("Could not complete migration", e))?;
        sync_dir(&self.state_dir).map_err(|e| io_error("Could not sync migration", e))?;
        ACTION_REVISION.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
