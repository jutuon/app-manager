//! HTTP API types and request handlers for all servers.

// Routes
pub mod manager;

pub mod utils;

use utoipa::{Modify, OpenApi};

use crate::{
    config::Config,
    server::{build::BuildManagerHandle, client::ApiManager, update::UpdateManagerHandle},
};

use utils::SecurityApiTokenDefault;

use manager_model as model;

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
        manager::get_system_info,
        manager::get_system_info_all,
    ),
    components(schemas(
        model::DataEncryptionKey,
        model::ServerNameText,
        model::SoftwareOptions,
        model::SoftwareOptionsQueryParam,
        model::DownloadType,
        model::DownloadTypeQueryParam,
        model::RebootQueryParam,
        model::SoftwareInfo,
        model::BuildInfo,
        model::SystemInfoList,
        model::SystemInfo,
        model::CommandOutput,
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
