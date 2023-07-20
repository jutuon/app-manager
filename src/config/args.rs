use std::{
    convert::{TryFrom, TryInto},
    path::PathBuf, process::exit,
};

use clap::{arg, command, value_parser, Command, PossibleValue};
use reqwest::Url;


// Config given as command line arguments
pub struct ArgsConfig;

pub fn get_config() -> ArgsConfig {
    let matches = command!()
        .arg(
            arg!(--"build-info" "Print build info and quit.")
                .required(false)
        )
        .get_matches();

    if matches.is_present("build-info") {
        println!(
            "{}",
            super::info::build_info()
        );
        exit(0)
    }

    ArgsConfig
}
