//! CLI API client
//!

use error_stack::{Result, ResultExt};
use manager_api::{ApiKey, Configuration, ManagerApi};
use manager_model::ResetDataQueryParam;
use reqwest::Certificate;
use url::Url;

use crate::{
    config::args::{ApiClientMode, ApiCommand},
    server::client::ApiError,
};

pub async fn handle_api_client_mode(args: ApiClientMode) -> Result<(), ApiError> {
    let api_key = args
        .api_key()
        .change_context(ApiError::MissingConfiguration)?;
    let api_url = args
        .api_url()
        .change_context(ApiError::MissingConfiguration)?;
    let certificate = args
        .root_certificate()
        .change_context(ApiError::MissingConfiguration)?;
    let configuration = create_configration(api_key, api_url, certificate)?;

    match args.api_command {
        ApiCommand::EncryptionKey {
            encryption_key_name,
        } => {
            let key = ManagerApi::get_encryption_key(&configuration, &encryption_key_name)
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("Name: {}", encryption_key_name);
            println!("Key:  {}", key.key);
        }
        ApiCommand::LatestBuildInfo { software } => {
            let info = ManagerApi::get_latest_build_info(&configuration, software)
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("{:#?}", info);
        }
        ApiCommand::RequestBuildSoftware { software } => {
            ManagerApi::request_build_software_from_build_server(&configuration, software)
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("Build requested for {:?}", software);
        }
        ApiCommand::RequestUpdateSoftware {
            software,
            reboot,
            reset_data,
        } => {
            ManagerApi::request_update_software(
                &configuration,
                software,
                reboot,
                ResetDataQueryParam { reset_data },
            )
            .await
            .change_context(ApiError::ApiRequest)?;
            println!(
                "Update requested for {:?}, reboot: {}, reset_data: {}",
                software, reboot, reset_data
            );
        }
        ApiCommand::RequestRestartBackend { reset_data } => {
            ManagerApi::restart_backend(&configuration, ResetDataQueryParam { reset_data })
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("Restart backend requested, reset_data: {}", reset_data);
        }
        ApiCommand::SystemInfoAll => {
            let info = ManagerApi::system_info_all(&configuration)
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("{:#?}", info);
        }
        ApiCommand::SystemInfo => {
            let info = ManagerApi::system_info(&configuration)
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("{:#?}", info);
        }
        ApiCommand::SoftwareInfo => {
            let info = ManagerApi::software_info(&configuration)
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("{:#?}", info);
        }
    }

    Ok(())
}

pub fn create_configration(
    api_key: String,
    base_url: Url,
    root_certificate: Option<Certificate>,
) -> Result<Configuration, ApiError> {
    let api_key = ApiKey {
        prefix: None,
        key: api_key,
    };

    let client = reqwest::ClientBuilder::new().tls_built_in_root_certs(false);
    let client = if let Some(cert) = root_certificate {
        client.add_root_certificate(cert)
    } else {
        client
    }
    .build()
    .change_context(ApiError::ClientBuildFailed)?;

    let url = base_url.as_str().trim_end_matches('/').to_string();

    let configuration = Configuration {
        base_path: url,
        client,
        api_key: Some(api_key.clone()),
        ..Configuration::default()
    };

    Ok(configuration)
}
