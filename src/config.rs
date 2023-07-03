pub mod args;
pub mod file;

use std::{
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    vec,
};

use error_stack::{IntoReport, Result, ResultExt};
use reqwest::Url;
use rustls_pemfile::{certs, rsa_private_keys};
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

use crate::{utils::IntoReportExt};

use self::{
    file::{
        ConfigFile,
        SocketConfig, ServerEncryptionKey, EncryptionKeyProviderConfig, SoftwareUpdateProviderConfig,
    },
};

#[derive(thiserror::Error, Debug)]
pub enum GetConfigError {
    #[error("Get working directory error")]
    GetWorkingDir,
    #[error("File loading failed")]
    LoadFileError,
    #[error("Load config file")]
    LoadConfig,

    #[error("TLS config is required when debug mode is off")]
    TlsConfigMissing,
    #[error("TLS config creation error")]
    CreateTlsConfig,

    // Server runtime errors
    #[error("Encryption key loading failed")]
    EncryptionKeyLoadingFailed,
}

#[derive(Debug)]
pub struct Config {
    file: ConfigFile,

    // TLS
    public_api_tls_config: Option<Arc<ServerConfig>>,
    root_certificate: Option<reqwest::Certificate>,
}

impl Config {
    pub fn socket(&self) -> &SocketConfig {
        &self.file.socket
    }

    /// Server should run in debug mode.
    ///
    /// Debug mode changes:
    /// * Swagger UI is enabled.
    /// * Disabling HTTPS is possbile.
    pub fn debug_mode(&self) -> bool {
        self.file.debug.unwrap_or(false)
    }

    pub fn encryption_keys(&self) -> &[ServerEncryptionKey] {
        self.file.server_encryption_keys
            .as_ref()
            .map(|d| d.as_slice())
            .unwrap_or(&[])
    }

    pub fn encryption_key_provider(&self) -> Option<&EncryptionKeyProviderConfig> {
        self.file.encryption_key_provider
            .as_ref()
    }

    pub fn software_update_provider(&self) -> Option<&SoftwareUpdateProviderConfig> {
        self.file.software_update_provider
            .as_ref()
    }

    pub fn api_key(&self) -> &str {
        &self.file.api_key
    }

    pub fn public_api_tls_config(&self) -> Option<&Arc<ServerConfig>> {
        self.public_api_tls_config.as_ref()
    }

    pub fn root_certificate(&self) -> Option<&reqwest::Certificate> {
        self.root_certificate.as_ref()
    }
}

pub fn get_config() -> Result<Config, GetConfigError> {
    let current_dir = std::env::current_dir().into_error(GetConfigError::GetWorkingDir)?;
    let mut file_config =
        file::ConfigFile::load(current_dir).change_context(GetConfigError::LoadFileError)?;
    let args_config = args::get_config();

    let public_api_tls_config = match file_config.tls.clone() {
        Some(tls_config) => Some(Arc::new(generate_server_config(
            tls_config.public_api_key.as_path(),
            tls_config.public_api_cert.as_path(),
        )?)),
        None => None,
    };

    let root_certificate = match file_config.tls.clone() {
        Some(tls_config) =>
            Some(load_root_certificate(&tls_config.root_certificate)?),
        None => None,
    };

    if public_api_tls_config.is_none() && !file_config.debug.unwrap_or_default() {
        return Err(GetConfigError::TlsConfigMissing)
            .into_report()
            .attach_printable("TLS must be configured when debug mode is false");
    }

    Ok(Config {
        file: file_config,
        public_api_tls_config,
        root_certificate,
    })
}

fn load_root_certificate(
    cert_path: &Path,
) -> Result<reqwest::Certificate, GetConfigError> {
    let mut cert_reader =
        BufReader::new(std::fs::File::open(cert_path).into_error(GetConfigError::CreateTlsConfig)?);
    let all_certs = certs(&mut cert_reader).into_error(GetConfigError::CreateTlsConfig)?;
    let cert = if let [cert] = &all_certs[..] {
        reqwest::Certificate::from_der(&cert.clone())
    } else if all_certs.is_empty() {
        return Err(GetConfigError::CreateTlsConfig)
            .into_report()
            .attach_printable("No cert found");
    } else {
        return Err(GetConfigError::CreateTlsConfig)
            .into_report()
            .attach_printable("Only one cert supported");
    }.into_error(GetConfigError::CreateTlsConfig)?;
    Ok(cert)
}

fn generate_server_config(
    key_path: &Path,
    cert_path: &Path,
) -> Result<ServerConfig, GetConfigError> {
    let mut key_reader =
        BufReader::new(std::fs::File::open(key_path).into_error(GetConfigError::CreateTlsConfig)?);
    let all_keys = rsa_private_keys(&mut key_reader).into_error(GetConfigError::CreateTlsConfig)?;

    let key = if let [key] = &all_keys[..] {
        PrivateKey(key.clone())
    } else if all_keys.is_empty() {
        return Err(GetConfigError::CreateTlsConfig)
            .into_report()
            .attach_printable("No key found");
    } else {
        return Err(GetConfigError::CreateTlsConfig)
            .into_report()
            .attach_printable("Only one key supported");
    };

    let mut cert_reader =
        BufReader::new(std::fs::File::open(cert_path).into_error(GetConfigError::CreateTlsConfig)?);
    let all_certs = certs(&mut cert_reader).into_error(GetConfigError::CreateTlsConfig)?;
    let cert = if let [cert] = &all_certs[..] {
        Certificate(cert.clone())
    } else if all_certs.is_empty() {
        return Err(GetConfigError::CreateTlsConfig)
            .into_report()
            .attach_printable("No cert found");
    } else {
        return Err(GetConfigError::CreateTlsConfig)
            .into_report()
            .attach_printable("Only one cert supported");
    };

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth() // TODO: configure at some point
        .with_single_cert(vec![cert], key)
        .into_error(GetConfigError::CreateTlsConfig)?;

    Ok(config)
}
