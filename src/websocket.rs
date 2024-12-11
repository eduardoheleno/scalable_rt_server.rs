use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
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

#[derive(Serialize, Deserialize)]
struct ConnectedUser {
    ip: String
}

impl ConnectedUser {
    fn from_vec(users: Vec<(String, String)>) -> Vec<ConnectedUser> {
        let mut connected_users_vec: Vec<ConnectedUser> = Vec::new();
        for (field, _) in users {
            connected_users_vec.push(ConnectedUser { ip: field });
        }

        connected_users_vec
    }
}

#[derive(Serialize, Deserialize)]
struct UserMessage {
    ip: String,
    message: String
}

impl UserMessage {
    fn new(ip: String, message: String) -> Self {
        UserMessage { ip, message }
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
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
                eprintln!("error while accepting connection: {}", e);
                continue;
            }
        };
        let (mut ws_send, ws_read) = ws_stream.split();
        send_current_connected_users(&mut ws_send, &mut redis_client).await;

        redis_client.hset::<&str, String, String, ()>("users", peer_addr.to_string(), node_uid.to_string()).unwrap();

        let redis_conn_messages = redis_client.get_connection().unwrap();
        let redis_conn_queue = redis_client.get_connection().unwrap();

        tokio::spawn(handle_client_messages(ws_read, redis_conn_messages, peer_addr.to_string()));
        tokio::spawn(handle_client_redis(ws_send, redis_conn_queue, peer_addr.to_string()));

        notify_user_connection(redis_client.get_connection().unwrap(), peer_addr.to_string());

        println!("connected addr: {}", peer_addr);
    }
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

                    let user_message = UserMessage::new(peer_addr.to_string(), validated_message.content);
                    let json_redis_message = serialize_redis_message(MessageContext::Content, Some(user_message.to_string()));
                    redis_conn.publish::<String, String, ()>(validated_message.target_ip, json_redis_message).unwrap();
                },
                Message::Close(_) => {
                    redis_conn.hdel::<&str, String, ()>("users", peer_addr.to_string()).unwrap();

                    let json_redis_message = serialize_redis_message(MessageContext::Close, None);
                    redis_conn.publish::<String, String, ()>(peer_addr.clone(), json_redis_message).unwrap();

                    notify_user_disconnection(redis_conn, peer_addr.to_string());

                    println!("IP {}: websocket has disconnected", peer_addr);
                    break;
                },
                _ => ()
            }
            Err(e) => eprintln!("{}", e)
        }
    }
}

fn serialize_redis_message(context: MessageContext, content: Option<String>) -> String {
    let redis_message = RedisMessage::new(context, content);
    redis_message.to_string()
}

async fn send_current_connected_users(ws_send: &mut SplitSink<WebSocketStream<TcpStream>, Message>, redis_client: &mut Client) {
    let users: Vec<(String, String)> = redis_client.hgetall("users").unwrap();
    let connected_users = ConnectedUser::from_vec(users);
    let connected_users_json = serde_json::to_string(&connected_users).unwrap();

    let user_list_message = RedisMessage::new(MessageContext::UserList, Some(connected_users_json));
    ws_send.send(Message::Text(user_list_message.to_string())).await.unwrap();
}

fn notify_user_disconnection(mut redis_conn: Connection, disconnected_user_ip: String) {
    let users: Vec<(String, String)> = redis_conn.hgetall("users").unwrap();
    let disconnection_message = RedisMessage::new(MessageContext::UserDisconnected, Some(disconnected_user_ip));

    for (field, _) in users {
        redis_conn.publish::<String, String, ()>(field, disconnection_message.to_string()).unwrap();
    }
}

fn notify_user_connection(mut redis_conn: Connection, connected_user_ip: String) {
    let users: Vec<(String, String)> = redis_conn.hgetall("users").unwrap();
    let connection_message = RedisMessage::new(MessageContext::UserConnected, Some(connected_user_ip));

    for (field, _) in users {
        redis_conn.publish::<String, String, ()>(field, connection_message.to_string()).unwrap();
    }
}
