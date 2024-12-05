use std::env;
use tokio::net::TcpListener;
use redis::Client;
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

        ServerConfig { node_uid, client, server }
    }
}
