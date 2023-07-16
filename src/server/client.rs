
use std::{net::SocketAddr, io::BufReader, collections::VecDeque};

use manager_api_client::{apis::{configuration::{Configuration, ApiKey}, manager_api::{get_encryption_key, post_request_build_software}}, models::DataEncryptionKey, manual_additions::get_latest_software_fixed};
use reqwest::Certificate;
use tracing::info;
use tracing_subscriber::fmt::format;
use url::Url;

use crate::{config::{Config, file::SoftwareUpdateProviderConfig}, api::{self, manager::data::{SoftwareInfo, SoftwareOptions, BuildInfo},}, utils::IntoReportExt};

use error_stack::{Result, ResultExt, IntoReport};


#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Client build failed")]
    ClientBuildFailed,

    #[error("API request failed")]
    ApiRequest,

    #[error("Database call failed")]
    DatabaseError,

    #[error("Manager API URL not configured for {0}")]
    ManagerApiUrlNotConfigured(&'static str),

    #[error("Missing value")]
    MissingValue,

    #[error("Invalid value")]
    InvalidValue,

    #[error("Missing configuration")]
    MissingConfiguration,
}


#[derive(Debug)]
pub struct ApiClient {
    encryption_key_provider: Option<Configuration>,
    software_update_provider: Option<Configuration>,
}

impl ApiClient {
    pub fn new(config: &Config) -> Result<Self, ApiError> {
        let api_key = manager_api_client::apis::configuration::ApiKey {
            prefix: None,
            key: config.api_key().to_string(),
        };

        let mut client = reqwest::ClientBuilder::new()
            .tls_built_in_root_certs(false);
        if let Some(cert) = config.root_certificate() {
            client = client.add_root_certificate(cert.clone());
        }

        let client = client.build().into_error(ApiError::ClientBuildFailed)?;

        let encryption_key_provider = config.encryption_key_provider().map(|url| {
            let url = url.manager_base_url.as_str().trim_end_matches('/').to_string();

            info!("encryption_key_provider API base url: {}", url);

            Configuration {
                base_path: url,
                client: client.clone(),
                api_key: Some(api_key.clone()),
                ..Configuration::default()
            }
        });

        let software_update_provider = config.software_update_provider().map(|url| {
            let url = url.manager_base_url.as_str().trim_end_matches('/').to_string();

            info!("software_update_provider API base url: {}", url);

            Configuration {
                base_path: url,
                client: client.clone(),
                api_key: Some(api_key.clone()),
                ..Configuration::default()
            }
        });

        Ok(Self {
            encryption_key_provider,
            software_update_provider,
        })
    }

    pub fn encryption_key_provider_config(&self) -> Result<&Configuration, ApiError> {
        self.encryption_key_provider
            .as_ref()
            .ok_or(ApiError::ManagerApiUrlNotConfigured("encryption_key_provider_config").into())
    }

    pub fn software_update_provider_config(&self) -> Result<&Configuration, ApiError> {
        self.software_update_provider
            .as_ref()
            .ok_or(ApiError::ManagerApiUrlNotConfigured("software_update_provider_config").into())
    }
}

pub struct ApiManager<'a> {
    config: &'a Config,
    api_client: &'a ApiClient,
}

impl<'a> ApiManager<'a> {
    pub fn new(
        config: &'a Config,
        api_client: &'a ApiClient,
    ) -> Self {
        Self {
            config,
            api_client,
        }
    }

    pub async fn get_encryption_key(
        &self,
    ) -> Result<DataEncryptionKey, ApiError> {
        let provider =
            self.config.encryption_key_provider().ok_or(ApiError::MissingConfiguration)?;

        let key = get_encryption_key(
            self.api_client.encryption_key_provider_config()?,
            &provider.key_name,
        ).await.into_error(ApiError::ApiRequest)?;

        Ok(DataEncryptionKey { key: key.key })
    }

    pub async fn get_latest_build_info_raw(
        &self,
        options: SoftwareOptions,
    ) -> Result<Vec<u8>, ApiError> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        get_latest_software_fixed(
            self.api_client.software_update_provider_config()?,
            converted_options,
            manager_api_client::models::DownloadType::Info,
        ).await.into_error(ApiError::ApiRequest)
    }

    pub async fn get_latest_build_info(
        &self,
        options: SoftwareOptions,
    ) -> Result<BuildInfo, ApiError> {
        let info_json = self.get_latest_build_info_raw(options).await?;
        let info: BuildInfo = serde_json::from_slice(&info_json)
            .into_error(ApiError::InvalidValue)?;
        Ok(info)
    }

    pub async fn get_latest_encrypted_software_binary(
        &self,
        options: SoftwareOptions,
    ) -> Result<Vec<u8>, ApiError> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        let binary = get_latest_software_fixed(
            self.api_client.software_update_provider_config()?,
            converted_options,
            manager_api_client::models::DownloadType::EncryptedBinary,
        ).await.into_error(ApiError::ApiRequest)?;

        Ok(binary)
    }

    pub async fn request_build_software_from_build_server(
        &self,
        options: SoftwareOptions,
    ) -> Result<(), ApiError> {
        let converted_options = match options {
            SoftwareOptions::Manager => manager_api_client::models::SoftwareOptions::Manager,
            SoftwareOptions::Backend => manager_api_client::models::SoftwareOptions::Backend,
        };

        post_request_build_software(
            self.api_client.software_update_provider_config()?,
            converted_options,
        ).await.into_error(ApiError::ApiRequest)
    }
}
