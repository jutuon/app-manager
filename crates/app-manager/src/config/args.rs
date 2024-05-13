//! Config given as command line arguments

use std::{path::PathBuf, process::exit};

use clap::{arg, command, Args, Parser};
use error_stack::{Result, ResultExt};
use manager_model::SoftwareOptions;
use reqwest::Certificate;
use url::Url;

use super::{file::ConfigFile, load_root_certificate, GetConfigError};

const DEFAULT_HTTP_LOCALHOST_URL: &str = "http://localhost:5000";
const DEFAULT_HTTPS_LOCALHOST_URL: &str = "https://localhost:5000";

#[derive(Parser)]
#[command(author, version, about)]
pub struct ArgsConfig {
    /// Print build info and quit.
    #[arg(short, long)]
    pub build_info: bool,

    #[command(subcommand)]
    pub app_mode: Option<AppMode>,
}

pub fn get_config() -> ArgsConfig {
    let matches = ArgsConfig::parse();

    if matches.build_info {
        println!("{}", super::info::build_info());
        exit(0)
    }

    matches
}

#[derive(Parser, Debug, Clone)]
pub enum AppMode {
    /// Make API requests using CLI
    Api(ApiClientMode),
}

#[derive(Args, Debug, Clone)]
pub struct ApiClientMode {
    /// API key for accessing the manager API. If not present, config file
    /// api_key is tried to accessed from current directory.
    #[arg(short = 'k', long, value_name = "KEY")]
    api_key: Option<String>,
    /// API URL for accessing the manager API. If not present, config file
    /// TLS config is red from current directory. If it exists, then
    /// "https://localhost:5000" is used as the default value. If not, then
    /// "http://localhost:5000" is used as the default value.
    /// If config file does not exist, then "https://localhost:5000" is the
    /// default value.
    #[arg(
        short = 'u',
        long,
        value_name = "URL"
    )]
    pub api_url: Option<Url>,
    /// Root certificate for HTTP client. If not present, config file
    /// TLS config is red from current directory. If it exists, then
    /// root certificate value from there is used. If not, then HTTP client
    /// uses system root certificates.
    #[arg(
        short = 'c',
        long,
        value_name = "FILE"
    )]
    pub root_certificate: Option<PathBuf>,

    #[command(subcommand)]
    pub api_command: ApiCommand,
}

impl ApiClientMode {
    pub fn api_key(&self) -> Result<String, GetConfigError> {
        if let Some(api_key) = self.api_key.clone() {
            Ok(api_key)
        } else {
            let current_dir = std::env::current_dir().change_context(GetConfigError::GetWorkingDir)?;
            let file_config =
                ConfigFile::load_config(current_dir).change_context(GetConfigError::LoadFileError)?;

            Ok(file_config.api_key)
        }
    }

    pub fn api_url(&self) -> Result<Url, GetConfigError> {
        if let Some(api_url) = self.api_url.clone() {
            return Ok(api_url)
        }

        let current_dir = std::env::current_dir().change_context(GetConfigError::GetWorkingDir)?;

        let url_str = if ConfigFile::exists(&current_dir)
            .change_context(GetConfigError::CheckConfigFileExistanceError)? {

            let file_config =
                super::file::ConfigFile::load_config(current_dir).change_context(GetConfigError::LoadFileError)?;

            if file_config.tls.is_some() {
                DEFAULT_HTTPS_LOCALHOST_URL
            } else {
                DEFAULT_HTTP_LOCALHOST_URL
            }
        } else {
            DEFAULT_HTTPS_LOCALHOST_URL
        };

        Url::parse(url_str)
            .change_context(GetConfigError::InvalidConstant)
    }

    fn root_certificate_file(&self) -> Result<Option<PathBuf>, GetConfigError> {
        if let Some(root_certificate) = self.root_certificate.clone() {
            return Ok(Some(root_certificate));
        }

        let current_dir = std::env::current_dir().change_context(GetConfigError::GetWorkingDir)?;

        if ConfigFile::exists(&current_dir)
            .change_context(GetConfigError::CheckConfigFileExistanceError)? {

            let file_config =
                super::file::ConfigFile::save_default_if_not_exist_and_load(current_dir).change_context(GetConfigError::LoadFileError)?;

            Ok(file_config.tls.map(|v| v.root_certificate))
        } else {
            Ok(None)
        }
    }

    pub fn root_certificate(&self) -> Result<Option<Certificate>, GetConfigError> {
        if let Some(root_certificate_file) = self.root_certificate_file()? {
            let cert = load_root_certificate(&root_certificate_file)
                .change_context(GetConfigError::ReadCertificateError)?;
            Ok(Some(cert))
        } else {
            Ok(None)
        }
    }
}

#[derive(Parser, Debug, Clone)]
pub enum ApiCommand {
    EncryptionKey {
        encryption_key_name: String,
    },
    LatestBuildInfo {
        #[arg(value_enum)]
        software: SoftwareOptions,
    },
    RequestBuildSoftware {
        #[arg(value_enum)]
        software: SoftwareOptions,
    },
    RequestUpdateSoftware {
        #[arg(value_enum)]
        software: SoftwareOptions,
        #[arg(short, long)]
        reboot: bool,
        #[arg(long)]
        reset_data: bool,
    },
    RequestRestartBackend {
        #[arg(long)]
        reset_data: bool,
    },
    SystemInfoAll,
    SystemInfo,
    SoftwareInfo,
}
