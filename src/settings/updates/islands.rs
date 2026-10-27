//! Optional release companion. PackageKit installs; the existing helper activates.
use super::{
    Failure, Identity, Settings, SettingsMessage, Setup, Task, enrollment, identity, packagekit,
};
use crate::desktop::islands::{CompatibilityStatus, Installation, Provenance};
use std::{path::Path, process::Stdio, sync::Arc, time::Duration};
use tokio::sync::watch;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Preparing,
    Ready,
    Installing,
    Enabling,
    Failed(Failure),
}
#[derive(Default)]
pub struct State {
    pub phase: Phase,
    generation: u64,
    cancellation: Option<watch::Sender<bool>>,
    prepared: Option<Arc<enrollment::Prepared>>,
    expected: Option<Identity>,
}
impl State {
    pub fn busy(&self) -> bool {
        matches!(
            self.phase,
            Phase::Preparing | Phase::Installing | Phase::Enabling
        )
    }
    fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(cancel) = self.cancellation.take() {
            let _ = cancel.send(true);
        }
        self.prepared = None;
        self.expected = None;
        self.phase = Phase::Idle;
    }
}
impl Drop for State {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancellation.take() {
            let _ = cancel.send(true);
        }
    }
}
#[derive(Clone, Debug)]
pub enum Message {
    Prepare,
    Prepared(u64, Result<(Identity, Arc<enrollment::Prepared>), Failure>),
    Cancel,
    Install,
    Installed(u64, Result<Installation, Failure>),
    Enable,
    Enabled(u64, Result<Installation, Failure>),
    Recheck,
}
fn message(value: Message) -> SettingsMessage {
    SettingsMessage::Updates(super::Message::Islands(value))
}
pub(super) fn update(settings: &mut Settings, value: Message) -> Task<SettingsMessage> {
    let state = &mut settings.updates.islands;
    match value {
        Message::Prepare if !state.busy() && settings.updates.setup != Setup::Installing => {
            state.reset();
            state.phase = Phase::Preparing;
            let (sender, cancel) = watch::channel(false);
            state.cancellation = Some(sender);
            let token = state.generation;
            return cosmic::task::future(async move {
                message(Message::Prepared(token, prepare(cancel).await))
            });
        }
        Message::Prepared(token, result)
            if token == state.generation && state.phase == Phase::Preparing =>
        {
            state.cancellation = None;
            match result {
                Ok((expected, prepared)) => {
                    state.expected = Some(expected);
                    state.prepared = Some(prepared);
                    state.phase = Phase::Ready;
                }
                Err(error) => state.phase = Phase::Failed(error),
            }
        }
        Message::Cancel if matches!(state.phase, Phase::Preparing | Phase::Ready) => state.reset(),
        Message::Install if state.phase == Phase::Ready => {
            let (Some(expected), Some(prepared)) = (state.expected.clone(), state.prepared.take())
            else {
                state.phase = Phase::Failed(Failure::InstallationChanged);
                return Task::none();
            };
            state.phase = Phase::Installing;
            let token = state.generation;
            return cosmic::task::future(async move {
                message(Message::Installed(token, install(expected, prepared).await))
            });
        }
        Message::Installed(token, result)
            if token == state.generation && state.phase == Phase::Installing =>
        {
            state.expected = None;
            state.phase = result
                .as_ref()
                .err()
                .cloned()
                .map(Phase::Failed)
                .unwrap_or_default();
            if let Ok(installation) = result {
                settings.desktop.islands.installation = installation;
            }
            return settings.handle(SettingsMessage::RecheckInstallation);
        }
        Message::Enable if !state.busy() && can_enable(&settings.desktop.islands.installation) => {
            state.reset();
            state.phase = Phase::Enabling;
            let token = state.generation;
            return cosmic::task::future(async move {
                message(Message::Enabled(token, enable().await))
            });
        }
        Message::Enabled(token, result)
            if token == state.generation && state.phase == Phase::Enabling =>
        {
            state.phase = result
                .as_ref()
                .err()
                .cloned()
                .map(Phase::Failed)
                .unwrap_or_default();
            if let Ok(installation) = result {
                settings.desktop.islands.installation = installation;
            }
            return settings.handle(SettingsMessage::RecheckInstallation);
        }
        Message::Recheck if !state.busy() => {
            state.reset();
            return settings.handle(SettingsMessage::RecheckInstallation);
        }
        _ => {}
    }
    Task::none()
}
pub fn can_enable(installation: &Installation) -> bool {
    let Installation::Installed {
        provenance: Provenance::Package,
        enabled: false,
        compatible,
        status,
        ..
    } = installation
    else {
        return false;
    };
    // Disabled packages intentionally skip native measurement. Offer the explicit
    // request, then let the guarded helper qualify/stage it under its own lock.
    // Failure/ownership warnings must not become a generic retry button.
    matches!(
        (compatible, status),
        (Some(true), None | Some(CompatibilityStatus::Disabled))
            | (None, Some(CompatibilityStatus::Disabled))
    )
}
async fn discover() -> Result<Installation, Failure> {
    tokio::task::spawn_blocking(|| {
        crate::desktop::DesktopStore::from_env()
            .map(|store| Installation::discover(&store))
            .map_err(|_| Failure::Failed)
    })
    .await
    .map_err(|_| Failure::Failed)?
}
async fn candidate() -> Result<Identity, Failure> {
    // Never let fixtures, screenshots, or arbitrary source builds reach the host installer.
    if identity::isolated() {
        return Err(Failure::InstallationChanged);
    }
    let current = identity::inspect().await;
    if !current.can_install_islands() {
        return Err(Failure::ReleaseRequired);
    }
    if !identity::islands_absent(Path::new("/")) || discover().await? != Installation::Absent {
        return Err(Failure::InstallationChanged);
    }
    Ok(current)
}
async fn prepare(
    cancel: watch::Receiver<bool>,
) -> Result<(Identity, Arc<enrollment::Prepared>), Failure> {
    let mut interrupted = cancel.clone();
    let operation = async {
        let expected = candidate().await?;
        let prepared = enrollment::prepare_kind(enrollment::Kind::Islands, cancel).await?;
        if !expected.same_package(&candidate().await?) {
            return Err(Failure::InstallationChanged);
        }
        packagekit::prepare_islands(&prepared).await?;
        Ok((expected, prepared))
    };
    tokio::select! {
        biased;
        _ = packagekit::cancelled(&mut interrupted) => Err(Failure::Cancelled),
        result = operation => result,
    }
}
async fn install(
    expected: Identity,
    prepared: Arc<enrollment::Prepared>,
) -> Result<Installation, Failure> {
    if !expected.same_package(&candidate().await?) {
        return Err(Failure::InstallationChanged);
    }
    packagekit::install_islands(prepared).await?;
    if identity::islands_version(Path::new("/")).as_deref()
        != Some(enrollment::islands_version()?.as_str())
    {
        return Err(Failure::InstallationChanged);
    }
    let result = discover().await?;
    if !matches!(
        result,
        Installation::Installed {
            provenance: Provenance::Package,
            ..
        }
    ) {
        return Err(Failure::InstallationChanged);
    }
    Ok(result)
}
async fn enable() -> Result<Installation, Failure> {
    if identity::isolated()
        || !identity::inspect().await.can_install_islands()
        || identity::islands_version(Path::new("/")).as_deref()
            != Some(enrollment::islands_version()?.as_str())
        || !identity::islands_cli_verified()
        || !can_enable(&discover().await?)
    {
        return Err(Failure::InstallationChanged);
    }
    // No sudo, panel restart, latch removal, or alternate activation path.
    // The helper owns locking, source/ABI admission, ownership and rollback.
    let status = tokio::time::timeout(
        Duration::from_secs(60),
        tokio::process::Command::new("/usr/bin/halcyon-islands")
            .arg("enable")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await
    .map_err(|_| Failure::Timeout)?
    .map_err(|_| Failure::Failed)?;
    if !status.success() {
        return Err(Failure::Activation);
    }
    let result = discover().await?;
    if !matches!(
        result,
        Installation::Installed {
            provenance: Provenance::Package,
            enabled: true,
            ..
        }
    ) {
        return Err(Failure::Activation);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn isolated_flows_cannot_download_install_or_activate() {
        assert_eq!(candidate().await, Err(Failure::InstallationChanged));
        assert_eq!(enable().await, Err(Failure::InstallationChanged));
    }
    #[test]
    fn activation_requires_disabled_package_and_defers_unknown_measurement_to_helper() {
        for provenance in [Provenance::Package, Provenance::Manual] {
            for enabled in [false, true] {
                for compatible in [None, Some(false), Some(true)] {
                    for status in [
                        None,
                        Some(CompatibilityStatus::Disabled),
                        Some(CompatibilityStatus::UpdateNeeded),
                    ] {
                        let state = Installation::Installed {
                            provenance,
                            enabled,
                            compatible,
                            status,
                            diagnostics: String::new(),
                        };
                        assert_eq!(
                            can_enable(&state),
                            provenance == Provenance::Package
                                && !enabled
                                && (compatible == Some(true) && status.is_none()
                                    || compatible != Some(false)
                                        && status == Some(CompatibilityStatus::Disabled))
                        );
                    }
                }
            }
        }
    }
    #[tokio::test]
    async fn cancellation_invalidates_in_flight_results() {
        let mut state = State::default();
        let token = state.generation;
        let (sender, mut receiver) = watch::channel(false);
        state.cancellation = Some(sender);
        state.phase = Phase::Preparing;
        state.reset();
        receiver.changed().await.unwrap();
        assert!(*receiver.borrow());
        assert_ne!(state.generation, token);
        assert_eq!(state.phase, Phase::Idle);
    }
}
