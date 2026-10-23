use super::*;
impl Settings {
    pub(super) fn refresh_usage(&mut self) {
        if self.usage_busy {
            return;
        }
        match crate::usage::preference() {
            Ok(enabled) => self.usage_enabled = enabled,
            Err(error) => {
                self.usage_enabled = false;
                self.usage_error = Some(error);
            }
        }
    }
    pub(super) fn change_usage(&mut self, enabled: bool) -> Task<Message> {
        if self.usage_busy {
            return Task::none();
        }
        if !enabled {
            crate::usage::pause();
        }
        self.usage_busy = true;
        self.usage_error = None;
        cosmic::task::future(async move {
            let result = tokio::task::spawn_blocking(move || crate::usage::set_preference(enabled))
                .await
                .unwrap_or_else(|_| Err("Could not save reporting choice".into()));
            Message::UsageSaved(result)
        })
    }
}
