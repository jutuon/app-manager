//! CLI API client
//!

use url::Url;

use crate::{config::{args::{ApiCommand, ApiClientMode}}, server::client::{ApiError}};


use error_stack::{Result, ResultExt};
use manager_api::{ApiKey, Configuration, ManagerApi};
use manager_model::{ResetDataQueryParam};


pub async fn handle_api_client_mode(
    args: ApiClientMode,
) -> Result<(), ApiError> {
    let configuration = create_configration(
        args.api_key,
        args.api_url,
    )?;

    match args.api_command {
        ApiCommand::EncryptionKey { encryption_key_name } => {
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
        ApiCommand::RequestUpdateSoftware { software, reboot, reset_data } => {
            ManagerApi::request_update_software(&configuration, software, reboot, ResetDataQueryParam { reset_data })
                .await
                .change_context(ApiError::ApiRequest)?;
            println!("Update requested for {:?}, reboot: {}, reset_data: {}", software, reboot, reset_data);
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
) -> Result<Configuration, ApiError> {
    let api_key = ApiKey {
        prefix: None,
        key: api_key,
    };

    let client = reqwest::ClientBuilder::new()
        .tls_built_in_root_certs(false);
    // TODO: TLS support
    // if let Some(cert) = config.root_certificate() {
    //     client = client.add_root_certificate(cert.clone());
    // }

    let client = client.build().change_context(ApiError::ClientBuildFailed)?;


    let url = base_url
        .as_str()
        .trim_end_matches('/')
        .to_string();

    let configuration = Configuration {
        base_path: url,
        client: client.clone(),
        api_key: Some(api_key.clone()),
        ..Configuration::default()
    };

    Ok(configuration)
}
