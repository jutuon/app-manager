use std::{
    net::SocketAddr,
    sync::atomic::{AtomicBool, Ordering},
};

use axum::{extract::ConnectInfo, middleware::Next, response::Response};
use headers::{Header, HeaderValue};
use hyper::{header, Request, StatusCode};
use utoipa::{
    openapi::security::{ApiKeyValue, SecurityScheme},
    Modify,
};

use super::GetConfig;

/// If true then password has been guessed and manager API is now locked.
static API_SECURITY_LOCK: AtomicBool = AtomicBool::new(false);

pub const API_KEY_HEADER_STR: &str = "x-api-key";
pub static API_KEY_HEADER: header::HeaderName = header::HeaderName::from_static(API_KEY_HEADER_STR);

pub async fn authenticate_with_api_key<T, S: GetConfig>(
    state: S,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<T>,
    next: Next<T>,
) -> Result<Response, StatusCode> {
    let header = req
        .headers()
        .get(API_KEY_HEADER_STR)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let key_str = header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;

    if API_SECURITY_LOCK.load(Ordering::Relaxed) {
        Err(StatusCode::LOCKED)
    } else {
        if state.config().api_key() == key_str {
            Ok(next.run(req).await)
        } else {
            API_SECURITY_LOCK.store(true, Ordering::Relaxed);
            tracing::error!(
                "API key has been guessed. API is now locked. Guesser information, addr: {}",
                addr
            );
            Err(StatusCode::LOCKED)
        }
    }
}

pub struct ApiKeyHeader(String);

impl ApiKeyHeader {
    pub fn key(&self) -> &String {
        &self.0
    }
}

impl Header for ApiKeyHeader {
    fn name() -> &'static headers::HeaderName {
        &API_KEY_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i headers::HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;
        let value = value.to_str().map_err(|_| headers::Error::invalid())?;
        Ok(ApiKeyHeader(value.to_string()))
    }

    fn encode<E: Extend<headers::HeaderValue>>(&self, values: &mut E) {
        let header = HeaderValue::from_str(self.0.as_str()).unwrap();
        values.extend(std::iter::once(header))
    }
}

/// Utoipa API doc security config
pub struct SecurityApiTokenDefault;

impl Modify for SecurityApiTokenDefault {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(utoipa::openapi::security::ApiKey::Header(
                    ApiKeyValue::new(API_KEY_HEADER_STR),
                )),
            )
        }
    }
}
