use std::{
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    vec,
};

use error_stack::{Result, ResultExt};
use rustls_pemfile::{certs, rsa_private_keys};
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tracing::{info, log::warn};

use self::file::{
    ConfigFile, EncryptionKeyProviderConfig, RebootIfNeededConfig, ServerEncryptionKey,
    SocketConfig, SoftwareBuilderConfig, SoftwareUpdateProviderConfig, SystemInfoConfig,
};


pub mod args;
pub mod file;
pub mod info;

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

    #[error("Missing script")]
    ScriptLocationError,
}

#[derive(Debug)]
pub struct Config {
    file: ConfigFile,
    script_locations: ScriptLocations,

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
    /// * Checking available scripts is disabled.
    pub fn debug_mode(&self) -> bool {
        self.file.debug.unwrap_or(false)
    }

    pub fn encryption_keys(&self) -> &[ServerEncryptionKey] {
        self.file
            .server_encryption_keys
            .as_ref()
            .map(|d| d.as_slice())
            .unwrap_or(&[])
    }

    pub fn encryption_key_provider(&self) -> Option<&EncryptionKeyProviderConfig> {
        self.file.encryption_key_provider.as_ref()
    }

    pub fn software_update_provider(&self) -> Option<&SoftwareUpdateProviderConfig> {
        self.file.software_update_provider.as_ref()
    }

    pub fn software_builder(&self) -> Option<&SoftwareBuilderConfig> {
        self.file.software_builder.as_ref()
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

    pub fn script_locations(&self) -> &ScriptLocations {
        &self.script_locations
    }

    pub fn system_info(&self) -> Option<&SystemInfoConfig> {
        self.file.system_info.as_ref()
    }

    pub fn secure_storage_dir(&self) -> &Path {
        &self.file.environment.secure_storage_dir
    }

    pub fn reboot_if_needed(&self) -> &Option<RebootIfNeededConfig> {
        &self.file.reboot_if_needed
    }
}

pub fn get_config() -> Result<Config, GetConfigError> {
    let _args_config = args::get_config();

    let current_dir = std::env::current_dir().change_context(GetConfigError::GetWorkingDir)?;
    let file_config =
        file::ConfigFile::load(current_dir).change_context(GetConfigError::LoadFileError)?;

    let public_api_tls_config = match file_config.tls.clone() {
        Some(tls_config) => Some(Arc::new(generate_server_config(
            tls_config.public_api_key.as_path(),
            tls_config.public_api_cert.as_path(),
        )?)),
        None => None,
    };

    let root_certificate = match file_config.tls.clone() {
        Some(tls_config) => Some(load_root_certificate(&tls_config.root_certificate)?),
        None => None,
    };

    if public_api_tls_config.is_none() && !file_config.debug.unwrap_or_default() {
        return Err(GetConfigError::TlsConfigMissing)
            .attach_printable("TLS must be configured when debug mode is false");
    }

    let script_locations = check_script_locations(
        &file_config.environment.scripts_dir,
        file_config.debug.unwrap_or_default(),
    )?;

    Ok(Config {
        file: file_config,
        script_locations,
        public_api_tls_config,
        root_certificate,
    })
}

fn check_script_locations(
    script_dir: &Path,
    is_debug: bool,
) -> Result<ScriptLocations, GetConfigError> {
    let open_encryption = script_dir.join("open-encryption.sh");
    let close_encryption = script_dir.join("close-encryption.sh");
    let is_default_encryption_password = script_dir.join("is-default-encryption-password.sh");
    let change_encryption_password = script_dir.join("change-encryption-password.sh");
    let start_backend = script_dir.join("start-backend.sh");
    let stop_backend = script_dir.join("stop-backend.sh");
    let print_logs = script_dir.join("print-logs.sh");

    let mut errors = vec![];

    if !open_encryption.exists() {
        errors.push(format!("Script not found: {}", open_encryption.display()));
    }
    if !close_encryption.exists() {
        errors.push(format!("Script not found: {}", close_encryption.display()));
    }
    if !is_default_encryption_password.exists() {
        errors.push(format!(
            "Script not found: {}",
            is_default_encryption_password.display()
        ));
    }
    if !change_encryption_password.exists() {
        errors.push(format!(
            "Script not found: {}",
            change_encryption_password.display()
        ));
    }
    if !start_backend.exists() {
        errors.push(format!("Script not found: {}", start_backend.display()));
    }
    if !stop_backend.exists() {
        errors.push(format!("Script not found: {}", stop_backend.display()));
    }
    if !print_logs.exists() {
        errors.push(format!("Script not found: {}", print_logs.display()));
    }

    if errors.is_empty() || is_debug {
        if errors.is_empty() {
            info!("All scripts found");
        } else {
            warn!("Some scripts are missing.\n{}", errors.join("\n"));
        }
        Ok(ScriptLocations {
            open_encryption,
            close_encryption,
            is_default_encryption_password,
            change_encryption_password,
            start_backend,
            stop_backend,
            print_logs,
        })
    } else {
        Err(GetConfigError::ScriptLocationError)
            .attach_printable(errors.join("\n"))
    }
}

fn load_root_certificate(cert_path: &Path) -> Result<reqwest::Certificate, GetConfigError> {
    let mut cert_reader =
        BufReader::new(std::fs::File::open(cert_path).change_context(GetConfigError::CreateTlsConfig)?);
    let all_certs = certs(&mut cert_reader).change_context(GetConfigError::CreateTlsConfig)?;
    let cert = if let [cert] = &all_certs[..] {
        reqwest::Certificate::from_der(&cert.clone())
    } else if all_certs.is_empty() {
        return Err(GetConfigError::CreateTlsConfig)
            .attach_printable("No cert found");
    } else {
        return Err(GetConfigError::CreateTlsConfig)
            .attach_printable("Only one cert supported");
    }
    .change_context(GetConfigError::CreateTlsConfig)?;
    Ok(cert)
}

fn generate_server_config(
    key_path: &Path,
    cert_path: &Path,
) -> Result<ServerConfig, GetConfigError> {
    let mut key_reader =
        BufReader::new(std::fs::File::open(key_path).change_context(GetConfigError::CreateTlsConfig)?);
    let all_keys = rsa_private_keys(&mut key_reader).change_context(GetConfigError::CreateTlsConfig)?;

    let key = if let [key] = &all_keys[..] {
        PrivateKey(key.clone())
    } else if all_keys.is_empty() {
        return Err(GetConfigError::CreateTlsConfig)
            .attach_printable("No key found");
    } else {
        return Err(GetConfigError::CreateTlsConfig)
            .attach_printable("Only one key supported");
    };

    let mut cert_reader =
        BufReader::new(std::fs::File::open(cert_path).change_context(GetConfigError::CreateTlsConfig)?);
    let all_certs = certs(&mut cert_reader).change_context(GetConfigError::CreateTlsConfig)?;
    let cert = if let [cert] = &all_certs[..] {
        Certificate(cert.clone())
    } else if all_certs.is_empty() {
        return Err(GetConfigError::CreateTlsConfig)
            .attach_printable("No cert found");
    } else {
        return Err(GetConfigError::CreateTlsConfig)
            .attach_printable("Only one cert supported");
    };

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth() // TODO: configure at some point
        .with_single_cert(vec![cert], key)
        .change_context(GetConfigError::CreateTlsConfig)?;

    Ok(config)
}

#[derive(Debug)]
pub struct ScriptLocations {
    pub open_encryption: PathBuf,
    pub close_encryption: PathBuf,
    pub is_default_encryption_password: PathBuf,
    pub change_encryption_password: PathBuf,
    pub start_backend: PathBuf,
    pub stop_backend: PathBuf,
    pub print_logs: PathBuf,
}

impl ScriptLocations {
    pub fn open_encryption(&self) -> &Path {
        &self.open_encryption
    }

    pub fn close_encryption(&self) -> &Path {
        &self.close_encryption
    }

    pub fn is_default_encryption_password(&self) -> &Path {
        &self.is_default_encryption_password
    }

    pub fn change_encryption_password(&self) -> &Path {
        &self.change_encryption_password
    }

    pub fn start_backend(&self) -> &Path {
        &self.start_backend
    }

    pub fn stop_backend(&self) -> &Path {
        &self.stop_backend
    }

    pub fn print_logs(&self) -> &Path {
        &self.print_logs
    }
}
