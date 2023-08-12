use std::{
    io::Write,
    net::SocketAddr,
    num::{NonZeroU8},
    path::{Path, PathBuf},
};

use error_stack::{Report, Result, ResultExt};
use manager_model::DataEncryptionKey;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{utils::IntoReportExt, api::{GetConfig}};

use super::GetConfigError;

pub const CONFIG_FILE_NAME: &str = "manager_config.toml";

pub const DEFAULT_CONFIG_FILE_TEXT: &str = r#"

# Required
# api_key = "password"

[socket]
public_api = "127.0.0.1:5000"

[environment]
secure_storage_dir = "/app-secure-storage/app"
scripts_dir = "/app-server-tools/manager-tools"

# [encryption_key_provider]
# manager_base_url = "http://127.0.0.1:5000"
# key_name = "test-server"

# [[server_encryption_keys]]
# name = "test-server"
# key_path = "data-key.key"

# [software_update_provider]
# manager_base_url = "http://127.0.0.1:5000"
# binary_signing_public_key = "binary-key.pub"
# manager_install_location = "/home/app/binaries/app-manager"
# backend_install_location = "/app-secure-storage/app/binaries/app-backend"

# [software_builder]
# manager_download_key_path = "app-manager.key"
# manager_download_git_address = "git repository ssh address"
# manager_branch = "main"
# manager_binary = "app-manager"
# manager_pre_build_script = "/path/to/script/app-manager-pre-build.sh" # Optional
# backend_download_key_path = "app-backend.key"
# backend_download_git_address = "git repository ssh address"
# backend_branch = "main"
# backend_binary = "app-backend"
# backend_pre_build_script = "/path/to/script/app-backend-pre-build.sh" # Optional

# [reboot_if_needed]
# time = "12:00"

# [system_info]
# log_services = ["app-manager", "app-backend"]
# [[system_info.remote_managers]]
# name = "test-server"
# manager_base_url = "http://127.0.0.1:5000"

# [tls]
# public_api_cert = "server_config/public_api.cert"
# public_api_key = "server_config/public_api.key"
# root_certificate = "server_config/public_api.key"
"#;

#[derive(thiserror::Error, Debug)]
pub enum ConfigFileError {
    #[error("Save default")]
    SaveDefault,
    #[error("Not a directory")]
    NotDirectory,
    #[error("Load config file")]
    LoadConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    pub debug: Option<bool>,
    /// API key for manager API. All managers instances must use the same key.
    ///
    /// If the key is wrong the API access is denied untill manager is restarted.
    pub api_key: String,
    pub server_encryption_keys: Option<Vec<ServerEncryptionKey>>,
    pub encryption_key_provider: Option<EncryptionKeyProviderConfig>,
    pub reboot_if_needed: Option<RebootIfNeededConfig>,
    pub software_update_provider: Option<SoftwareUpdateProviderConfig>,
    pub software_builder: Option<SoftwareBuilderConfig>,
    pub system_info: Option<SystemInfoConfig>,
    pub socket: SocketConfig,
    pub environment: EnvironmentConfig,
    /// TLS is required if debug setting is false.
    pub tls: Option<TlsConfig>,
}

impl ConfigFile {
    pub fn save_default(dir: impl AsRef<Path>) -> Result<(), ConfigFileError> {
        let file_path =
            Self::default_config_file_path(dir).change_context(ConfigFileError::SaveDefault)?;
        let mut file = std::fs::File::create(file_path).into_error(ConfigFileError::SaveDefault)?;
        file.write_all(DEFAULT_CONFIG_FILE_TEXT.as_bytes())
            .into_error(ConfigFileError::SaveDefault)?;
        Ok(())
    }

    pub fn load(dir: impl AsRef<Path>) -> Result<ConfigFile, ConfigFileError> {
        let file_path =
            Self::default_config_file_path(&dir).change_context(ConfigFileError::LoadConfig)?;
        if !file_path.exists() {
            Self::save_default(dir).change_context(ConfigFileError::LoadConfig)?;
        }

        let config_string =
            std::fs::read_to_string(file_path).into_error(ConfigFileError::LoadConfig)?;
        toml::from_str(&config_string).into_error(ConfigFileError::LoadConfig)
    }

    pub fn default_config_file_path(dir: impl AsRef<Path>) -> Result<PathBuf, ConfigFileError> {
        if !dir.as_ref().is_dir() {
            return Err(Report::new(ConfigFileError::NotDirectory));
        }
        let mut file_path = dir.as_ref().to_path_buf();
        file_path.push(CONFIG_FILE_NAME);
        return Ok(file_path);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SocketConfig {
    pub public_api: SocketAddr,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TlsConfig {
    pub public_api_cert: PathBuf,
    pub public_api_key: PathBuf,

    /// Root certificate for HTTP client for checking API calls.
    pub root_certificate: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerEncryptionKey {
    pub name: String,
    pub key_path: PathBuf,
}

impl ServerEncryptionKey {
    pub async fn read_encryption_key(&self) -> Result<DataEncryptionKey, GetConfigError> {
        tokio::fs::read_to_string(self.key_path.as_path())
            .await
            .into_error(GetConfigError::EncryptionKeyLoadingFailed)
            .map(|key| DataEncryptionKey { key })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EncryptionKeyProviderConfig {
    /// Name of key which will be requested.
    pub key_name: String,
    pub manager_base_url: Url,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SoftwareUpdateProviderConfig {
    /// Manager instance URL which is used to check if new software is available.
    pub manager_base_url: Url,
    /// GPG public key
    pub binary_signing_public_key: PathBuf,
    pub manager_install_location: PathBuf,
    pub backend_install_location: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SoftwareBuilderConfig {
    pub manager_download_key_path: PathBuf,
    pub manager_download_git_address: String,
    pub manager_branch: String,
    pub manager_binary: String,
    /// Optional. Working dir of the script is repository root.
    pub manager_pre_build_script: Option<PathBuf>,
    pub backend_download_key_path: PathBuf,
    pub backend_download_git_address: String,
    pub backend_branch: String,
    pub backend_binary: String,
    /// Optional. Working dir of the script is repository root.
    pub backend_pre_build_script: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RebootIfNeededConfig {
    /// Time when reboot should be done. Format "hh:mm". For example "12:00".
    pub time: TimeValue,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(try_from = "String")]
pub struct TimeValue {
    pub hours: u8,
    pub minutes: u8,
}

impl TryFrom<String> for TimeValue {
    type Error = String;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let mut iter = value.trim().split(':');
        let values: Vec<&str> = iter.collect();
        match values[..] {
            [hours, minutes] => {
                let hours: u8 = hours.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                let minutes: u8 = minutes.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                Ok(TimeValue {hours, minutes})
            }
            _ => {
                Err(format!("Unknown values: {:?}", values))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentConfig {
    pub secure_storage_dir: PathBuf,
    pub scripts_dir: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemInfoConfig {
    pub log_services: Vec<String>,
    pub remote_managers: Option<Vec<ManagerInstance>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ManagerInstance {
    pub name: String,
    pub manager_base_url: Url,
}
