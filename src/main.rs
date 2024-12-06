mod websocket;
mod redis;
mod utils;

use websocket::handle_client_conn;
use utils::ServerConfig;

#[tokio::main]
async fn main() {
    println!("teste final");
    let server_config = ServerConfig::init_server().await;
    let _ = tokio::spawn(
        handle_client_conn(
            server_config.server,
            server_config.client
        )
    ).await;
}
