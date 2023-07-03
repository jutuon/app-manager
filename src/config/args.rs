use std::{
    convert::{TryFrom, TryInto},
    path::PathBuf,
};

use clap::{arg, command, value_parser, Command, PossibleValue};
use reqwest::Url;


// Config given as command line arguments
pub struct ArgsConfig {
    pub database_dir: Option<PathBuf>,
}

pub fn get_config() -> ArgsConfig {
    let matches = command!()
        .arg(
            arg!(--todo <DIR> "TODO")
                .required(false)
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    ArgsConfig {
        database_dir: matches
            .get_one::<PathBuf>("todo")
            .map(ToOwned::to_owned),
    }
}
