use crate::kv::KVCommand;
use crate::server::Server;
use omnipaxos::*;
use omnipaxos_storage::memory_storage::MemoryStorage;
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

type OmniPaxosKV = OmniPaxos<KVCommand, MemoryStorage<KVCommand>>;

#[tokio::main]
async fn main() {
    let server_config = ServerConfig {
        pid: *PID,
        election_tick_timeout: 5,
        ..Default::default()
    };
    let cluster_config = ClusterConfig {
        configuration_id: 1,
        nodes: (*NODES).clone(),
        ..Default::default()
    };
    let op_config = OmniPaxosConfig {
        server_config,
        cluster_config,
    };
    let omni_paxos = op_config
        .build(MemoryStorage::default())
        .expect("failed to build OmniPaxos");
    let mut server = Server {
        omni_paxos,
        network: network::Network::new().await,
        database: database::Database::new(format!("db_{}", *PID).as_str()),
        last_decided_idx: 0,
    };
    server.run().await;
}
