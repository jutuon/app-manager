#![deny(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unused_features)]
#![warn(unused_crate_dependencies)]

//! This crate provides a wrapper for the internal API of the server.
//! Prevents exposing api_client crate model types to server code.

pub use manager_api_client::apis::{
    configuration::{ApiKey, Configuration},
    manager_api::{
        GetEncryptionKeyError, GetSoftwareInfoError, GetSystemInfoAllError, GetSystemInfoError,
        PostRequestBuildSoftwareError, PostRequestSoftwareUpdateError,
    },
    Error,
};
use manager_api_client::{
    apis::manager_api::{
        get_encryption_key, post_request_build_software, post_request_software_update,
        GetLatestSoftwareError, get_system_info_all, get_software_info,
    },
    manual_additions::get_latest_software_fixed,
};
use manager_model::{BuildInfo, CommandOutput, DataEncryptionKey, SoftwareOptions, SystemInfo, SystemInfoList, SoftwareInfo, ResetDataQueryParam};

pub struct ManagerApi;

impl ManagerApi {
    pub async fn get_encryption_key(
        configuration: &Configuration,
        server: &str,
    ) -> Result<DataEncryptionKey, Error<GetEncryptionKeyError>> {
        let key = get_encryption_key(configuration, server).await?;

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
        )
        .await
    }

    pub async fn get_latest_build_info(
        configuration: &Configuration,
        options: SoftwareOptions,
    ) -> Result<BuildInfo, Error<GetLatestSoftwareError>> {
        let info_json = Self::get_latest_build_info_raw(configuration, options).await?;
        let info: BuildInfo = serde_json::from_slice(&info_json).map_err(Error::Serde)?;
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
        )
        .await?;

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

        post_request_build_software(configuration, converted_options).await
    }

    pub async fn system_info_all(
        configuration: &Configuration,
    ) -> Result<SystemInfoList, Error<GetSystemInfoAllError>> {
        let system_info =
            get_system_info_all(configuration)
                .await?;

        let info_vec = system_info
            .info
            .into_iter()
            .map(|info| {
                let cmd_vec = info
                    .info
                    .into_iter()
                    .map(|info| CommandOutput {
                        name: info.name,
                        output: info.output,
                    })
                    .collect::<Vec<CommandOutput>>();
                SystemInfo {
                    name: info.name,
                    info: cmd_vec,
                }
            })
            .collect::<Vec<SystemInfo>>();

        Ok(SystemInfoList { info: info_vec })
    }

    pub async fn system_info(
        configuration: &Configuration,
    ) -> Result<SystemInfo, Error<GetSystemInfoError>> {
        let system_info =
            manager_api_client::apis::manager_api::get_system_info(configuration).await?;

        let info_vec = system_info
            .info
            .into_iter()
            .map(|info| CommandOutput {
                name: info.name,
                output: info.output,
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
        reset_data: ResetDataQueryParam,
    ) -> Result<(), Error<PostRequestSoftwareUpdateError>> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        post_request_software_update(configuration, converted_options, reboot).await
    }

     pub async fn software_info(
        configuration: &Configuration,
    ) -> Result<SoftwareInfo, Error<GetSoftwareInfoError>> {
        let info =
            get_software_info(configuration)
                .await?;

        let info_vec = info
            .current_software
            .into_iter()
            .map(|info| BuildInfo {
                commit_sha: info.commit_sha,
                build_info: info.build_info,
                name: info.name,
                timestamp: info.timestamp,
            })
            .collect::<Vec<BuildInfo>>();

        Ok(SoftwareInfo {
            current_software: info_vec,
        })
    }
}
