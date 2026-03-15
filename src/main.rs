use std::sync::{Arc, RwLock};

use language_server::server::LSPServerManager;

use crate::server::{NixLSPServer, flake_cache::FlakeCache};

pub mod server;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let server = LSPServerManager {
        server: NixLSPServer {
            flake_cache: Some(Arc::new(RwLock::new(FlakeCache::try_new()?))),
            //connection: LSPConnection::new_network(4874),
            ..Default::default()
        },
    };
    log::info!("Starting language_server");
    server.run().unwrap();

    Ok(())
}
