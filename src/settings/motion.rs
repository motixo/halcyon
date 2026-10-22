//! Motion page state: whether apps animate (`desktop::motion`), read when
//! the page opens and written only by the toggle, off the UI thread. The
//! sidebar's own slide is an ordinary Desktop Cards preference.
use crate::desktop::DesktopStore;
use crate::desktop::motion::AppAnimations;

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// The toggle: ask apps to animate (`true`) or reduce motion.
    Animations(bool),
    /// A write finished: the value read back, or why it failed.
    Written(bool, Result<bool, String>),
    Refresh,
}

impl Message {
    pub fn writes(&self) -> bool {
        matches!(self, Self::Animations(_))
    }
}

/// A write to run on the blocking pool.
pub struct Job {
    pub animations: AppAnimations,
    pub on: bool,
}

impl Job {
    pub fn run(self) -> Message {
        Message::Written(self.on, self.animations.write(self.on))
    }
}

#[derive(Default)]
pub struct MotionEditor {
    animations: Option<Result<AppAnimations, String>>,
    /// The value apps see, or why the setting can't be used.
    current: Option<Result<bool, String>>,
    /// A write is running (the value it sets).
    writing: Option<bool>,
}

impl MotionEditor {
    #[cfg(test)]
    pub fn with_store(store: &DesktopStore) -> Self {
        Self {
            animations: Some(Ok(AppAnimations::for_store(store))),
            ..Self::default()
        }
    }
    /// Reads the current value (opening the store the first time).
    pub fn open(&mut self) {
        if self.animations.is_none() {
            self.animations = Some(
                DesktopStore::from_env()
                    .map(|store| AppAnimations::for_store(&store))
                    .map_err(|error| error.to_string()),
            );
        }
        self.refresh();
    }
    pub fn opened(&self) -> bool {
        self.animations.is_some()
    }
    pub fn refresh(&mut self) {
        if self.writing.is_some() {
            return;
        }
        self.current = self.animations.as_ref().map(|animations| match animations {
            Ok(animations) => animations.read(),
            Err(error) => Err(error.clone()),
        });
    }
    /// The value apps see (`None` before the page opened).
    pub fn current(&self) -> Option<&Result<bool, String>> {
        self.current.as_ref()
    }
    /// What the toggle shows: the value being written, else the current.
    pub fn shown(&self) -> Option<bool> {
        self.writing.or_else(|| {
            self.current
                .as_ref()
                .and_then(|current| current.clone().ok())
        })
    }
    /// Why the toggle can't change anything, if it can't.
    pub fn read_only(&self) -> Option<&str> {
        match &self.animations {
            Some(Ok(animations)) => animations.read_only_reason(),
            _ => None,
        }
    }
    pub fn can_change(&self) -> bool {
        self.writing.is_none() && self.read_only().is_none() && matches!(self.current, Some(Ok(_)))
    }
    /// The toggle was used: the write to run, if one is needed and allowed.
    pub fn toggle(&mut self, on: bool) -> Option<Job> {
        if !self.can_change() || self.shown() == Some(on) {
            return None;
        }
        let Some(Ok(animations)) = &self.animations else {
            return None;
        };
        self.writing = Some(on);
        Some(Job {
            animations: animations.clone(),
            on,
        })
    }
    /// A write finished: the status line to show (`Err` for an error).
    pub fn written(&mut self, result: Result<bool, String>) -> Result<String, String> {
        self.writing = None;
        match result {
            Ok(on) => {
                self.current = Some(Ok(on));
                Ok(if on {
                    "Apps animate again".into()
                } else {
                    "Apps now reduce motion".into()
                })
            }
            Err(error) => {
                self.refresh();
                Err(format!("App animations weren’t changed. {error}"))
            }
        }
    }
    pub fn state_key(&self) -> String {
        format!(
            "{:?}:{:?}:{:?}",
            self.current,
            self.writing,
            self.read_only()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::motion::{KEY, SCHEMA};
    use crate::desktop::store::tests::Fixture;

    #[test]
    fn opening_reads_and_a_toggle_writes_once_off_the_ui_thread() {
        let fixture = Fixture::new("motion-editor");
        let dir = fixture.0.join("gsettings").join(SCHEMA);
        std::fs::create_dir_all(&dir).unwrap();
        let mut editor = MotionEditor::with_store(&fixture.store());
        editor.refresh();
        assert_eq!(editor.current(), Some(&Ok(true)));
        assert!(!dir.join(KEY).exists(), "opening wrote nothing");
        assert!(editor.toggle(true).is_none(), "already on");
        let job = editor.toggle(false).unwrap();
        assert_eq!(editor.shown(), Some(false));
        assert!(editor.toggle(true).is_none(), "one write at a time");
        assert!(!dir.join(KEY).exists(), "nothing written before the job");
        let Message::Written(false, result) = job.run() else {
            panic!("a write result");
        };
        assert_eq!(editor.written(result).unwrap(), "Apps now reduce motion");
        assert_eq!(std::fs::read_to_string(dir.join(KEY)).unwrap(), "false");
        assert_eq!(editor.current(), Some(&Ok(false)));
    }

    #[test]
    fn unavailable_or_read_only_settings_offer_no_change() {
        let fixture = Fixture::new("motion-editor-missing");
        let mut editor = MotionEditor::with_store(&fixture.store());
        editor.refresh();
        assert!(matches!(editor.current(), Some(Err(_))));
        assert!(!editor.can_change() && editor.toggle(false).is_none());
        let dir = fixture.0.join("gsettings").join(SCHEMA);
        std::fs::create_dir_all(&dir).unwrap();
        let mut preview = MotionEditor::with_store(&fixture.store().read_only("Preview"));
        preview.refresh();
        assert_eq!(preview.current(), Some(&Ok(true)));
        assert_eq!(preview.read_only(), Some("Preview"));
        assert!(preview.toggle(false).is_none());
        assert!(!dir.join(KEY).exists());
        // A failed write keeps the value apps actually see.
        let mut editor = MotionEditor::with_store(&fixture.store());
        editor.refresh();
        let _job = editor.toggle(false).unwrap();
        assert!(editor.written(Err("dconf is busy".into())).is_err());
        assert_eq!(editor.shown(), Some(true));
    }
}
