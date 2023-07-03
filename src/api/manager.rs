pub mod data;

use std::net::SocketAddr;

use axum::{Json, TypedHeader, extract::{Path, ConnectInfo}, Extension};

use futures::FutureExt;
use hyper::StatusCode;

use self::data::{DataEncryptionKey, ServerNameText};

use super::{GetConfig};

use tracing::{error, info};

use super::{utils::ApiKeyHeader};

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
    if let Some(s) = state.config().encryption_keys().iter().find(|s| s.name == server.server) {
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
