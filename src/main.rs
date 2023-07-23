use app_manager::server::AppServer;

fn main() {
    let config = app_manager::config::get_config().unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async { AppServer::new(config).run().await })
}
