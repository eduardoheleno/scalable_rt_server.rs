use redis::Connection;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use serde_json::Result;
use tokio_tungstenite::tungstenite::Message::{self, Text};
use crate::websocket::WebSocketSend;

#[derive(Serialize, Deserialize)]
pub enum MessageContext {
    Content,
    UserConnected,
    UserDisconnected,
    Close
}

#[derive(Serialize, Deserialize)]
pub struct RedisMessage {
    context: MessageContext,
    content: Option<String>
}

impl RedisMessage {
    pub fn new(context: MessageContext, content: Option<String>) -> Self {
        RedisMessage { context, content }
    }

    pub fn validate_message(raw_message: String) -> Result<RedisMessage> {
        let message: RedisMessage = serde_json::from_str(&raw_message)?;
        Ok(message)
    }
}

pub async fn handle_client_redis(mut sender: WebSocketSend, mut redis_conn: Connection, peer_addr: String) {
    let mut pubsub = redis_conn.as_pubsub();
    pubsub.subscribe(peer_addr.clone()).unwrap();

    loop {
        let msg = pubsub.get_message().unwrap();
        let payload: String = msg.get_payload().unwrap();

        let redis_message = match RedisMessage::validate_message(payload.to_string()) {
            Ok(message) => message,
            Err(e) => {
                eprintln!("Invalid redis message, error: {}", e);
                continue;
            }
        };

        match redis_message.context {
            MessageContext::Content | MessageContext::UserConnected | MessageContext::UserDisconnected => {
                let websocket_message = Text(payload);
                sender.send(websocket_message).await.unwrap();
            }
            MessageContext::Close => {
                let _ = sender.send(Message::Close(None)).await;

                println!("IP {}: redis has disconnected", peer_addr);
                break;
            }
        }
    }
}
