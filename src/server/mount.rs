//! Mount secure file storage if needed
//!


use std::{sync::Arc, path::Path, process::{Stdio, ExitStatus}};

use api_client::models::DataEncryptionKey;
use http::StatusCode;
use tokio::{process::Command, io::{Stdin, AsyncWriteExt}};

use error_stack::{Result, ResultExt, IntoReport};
use tracing::{info, log::warn};


use crate::{config::{Config, file::EncryptionKeyProviderConfig}, api::GetApiManager, utils::IntoReportExt};

use super::app::AppState;

const ENCRYPTED_HOME_DIR_LOCATION: &str = "/app-secure-storage/app";

const OPEN_ENCRYPTION_SCRIPT: &str = "/app-server-tools/manager-tools/open-encryption.sh";
const CLOSE_ENCRYPTION_SCRIPT: &str = "/app-server-tools/manager-tools/close-encryption.sh";

const IS_DEFAULT_ENCRYPTION_PASSWORD_SCRIPT: &str = "/app-server-tools/manager-tools/is-default-encryption-password.sh";
const CHANGE_ENCRYPTION_PASSWORD_SCRIPT: &str = "/app-server-tools/manager-tools/change-encryption-password.sh";


#[derive(thiserror::Error, Debug)]
pub enum MountError {
    #[error("Getting key failed")]
    GetKeyFailed,

    #[error("Process start failed")]
    ProcessStartFailed,

    #[error("Process stdin writing failed")]
    ProcessStdinFailed,

    #[error("Command failed with exit status: {0}")]
    CommandFailed(ExitStatus),
}

pub struct MountManager {
    config: Arc<Config>,
    app_state: AppState,
}

impl MountManager {
    pub fn new(config: Arc<Config>, app_state: AppState) -> Self {
        Self {
            config,
            app_state,
        }
    }

    pub async fn mount_if_needed(&self, provider: &EncryptionKeyProviderConfig) -> Result<(), MountError> {
        if Path::new(ENCRYPTED_HOME_DIR_LOCATION).exists() {
            info!("Encrypted storage is already mounted");
            // Already mounted.
            return Ok(());
        }

        let key = self.app_state.api_manager()
            .get_encryption_key()
            .await
            .change_context(MountError::GetKeyFailed)?;

        self.change_password_if_needed(key.clone()).await?;

        info!("Opening encrypted data file");

        // Open encryption.
        let mut c = Command::new("sudo")
            .arg(OPEN_ENCRYPTION_SCRIPT)
            .stdin(Stdio::piped())
            .spawn()
            .into_error(MountError::ProcessStartFailed)?;

        if let Some(stdin) = c.stdin.as_mut() {
            stdin.write_all(key.key.as_bytes())
                .await
                .into_error(MountError::ProcessStdinFailed)?;
            stdin.shutdown()
                .await
                .into_error(MountError::ProcessStdinFailed)?;
        }

        let status = c.wait()
            .await
            .into_error(MountError::ProcessStartFailed)?;

        if status.success() {
            info!("Opening was successfull.");
        } else {
            tracing::error!("Opening failed.");
            return Err(MountError::CommandFailed(status)).into_report();
        }

        Ok(())
    }

    pub async fn unmount_if_needed(&self) -> Result<(), MountError> {
        if !Path::new(ENCRYPTED_HOME_DIR_LOCATION).exists() {
            info!("Encrypted storage is already unmounted");
            // Already unmounted.
            return Ok(());
        }

        info!("Unmounting encrypted data file");

        // Run command.
        let mut c = Command::new("sudo")
            .arg(CLOSE_ENCRYPTION_SCRIPT)
            .status()
            .await
            .into_error(MountError::ProcessStartFailed)?;

        if c.success() {
            info!("Closing was successfull.");
        } else {
            tracing::error!("Closing failed.");
            return Err(MountError::CommandFailed(c)).into_report();
        }

        Ok(())
    }

    async fn change_password_if_needed(&self, key: DataEncryptionKey) -> Result<(), MountError> {
        let c = Command::new("sudo")
            .arg(IS_DEFAULT_ENCRYPTION_PASSWORD_SCRIPT)
            .status()
            .await
            .into_error(MountError::ProcessStartFailed)?;
        if c.success() {
            info!("Default password is used. Password will be changed.");
            let mut c = Command::new("sudo")
                .arg(CHANGE_ENCRYPTION_PASSWORD_SCRIPT)
                .stdin(Stdio::piped())
                .spawn()
                .into_error(MountError::ProcessStartFailed)?;

            if let Some(stdin) = c.stdin.as_mut() {
                stdin.write_all(key.key.as_bytes())
                    .await
                    .into_error(MountError::ProcessStdinFailed)?;
                stdin.shutdown()
                    .await
                    .into_error(MountError::ProcessStdinFailed)?;
            }

            let status = c.wait()
                .await
                .into_error(MountError::ProcessStartFailed)?;

            if status.success() {
                info!("Password change was successfull.");
                Ok(())
            } else {
                tracing::error!("Password change failed.");
                Err(MountError::CommandFailed(status)).into_report()
            }
        } else {
            info!("Encryption password is not the default password.");
            Ok(())
        }
    }
}
