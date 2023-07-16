//! Start and stop backend


use std::{process::ExitStatus, sync::Arc, path::{PathBuf, Path}};

use serde::{Serialize, Deserialize};
use tokio::{task::JoinHandle, sync::mpsc, process::Command};
use tracing::{info, warn};
use url::Url;

use crate::{config::{Config, file::SoftwareBuilderConfig}, utils::IntoReportExt, api::manager::data::{DownloadType, SoftwareOptions, BuildInfo}};

use super::ServerQuitWatcher;

use error_stack::Result;


#[derive(thiserror::Error, Debug)]
pub enum ControllerError {
    #[error("Process start failed")]
    ProcessStartFailed,

    #[error("Process wait failed")]
    ProcessWaitFailed,

    #[error("Process stdin writing failed")]
    ProcessStdinFailed,

    #[error("Command failed with exit status: {0}")]
    CommandFailed(ExitStatus),
}

pub struct BackendController<'a> {
    config: &'a Config,
}

impl <'a> BackendController<'a> {
    pub fn new(
        config: &'a Config,
    ) -> Self {
        Self {
            config,
        }
    }

    pub async fn start_backend(&self) -> Result<(), ControllerError> {
        let status = Command::new("sudo")
            .arg(self.config.script_locations().start_backend())
            .status()
            .await
            .into_error(ControllerError::ProcessWaitFailed)?;

        if !status.success() {
            tracing::error!("Start backend failed with status: {:?}", status);
            return Err(ControllerError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn stop_backend(&self) -> Result<(), ControllerError> {
        let status = Command::new("sudo")
            .arg(self.config.script_locations().stop_backend())
            .status()
            .await
            .into_error(ControllerError::ProcessWaitFailed)?;

        if !status.success() {
            tracing::error!("Start backend failed with status: {:?}", status);
            return Err(ControllerError::CommandFailed(status).into());
        }

        Ok(())
    }
}