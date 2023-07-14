//! HTTP API types and request handlers for all servers.

// Routes
pub mod manager;

pub mod model;
pub mod utils;

use utoipa::{Modify, OpenApi};

use crate::{
    config::Config,
    server::{client::{ApiClient, ApiManager}, build::{BuildManager, BuildManagerHandle}, update::UpdateManagerHandle
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
        manager::get_software_info,
        manager::get_latest_software,
        manager::post_request_build_software,
        manager::post_request_software_update,
    ),
    components(schemas(
        manager::data::DataEncryptionKey,
        manager::data::ServerNameText,
        manager::data::SoftwareOptions,
        manager::data::SoftwareOptionsQueryParam,
        manager::data::DownloadType,
        manager::data::DownloadTypeQueryParam,
        manager::data::RebootQueryParam,
        manager::data::SoftwareInfo,
        manager::data::BuildInfo,
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

pub trait GetBuildManager {
    fn build_manager(&self) -> &BuildManagerHandle;
}

pub trait GetUpdateManager {
    fn update_manager(&self) -> &UpdateManagerHandle;
}
