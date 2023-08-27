//! Config given as command line arguments

use std::{process::exit, default};

use clap::{arg, command, Command, builder::{Str, PossibleValue}, ArgMatches, Parser, Subcommand, FromArgMatches};
use manager_model::SoftwareOptions;

#[derive(Parser)]
#[command(author, version, about)]
pub struct ArgsConfig {
    /// Print build info and quit.
    #[arg(short, long)]
    pub build_info: bool,
    /// API key for accessing the manager API
    #[arg(short = 'k', long, default_value = "password", value_name = "KEY")]
    pub api_key: String,
    /// API URL for accessing the manager API
    #[arg(short = 'u', long, default_value = "http://localhost:5000", value_name = "URL")]
    pub api_url: String,

    #[command(subcommand)]
    api_command: Option<ApiCommand>,
}

pub fn get_config() -> ArgsConfig {
    let matches = ArgsConfig::parse();

    if matches.build_info {
        println!("{}", super::info::build_info());
        exit(0)
    }

    matches
}

#[derive(Parser, Debug)]
pub enum ApiCommand {
    GetEncryptionKey {
        encryption_key_name: String
    },
    GetLatestBuildInfoJson {
        #[arg(value_parser = SoftwareOptionsParser)]
        software: SoftwareOptions
    },
    RequestBuildSoftware {
        #[arg(value_parser = SoftwareOptionsParser)]
        software: SoftwareOptions
    },
    RequestUpdateSoftware {
        #[arg(value_parser = SoftwareOptionsParser)]
        software: SoftwareOptions,
        reboot: bool,
        reset_data: bool
    },
    SystemInfoAll,
    SystemInfo,
    SoftwareInfo,
}

#[derive(Debug, Clone)]
pub struct SoftwareOptionsParser;

impl clap::builder::TypedValueParser for SoftwareOptionsParser {
    type Value = SoftwareOptions;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        value
            .to_str()
            .ok_or(clap::Error::raw(
                clap::error::ErrorKind::InvalidUtf8,
                "Text was not UTF-8.",
            ))?
            .try_into()
            .map_err(|_| clap::Error::raw(clap::error::ErrorKind::InvalidValue, "Invalid value"))
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        Some(Box::new(
            [
                SoftwareOptions::Backend,
                SoftwareOptions::Manager,
            ]
            .iter()
            .map(|value| PossibleValue::new(value.to_str())),
        ))
    }
}
