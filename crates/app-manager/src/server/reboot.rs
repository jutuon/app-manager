//! Handle automatic reboots

use std::{
    path::Path,
    process::ExitStatus,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use error_stack::{Result, ResultExt};
use time::{OffsetDateTime, Time};
use tokio::{process::Command, sync::mpsc, task::JoinHandle, time::sleep};
use tracing::{debug, info, warn};

use super::{
    client::{ApiClient, ApiManager},
    state::StateStorage,
    ServerQuitWatcher,
};
use crate::{config::Config, server::mount::MountMode};

/// If this file exists reboot system at some point. Works at least on Ubuntu.
const REBOOT_REQUIRED_PATH: &str = "/var/run/reboot-required";

pub static REBOOT_ON_NEXT_CHECK: AtomicBool = AtomicBool::new(false);

#[derive(thiserror::Error, Debug)]
pub enum RebootError {
    #[error("Reboot manager not available")]
    RebootManagerNotAvailable,

    #[error("Time related error")]
    TimeError,

    #[error("Config related error")]
    ConfigError,

    #[error("Command failed with exit status: {0}")]
    CommandFailed(ExitStatus),

    #[error("Process start failed")]
    ProcessStartFailed,

    #[error("Process wait failed")]
    ProcessWaitFailed,

    #[error("Invalid output")]
    InvalidOutput,

    #[error("Getting encryption key failed")]
    GetKeyFailed,
}

#[derive(Debug)]
pub struct RebootManagerQuitHandle {
    task: JoinHandle<()>,
    // Make sure Receiver works until the manager quits.
    _sender: mpsc::Sender<RebootManagerMessage>,
}

impl RebootManagerQuitHandle {
    pub async fn wait_quit(self) {
        match self.task.await {
            Ok(()) => (),
            Err(e) => {
                warn!("Reboot manager quit failed. Error: {:?}", e);
            }
        }
    }
}

#[derive(Debug)]
pub enum RebootManagerMessage {
    RebootNow,
}

#[derive(Debug, Clone)]
pub struct RebootManagerHandle {
    sender: mpsc::Sender<RebootManagerMessage>,
}

impl RebootManagerHandle {
    pub async fn reboot_now(&self) -> Result<(), RebootError> {
        self.sender
            .send(RebootManagerMessage::RebootNow)
            .await
            .change_context(RebootError::RebootManagerNotAvailable)?;

        Ok(())
    }
}

pub struct RebootManager {
    config: Arc<Config>,
    api_client: Arc<ApiClient>,
    state: Arc<StateStorage>,
    receiver: mpsc::Receiver<RebootManagerMessage>,
}

impl RebootManager {
    pub fn new_manager(
        config: Arc<Config>,
        api_client: Arc<ApiClient>,
        state: Arc<StateStorage>,
        quit_notification: ServerQuitWatcher,
    ) -> (RebootManagerQuitHandle, RebootManagerHandle) {
        let (sender, receiver) = mpsc::channel(1);

        let manager = Self {
            config,
            receiver,
            api_client,
            state,
        };

        let task = tokio::spawn(manager.run(quit_notification));

        let handle = RebootManagerHandle { sender };

        let quit_handle = RebootManagerQuitHandle {
            task,
            _sender: handle.sender.clone(),
        };

        (quit_handle, handle)
    }

    pub async fn run(mut self, mut quit_notification: ServerQuitWatcher) {
        info!(
            "Automatic reboot status: {:?}",
            self.config.reboot_if_needed().is_some()
        );

        let mut check_cooldown = false;

        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(120)), if check_cooldown => {
                    check_cooldown = false;
                }
                result = Self::sleep_until_reboot_check(&self.config), if !check_cooldown => {
                    match result {
                        Ok(()) => {
                            debug!("Sleep completed");
                            if self.reboot_if_needed().await {
                                debug!("Reboot requesting complete");
                            }
                        },
                        Err(e) => {
                            warn!("Sleep until reboot check failed. Error: {:?}", e);
                        }
                    }
                    check_cooldown = true;
                }
                message = self.receiver.recv() => {
                    match message {
                        Some(message) => {
                            self.handle_message(message).await;
                        }
                        None => {
                            warn!("Reboot manager channel closed");
                            return;
                        }
                    }
                }
                _ = quit_notification.recv() => {
                    return;
                }
            }
        }
    }

    pub async fn handle_message(&self, message: RebootManagerMessage) {
        match message {
            RebootManagerMessage::RebootNow => match self.run_reboot().await {
                Ok(()) => {
                    info!("Reboot successful");
                }
                Err(e) => {
                    warn!("Reboot failed. Error: {:?}", e);
                }
            },
        }
    }

    pub async fn reboot_if_needed(&self) -> bool {
        if Path::new(REBOOT_REQUIRED_PATH).exists() {
            info!("Reboot required file exists. Rebooting system");
            self.run_reboot_and_log_error().await;
            true
        } else if REBOOT_ON_NEXT_CHECK.load(Ordering::Relaxed) {
            info!("Reboot was requested at some point. Rebooting system");
            self.run_reboot_and_log_error().await;
            true
        } else {
            info!("No reboot needed");
            false
        }
    }

    pub async fn run_reboot_and_log_error(&self) {
        match self.run_reboot().await {
            Ok(()) => {
                info!("Reboot successful");
            }
            Err(e) => {
                warn!("Reboot failed. Error: {:?}", e);
            }
        }
    }

    pub async fn run_reboot(&self) -> Result<(), RebootError> {
        match self.state.get(|s| s.mount_state.mode()).await {
            MountMode::MountedWithRemoteKey => {
                info!("Remote encryption key detected. Checking encryption key availability before rebooting");
                self.api_manager()
                    .get_encryption_key()
                    .await
                    .change_context(RebootError::GetKeyFailed)?;
                info!("Remote encryption key is available");
            }
            _ => (),
        }

        info!("Rebooting system");
        let status = Command::new("sudo")
            .arg("reboot")
            .status()
            .await
            .change_context(RebootError::ProcessStartFailed)?;

        if !status.success() {
            return Err(RebootError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn sleep_until_reboot_check(config: &Config) -> Result<(), RebootError> {
        debug!("Calculating sleep time");

        let now = OffsetDateTime::now_utc();

        let target_time = if let Some(reboot) = config.reboot_if_needed() {
            Time::from_hms(reboot.time.hours, reboot.time.minutes, 0)
                .change_context(RebootError::TimeError)?
        } else {
            futures::future::pending::<()>().await;
            return Err(RebootError::ConfigError.into());
        };

        let target_date_time = now.replace_time(target_time);

        let duration = if target_date_time > now {
            target_date_time - now
        } else {
            let tomorrow = now + Duration::from_secs(24 * 60 * 60);
            let tomorrow_target_date_time = tomorrow.replace_time(target_time);
            tomorrow_target_date_time - now
        };

        debug!("Time until reboot check: {}", duration);
        sleep(duration.unsigned_abs()).await;

        Ok(())
    }

    fn api_manager(&self) -> ApiManager<'_> {
        ApiManager::new(&self.config, &self.api_client)
    }
}
