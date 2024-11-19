mod websocket;
mod utils;
mod redis;

use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::HashMap;

use redis::handle_redis_channel;
use websocket::{
    handle_client_conn,
    Conns
};
use utils::ServerConfig;

fn main() {
    let server_config = ServerConfig::init_server();
    let connections: Conns = HashMap::new();
    let sp_connections = Arc::new(Mutex::new(connections));

    let redis_conn_handle_redis_channel = server_config.client.get_connection().unwrap();
    let redis_connections = Arc::clone(&sp_connections);
    thread::spawn(
        move || handle_redis_channel(
            redis_conn_handle_redis_channel,
            redis_connections,
            server_config.node_uid
        )
    );

    let client_connections = Arc::clone(&sp_connections);
    let handle_client_thread = thread::spawn(
        move || handle_client_conn(
            server_config.server,
            client_connections,
            server_config.node_uid,
            server_config.client
        )
    );
    handle_client_thread.join().unwrap();
}
