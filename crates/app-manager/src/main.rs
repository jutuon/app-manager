#![deny(unsafe_code)]
#![warn(unused_crate_dependencies)]


pub mod api;
pub mod config;
pub mod server;
pub mod utils;


fn main() {
    let config = self::config::get_config().unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async { self::server::AppServer::new(config).run().await })
}
