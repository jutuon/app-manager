//! Config given as command line arguments

use std::process::exit;

use clap::{arg, command, Args, Parser};
use error_stack::{Result, ResultExt};
use manager_model::SoftwareOptions;
use url::Url;

use super::{file::ConfigFile, GetConfigError};

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
    #[arg(short = 'k', long, default_value = "password", value_name = "KEY")]
    api_key: Option<String>,
    /// API URL for accessing the manager API
    #[arg(
        short = 'u',
        long,
        default_value = "http://localhost:5000",
        value_name = "URL"
    )]
    pub api_url: Url,

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
                super::file::ConfigFile::load(current_dir).change_context(GetConfigError::LoadFileError)?;

            Ok(file_config.api_key)
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
