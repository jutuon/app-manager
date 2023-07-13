
//! Handle software updates

use std::{process::ExitStatus, sync::Arc, path::{PathBuf, Path}};

use serde::{Serialize, Deserialize};
use tokio::{task::JoinHandle, sync::mpsc, process::Command};
use tracing::{info, warn};
use url::Url;

use crate::{config::{Config, file::SoftwareBuilderConfig}, utils::IntoReportExt, api::manager::data::{DownloadType, SoftwareOptions, BuildInfo}};

use super::ServerQuitWatcher;

use error_stack::Result;

#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    #[error("Update manager related config is missing")]
    UpdateManagerConfigMissing,

    #[error("Process start failed")]
    ProcessStartFailed,

    #[error("Process wait failed")]
    ProcessWaitFailed,

    #[error("Process stdin writing failed")]
    ProcessStdinFailed,

    #[error("Command failed with exit status: {0}")]
    CommandFailed(ExitStatus),

    #[error("Invalid key path")]
    InvalidKeyPath,

    #[error("File copying failed")]
    FileCopyingFailed,

    #[error("File reading failed")]
    FileReadingFailed,

    #[error("File writing failed")]
    FileWritingFailed,

    #[error("Invalid output")]
    InvalidOutput,

    #[error("Invalid input")]
    InvalidInput,

    #[error("Update manager is not available")]
    UpdateManagerNotAvailable,
}

#[derive(Debug)]
pub struct UpdateManagerQuitHandle {
    task: JoinHandle<()>,
}

impl UpdateManagerQuitHandle {
    pub async fn wait_quit(self) {
        match self.task.await {
            Ok(()) => (),
            Err(e) => {
                warn!("Update manager quit failed. Error: {:?}", e);
            }
        }
    }
}

#[derive(Debug)]
pub enum UpdateManagerMessage {
    UpdateManager,
    UpdateBackend,
}

#[derive(Debug)]
pub struct UpdateManagerHandle {
    sender: mpsc::Sender<UpdateManagerMessage>,
}

impl UpdateManagerHandle {
    pub async fn send_update_manager_request(&self) -> Result<(), UpdateError> {
        self.sender.try_send(UpdateManagerMessage::UpdateManager)
            .into_error(UpdateError::UpdateManagerNotAvailable)?;

        Ok(())
    }

    pub async fn send_update_backend_request(&self) -> Result<(), UpdateError> {
        self.sender.try_send(UpdateManagerMessage::UpdateManager)
            .into_error(UpdateError::UpdateManagerNotAvailable)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdateManager {
    config: Arc<Config>,
    receiver: mpsc::Receiver<UpdateManagerMessage>,
}

impl UpdateManager {
    pub fn new(
        config: Arc<Config>,
        quit_notification: ServerQuitWatcher,
    ) -> (UpdateManagerQuitHandle, UpdateManagerHandle) {
        let (sender, receiver) = mpsc::channel(1);

        let manager = Self {
            config,
            receiver,
        };

        let task = tokio::spawn(manager.run(quit_notification));

        let handle = UpdateManagerHandle {
            sender,
        };

        let quit_handle = UpdateManagerQuitHandle {
            task,
        };

        (quit_handle, handle)
    }

    pub async fn run(
        mut self,
        mut quit_notification: ServerQuitWatcher,
    ) {
        loop {
            tokio::select! {
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
        message: Option<UpdateManagerMessage>,
    ) {
        match message {
            Some(UpdateManagerMessage::UpdateBackend) => {

            }
            Some(UpdateManagerMessage::UpdateManager) => {

            }
            None => {
                warn!("Update manager channel closed");
            }
        }
    }
}
