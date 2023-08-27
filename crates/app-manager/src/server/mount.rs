//! Mount secure file storage if needed
//!

use std::{
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use error_stack::{Result, ResultExt};
use manager_model::DataEncryptionKey;
use tokio::{io::AsyncWriteExt, process::Command};
use tracing::{info, error};

use super::app::AppState;
use crate::{
    api::GetApiManager,
    config::{file::SecureStorageConfig, Config}, utils::ContextExt,

};

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
        Self { config, app_state }
    }

    pub async fn mount_if_needed(
        &self,
        storage_config: &SecureStorageConfig,
    ) -> Result<(), MountError> {
        if storage_config.availability_check_path.exists() {
            info!("Secure storage is already mounted");
            return Ok(());
        }

        let key = self
            .app_state
            .api_manager()
            .get_encryption_key()
            .await
            .change_context(MountError::GetKeyFailed)?;

        self.change_password_if_needed(key.clone()).await?;

        info!("Mounting secure storage");

        let mut c = Command::new("sudo")
            .arg(self.config.script_locations().open_encryption())
            .stdin(Stdio::piped())
            .spawn()
            .change_context(MountError::ProcessStartFailed)?;

        if let Some(stdin) = c.stdin.as_mut() {
            stdin
                .write_all(key.key.as_bytes())
                .await
                .change_context(MountError::ProcessStdinFailed)?;
            stdin
                .shutdown()
                .await
                .change_context(MountError::ProcessStdinFailed)?;
        }

        let status = c.wait().await.change_context(MountError::ProcessStartFailed)?;

        if status.success() {
            info!("Mounting was successfull.");
            Ok(())
        } else {
            error!("Mounting failed.");
            Err(MountError::CommandFailed(status).report())
        }
    }

    pub async fn unmount_if_needed(&self, storage_config: &SecureStorageConfig) -> Result<(), MountError> {
        if !storage_config.availability_check_path.exists() {
            info!("Secure storage is already unmounted");
            return Ok(());
        }

        info!("Unmounting secure storage");

        // Run command.
        let c = Command::new("sudo")
            .arg(self.config.script_locations().close_encryption())
            .status()
            .await
            .change_context(MountError::ProcessStartFailed)?;

        if c.success() {
            info!("Unmounting was successfull.");
            Ok(())
        } else {
            error!("Unmounting failed.");
            Err(MountError::CommandFailed(c).report())
        }
    }

    async fn change_password_if_needed(&self, key: DataEncryptionKey) -> Result<(), MountError> {
        let c = Command::new("sudo")
            .arg(
                self.config
                    .script_locations()
                    .is_default_encryption_password(),
            )
            .status()
            .await
            .change_context(MountError::ProcessStartFailed)?;
        if c.success() {
            info!("Default password is used. Password will be changed.");
            let mut c = Command::new("sudo")
                .arg(self.config.script_locations().change_encryption_password())
                .stdin(Stdio::piped())
                .spawn()
                .change_context(MountError::ProcessStartFailed)?;

            if let Some(stdin) = c.stdin.as_mut() {
                stdin
                    .write_all(key.key.as_bytes())
                    .await
                    .change_context(MountError::ProcessStdinFailed)?;
                stdin
                    .shutdown()
                    .await
                    .change_context(MountError::ProcessStdinFailed)?;
            }

            let status = c.wait().await.change_context(MountError::ProcessStartFailed)?;

            if status.success() {
                info!("Password change was successfull.");
                Ok(())
            } else {
                error!("Password change failed.");
                Err(MountError::CommandFailed(status).report())
            }
        } else {
            info!("Encryption password is not the default password.");
            Ok(())
        }
    }
}
