//! Explicit update discovery and verified source enrollment. The OS owns authentication and installation.
mod enrollment;
mod identity;
pub(super) mod islands;
mod packagekit;
#[cfg(test)]
mod tests;

use super::{Message as SettingsMessage, Settings, Task};
pub(super) use identity::{Enrollment, Identity, UpdateChannel};
use std::time::Duration;
use tokio::sync::watch;

const RELEASES: &str = "https://github.com/profilaz3/halcyon/releases";
const INFORMATION: &str = "https://github.com/profilaz3/halcyon/blob/main/docs/DISTRIBUTION.md";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    MissingPackageKit,
    Offline,
    Timeout,
    Cancelled,
    Authorization,
    Untrusted,
    InvalidResponse,
    Failed,
    InstallationChanged,
    Busy,
    DependencyChanges,
    ReleaseRequired,
    Activation,
}
impl Failure {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ReleaseRequired => {
                "Install the Halcyon release package to add Islands here. Source installations use the development guide."
            }
            Self::Activation => {
                "Islands could not be enabled. Check Islands diagnostics and the installation guide before retrying."
            }
            Self::MissingPackageKit => "System update service unavailable.",
            Self::Offline => "Couldn’t reach the package sources. Try again when online.",
            Self::Timeout => "The system did not finish in time. Recheck its status.",
            Self::Cancelled => "Operation cancelled.",
            Self::Authorization => "System authorization was not granted.",
            Self::Untrusted => "Package sources could not be verified.",
            Self::InvalidResponse => "The system returned an incomplete update result.",
            Self::Failed => "The system could not complete this operation.",
            Self::InstallationChanged => "Installation changed. Reopen Settings to check it again.",
            Self::Busy => {
                "Another system installation is running. Wait for it to finish, then check again."
            }
            Self::DependencyChanges => {
                "This add-on needs additional system changes. Use the installation guide to review them in your package manager."
            }
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Check {
    #[default]
    NotChecked,
    Checking,
    Current,
    Available(String),
    Failed(Failure),
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Setup {
    #[default]
    Idle,
    Preparing,
    Ready,
    Installing,
    Failed(Failure),
    Enrolled,
}
#[derive(Clone, Copy, Debug, Default)]
pub enum InstallPhase {
    #[default]
    Inspecting,
    Authorizing,
    Installing,
    Verifying,
}
#[derive(Clone, Debug)]
pub enum Message {
    Islands(islands::Message),
    Inspect,
    Inspected(u64, Identity),
    Check,
    Cancel,
    Checked(u64, Result<Option<String>, Failure>),
    OpenStore,
    StoreOpened(u64, Result<(), String>),
    Releases,
    Information,
    Prepare,
    Prepared(u64, Result<std::sync::Arc<enrollment::Prepared>, Failure>),
    Install,
    Installed(u64, Result<Identity, Failure>),
    InstallPhase(u64, InstallPhase),
    CopyInstructions,
    Notes,
}
#[derive(Default)]
pub struct Updates {
    pub islands: islands::State,
    pub identity: Option<Identity>,
    pub check: Check,
    pub store_error: Option<String>,
    pub store_busy: bool,
    pub setup: Setup,
    pub install_phase: InstallPhase,
    pub prepared: Option<std::sync::Arc<enrollment::Prepared>>,
    pub last_checked: Option<chrono::DateTime<chrono::Local>>,
    pub copied: bool,
    generation: u64,
    cancellation: Option<watch::Sender<bool>>,
}
impl Drop for Updates {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancellation.take() {
            let _ = cancel.send(true);
        }
    }
}
impl Updates {
    pub fn cancel(&mut self) {
        // Installation belongs to the OS. Closing this window must not kill it.
        if self.setup == Setup::Installing {
            return;
        }
        if self.setup == Setup::Preparing {
            self.setup = Setup::Idle;
        }
        self.generation = self.generation.wrapping_add(1);
        if let Some(cancel) = self.cancellation.take() {
            let _ = cancel.send(true);
        }
        if self.check == Check::Checking {
            self.check = Check::Failed(Failure::Cancelled);
        }
        self.store_busy = false;
    }
    fn begin(&mut self) -> (u64, watch::Receiver<bool>) {
        self.cancel();
        let (sender, receiver) = watch::channel(false);
        self.cancellation = Some(sender);
        self.check = Check::Checking;
        (self.generation, receiver)
    }
    fn finish(&mut self, token: u64, result: Result<Option<String>, Failure>) {
        if token != self.generation || self.check != Check::Checking {
            return;
        }
        self.cancellation = None;
        if result.is_ok() {
            self.last_checked = Some(chrono::Local::now());
        }
        self.check = match result {
            Ok(Some(version)) => Check::Available(version),
            Ok(None) => Check::Current,
            Err(error) => Check::Failed(error),
        };
    }
}
impl Settings {
    pub(super) fn update_updates(&mut self, message: Message) -> Task<SettingsMessage> {
        match message {
            Message::Islands(message) => return islands::update(self, message),
            Message::Inspect => {
                // Focus changes during PolicyKit authentication must not cancel
                // the check or start another local measurement.
                if self.updates.check == Check::Checking
                    || self.updates.store_busy
                    || matches!(self.updates.setup, Setup::Preparing | Setup::Installing)
                {
                    return Task::none();
                }
                self.updates.cancel();
                self.updates.identity = None;
                self.updates.setup = Setup::Idle;
                self.updates.check = Check::NotChecked;
                self.updates.store_error = None;
                let token = self.updates.generation;
                return cosmic::task::future(async move {
                    SettingsMessage::Updates(Message::Inspected(token, identity::inspect().await))
                });
            }
            Message::Inspected(token, value) if token == self.updates.generation => {
                self.updates.identity = Some(value);
            }
            Message::Check => {
                let Some(identity) = self.updates.identity.clone().filter(|i| i.can_check()) else {
                    return Task::none();
                };
                if self.updates.check == Check::Checking {
                    return Task::none();
                }
                let (token, cancel) = self.updates.begin();
                return cosmic::task::future(async move {
                    let result = check_installation(identity, cancel).await;
                    SettingsMessage::Updates(Message::Checked(token, result))
                });
            }
            Message::Cancel => self.updates.cancel(),
            Message::Checked(token, result) => self.updates.finish(token, result),
            Message::OpenStore => {
                if self.updates.store_busy
                    || !self
                        .updates
                        .identity
                        .as_ref()
                        .is_some_and(|i| i.can_check() && i.store.is_some())
                {
                    return Task::none();
                }
                self.updates.store_busy = true;
                self.updates.store_error = None;
                let token = self.updates.generation;
                return cosmic::task::future(async move {
                    SettingsMessage::Updates(Message::StoreOpened(
                        token,
                        identity::open_store().await,
                    ))
                });
            }
            Message::StoreOpened(token, result) if token == self.updates.generation => {
                self.updates.store_busy = false;
                self.updates.store_error = result.err();
            }
            Message::Releases => {
                return super::run_command(
                    "/usr/bin/xdg-open".into(),
                    vec![RELEASES.into()],
                    "Releases opened",
                );
            }
            Message::Prepare => {
                if !self
                    .updates
                    .identity
                    .as_ref()
                    .is_some_and(Identity::can_enroll)
                    || matches!(self.updates.setup, Setup::Preparing | Setup::Installing)
                {
                    return Task::none();
                }
                self.updates.cancel();
                self.updates.prepared = None;
                self.updates.copied = false;
                let (sender, cancel) = watch::channel(false);
                self.updates.cancellation = Some(sender);
                self.updates.setup = Setup::Preparing;
                let token = self.updates.generation;
                return cosmic::task::future(async move {
                    SettingsMessage::Updates(Message::Prepared(
                        token,
                        enrollment::prepare(cancel).await,
                    ))
                });
            }
            Message::Prepared(token, result)
                if token == self.updates.generation && self.updates.setup == Setup::Preparing =>
            {
                self.updates.cancellation = None;
                match result {
                    Ok(files) => {
                        self.updates.prepared = Some(files);
                        self.updates.setup = Setup::Ready;
                    }
                    Err(error) => self.updates.setup = Setup::Failed(error),
                }
            }
            Message::Install => {
                if !matches!(self.updates.setup, Setup::Ready | Setup::Failed(_)) {
                    return Task::none();
                }
                let Some(files) = self.updates.prepared.clone() else {
                    return Task::none();
                };
                let Some(expected) = self.updates.identity.clone().filter(Identity::can_enroll)
                else {
                    return Task::none();
                };
                self.updates.setup = Setup::Installing;
                self.updates.install_phase = InstallPhase::Inspecting;
                let token = self.updates.generation;
                return cosmic::task::stream(cosmic::iced::stream::channel(
                    8,
                    async move |mut output| {
                        use cosmic::iced::futures::SinkExt;
                        let (progress, mut receive) = tokio::sync::mpsc::unbounded_channel();
                        let work = async move {
                            let current = identity::inspect().await;
                            if !expected.same_enrollment_candidate(&current) {
                                return Err(Failure::InstallationChanged);
                            }
                            packagekit::install_bootstrap(files, Some(progress.clone())).await?;
                            let _ = progress.send(InstallPhase::Verifying);
                            let current = identity::inspect().await;
                            if !current.can_check() {
                                return Err(Failure::InstallationChanged);
                            }
                            Ok(current)
                        };
                        tokio::pin!(work);
                        loop {
                            tokio::select! {
                                result=&mut work => { let _=output.send(SettingsMessage::Updates(Message::Installed(token,result))).await; break; },
                                Some(phase)=receive.recv() => { if output.send(SettingsMessage::Updates(Message::InstallPhase(token,phase))).await.is_err() { break; } },
                            }
                        }
                    },
                ));
            }
            Message::InstallPhase(token, phase)
                if token == self.updates.generation && self.updates.setup == Setup::Installing =>
            {
                self.updates.install_phase = phase
            }
            Message::Installed(token, result) if token == self.updates.generation => match result {
                Ok(identity) => {
                    self.updates.identity = Some(identity);
                    self.updates.setup = Setup::Enrolled;
                    self.updates.prepared = None;
                }
                Err(error) => self.updates.setup = Setup::Failed(error),
            },
            Message::CopyInstructions => {
                if let Some(files) = &self.updates.prepared {
                    // Manual installation may outlive this window too.
                    files
                        .handed_off
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    self.updates.copied = true;
                    return cosmic::iced::clipboard::write(files.instructions());
                }
            }
            Message::Notes => {
                if let Some(url) = self.updates.notes_url() {
                    return super::run_command(
                        "/usr/bin/xdg-open".into(),
                        vec![url],
                        "Release notes opened",
                    );
                }
            }
            Message::Information => {
                return super::run_command(
                    "/usr/bin/xdg-open".into(),
                    vec![INFORMATION.into()],
                    "Update information opened",
                );
            }
            _ => {}
        }
        Task::none()
    }
}
async fn check_installation(
    identity: Identity,
    mut cancel: watch::Receiver<bool>,
) -> Result<Option<String>, Failure> {
    // Revalidate before contacting the bus, and before accepting its result.
    // No fixture/capture can ever fall through to the host system bus.
    if identity::isolated() {
        return Err(Failure::InstallationChanged);
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    verify_identity(&identity, &mut cancel, deadline).await?;
    let result = packagekit::check_system(
        cancel.clone(),
        deadline.saturating_duration_since(tokio::time::Instant::now()),
    )
    .await?;
    verify_identity(&identity, &mut cancel, deadline).await?;
    tokio::select! {
        biased;
        _ = packagekit::cancelled(&mut cancel) => Err(Failure::Cancelled),
        result = tokio::time::timeout_at(deadline,packagekit::candidate(&identity,&result)) => result.unwrap_or(Err(Failure::Timeout)),
    }
}
async fn verify_identity(
    identity: &Identity,
    cancel: &mut watch::Receiver<bool>,
    deadline: tokio::time::Instant,
) -> Result<(), Failure> {
    let current = tokio::select! {
        biased;
        _ = packagekit::cancelled(cancel) => return Err(Failure::Cancelled),
        current = tokio::time::timeout_at(deadline,identity::inspect()) => current.map_err(|_|Failure::Timeout)?,
    };
    if identity.same_installation(&current) {
        Ok(())
    } else {
        Err(Failure::InstallationChanged)
    }
}

/// The existing helper's selected generation must match the installed payload.
/// A newer archive candidate or an unstaged installed generation isn't ready.
pub(super) fn pending_islands(installation: &crate::desktop::islands::Installation) -> bool {
    use crate::desktop::islands::{CompatibilityStatus, Installation};
    let Installation::Installed {
        enabled: true,
        compatible: Some(true),
        status: Some(CompatibilityStatus::LogoutRequired),
        diagnostics,
        ..
    } = installation
    else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(diagnostics) else {
        return false;
    };
    let Some(installed) = value["installed_generation"].as_str() else {
        return false;
    };
    installed.len() == 64
        && installed.bytes().all(|c| c.is_ascii_hexdigit())
        && value["active_generation"].as_str() == Some(installed)
}

impl Updates {
    pub fn notes_url(&self) -> Option<String> {
        let identity = self
            .identity
            .as_ref()
            .filter(|i| i.can_check() || i.can_enroll())?;
        let version = match &self.check {
            Check::Available(v) => v,
            _ => identity.installed_version.as_ref()?,
        };
        // Only stable upstream x.y.z tags; do not invent tags for epochs or prereleases.
        let upstream = version.split_once('-').map_or(version.as_str(), |(v, _)| v);
        let parts: Vec<_> = upstream.split('.').collect();
        (parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|c| c.is_ascii_digit())))
        .then(|| format!("{RELEASES}/tag/v{upstream}"))
    }
}

impl Updates {
    /// Isolated native captures only. Checks/installations still reject fixtures.
    pub(super) fn capture_fixture(&mut self, state: &str) {
        if !super::capture_commands() || !identity::isolated() {
            return;
        }
        let mut identity = Identity {
            installed_version: Some("0.2.5".into()),
            architecture: Some("amd64".into()),
            ..Default::default()
        };
        match state {
            "manual" => {
                identity.channel = UpdateChannel::ManualDeb;
                identity.enrollment = Enrollment::Available;
                identity.reason = "System updates are not enabled.";
                identity.evidence = Some("capture fixture".into());
            }
            "enrolled" | "available" => {
                identity.channel = UpdateChannel::Apt;
                identity.enrollment = Enrollment::Enrolled;
                identity.reason = "System updates enabled.";
                identity.evidence = Some("capture fixture".into());
                identity.store = Some("/fixture/store.desktop".into());
            }
            "disabled" => {
                identity.channel = UpdateChannel::ManualDeb;
                identity.enrollment = Enrollment::Disabled;
                identity.reason = "The Halcyon package source is disabled.";
            }
            _ => return,
        }
        self.cancel();
        self.identity = Some(identity);
        self.setup = Setup::Idle;
        self.check = if state == "available" {
            Check::Available("0.3.0".into())
        } else {
            Check::NotChecked
        };
    }
}
