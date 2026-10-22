//! Profile automation on the desktop: the shared wake asks the automator
//! when to look, file changes reload its settings, compositor output events
//! hint that displays changed, and switches run off the UI thread through
//! the profile engine. The widget configuration a switch changes reaches the
//! cards through the usual file watch.
use super::*;
use crate::automation::run::Ran;
use crate::automation::{Due, runtime::Job};

impl Cards {
    pub(super) fn automation_wake(&self) -> Option<Instant> {
        self.automator.wake()
    }

    /// Evaluates if it is time; starts a job for due switches.
    pub(super) fn automation_tick(&mut self) -> Task<Message> {
        let now = Instant::now();
        if !self.automator.due(now) {
            return Task::none();
        }
        match self.automator.evaluate(chrono::Utc::now(), now) {
            Some(job) => switch(job),
            None => Task::none(),
        }
    }

    /// The settings or the Settings hold may have changed.
    pub(super) fn automation_reload(&mut self) -> Task<Message> {
        if self.automator.reload() {
            return self.automation_tick();
        }
        Task::none()
    }

    pub(super) fn automation_displays(&mut self) -> Task<Message> {
        self.automator.displays_changed(Instant::now());
        self.automation_tick()
    }

    pub(super) fn automation_done(&mut self, results: Vec<(Due, Ran)>) -> Task<Message> {
        if self
            .automator
            .finished(results, chrono::Utc::now().timestamp(), Instant::now())
        {
            return self.automation_tick();
        }
        Task::none()
    }
}

/// Runs a job on the blocking pool with the desktop's own store.
fn switch(job: Job) -> Task<Message> {
    cosmic::task::future(async move {
        let due = job.due.clone();
        let results =
            tokio::task::spawn_blocking(move || match crate::desktop::DesktopStore::from_env() {
                Ok(store) => job.run(&store),
                Err(error) => job
                    .due
                    .into_iter()
                    .map(|due| (due, Ran::Failed(error.to_string())))
                    .collect(),
            })
            .await
            .unwrap_or_else(|_| {
                due.into_iter()
                    .map(|due| (due, Ran::Failed("The switch stopped unexpectedly.".into())))
                    .collect()
            });
        Message::Switched(results)
    })
}
