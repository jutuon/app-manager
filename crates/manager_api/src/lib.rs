#![deny(unsafe_code)]
#![warn(unused_crate_dependencies)]


//! This crate provides a wrapper for the internal API of the server.
//! Prevents exposing api_client crate model types to server code.


use manager_api_client::{apis::{manager_api::{get_encryption_key, post_request_build_software, GetLatestSoftwareError, post_request_software_update}}, manual_additions::get_latest_software_fixed};
use manager_model::{SoftwareOptions, BuildInfo, SystemInfo, CommandOutput, DataEncryptionKey};

pub use manager_api_client::apis::{Error, configuration::{ApiKey, Configuration}};
pub use manager_api_client::apis::manager_api::{
    GetEncryptionKeyError,
    GetSystemInfoAllError,
    GetSystemInfoError,
    GetSoftwareInfoError,
    PostRequestBuildSoftwareError,
    PostRequestSoftwareUpdateError,
};

pub struct ManagerApi;

impl ManagerApi {
    pub async fn get_encryption_key(
        configuration: &Configuration,
        server: &str,
    ) -> Result<DataEncryptionKey, Error<GetEncryptionKeyError>> {
        let key = get_encryption_key(
            configuration,
            server,
        ).await?;

        Ok(DataEncryptionKey { key: key.key })
    }

    pub async fn get_latest_build_info_raw(
        configuration: &Configuration,
        options: SoftwareOptions,
    ) -> Result<Vec<u8>, Error<GetLatestSoftwareError>> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        get_latest_software_fixed(
            configuration,
            converted_options,
            manager_api_client::models::DownloadType::Info,
        ).await
    }

    pub async fn get_latest_build_info(
        configuration: &Configuration,
        options: SoftwareOptions,
    ) -> Result<BuildInfo, Error<GetLatestSoftwareError>> {
        let info_json = Self::get_latest_build_info_raw(configuration, options).await?;
        let info: BuildInfo = serde_json::from_slice(&info_json)
            .map_err(Error::Serde)?;
        Ok(info)
    }

    pub async fn get_latest_encrypted_software_binary(
        configuration: &Configuration,
        options: SoftwareOptions,
    ) -> Result<Vec<u8>, Error<GetLatestSoftwareError>> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        let binary = get_latest_software_fixed(
            configuration,
            converted_options,
            manager_api_client::models::DownloadType::EncryptedBinary,
        ).await?;

        Ok(binary)
    }

    pub async fn request_build_software_from_build_server(
        configuration: &Configuration,
        options: SoftwareOptions,
    ) -> Result<(), Error<PostRequestBuildSoftwareError>> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        post_request_build_software(
            configuration,
            converted_options,
        ).await
    }

    pub async fn system_info(
        configuration: &Configuration,
    ) -> Result<SystemInfo, Error<GetSystemInfoError>> {
        let system_info = manager_api_client::apis::manager_api::get_system_info(
            configuration,
        ).await?;

        let info_vec = system_info.info
            .into_iter()
            .map(|info| {
                CommandOutput {
                    name: info.name,
                    output: info.output,
                }
            })
            .collect::<Vec<CommandOutput>>();

        Ok(SystemInfo {
            name: system_info.name,
            info: info_vec,
        })
    }

    pub async fn request_update_software(
        configuration: &Configuration,
        options: SoftwareOptions,
        reboot: bool,
    ) -> Result<(), Error<PostRequestSoftwareUpdateError>> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        post_request_software_update(configuration, converted_options, reboot)
            .await
    }
}
