pub mod private_routers;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Json, Router,
};


use utoipa::OpenApi;

use crate::{
    api::{
        self, ApiDoc, GetConfig, GetApiManager, GetBuildManager,
    },
    config::Config,
};

use self::private_routers::PrivateRoutes;

use super::{client::{ApiManager, ApiClient}, build::BuildManagerHandle};

#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    api: Arc<ApiClient>,
    build_manager: Arc<BuildManagerHandle>,
}

impl GetConfig for AppState {
    fn config(&self) -> &Config {
        &self.config
    }
}

impl GetBuildManager for AppState {
    fn build_manager(&self) -> &BuildManagerHandle {
        &self.build_manager
    }
}

impl GetApiManager for AppState {
    fn api_manager(&self) -> ApiManager<'_> {
        ApiManager::new(
            &self.config,
            &self.api,
        )
    }
}

pub struct App {
    pub state: AppState,
}

impl App {
    pub async fn new(
        config: Arc<Config>,
        api_client: Arc<ApiClient>,
        build_manager: Arc<BuildManagerHandle>,
    ) -> Self {
        let state = AppState {
            config: config.clone(),
            api: api_client.clone(),
            build_manager,
        };

        Self {
            state,
        }
    }

    pub fn state(&self) -> AppState {
        self.state.clone()
    }

    pub fn create_manager_server_router(&self) -> Router {
        let public = Router::new();
        public.merge(PrivateRoutes::new(self.state.clone()).private_manager_server_router())
    }
}
