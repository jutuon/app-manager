//! Config given as command line arguments

use std::process::exit;

use clap::{arg, command, Args, Parser};
use manager_model::SoftwareOptions;
use url::Url;

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
    /// API key for accessing the manager API
    #[arg(short = 'k', long, default_value = "password", value_name = "KEY")]
    pub api_key: String,
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
    SystemInfoAll,
    SystemInfo,
    SoftwareInfo,
}
