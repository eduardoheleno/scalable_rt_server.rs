use std::time::Duration;
use futures_util::stream::{SplitSink, SplitStream};
use redis::{Client, Commands, Connection};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};
use futures_util::StreamExt;
use crate::redis::handle_client_redis;
use serde_json::Result;
use serde::{Serialize, Deserialize};

pub type WebSocketSend = SplitSink<WebSocketStream<TcpStream>, Message>;
type WebSocketRead = SplitStream<WebSocketStream<TcpStream>>;

#[derive(Serialize, Deserialize)]
struct WebSocketMessage {
    target_ip: String,
    content: String
}

impl WebSocketMessage {
    fn validate_message(raw_message: String) -> Result<WebSocketMessage> {
        let message: WebSocketMessage = serde_json::from_str(&raw_message)?;
        Ok(message)
    }
}

pub async fn handle_client_conn(
    server: TcpListener,
    redis_client: Client
) {
    while let Ok((stream, _)) = server.accept().await {
        let peer_addr = stream.peer_addr().unwrap();
        let ws_stream = match accept_async(stream).await {
            Ok(ws_stream) => ws_stream,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };
        let (ws_send, ws_read) = ws_stream.split();

        let redis_conn_messages = redis_client.get_connection().unwrap();
        let redis_conn_queue = redis_client.get_connection().unwrap();

        tokio::spawn(handle_client_messages(ws_read, redis_conn_messages));
        tokio::spawn(handle_client_redis(ws_send, redis_conn_queue, peer_addr.to_string()));

        println!("connected addr: {}", peer_addr);
    }
}

async fn handle_client_messages(mut ws_read: WebSocketRead, mut redis_conn: Connection) {
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    loop {
        tokio::select! {
            msg = ws_read.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg.unwrap();
                        let message = match WebSocketMessage::validate_message(msg.to_string()) {
                            Ok(message) => message,
                            Err(e) => {
                                eprintln!("Invalid message, error: {}", e);
                                continue;
                            }
                        };

                        redis_conn.publish::<String, String, ()>(message.target_ip, message.content).unwrap();
                    }
                    None => break,
                }
            }
            _ = interval.tick() => {}
        }
    }
}
