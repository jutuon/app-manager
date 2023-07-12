use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};


#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default, PartialEq, Eq)]
pub struct DataEncryptionKey {
    /// Base64 key
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, IntoParams)]
pub struct ServerNameText {
    pub server: String,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct SoftwareOptionsQueryParam {
    pub software_options: SoftwareOptions,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, ToSchema)]
pub enum SoftwareOptions {
    Manager,
    Backend,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DownloadTypeQueryParam {
    pub download_type: DownloadType,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, ToSchema)]
pub enum DownloadType {
    /// HTTP GET returns JSON with software info.
    Info,
    /// HTTP GET returns encrypted binary.
    EncryptedBinary,
}

/// Reboot computer directly after software update.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct RebootQueryParam {
    pub reboot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct SoftwareInfo {
    pub current_software: Vec<BuildInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuildInfo {
    pub commit_sha: String,
    pub name: String,
    pub timestamp: String,
}
