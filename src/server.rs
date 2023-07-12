pub mod app;
pub mod client;
pub mod mount;
pub mod build;
pub mod reboot;
pub mod update;

use std::{net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use axum::{Router};
use futures::future::poll_fn;
use hyper::server::{
    accept::Accept,
    conn::{AddrIncoming, Http},
};

use tokio::{
    net::TcpListener,
    signal::{self, unix::{SignalKind, Signal}},
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use tower::{MakeService};
use tower_http::trace::{TraceLayer};
use tracing::{error, info, log::warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api::{ApiDoc, GetBuildManager},
    config::Config,
    server::{
        app::{App}, client::ApiClient, mount::MountManager, build::BuildManager,
    },
};


/// Drop this when quit starts
pub type ServerQuitHandle = broadcast::Sender<()>;

/// Use resubscribe() for cloning.
pub type ServerQuitWatcher = broadcast::Receiver<()>;

pub struct AppServer {
    config: Arc<Config>,
}

impl AppServer {
    pub fn new(config: Config) -> Self {
        Self {
            config: config.into(),
        }
    }

    pub async fn run(self) {
        tracing_subscriber::fmt::init();
        if self.config.debug_mode() {
            warn!("Debug mode is enabled");
        }

        let (server_quit_handle, server_quit_watcher) = broadcast::channel(1);
        let mut terminate_signal = signal::unix::signal(SignalKind::terminate()).unwrap();

        // Start build manager

        let (build_manager_quit_handle, build_manager_handle) =
            BuildManager::new(
                self.config.clone(),
                server_quit_watcher.resubscribe()
            );

        // Start reboot manager

        let (reboot_manager_quit_handle, reboot_manager_handle) =
            reboot::RebootManager::new(
                self.config.clone(),
                server_quit_watcher.resubscribe(),
            );

        // Create app

        let api_client = ApiClient::new(&self.config).unwrap().into();
        let mut app = App::new(self.config.clone(), api_client, build_manager_handle.into()).await;

        // Start API server

        let server_task = self
            .create_public_api_server_task(&mut app, server_quit_watcher.resubscribe())
            .await;

        // Mount encrypted storage if needed

        let mount_manager =
            MountManager::new(self.config.clone(), app.state());

        if let Some(encryption_key_provider) = self.config.encryption_key_provider() {
            loop {
                match mount_manager.mount_if_needed(encryption_key_provider).await {
                    Ok(()) => {
                        break;
                    }
                    Err(e) => {
                        warn!("Failed to mount encrypted storage. Error: {:?}", e);
                    }
                }

                info!("Retrying after one hour");

                tokio::select! {
                    _ = Self::wait_quit_signal(&mut terminate_signal) => {
                        return;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(60*60)) => {} // check again in an hour
                }
            }
        }

        // Build new version if needed

        if self.config.software_builder().is_some() {
            match app.state().build_manager().send_build_new_backend_version().await {
                Ok(()) => {
                    info!("Build finished");
                }
                Err(e) => {
                    warn!("Build failed. Error: {:?}", e);
                }
            }
        }

        // Wait until quit signal
        Self::wait_quit_signal(&mut terminate_signal).await;

        // Quit started

        info!("Manager quit started");

        drop(server_quit_handle);

        // Wait until all tasks quit
        server_task
            .await
            .expect("Manager API server task panic detected");

        build_manager_quit_handle.wait_quit().await;
        reboot_manager_quit_handle.wait_quit().await;

        drop(app);

        if self.config.encryption_key_provider().is_some() {
            match mount_manager.unmount_if_needed().await {
                Ok(()) => {
                    info!("Encrypted storage is now unmounted");
                }
                Err(e) => {
                    warn!("Failed to unmount encrypted storage. Error: {:?}", e);
                }
            }
        }

        info!("Manager quit done");
    }

    pub async fn wait_quit_signal(terminate_signal: &mut Signal) {
        tokio::select! {
            _ = terminate_signal.recv() => {}
            result = signal::ctrl_c() => {
                match result {
                    Ok(()) => (),
                    Err(e) => error!("Failed to listen CTRL+C. Error: {}", e),
                }
            }
        }
    }

    /// Public API. This can have WAN access.
    pub async fn create_public_api_server_task(
        &self,
        app: &mut App,
        quit_notification: ServerQuitWatcher,
    ) -> JoinHandle<()> {
        let router = {
            let router = self.create_public_router(app);
            let router = if self.config.debug_mode() {
                router
                    .merge(Self::create_swagger_ui())
            } else {
                router
            };
            let router = if self.config.debug_mode() {
                router.route_layer(TraceLayer::new_for_http())
            } else {
                router
            };
            router
        };

        let addr = self.config.socket().public_api;
        info!("Public API is available on {}", addr);

        if let Some(tls_config) = self.config.public_api_tls_config() {
            self.create_server_task_with_tls(addr, router, tls_config.clone(), quit_notification)
                .await
        } else {
            self.create_server_task_no_tls(router, addr, "Public API", quit_notification)
        }
    }

    pub async fn create_server_task_with_tls(
        &self,
        addr: SocketAddr,
        router: Router,
        tls_config: Arc<ServerConfig>,
        mut quit_notification: ServerQuitWatcher,
    ) -> JoinHandle<()> {
        let listener = TcpListener::bind(addr)
            .await
            .expect("Address not available");
        let mut listener =
            AddrIncoming::from_listener(listener).expect("AddrIncoming creation failed");
        listener.set_sleep_on_errors(true);

        let protocol = Arc::new(Http::new());
        let acceptor = TlsAcceptor::from(tls_config);

        let mut app_service = router.into_make_service_with_connect_info::<SocketAddr>();

        tokio::spawn(async move {
            let (drop_after_connection, mut wait_all_connections) = mpsc::channel::<()>(1);

            loop {
                let next_addr_stream = poll_fn(|cx| Pin::new(&mut listener).poll_accept(cx));

                let stream = tokio::select! {
                    _ = quit_notification.recv() => {
                        break;
                    }
                    addr = next_addr_stream => {
                        match addr {
                            None => {
                                error!("Socket closed");
                                break;
                            }
                            Some(Err(e)) => {
                                error!("Address stream error {e}");
                                continue;
                            }
                            Some(Ok(stream)) => {
                                stream
                            }
                        }
                    }
                };

                let acceptor = acceptor.clone();
                let protocol = protocol.clone();
                let service = app_service.make_service(&stream);

                let mut quit_notification = quit_notification.resubscribe();
                let drop_on_quit = drop_after_connection.clone();
                tokio::spawn(async move {
                    tokio::select! {
                        _ = quit_notification.recv() => {} // Graceful shutdown for connections?
                        connection = acceptor.accept(stream) => {
                            match connection {
                                Ok(connection) => {
                                    if let Ok(service) = service.await {
                                        let _ = protocol.serve_connection(connection, service).with_upgrades().await;
                                    }
                                }
                                Err(_) => {},
                            }
                        }
                    }

                    drop(drop_on_quit);
                });
            }
            drop(drop_after_connection);
            drop(quit_notification);

            loop {
                match wait_all_connections.recv().await {
                    Some(()) => (),
                    None => break,
                }
            }
        })
    }

    pub fn create_server_task_no_tls(
        &self,
        router: Router,
        addr: SocketAddr,
        name_for_log_message: &'static str,
        mut quit_notification: ServerQuitWatcher,
    ) -> JoinHandle<()> {
        let normal_api_server = {
            axum::Server::bind(&addr)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        };

        tokio::spawn(async move {
            let shutdown_handle = normal_api_server.with_graceful_shutdown(async {
                let _ =  quit_notification.recv().await;
            });

            match shutdown_handle.await {
                Ok(()) => {
                    info!("{name_for_log_message} server future returned Ok()");
                }
                Err(e) => {
                    error!("{name_for_log_message} server future returned error: {}", e);
                }
            }
        })
    }

    pub fn create_public_router(&self, app: &mut App) -> Router {
        let router = app.create_manager_server_router();
        router
    }

    pub fn create_swagger_ui() -> SwaggerUi {
        SwaggerUi::new("/swagger-ui").url("/api-doc/app_api.json", ApiDoc::openapi())
    }
}
