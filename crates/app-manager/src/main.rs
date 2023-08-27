#![deny(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unused_features)]
#![warn(unused_crate_dependencies)]

use config::args::AppMode;

pub mod api;
pub mod client;
pub mod config;
pub mod server;
pub mod utils;

fn main() {
    let args = crate::config::args::get_config();

    if let Some(AppMode::Api(api_client_mode)) = args.app_mode {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        runtime.block_on(async {
            let result = self::client::handle_api_client_mode(api_client_mode).await;
            match result {
                Ok(_) => std::process::exit(0),
                Err(e) => {
                    eprintln!("{:?}", e);
                    std::process::exit(1)
                }
            }
        })
    } else {
        let config = crate::config::get_config(args).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async { self::server::AppServer::new(config).run().await })
    }
}
