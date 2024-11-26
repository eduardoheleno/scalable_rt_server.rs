use redis::Connection;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message::Text;
use crate::websocket::WebSocketSend;

pub async fn handle_client_redis(mut sender: WebSocketSend, mut redis_conn: Connection, peer_addr: String) {
    let mut pubsub = redis_conn.as_pubsub();
    pubsub.subscribe(peer_addr).unwrap();

    loop {
        let msg = pubsub.get_message().unwrap();
        let payload: String = msg.get_payload().unwrap();

        let websocket_message = Text(payload);
        sender.send(websocket_message).await.unwrap();
    }
}
