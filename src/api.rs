//! HTTP API types and request handlers for all servers.

// Routes
pub mod manager;

pub mod model;
pub mod utils;

use utoipa::{Modify, OpenApi};

use crate::{
    config::Config,
    server::{client::{ApiClient, ApiManager}
    },
};

use utils::SecurityApiTokenDefault;

// Paths

pub const PATH_PREFIX: &str = "/api/v1/";

// API docs

#[derive(OpenApi)]
#[openapi(
    paths(
        manager::get_encryption_key,
    ),
    components(schemas(
        manager::data::DataEncryptionKey,
        manager::data::ServerNameText,
    )),
    modifiers(&SecurityApiTokenDefault),
    info(
        title = "app-manager",
        description = "App manager API",
        version = "0.1.0"
    )
)]
pub struct ApiDoc;

// App state getters


pub trait GetConfig {
    fn config(&self) -> &Config;
}

pub trait GetApiManager {
    fn api_manager(&self) -> ApiManager;
}
