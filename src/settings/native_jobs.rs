//! Blocking add-on checks and native writes run on the task executor. The UI
//! keeps a display snapshot; the worker owns the model and any live lease.
use super::*;
use crate::desktop::islands::Installation;
use std::sync::{Arc, Mutex};

pub struct Delivery(Arc<Mutex<Option<Done>>>);
impl Clone for Delivery {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl std::fmt::Debug for Delivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Native completion")
    }
}
pub enum Done {
    Desktop(desktop::DesktopEditor, Option<cosmic::widget::Id>),
    Profiles(profiles::ProfilesEditor, profiles::Outcome),
    Wizard(wizard::Wizard, wizard::Outcome),
    Undo(
        Result<(), String>,
        Option<crate::desktop::store::UndoReview>,
    ),
    Material(Result<(), String>),
    Recovery(legacy::Recovery, Result<(), String>),
    Failed(String),
}
fn task(run: impl FnOnce() -> Done + Send + 'static) -> Task<Message> {
    cosmic::task::future(async move {
        let done = tokio::task::spawn_blocking(run)
            .await
            .unwrap_or_else(|e| Done::Failed(e.to_string()));
        Message::NativeDone(Delivery(Arc::new(Mutex::new(Some(done)))))
    })
}
impl Settings {
    pub(super) fn native_worker_needed(&self) -> bool {
        crate::desktop::DesktopStore::from_env().is_ok_and(|s| Installation::needs_worker(&s))
    }
    fn begin_native(&mut self) {
        self.native_busy = true;
        self.status = "Applying desktop settings…".into();
        self.error = false;
    }
    pub(super) fn dispatch_native(&mut self, message: &Message) -> Option<Task<Message>> {
        if !self.native_worker_needed() {
            return None;
        }
        if self.native_busy
            && matches!(
                message,
                Message::DesktopEditor(desktop::Message::Islands(desktop::islands::Edit::Sync))
            )
        {
            return Some(Task::none());
        }
        // Even a cheap preference write would wait on the worker's config
        // lock during authoritative discovery. Defer its completed action too.
        if self.native_busy
            && (matches!(
                message,
                Message::DesktopEditor(_)
                    | Message::Profiles(_)
                    | Message::Wizard(_)
                    | Message::UndoDesktop
                    | Message::UndoDesktopRestore(_)
                    | Message::UndoPreferences
                    | Message::ResolveUndo(_)
                    | Message::MaterialChoose(_)
                    | Message::MaterialSave
                    | Message::MaterialTarget(_)
                    | Message::MaterialValue(..)
                    | Message::Page(_)
                    | Message::FlushEdits
                    | Message::Window(WindowControl::Close)
                    | Message::ResetThemeSurface(_)
                    | Message::RecoveryApply
                    | Message::RecoveryApplyOne(_)
            ) || desktop_write_message(message)
                || direct::completion(message).is_some()
                || !self.preference_scope(message).is_empty())
        {
            self.native_deferred.push_back(message.clone());
            return Some(Task::none());
        }
        match message {
            Message::DesktopEditor(edit) if self.page == Page::DesktopDock => {
                let display = self.desktop.display_snapshot();
                let mut editor = std::mem::replace(&mut self.desktop, display);
                let edit = edit.clone();
                let start = if editor.selected == crate::desktop::panels::PanelKind::Dock {
                    8
                } else {
                    16
                };
                self.native_property |= match edit {
                    desktop::Message::Opacity(_) => 1 << start,
                    desktop::Message::Radius(_) | desktop::Message::RadiusChoice(_) => {
                        1 << (start + 1)
                    }
                    desktop::Message::Padding(_) => 1 << (start + 2),
                    desktop::Message::Spacing(_) => 1 << (start + 3),
                    _ => 0,
                };
                let property = direct::property(self, message);
                if property != 0 {
                    self.native_property = 0;
                }
                let complete = direct::completion(message).is_some();
                let label = direct::label(message);
                self.begin_native();
                Some(task(move || {
                    use crate::desktop::actions;
                    let prior = actions::history_store()
                        .ok()
                        .and_then(|s| s.latest_change().ok())
                        .flatten()
                        .map(|v| v.id);
                    let focus = actions::in_action(&label, || {
                        actions::property(property, || {
                            let focus = editor.update(edit);
                            if complete {
                                editor.update(desktop::Message::Apply);
                            }
                            focus
                        })
                    });
                    if property != 0
                        && editor.writable()
                        && !matches!(editor.notice, Some(desktop::Notice::Error(_)))
                    {
                        let current = actions::history_store()
                            .ok()
                            .and_then(|s| s.latest_change().ok())
                            .flatten()
                            .map(|v| v.id);
                        if current == prior {
                            if let Err(e) = actions::complete_property(property, &label) {
                                editor.notice = Some(desktop::Notice::Error(e));
                            }
                        }
                    }
                    Done::Desktop(editor, focus)
                }))
            }
            Message::Profiles(edit)
                if edit.writes() && matches!(self.page, Page::DesktopProfiles | Page::Themes) =>
            {
                // Preserve the same page/scope admission as the synchronous path.
                if self.page == Page::Themes
                    && matches!(edit, profiles::Message::Apply)
                    && self.profiles.profile().is_some_and(|p| !p.appearance_only)
                {
                    return None;
                }
                self.flush_edits();
                if self.native_busy {
                    self.native_deferred.push_back(message.clone());
                    return Some(Task::none());
                }
                let mut editor = self.profiles.clone();
                let edit = edit.clone();
                self.begin_native();
                Some(task(move || {
                    let result = editor.update(edit);
                    Done::Profiles(editor, result)
                }))
            }
            Message::RecoveryApply | Message::RecoveryApplyOne(_) => {
                let mut recovery = self.recovery_sheet.clone();
                let index = match message {
                    Message::RecoveryApplyOne(i) => Some(*i),
                    _ => None,
                };
                self.begin_native();
                Some(task(move || {
                    let result = match index {
                        Some(i) => recovery.apply_one(i),
                        None => recovery.apply(),
                    };
                    Done::Recovery(recovery, result)
                }))
            }
            Message::ResetThemeSurface(domain) => {
                let mut editor = self.profiles.clone();
                let domain = *domain;
                self.begin_native();
                Some(task(move || {
                    editor.open();
                    editor.update(profiles::Message::ResetSurface(domain));
                    let result = editor.update(profiles::Message::Apply);
                    Done::Profiles(editor, result)
                }))
            }
            Message::Wizard(edit) => {
                let mut wizard = self.wizard.as_ref()?.clone();
                let edit = edit.clone();
                let config = self.config.clone();
                self.begin_native();
                Some(task(move || {
                    let store = crate::desktop::DesktopStore::from_env().ok();
                    let result = wizard.update(edit, store.as_ref(), &config);
                    Done::Wizard(wizard, result)
                }))
            }
            _ => None,
        }
    }
    pub(super) fn background_undo(
        &mut self,
        shown: Option<String>,
        review: Option<(crate::desktop::store::UndoReview, bool)>,
    ) -> Task<Message> {
        self.begin_native();
        task(move || {
            let mut conflict = None;
            let result = crate::desktop::actions::history_store()
                .and_then(|store| {
                    if let Some((review, restore)) = review {
                        return if restore {
                            store.accept_undo_review(&review)
                        } else {
                            store.keep_undo_current(&review)
                        };
                    }
                    let id = shown.ok_or_else(|| {
                        crate::desktop::DesktopError::UndoUnavailable(
                            "No settings action to undo".into(),
                        )
                    })?;
                    let result = store.undo_latest(&id);
                    if matches!(
                        result,
                        Err(crate::desktop::DesktopError::Conflict(_)
                            | crate::desktop::DesktopError::FieldConflict { .. })
                    ) {
                        conflict = store.review_undo(&id).ok();
                    }
                    result
                })
                .map(|_| ())
                .map_err(|e| e.to_string());
            Done::Undo(result, conflict)
        })
    }
    pub(super) fn background_material(
        &mut self,
        target: crate::materials::Target,
        finish: Option<crate::materials::Finish>,
        expected: Option<crate::materials::Materials>,
    ) {
        self.begin_native();
        self.native_tasks.push(task(move || {
            Done::Material(
                crate::desktop::actions::history_store()
                    .map_err(|e| e.to_string())
                    .and_then(|s| {
                        crate::desktop::materials::apply_from(&s, target, finish, expected)
                    }),
            )
        }));
    }
    pub(super) fn finish_native(&mut self, delivery: Delivery) -> Task<Message> {
        let Some(done) = delivery.0.lock().unwrap().take() else {
            return Task::none();
        };
        self.native_busy = false;
        self.status.clear();
        let task = match done {
            Done::Desktop(mut editor, focus) => {
                editor.islands.installation = self.desktop.islands.installation.clone();
                self.desktop = editor;
                let refresh = self.after_desktop_change();
                Task::batch([
                    refresh,
                    focus
                        .map(cosmic::widget::button::focus)
                        .unwrap_or_else(Task::none),
                ])
            }
            Done::Profiles(editor, outcome) => {
                self.profiles = editor;
                self.profile_outcome(outcome)
            }
            Done::Wizard(wizard, outcome) => {
                self.wizard = Some(wizard);
                self.wizard_outcome(outcome)
            }
            Done::Undo(result, review) => {
                self.undo_review = review;
                match result {
                    Ok(()) => {
                        self.error = false;
                        #[cfg(test)]
                        {
                            self.preference_undo = None;
                        }
                    }
                    Err(e) => self.failed(e),
                }
                self.after_desktop_change()
            }
            Done::Material(result) => {
                match result {
                    Ok(()) => self.saved(),
                    Err(e) => self.failed(e),
                }
                self.after_desktop_change()
            }
            Done::Recovery(recovery, result) => {
                self.recovery_sheet = recovery;
                match result {
                    Ok(()) => self.error = false,
                    Err(e) => self.failed(e),
                }
                self.after_desktop_change()
            }
            Done::Failed(e) => {
                self.failed(e);
                Task::none()
            }
        };
        self.last_desktop_change = navigation::latest_desktop_change();
        self.checkpoint_session_drafts();
        let mut tasks = vec![task];
        while !self.native_busy {
            let Some(message) = self.native_deferred.pop_front() else {
                break;
            };
            tasks.push(self.update(message));
        }
        Task::batch(tasks)
    }
}
