use redis::Connection;
use uuid::Uuid;

use crate::websocket::ArcConns;

pub fn handle_redis_channel(mut redis_conn: Connection, connections: ArcConns, node_uid: Uuid) {
    let mut pubsub = redis_conn.as_pubsub();
    pubsub.subscribe(node_uid.to_string()).unwrap();

    loop {
        let msg = pubsub.get_message().unwrap();
        let payload: String = msg.get_payload().unwrap();

        println!("channel '{}': {}", msg.get_channel_name(), payload);
    }
}
