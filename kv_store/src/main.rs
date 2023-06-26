use crate::server::Server;
use std::env;
use tokio;

#[macro_use]
extern crate lazy_static;

mod database;
mod kv;
mod network;
mod server;

lazy_static! {
    pub static ref NODES: Vec<u64> = if let Ok(var) = env::var("NODES") {
        serde_json::from_str::<Vec<u64>>(&var).expect("wrong config format")
    } else {
        vec![]
    };
    pub static ref PID: u64 = if let Ok(var) = env::var("PID") {
        let x = var.parse().expect("PIDs must be u64");
        if x == 0 {
            panic!("PIDs cannot be 0")
        } else {
            x
        }
    } else {
        panic!("missing PID")
    };
}

#[tokio::main]
async fn main() {
    let mut server = Server {
        network: network::Network::new().await,
        database: database::Database::new(format!("db_{}", *PID).as_str()),
    };
    server.run().await;
}
