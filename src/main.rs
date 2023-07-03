pub mod api;
pub mod config;
pub mod server;
pub mod utils;

use server::AppServer;

fn main() {
    // TODO: print commit ID to logs if build directory was clean
    let config = config::get_config().unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async { AppServer::new(config).run().await })
}
