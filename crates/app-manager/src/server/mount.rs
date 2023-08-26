//! Mount secure file storage if needed
//!

use std::{
    path::Path,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use error_stack::{IntoReport, Result, ResultExt};
use manager_model::DataEncryptionKey;
use tokio::{io::AsyncWriteExt, process::Command};
use tracing::info;

use super::app::AppState;
use crate::{
    api::GetApiManager,
    config::{file::EncryptionKeyProviderConfig, Config}, utils::ContextExt,

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
        _provider: &EncryptionKeyProviderConfig,
    ) -> Result<(), MountError> {
        if Path::new(self.config.secure_storage_dir()).exists() {
            info!("Encrypted storage is already mounted");
            // Already mounted.
            return Ok(());
        }

        let key = self
            .app_state
            .api_manager()
            .get_encryption_key()
            .await
            .change_context(MountError::GetKeyFailed)?;

        self.change_password_if_needed(key.clone()).await?;

        info!("Opening encrypted data file");

        // Open encryption.
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
            info!("Opening was successfull.");
        } else {
            tracing::error!("Opening failed.");
            return Err(MountError::CommandFailed(status).report());
        }

        Ok(())
    }

    pub async fn unmount_if_needed(&self) -> Result<(), MountError> {
        if !Path::new(self.config.secure_storage_dir()).exists() {
            info!("Encrypted storage is already unmounted");
            // Already unmounted.
            return Ok(());
        }

        info!("Unmounting encrypted data file");

        // Run command.
        let c = Command::new("sudo")
            .arg(self.config.script_locations().close_encryption())
            .status()
            .await
            .change_context(MountError::ProcessStartFailed)?;

        if c.success() {
            info!("Closing was successfull.");
        } else {
            tracing::error!("Closing failed.");
            return Err(MountError::CommandFailed(c).report());
        }

        Ok(())
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
                tracing::error!("Password change failed.");
                Err(MountError::CommandFailed(status).report())
            }
        } else {
            info!("Encryption password is not the default password.");
            Ok(())
        }
    }
}
