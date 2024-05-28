//! Build backend server binary

use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
};

use error_stack::{Result, ResultExt};
use manager_model::{
    BuildInfo, DownloadType, SoftwareOptions, BACKEND_REPOSITORY_NAME, MANAGER_REPOSITORY_NAME,
};
use tokio::{process::Command, task::JoinHandle};
use tracing::{info, warn};

use super::ServerQuitWatcher;
use crate::{
    config::{file::SoftwareBuilderConfig, Config},
    utils::{InProgressChannel, InProgressReceiver, InProgressSender},
};

pub const GPG_KEY_NAME: &str = "app-manager-software-builder";

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Software builder config is missing")]
    SoftwareBuilderConfigMissing,

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

    #[error("Send message failed")]
    SendMessageFailed,
}

pub struct BinaryBuildInfoOutput(String);

#[derive(Debug)]
pub struct BuildManagerQuitHandle {
    task: JoinHandle<()>,
    // Make sure that Receiver works until the end of the task.
    _sender: InProgressSender<BuildManagerMessage>,
}

impl BuildManagerQuitHandle {
    pub async fn wait_quit(self) {
        match self.task.await {
            Ok(()) => (),
            Err(e) => {
                warn!("Build manager quit failed. Error: {:?}", e);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum BuildManagerMessage {
    BuildNewBackendVersion,
    BuildNewManagerVersion,
}

#[derive(Debug)]
pub struct BuildManagerHandle {
    sender: InProgressSender<BuildManagerMessage>,
}

impl BuildManagerHandle {
    pub async fn send_build_request(&self, software: SoftwareOptions) -> Result<(), BuildError> {
        match software {
            SoftwareOptions::Manager => {
                self.send_message(BuildManagerMessage::BuildNewManagerVersion)
                    .await
            }
            SoftwareOptions::Backend => {
                self.send_message(BuildManagerMessage::BuildNewBackendVersion)
                    .await
            }
        }
    }

    pub async fn send_build_new_backend_version(&self) -> Result<(), BuildError> {
        self.send_message(BuildManagerMessage::BuildNewBackendVersion)
            .await
    }

    pub async fn send_message(&self, message: BuildManagerMessage) -> Result<(), BuildError> {
        self.sender
            .send_message(message)
            .await
            .change_context(BuildError::SendMessageFailed)
    }
}

#[derive(Debug)]
pub struct BuildManager {
    config: Arc<Config>,
    receiver: InProgressReceiver<BuildManagerMessage>,
}

impl BuildManager {
    pub fn new(
        config: Arc<Config>,
        quit_notification: ServerQuitWatcher,
    ) -> (BuildManagerQuitHandle, BuildManagerHandle) {
        let (sender, receiver) = InProgressChannel::new();

        let manager = Self { config, receiver };

        let task = tokio::spawn(manager.run(quit_notification));

        let handle = BuildManagerHandle {
            sender: sender.clone(),
        };

        let quit_handle = BuildManagerQuitHandle {
            task,
            _sender: sender,
        };

        (quit_handle, handle)
    }

    pub async fn run(mut self, mut quit_notification: ServerQuitWatcher) {
        loop {
            tokio::select! {
                result = self.receiver.is_new_message_available() => {
                    match result {
                        Ok(()) => (),
                        Err(e) => {
                            warn!("Build manager channel broken. Error: {:?}", e);
                            return;
                        }
                    }

                    let container = self.receiver.lock_message_container().await;

                    match container.get_message() {
                        Some(message) => {
                            self.handle_message(message).await;
                        }
                        None => {
                            warn!("Unexpected empty container");
                        }
                    }
                }
                _ = quit_notification.recv() => {
                    return;
                }
            }
        }
    }

    pub async fn handle_message(&self, message: &BuildManagerMessage) {
        match message {
            BuildManagerMessage::BuildNewBackendVersion => {
                info!("Building backend version");
                match self.git_refresh_backend_if_needed().await {
                    Ok(()) => {
                        info!("Build finished");
                    }
                    Err(e) => {
                        warn!("Build failed. Error: {:?}", e);
                    }
                }
            }
            BuildManagerMessage::BuildNewManagerVersion => {
                info!("Building manager version");
                match self.git_refresh_manager_if_needed().await {
                    Ok(()) => {
                        info!("Build finished");
                    }
                    Err(e) => {
                        warn!("Build failed. Error: {:?}", e);
                    }
                }
            }
        }
    }

    pub fn create_build_dir_if_needed(&self) -> PathBuf {
        BuildDirCreator::create_build_dir_if_needed(&self.config)
    }

    pub fn create_history_dir_if_needed(&self) -> PathBuf {
        BuildDirCreator::create_history_dir_if_needed(&self.config)
    }

    pub fn create_latest_dir_if_needed(&self) -> PathBuf {
        BuildDirCreator::create_latest_dir_if_needed(&self.config)
    }

    pub fn manager_repository_name(&self) -> &'static str {
        MANAGER_REPOSITORY_NAME
    }

    pub fn manager_repository(&self) -> PathBuf {
        self.create_build_dir_if_needed()
            .join(self.manager_repository_name())
    }

    pub fn backend_repository_name(&self) -> &'static str {
        BACKEND_REPOSITORY_NAME
    }

    pub fn backend_repository(&self) -> PathBuf {
        self.create_build_dir_if_needed()
            .join(self.backend_repository_name())
    }

    pub async fn git_refresh_backend_if_needed(&self) -> Result<(), BuildError> {
        let builder_config = self.builder_config()?;
        self.git_refresh_if_needed(
            builder_config.backend_download_key_path.as_deref(),
            &builder_config.backend_download_git_address,
            &self.backend_repository().as_os_str().to_string_lossy(),
            self.backend_repository_name(),
            builder_config.backend_branch.as_str(),
            &builder_config.backend_binary,
            builder_config.backend_pre_build_script.as_deref(),
        )
        .await?;

        Ok(())
    }

    pub async fn git_refresh_manager_if_needed(&self) -> Result<(), BuildError> {
        let builder_config = self.builder_config()?;

        self.git_refresh_if_needed(
            builder_config.manager_download_key_path.as_deref(),
            &builder_config.manager_download_git_address,
            &self.manager_repository().as_os_str().to_string_lossy(),
            self.manager_repository_name(),
            builder_config.manager_branch.as_str(),
            &builder_config.manager_binary,
            builder_config.manager_pre_build_script.as_deref(),
        )
        .await?;

        Ok(())
    }

    pub async fn git_refresh_if_needed(
        &self,
        download_key: Option<&Path>,
        repository_address: &str,
        repository_path: &str,
        repository_name: &str,
        repository_branch: &str,
        binary: &str,
        pre_build_script: Option<&Path>,
    ) -> Result<(), BuildError> {
        // Avoid injecting additional args to SSH command.
        if let Some(download_key) = download_key {
            validate_path(&download_key)?;
        }

        Self::git_clone_repository_if_needed(
            download_key.map(|path| path.as_os_str().to_string_lossy().to_string()),
            &repository_address,
            &repository_path,
            repository_name,
            repository_branch,
        )
        .await?;

        Self::git_pull_repository(&repository_path, repository_name, repository_branch).await?;

        let latest_build_commit_sha = self.get_latest_build_commit_sha(binary).await?;
        let current_commit_sha =
            Self::git_get_commit_sha(&repository_path, repository_name).await?;

        if latest_build_commit_sha == current_commit_sha {
            info!("No new commits for {}", repository_name);
            return Ok(());
        }

        if let Some(script) = pre_build_script {
            self.run_pre_build_script(script, repository_name, &repository_path)
                .await?;
        }

        let build_info = self
            .cargo_build(&repository_path, repository_name, &binary)
            .await?;

        self.copy_and_sign_binary(&repository_path, repository_name, &binary, build_info)
            .await?;

        Ok(())
    }

    pub async fn git_clone_repository_if_needed(
        ssh_key_path: Option<String>,
        repository_address: &str,
        repository_path: &str,
        repository_name: &str,
        repository_branch: &str,
    ) -> Result<(), BuildError> {
        if Path::new(repository_path).exists() {
            return Ok(());
        }

        info!("Cloning {} repository", repository_name);
        let mut cmd = Command::new("git");
        cmd.arg("clone");

        if let Some(ssh_key_path) = ssh_key_path {
            cmd.arg("-c")
                .arg(format!("core.sshCommand=ssh -i {}", ssh_key_path));
        }

        let status = cmd
            .arg("-b")
            .arg(repository_branch)
            .arg(repository_address)
            .arg(repository_path)
            .status()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;

        if !status.success() {
            tracing::error!(
                "Git clone failed. Make sure that repository address is in SSH known hosts."
            );
            return Err(BuildError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn git_pull_repository(
        repository_path: &str,
        repository_name: &str,
        repository_branch: &str,
    ) -> Result<(), BuildError> {
        info!("Git pull {} repository", repository_name);
        let status = Command::new("git")
            .arg("-C")
            .arg(repository_path)
            .arg("pull")
            .arg("origin")
            .arg(repository_branch)
            .status()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;

        if !status.success() {
            tracing::error!("Git pull failed");
            return Err(BuildError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn git_get_commit_sha(
        repository_path: &str,
        repository_name: &str,
    ) -> Result<String, BuildError> {
        info!("Git get commit SHA from {} repository", repository_name);
        let output = Command::new("git")
            .arg("-C")
            .arg(repository_path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;

        if !output.status.success() {
            tracing::error!("Git rev-parse failed");
            return Err(BuildError::CommandFailed(output.status).into());
        }

        let sha = std::str::from_utf8(&output.stdout).change_context(BuildError::InvalidOutput)?;

        Ok(sha.to_string())
    }

    pub async fn get_latest_build_commit_sha(&self, binary: &str) -> Result<String, BuildError> {
        let latest_build_info = self
            .create_latest_dir_if_needed()
            .join(format!("{}.json", binary));

        if !latest_build_info.exists() {
            info!("No latest build info for {}", binary);
            return Ok("".to_string());
        }

        let build_info = tokio::fs::read_to_string(&latest_build_info)
            .await
            .change_context(BuildError::FileReadingFailed)?;

        let build_info: BuildInfo =
            serde_json::from_str(&build_info).change_context(BuildError::InvalidInput)?;

        info!(
            "Latest {} build is from commit {}",
            binary, build_info.commit_sha
        );
        Ok(build_info.commit_sha)
    }

    pub async fn run_pre_build_script(
        &self,
        pre_build_script_path: &Path,
        repository_name: &str,
        repository_path: &str,
    ) -> Result<(), BuildError> {
        info!(
            "Running pre-build script for {} repository",
            repository_name
        );
        let status: ExitStatus = Command::new("/bin/bash")
            .arg("-eux")
            .arg(pre_build_script_path)
            .current_dir(repository_path)
            .status()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;

        if !status.success() {
            tracing::error!("Running pre-build script failed.");
            return Err(BuildError::CommandFailed(status).into());
        }

        Ok(())
    }

    pub async fn cargo_build(
        &self,
        repository_path: &str,
        repository_name: &str,
        binary: &str,
    ) -> Result<BinaryBuildInfoOutput, BuildError> {
        info!("Cargo build {} repository", repository_name);
        let status = Command::new("nice")
            .arg("-n")
            .arg("19")
            .arg("cargo")
            .arg("build")
            .arg("--bin")
            .arg(binary)
            .arg("--release")
            .current_dir(repository_path)
            .status()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;

        if !status.success() {
            tracing::error!("Cargo build failed. Make sure that cargo is accessible.");
            return Err(BuildError::CommandFailed(status).into());
        }

        let binary_path = Path::new(repository_path)
            .join("target")
            .join("release")
            .join(binary);

        info!("Getting build info for {}", binary_path.display());
        let output = Command::new(binary_path)
            .arg("--build-info")
            .current_dir(repository_path)
            .output()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;

        if output.status.success() {
            let output =
                std::str::from_utf8(&output.stdout).change_context(BuildError::InvalidOutput)?;

            Ok(BinaryBuildInfoOutput(output.to_string()))
        } else {
            tracing::error!("Getting build info failed");
            Err(BuildError::CommandFailed(output.status).into())
        }
    }

    pub async fn copy_and_sign_binary(
        &self,
        repository_path: &str,
        repository_name: &str,
        binary: &str,
        bulid_info_output: BinaryBuildInfoOutput,
    ) -> Result<(), BuildError> {
        let binary_path = Path::new(repository_path)
            .join("target")
            .join("release")
            .join(binary);

        let current_time = time::OffsetDateTime::now_utc();

        let build_dir_for_current = self
            .create_history_dir_if_needed()
            .join(format!("{}-{}", repository_name, current_time,));

        Self::create_dir(&build_dir_for_current);
        let target_binary = build_dir_for_current.join(binary);
        tokio::fs::copy(&binary_path, target_binary)
            .await
            .change_context(BuildError::FileCopyingFailed)?;

        info!("Check that GPG key exists");
        let output = Command::new("gpg")
            .arg("--list-secret-keys")
            .output()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;
        if !output.status.success() {
            tracing::error!("Checking that GPG key exists failed");
            return Err(BuildError::CommandFailed(output.status).into());
        } else if output.stdout.is_empty() {
            info!("Generate GPG key");
            let status = Command::new("gpg")
                .arg("--batch")
                .arg("--passphrase")
                .arg("")
                .arg("--quick-generate-key")
                .arg(GPG_KEY_NAME)
                .arg("default")
                .arg("default")
                .arg("none")
                .status()
                .await
                .change_context(BuildError::ProcessWaitFailed)?;

            if !status.success() {
                tracing::error!("Generating GPG key failed");
                return Err(BuildError::CommandFailed(status).into());
            }
        }

        let signature_file_name = BuildDirCreator::encrypted_binary_name(binary);
        let signature_path = build_dir_for_current.join(&signature_file_name);
        info!("Signing and encrypting binary {}", binary);
        let status = Command::new("gpg")
            .arg("--output")
            .arg(&signature_path)
            .arg("--encrypt")
            .arg("--recipient")
            .arg(GPG_KEY_NAME)
            .arg("--sign")
            .arg(binary)
            .current_dir(&build_dir_for_current)
            .status()
            .await
            .change_context(BuildError::ProcessWaitFailed)?;
        if !status.success() {
            tracing::error!("Signing and encrypting binary failed");
            return Err(BuildError::CommandFailed(status).into());
        }

        let build_info = BuildInfo {
            commit_sha: Self::git_get_commit_sha(repository_path, repository_name).await?,
            name: repository_name.to_string(),
            timestamp: current_time.to_string(),
            build_info: bulid_info_output.0,
        };
        let build_info_file = BuildDirCreator::build_info_json_name(binary);
        let build_info_path = build_dir_for_current.join(&build_info_file);
        tokio::fs::write(
            &build_info_path,
            serde_json::to_string_pretty(&build_info)
                .change_context(BuildError::FileWritingFailed)?,
        )
        .await
        .change_context(BuildError::FileWritingFailed)?;

        let latest_dir = self.create_latest_dir_if_needed();
        tokio::fs::copy(&binary_path, latest_dir.join(binary))
            .await
            .change_context(BuildError::FileCopyingFailed)?;
        tokio::fs::copy(&signature_path, latest_dir.join(&signature_file_name))
            .await
            .change_context(BuildError::FileCopyingFailed)?;
        tokio::fs::copy(&build_info_path, latest_dir.join(&build_info_file))
            .await
            .change_context(BuildError::FileCopyingFailed)?;

        Ok(())
    }

    pub fn builder_config(&self) -> Result<&SoftwareBuilderConfig, BuildError> {
        self.config
            .software_builder()
            .ok_or(BuildError::SoftwareBuilderConfigMissing.into())
    }

    pub fn create_dir(dir: &Path) {
        if !Path::new(&dir).exists() {
            match std::fs::create_dir(&dir) {
                Ok(()) => {
                    info!("{} directory created", dir.display());
                }
                Err(e) => {
                    warn!(
                        "{} directory creation failed. Error: {:?}",
                        dir.display(),
                        e
                    );
                }
            }
        }
    }
}

const PATH_CHARACTERS_WHITELIST: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_./";

fn whitelist_chars(input: &str, whitelist: &str) -> String {
    let invalid_chars = input.chars().filter(|&c| !whitelist.contains(c)).collect();
    invalid_chars
}

fn validate_path(input: &Path) -> Result<(), BuildError> {
    if !input.is_absolute() {
        return Err(BuildError::InvalidKeyPath.into());
    }

    let unaccepted = whitelist_chars(
        input.as_os_str().to_string_lossy().as_ref(),
        PATH_CHARACTERS_WHITELIST,
    );
    if !unaccepted.is_empty() {
        tracing::error!(
            "Invalid characters {} in path: {}",
            unaccepted,
            input.display()
        );
        return Err(BuildError::InvalidKeyPath.into());
    }

    Ok(())
}

pub struct BuildDirCreator;

impl BuildDirCreator {
    pub fn create_build_dir_if_needed(config: &Config) -> PathBuf {
        let build_dir = config.storage_dir().join("build");

        if !Path::new(&build_dir).exists() {
            info!("Creating build directory");
            match std::fs::create_dir(&build_dir) {
                Ok(()) => {
                    info!("Build directory created");
                }
                Err(e) => {
                    warn!(
                        "Build directory creation failed. Error: {:?}, Directory: {}",
                        e,
                        build_dir.display()
                    );
                }
            }
        }

        build_dir
    }

    pub fn create_history_dir_if_needed(config: &Config) -> PathBuf {
        let history_dir = Self::create_build_dir_if_needed(config).join("history");

        if !Path::new(&history_dir).exists() {
            info!("Creating history directory");
            match std::fs::create_dir(&history_dir) {
                Ok(()) => {
                    info!("History directory created");
                }
                Err(e) => {
                    warn!("History directory creation failed. Error: {:?}", e);
                }
            }
        }

        history_dir
    }

    pub fn create_latest_dir_if_needed(config: &Config) -> PathBuf {
        let dir = Self::create_build_dir_if_needed(config).join("latest");

        if !Path::new(&dir).exists() {
            info!("Creating latest directory");
            match std::fs::create_dir(&dir) {
                Ok(()) => {
                    info!("Latest directory created");
                }
                Err(e) => {
                    warn!("Latest directory creation failed. Error: {:?}", e);
                }
            }
        }

        dir
    }

    pub fn encrypted_binary_name(binary: &str) -> String {
        format!("{}.gpg", binary)
    }

    pub fn build_info_json_name(binary: &str) -> String {
        format!("{}.json", binary)
    }

    pub async fn get_data(
        config: &Config,
        software: SoftwareOptions,
        download: DownloadType,
    ) -> Result<Vec<u8>, BuildError> {
        let builder_config = config
            .software_builder()
            .ok_or(BuildError::SoftwareBuilderConfigMissing)?;

        let binary = match software {
            SoftwareOptions::Manager => &builder_config.manager_binary,
            SoftwareOptions::Backend => &builder_config.backend_binary,
        };

        let latest_dir = Self::create_latest_dir_if_needed(config);

        match download {
            DownloadType::EncryptedBinary => {
                let binary_path = latest_dir.join(Self::encrypted_binary_name(binary));
                tokio::fs::read(&binary_path)
                    .await
                    .change_context(BuildError::FileReadingFailed)
                    .attach_printable(binary_path.display().to_string())
            }
            DownloadType::Info => {
                let path = latest_dir.join(Self::build_info_json_name(binary));
                tokio::fs::read(&path)
                    .await
                    .change_context(BuildError::FileReadingFailed)
                    .attach_printable(path.display().to_string())
            }
        }
    }
}
