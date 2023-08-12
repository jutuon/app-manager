//! Handle software updates

use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{atomic::Ordering, Arc},
};

use error_stack::{Result, ResultExt};
use manager_model::{BuildInfo, SoftwareInfo, SoftwareOptions};
use tokio::{process::Command, sync::mpsc, task::JoinHandle};
use tracing::{info, warn};

use super::{
    build::BuildDirCreator,
    client::{ApiClient, ApiManager},
    reboot::{RebootManagerHandle, REBOOT_ON_NEXT_CHECK},
    ServerQuitWatcher,
};
use crate::{
    config::{file::SoftwareUpdateProviderConfig, Config},
    utils::IntoReportExt,
};

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

    #[error("File moving failed")]
    FileMovingFailed,

    #[error("File removing failed")]
    FileRemovingFailed,

    #[error("Invalid output")]
    InvalidOutput,

    #[error("Invalid input")]
    InvalidInput,

    #[error("Update manager is not available")]
    UpdateManagerNotAvailable,

    #[error("Software updater related config is missing")]
    SoftwareUpdaterConfigMissing,

    #[error("Api request failed")]
    ApiRequest,

    #[error("Reboot failed")]
    RebootFailed,
}

#[derive(Debug)]
pub struct UpdateManagerQuitHandle {
    task: JoinHandle<()>,
    // Make sure Receiver works until the manager quits.
    _sender: mpsc::Sender<UpdateManagerMessage>,
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
    UpdateSoftware {
        force_reboot: bool,
        software: SoftwareOptions,
    },
}

#[derive(Debug)]
pub struct UpdateManagerHandle {
    sender: mpsc::Sender<UpdateManagerMessage>,
}

impl UpdateManagerHandle {
    pub async fn send_update_manager_request(&self, force_reboot: bool) -> Result<(), UpdateError> {
        self.sender
            .try_send(UpdateManagerMessage::UpdateSoftware {
                force_reboot,
                software: SoftwareOptions::Manager,
            })
            .into_error(UpdateError::UpdateManagerNotAvailable)?;

        Ok(())
    }

    pub async fn send_update_backend_request(&self, force_reboot: bool) -> Result<(), UpdateError> {
        self.sender
            .try_send(UpdateManagerMessage::UpdateSoftware {
                force_reboot,
                software: SoftwareOptions::Backend,
            })
            .into_error(UpdateError::UpdateManagerNotAvailable)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdateManager {
    config: Arc<Config>,
    api_client: Arc<ApiClient>,
    receiver: mpsc::Receiver<UpdateManagerMessage>,
    reboot_manager_handle: RebootManagerHandle,
}

impl UpdateManager {
    pub fn new(
        config: Arc<Config>,
        quit_notification: ServerQuitWatcher,
        api_client: Arc<ApiClient>,
        reboot_manager_handle: RebootManagerHandle,
    ) -> (UpdateManagerQuitHandle, UpdateManagerHandle) {
        let (sender, receiver) = mpsc::channel(1);

        let manager = Self {
            config,
            api_client,
            receiver,
            reboot_manager_handle,
        };

        let task = tokio::spawn(manager.run(quit_notification));

        let handle = UpdateManagerHandle { sender };

        let quit_handle = UpdateManagerQuitHandle {
            task,
            _sender: handle.sender.clone(),
        };

        (quit_handle, handle)
    }

    pub async fn run(mut self, mut quit_notification: ServerQuitWatcher) {
        loop {
            tokio::select! {
                message = self.receiver.recv() => {
                    match message {
                        Some(message) => {
                            self.handle_message(message).await;
                        }
                        None => {
                            warn!("Update manager channel closed");
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

    pub async fn handle_message(&self, message: UpdateManagerMessage) {
        match message {
            UpdateManagerMessage::UpdateSoftware {
                force_reboot,
                software,
            } => match self.update_software(force_reboot, software).await {
                Ok(()) => {
                    info!("Software update finished");
                }
                Err(e) => {
                    warn!("Software update failed. Error: {:?}", e);
                }
            },
        }
    }

    pub async fn download_latest_info(
        &self,
        software: SoftwareOptions,
    ) -> Result<BuildInfo, UpdateError> {
        let api = ApiManager::new(&self.config, &self.api_client);
        api.get_latest_build_info(software)
            .await
            .change_context(UpdateError::ApiRequest)
    }

    pub async fn download_latest_encrypted_binary(
        &self,
        software: SoftwareOptions,
    ) -> Result<Vec<u8>, UpdateError> {
        let api = ApiManager::new(&self.config, &self.api_client);
        api.get_latest_encrypted_software_binary(software)
            .await
            .change_context(UpdateError::ApiRequest)
    }

    /// Returns empty BuildInfo if it does not exists.
    pub async fn read_latest_build_info(
        &self,
        software: SoftwareOptions,
    ) -> Result<BuildInfo, UpdateError> {
        let update_dir = UpdateDirCreator::create_update_dir_if_needed(&self.config);
        let current_info =
            update_dir.join(BuildDirCreator::build_info_json_name(software.to_str()));
        self.read_build_info(&current_info).await
    }

    /// Returns empty BuildInfo if it does not exists.
    pub async fn read_latest_installed_build_info(
        &self,
        software: SoftwareOptions,
    ) -> Result<BuildInfo, UpdateError> {
        let update_dir = UpdateDirCreator::create_update_dir_if_needed(&self.config);
        let current_info = update_dir.join(UpdateDirCreator::installed_build_info_json_name(
            software.to_str(),
        ));
        self.read_build_info(&current_info).await
    }

    /// Returns empty BuildInfo if it does not exists.
    async fn read_build_info(&self, current_info: &Path) -> Result<BuildInfo, UpdateError> {
        if !current_info.exists() {
            return Ok(BuildInfo::default());
        }

        let current_build_info = tokio::fs::read_to_string(&current_info)
            .await
            .into_error(UpdateError::FileReadingFailed)?;

        let current_build_info =
            serde_json::from_str(&current_build_info).into_error(UpdateError::InvalidInput)?;

        Ok(current_build_info)
    }

    pub async fn download_and_decrypt_latest_software(
        &self,
        latest_version: &BuildInfo,
        software: SoftwareOptions,
    ) -> Result<(), UpdateError> {
        let encrypted_binary = self.download_latest_encrypted_binary(software).await?;

        let update_dir = UpdateDirCreator::create_update_dir_if_needed(&self.config);
        let encrypted_binary_path =
            update_dir.join(BuildDirCreator::encrypted_binary_name(software.to_str()));
        tokio::fs::write(&encrypted_binary_path, encrypted_binary)
            .await
            .into_error(UpdateError::FileWritingFailed)?;

        self.import_gpg_public_key().await?;
        let binary_path = update_dir.join(software.to_str());
        self.decrypt_encrypted_binary(&encrypted_binary_path, &binary_path)
            .await?;

        let latest_build_info_path =
            update_dir.join(BuildDirCreator::build_info_json_name(software.to_str()));
        tokio::fs::write(
            &latest_build_info_path,
            serde_json::to_string_pretty(&latest_version).into_error(UpdateError::InvalidInput)?,
        )
        .await
        .into_error(UpdateError::FileWritingFailed)?;

        Ok(())
    }

    pub async fn install_latest_software(
        &self,
        latest_version: &BuildInfo,
        force_reboot: bool,
        software: SoftwareOptions,
    ) -> Result<(), UpdateError> {
        let update_dir = UpdateDirCreator::create_update_dir_if_needed(&self.config);
        let binary_path = update_dir.join(software.to_str());

        let installed_build_info_path = update_dir.join(
            UpdateDirCreator::installed_build_info_json_name(software.to_str()),
        );

        if installed_build_info_path.exists() {
            let installed_old_build_info_path = update_dir.join(
                UpdateDirCreator::installed_old_build_info_json_name(software.to_str()),
            );
            tokio::fs::rename(&installed_build_info_path, &installed_old_build_info_path)
                .await
                .into_error(UpdateError::FileMovingFailed)?;
        }

        self.replace_binary(&binary_path, software).await?;

        tokio::fs::write(
            &installed_build_info_path,
            serde_json::to_string_pretty(&latest_version).into_error(UpdateError::InvalidInput)?,
        )
        .await
        .into_error(UpdateError::FileWritingFailed)?;

        REBOOT_ON_NEXT_CHECK.store(true, Ordering::Relaxed);

        if force_reboot {
            self.reboot_manager_handle
                .reboot_now()
                .await
                .change_context(UpdateError::RebootFailed)?;
            info!("Rebooting now");
        } else {
            info!("Rebooting on next check");
        }

        Ok(())
    }

    pub async fn update_software(
        &self,
        force_reboot: bool,
        software: SoftwareOptions,
    ) -> Result<(), UpdateError> {
        let current_version = self.read_latest_build_info(software).await?;
        let latest_version = self.download_latest_info(software).await?;

        if current_version != latest_version {
            info!(
                "Downloading and decrypting software...\n{:#?}",
                latest_version
            );
            self.download_and_decrypt_latest_software(&latest_version, software)
                .await?;
            info!("Software is now downloaded and decrypted.");
        } else {
            info!("Downloaded software is up to date.\n{:#?}", current_version);
        }

        let latest_installed_version = self.read_latest_installed_build_info(software).await?;
        if latest_version != latest_installed_version {
            info!("Installing software.\n{:#?}", latest_version);
            self.install_latest_software(&latest_version, force_reboot, software)
                .await?;
            info!("Software installation completed.");
        } else {
            info!(
                "Installed software is up to date.\n{:#?}",
                latest_installed_version
            );
        }

        Ok(())
    }

    pub async fn decrypt_encrypted_binary(
        &self,
        encrypted: &Path,
        decrypted: &Path,
    ) -> Result<(), UpdateError> {
        if decrypted.exists() {
            info!("Remove previous binary {}", decrypted.display());
            tokio::fs::remove_file(&decrypted)
                .await
                .into_error(UpdateError::FileRemovingFailed)?;
        }

        info!("Decrypting binary {}", encrypted.display());
        let status = Command::new("gpg")
            .arg("--output")
            .arg(&decrypted)
            .arg("--decrypt")
            .arg(&encrypted)
            .status()
            .await
            .into_error(UpdateError::ProcessWaitFailed)?;
        if !status.success() {
            tracing::error!("Decrypting binary failed");
            return Err(UpdateError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn import_gpg_public_key(&self) -> Result<(), UpdateError> {
        info!("Importing GPG key");
        let key_path = &self.updater_config()?.binary_signing_public_key;
        let status: ExitStatus = Command::new("gpg")
            .arg("--import")
            .arg(&key_path)
            .status()
            .await
            .into_error(UpdateError::ProcessWaitFailed)?;
        if !status.success() {
            tracing::error!("Decrypting binary failed");
            return Err(UpdateError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn replace_binary(
        &self,
        binary: &Path,
        software: SoftwareOptions,
    ) -> Result<(), UpdateError> {
        let target = match software {
            SoftwareOptions::Manager => self.updater_config()?.manager_install_location.clone(),
            SoftwareOptions::Backend => self.updater_config()?.backend_install_location.clone(),
        };

        if target.exists() {
            tokio::fs::rename(&target, &target.with_extension("old"))
                .await
                .into_error(UpdateError::FileMovingFailed)?;
        }

        tokio::fs::copy(&binary, &target)
            .await
            .into_error(UpdateError::FileCopyingFailed)?;

        let status = Command::new("chmod")
            .arg("u+x")
            .arg(&target)
            .status()
            .await
            .into_error(UpdateError::ProcessWaitFailed)?;
        if !status.success() {
            tracing::error!("Changing binary permissions failed");
            return Err(UpdateError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub fn updater_config(&self) -> Result<&SoftwareUpdateProviderConfig, UpdateError> {
        self.config
            .software_update_provider()
            .ok_or(UpdateError::SoftwareUpdaterConfigMissing.into())
    }
}

pub struct UpdateDirCreator;

impl UpdateDirCreator {
    pub fn create_update_dir_if_needed(config: &Config) -> PathBuf {
        let build_dir = config.secure_storage_dir().join("update");

        if !Path::new(&build_dir).exists() {
            info!("Creating update directory");
            match std::fs::create_dir(&build_dir) {
                Ok(()) => {
                    info!("Update directory created");
                }
                Err(e) => {
                    warn!("Update directory creation failed. Error: {:?}", e);
                }
            }
        }

        build_dir
    }

    pub fn installed_build_info_json_name(binary: &str) -> String {
        format!("{}.json.installed", binary)
    }

    pub fn installed_old_build_info_json_name(binary: &str) -> String {
        format!("{}.json.installed.old", binary)
    }

    pub async fn current_software(config: &Config) -> Result<SoftwareInfo, UpdateError> {
        let update_dir = Self::create_update_dir_if_needed(config);
        let manager_info_path = update_dir.join(Self::installed_build_info_json_name(
            SoftwareOptions::Manager.to_str(),
        ));
        let backend_info_path = update_dir.join(Self::installed_build_info_json_name(
            SoftwareOptions::Backend.to_str(),
        ));
        let mut info_vec = Vec::new();

        if manager_info_path.exists() {
            let manager_info = tokio::fs::read_to_string(&manager_info_path)
                .await
                .into_error(UpdateError::FileReadingFailed)?;
            let manager_info =
                serde_json::from_str(&manager_info).into_error(UpdateError::InvalidInput)?;
            info_vec.push(manager_info);
        }

        if backend_info_path.exists() {
            let backend_info = tokio::fs::read_to_string(&backend_info_path)
                .await
                .into_error(UpdateError::FileReadingFailed)?;
            let backend_info =
                serde_json::from_str(&backend_info).into_error(UpdateError::InvalidInput)?;
            info_vec.push(backend_info);
        }

        Ok(SoftwareInfo {
            current_software: info_vec,
        })
    }
}
