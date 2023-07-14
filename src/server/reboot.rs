
//! Handle automatic reboots

use std::{process::ExitStatus, sync::{Arc, atomic::{AtomicBool, Ordering}}, path::{PathBuf, Path}, time::Duration};

use serde::{Serialize, Deserialize};
use time::{OffsetDateTime, UtcOffset, Time, error};
use tokio::{task::JoinHandle, sync::mpsc, process::Command, time::sleep};
use tracing::{info, warn};
use url::Url;
use utoipa::openapi::info;

use crate::{config::{Config, file::SoftwareBuilderConfig}, utils::IntoReportExt, api::manager::data::{DownloadType, SoftwareOptions, BuildInfo}};

use super::ServerQuitWatcher;

use error_stack::Result;

/// If this file exists reboot system at some point. Works at least on Ubuntu.
const REBOOT_REQUIRED_PATH: &str = "/var/run/reboot-required";

pub static REBOOT_ON_NEXT_CHECK: AtomicBool = AtomicBool::new(false);

#[derive(thiserror::Error, Debug)]
pub enum RebootError {
    #[error("Reboot manager not available")]
    RebootManagerNotAvailable,

    #[error("Local time is not available")]
    LocalTimeNotAvailable,

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
}

#[derive(Debug)]
pub struct RebootManagerQuitHandle {
    task: JoinHandle<()>,
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
        self.sender.send(RebootManagerMessage::RebootNow)
            .await
            .into_error(RebootError::RebootManagerNotAvailable)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct RebootManager {
    config: Arc<Config>,
    receiver: mpsc::Receiver<RebootManagerMessage>,
}

impl RebootManager {
    pub fn new(
        config: Arc<Config>,
        quit_notification: ServerQuitWatcher,
    ) -> (RebootManagerQuitHandle, RebootManagerHandle) {
        let (sender, receiver) = mpsc::channel(1);

        let manager = Self {
            config,
            receiver,
        };

        let task = tokio::spawn(manager.run(quit_notification));

        let handle = RebootManagerHandle {
            sender,
        };

        let quit_handle = RebootManagerQuitHandle {
            task,
        };

        (quit_handle, handle)
    }

    pub async fn run(
        mut self,
        mut quit_notification: ServerQuitWatcher,
    ) {
        info!("Automatic reboot status: {:?}", self.config.reboot_if_needed().is_some());

        let mut check_cooldown = false;

        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(120)), if check_cooldown => {
                    check_cooldown = false;
                }
                result = Self::sleep_until_reboot_check(&self.config), if !check_cooldown => {
                    match result {
                        Ok(()) => {
                            info!("Sleep completed");
                            if self.reboot_if_needed().await {
                                info!("Reboot requesting complete");
                            }
                        },
                        Err(e) => {
                            warn!("Sleep until reboot check failed. Error: {:?}", e);
                        }
                    }
                    check_cooldown = true;
                }
                message = self.receiver.recv() => {
                    self.handle_message(message).await;
                }
                _ = quit_notification.recv() => {
                    return;
                }
            }
        }
    }

    pub async fn handle_message(
        &self,
        message: Option<RebootManagerMessage>,
    ) {
        match message {
            Some(RebootManagerMessage::RebootNow) => {
                match self.run_reboot().await {
                    Ok(()) => {
                        info!("Reboot successful");
                    }
                    Err(e) => {
                        warn!("Reboot failed. Error: {:?}", e);
                    }
                }
            }
            None => {
                warn!("Reboot manager channel closed");
            }
        }
    }

    pub async fn reboot_if_needed(
        &self,
    ) -> bool {
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

    pub async fn run_reboot(
        &self,
    ) -> Result<(), RebootError> {
        info!("Rebooting system");
        let status = Command::new("sudo")
            .arg("reboot")
            .status()
            .await
            .into_error(RebootError::ProcessStartFailed)?;

        if !status.success() {
            return Err(RebootError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn sleep_until_reboot_check(config: &Config) -> Result<(), RebootError> {
        info!("Calculating sleep time");

        let now = Self::get_local_time().await?;

        let target_time = if let Some(reboot) = config.reboot_if_needed() {
            Time::from_hms(reboot.time.hours, reboot.time.minutes, 0)
                .into_error(RebootError::TimeError)?
        } else {
            futures::future::pending::<()>().await;
            return Err(RebootError::ConfigError.into());
        };

        let target_date_time = now
            .replace_time(
                target_time
            );

        let duration = if target_date_time > now {
            target_date_time - now
        } else {
            let tomorrow = now + Duration::from_secs(24 * 60 * 60);
            let tomorrow_target_date_time = tomorrow
                .replace_time(
                    target_time
                );
            tomorrow_target_date_time - now
        };

        info!("Time until reboot check: {}", duration);
        sleep(duration.unsigned_abs()).await;

        Ok(())
    }

    pub async fn get_local_time() -> Result<OffsetDateTime, RebootError> {
        let now: OffsetDateTime = OffsetDateTime::now_utc();
        let offset = Self::get_utc_offset_hours().await?;
        let now = now.to_offset(
            UtcOffset::from_hms(offset, 0, 0)
                .into_error(RebootError::TimeError)?
        );
        Ok(now)
    }

    pub async fn get_utc_offset_hours() -> Result<i8, RebootError> {
        let output = Command::new("date")
            .arg("+%z")
            .output()
            .await
            .into_error(RebootError::ProcessWaitFailed)?;

        if !output.status.success() {
            tracing::error!("date command failed");
            return Err(RebootError::CommandFailed(output.status).into());
        }

        let offset = std::str::from_utf8(&output.stdout)
            .into_error(RebootError::InvalidOutput)?;

        let multiplier = match offset.chars().nth(0) {
            Some('-') => -1,
            _ => 1,
        };

        let hours = offset
            .chars()
            .skip(1)
            .take(2)
            .collect::<String>()
            .trim_start_matches('0')
            .parse::<i8>()
            .into_error(RebootError::InvalidOutput)?;

        Ok(hours * multiplier)
    }
}
