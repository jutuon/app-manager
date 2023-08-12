use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query},
    Json,
};

use futures::FutureExt;
use hyper::StatusCode;

use crate::server::{build::BuildDirCreator, info::SystemInfoGetter, update::UpdateDirCreator};

use manager_model::{
    DataEncryptionKey, DownloadType, DownloadTypeQueryParam, RebootQueryParam, ServerNameText,
    SoftwareInfo, SoftwareOptions, SoftwareOptionsQueryParam, SystemInfo, SystemInfoList,
};

use super::{GetApiManager, GetBuildManager, GetConfig, GetUpdateManager};

use tracing::{error, info};

use tokio_stream::StreamExt;

pub const PATH_GET_ENCRYPTION_KEY: &str = "/manager_api/encryption_key/:server";

/// Get encryption key for some server
#[utoipa::path(
    get,
    path = "/manager_api/encryption_key/{server}",
    params(ServerNameText),
    responses(
        (status = 200, description = "Encryption key found.", body = DataEncryptionKey),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn get_encryption_key<S: GetConfig>(
    Path(server): Path<ServerNameText>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    state: S,
) -> Result<Json<DataEncryptionKey>, StatusCode> {
    if let Some(s) = state
        .config()
        .encryption_keys()
        .iter()
        .find(|s| s.name == server.server)
    {
        let key = s.read_encryption_key().await.map_err(|e| {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        info!("Sending encryption key {} to {}", server.server, client);
        Ok(key.into())
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub const PATH_GET_LATEST_SOFTWARE: &str = "/manager_api/latest_software";

/// Download latest software.
///
/// Returns BuildInfo JSON or encrypted binary depending on
/// DownloadTypeQueryParam value.
#[utoipa::path(
    get,
    path = "/manager_api/latest_software",
    params(SoftwareOptionsQueryParam, DownloadTypeQueryParam),
    responses(
        (status = 200, description = "Encrypted binary or UTF-8 JSON", body = Vec<u8>),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn get_latest_software<S: GetConfig + GetApiManager>(
    Query(software): Query<SoftwareOptionsQueryParam>,
    Query(download): Query<DownloadTypeQueryParam>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    state: S,
) -> Result<Vec<u8>, StatusCode> {
    if state.config().software_builder().is_some() {
        info!(
            "Get latest software request received. Sending {:?} {:?} to {}",
            software.software_options, download.download_type, client,
        );
        BuildDirCreator::get_data(
            state.config(),
            software.software_options,
            download.download_type,
        )
        .await
        .map_err(|e| {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
    } else if state.config().software_update_provider().is_some() {
        info!(
            "Get latest software request received. Forwarding the request to the build server. Sending {:?} {:?} to {}",
            software.software_options,
            download.download_type,
            client,
        );
        match download.download_type {
            DownloadType::Info => state
                .api_manager()
                .get_latest_build_info_raw(software.software_options)
                .await
                .map_err(|e| {
                    error!("{e:?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                }),
            DownloadType::EncryptedBinary => state
                .api_manager()
                .get_latest_encrypted_software_binary(software.software_options)
                .await
                .map_err(|e| {
                    error!("{e:?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                }),
        }
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub const PATH_POST_REQUEST_BUILD_SOFTWARE: &str = "/manager_api/request_build_software";

/// Request building the latest software from git.
#[utoipa::path(
    post,
    path = "/manager_api/request_build_software",
    params(SoftwareOptionsQueryParam),
    responses(
        (status = 200, description = "Build server received the build request."),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn post_request_build_software<S: GetConfig + GetBuildManager + GetApiManager>(
    Query(software): Query<SoftwareOptionsQueryParam>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    state: S,
) -> Result<(), StatusCode> {
    if state.config().software_builder().is_some() {
        info!(
            "Building request from {} reveived. Building {:?}",
            client, software.software_options,
        );
        state
            .build_manager()
            .send_build_request(software.software_options)
            .await
            .map_err(|e| {
                error!("{e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })
    } else if state.config().software_update_provider().is_some() {
        info!(
            "Building request from {} reveived. Forwarding the request to the build server.",
            client,
        );
        state
            .api_manager()
            .request_build_software_from_build_server(software.software_options)
            .await
            .map_err(|e| {
                error!("{e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub const PATH_POST_RQUEST_SOFTWARE_UPDATE: &str = "/manager_api/request_software_update";

/// Request software update.
///
/// Manager will update the requested software and reboot the computer as soon
/// as possible if specified.
#[utoipa::path(
    post,
    path = "/manager_api/request_software_update",
    params(SoftwareOptionsQueryParam, RebootQueryParam),
    responses(
        (status = 200, description = "Request received"),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn post_request_software_update<S: GetConfig + GetUpdateManager>(
    Query(software): Query<SoftwareOptionsQueryParam>,
    Query(reboot): Query<RebootQueryParam>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    state: S,
) -> Result<(), StatusCode> {
    info!(
        "Update software request received from {}. Software {:?} and reboot {:?}",
        client, software.software_options, reboot.reboot,
    );

    match software.software_options {
        SoftwareOptions::Manager => {
            state
                .update_manager()
                .send_update_manager_request(reboot.reboot)
                .await
                .map_err(|e| {
                    error!("{e:?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
        SoftwareOptions::Backend => {
            state
                .update_manager()
                .send_update_backend_request(reboot.reboot)
                .await
                .map_err(|e| {
                    error!("{e:?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    Ok(())
}

pub const PATH_GET_SOFTWARE_INFO: &str = "/manager_api/software_info";

/// Get current software info about currently installed backend and manager.
#[utoipa::path(
    get,
    path = "/manager_api/software_info",
    responses(
        (status = 200, description = "Software info", body = SoftwareInfo),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn get_software_info<S: GetConfig>(
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    state: S,
) -> Result<Json<SoftwareInfo>, StatusCode> {
    info!("Get current software info received from {}.", client,);

    let info = UpdateDirCreator::current_software(state.config())
        .await
        .map_err(|e| {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(info.into())
}

pub const PATH_GET_SYSTEM_INFO: &str = "/manager_api/system_info";

/// Get system info about current operating system, hardware and software.
///
/// Returns system info related to current manager instance.
#[utoipa::path(
    get,
    path = "/manager_api/system_info",
    responses(
        (status = 200, description = "System info", body = SystemInfo),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn get_system_info<S: GetConfig>(
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    state: S,
) -> Result<Json<SystemInfo>, StatusCode> {
    info!("Get current system info received from {}.", client,);

    let info = SystemInfoGetter::system_info(state.config())
        .await
        .map_err(|e| {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(info.into())
}

pub const PATH_GET_SYSTEM_INFO_ALL: &str = "/manager_api/system_info_all";

/// Get system info about current operating system, hardware and software.
///
/// Returns system info related to current manager instance and ones
/// defined in config file.
#[utoipa::path(
    get,
    path = "/manager_api/system_info_all",
    responses(
        (status = 200, description = "Get all system infos available", body = SystemInfoList),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn get_system_info_all<S: GetConfig + GetApiManager>(
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    state: S,
) -> Result<Json<SystemInfoList>, StatusCode> {
    info!("Get all system infos received from {}.", client,);

    let info = SystemInfoGetter::system_info_all(state.config(), &state.api_manager())
        .await
        .map_err(|e| {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(info.into())
}
