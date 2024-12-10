use futures_util::{StreamExt, stream::{SplitSink, SplitStream}};
use redis::{Client, Commands, Connection};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};
use uuid::Uuid;
use crate::redis::{handle_client_redis, RedisMessage, MessageContext};
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
    mut redis_client: Client,
    node_uid: Uuid
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

        redis_client.hset::<&str, String, String, ()>("users", peer_addr.to_string(), node_uid.to_string()).unwrap();

        let redis_conn_messages = redis_client.get_connection().unwrap();
        let redis_conn_queue = redis_client.get_connection().unwrap();

        tokio::spawn(handle_client_messages(ws_read, redis_conn_messages, peer_addr.to_string()));
        tokio::spawn(handle_client_redis(ws_send, redis_conn_queue, peer_addr.to_string()));

        println!("connected addr: {}", peer_addr);
    }
}

fn serialize_redis_message(context: MessageContext, content: Option<String>) -> String {
    let redis_message = RedisMessage::new(context, content);
    serde_json::to_string(&redis_message).unwrap()
}

async fn handle_client_messages(mut ws_read: WebSocketRead, mut redis_conn: Connection, peer_addr: String) {
    while let Some(message) = ws_read.next().await {
        match message {
            Ok(message) => match message {
                Message::Text(data) => {
                    let validated_message = match WebSocketMessage::validate_message(data) {
                        Ok(message) => message,
                        Err(e) => {
                            eprintln!("Invalid websocket message, error: {}", e);
                            continue;
                        }
                    };

                    let json_redis_message = serialize_redis_message(MessageContext::Content, Some(validated_message.content));
                    redis_conn.publish::<String, String, ()>(validated_message.target_ip, json_redis_message).unwrap();
                },
                Message::Close(_) => {
                    redis_conn.hdel::<&str, String, ()>("users", peer_addr.to_string()).unwrap();

                    let json_redis_message = serialize_redis_message(MessageContext::Close, None);
                    redis_conn.publish::<String, String, ()>(peer_addr.clone(), json_redis_message).unwrap();

                    println!("IP {}: websocket has disconnected", peer_addr);
                    break;
                },
                _ => ()
            }
            Err(e) => eprintln!("{}", e)
        }
    }
}
