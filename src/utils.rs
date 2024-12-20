use std::{env, panic};
use tokio::net::TcpListener;
use redis::{Client, Commands};
use uuid::Uuid;

pub struct ServerConfig {
    pub node_uid: Uuid,
    pub client: Client,
    pub server: TcpListener
}

impl ServerConfig {
    pub async fn init_server() -> Self {
        let redis_env = env::var("REDIS_URL").unwrap();
        let redis_url = format!("redis://{}", redis_env);
        let client = redis::Client::open(redis_url).unwrap();

        let server_port = env::var("SERVER_PORT").unwrap();
        let server_addr = format!("0.0.0.0:{}", server_port);

        let server = TcpListener::bind(server_addr).await.unwrap();
        let node_uid = Uuid::new_v4();

        let panic_client = client.clone();
        panic::set_hook(Box::new(move |_| {
            let mut panic_connection = match panic_client.get_connection() {
                Ok(connection) => connection,
                Err(e) => {
                    eprintln!("Couldn't get 'panic_connection': {}", e);
                    std::process::exit(1);
                }
            };

            let users: Vec<(String, String)> = panic_connection.hgetall("users").unwrap();
            for (field, value) in users {
                if value == node_uid.to_string() {
                    panic_connection.hdel::<&str, String, ()>("users", field).unwrap();
                }
            }
        }));

        ServerConfig { node_uid, client, server }
    }
}
