use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use redis::{Client, Commands, Connection};
use tungstenite::{accept, Error, WebSocket};
use uuid::Uuid;

pub type WebSocketConn = Arc<Mutex<WebSocket<TcpStream>>>;
pub type Conns = HashMap<SocketAddr, WebSocketConn>;

pub type ArcConns = Arc<Mutex<Conns>>;

pub fn handle_client_conn(
    server: TcpListener,
    connections: ArcConns,
    node_uid: Uuid,
    mut redis_client: Client
) {
    for stream in server.incoming() {
        match stream {
            Ok(stream) => {
                let peer_addr = stream.peer_addr().unwrap();
                let websocket = Arc::new(Mutex::new(accept(stream).unwrap()));

                connections.lock().unwrap().insert(peer_addr, Arc::clone(&websocket));
                println!("connected addr: {}", peer_addr);

                redis_client.hset::<&str, String, String, ()>("users", peer_addr.to_string(), node_uid.to_string()).unwrap();

                let redis_conn = redis_client.get_connection().unwrap();
                thread::spawn(move || handle_client_messages(Arc::clone(&websocket), redis_conn, node_uid));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn handle_client_messages(websocket: WebSocketConn, mut redis_conn: Connection, node_uid: Uuid) {
    loop {
        let mut locked_websocket = websocket.lock().unwrap();
        match locked_websocket.read() {
            Ok(message) => {
                redis_conn.publish::<String, String, ()>(node_uid.to_string(), message.to_string()).unwrap();
            },
            Err(Error::Io(ref err)) if err.kind() == std::io::ErrorKind::WouldBlock => {
                println!("teste");
            }
            Err(e) => eprintln!("{}", e)
        }
    }
}
