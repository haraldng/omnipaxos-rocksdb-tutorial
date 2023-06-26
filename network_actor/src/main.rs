use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::env;
use tokio;

#[macro_use]
extern crate lazy_static;

mod network;

lazy_static! {
    /// Port to port mapping, for which sockets should be proxied to each other.
    pub static ref PORT_MAPPINGS: HashMap<u64, u64> = if let Ok(var) = env::var("PORT_MAPPINGS") {
        let mut map = HashMap::new();
        let x: Vec<Vec<u64>> = serde_json::from_str(&var).expect("wrong config format");
        for mapping in x {
            if mapping.len() != 2 {
                panic!("wrong config format");
            }
            map.insert(mapping[0], mapping[1]);
            map.insert(mapping[1], mapping[0]);
        }
        map
    } else {
        panic!("missing config")
    };
    /// Ports on which the nodes are supposed to connect with their client API socket.
    pub static ref CLIENT_PORTS: Vec<u64> = if let Ok(var) = env::var("CLIENT_PORTS") {
        serde_json::from_str(&var).expect("wrong config format")
    } else {
        panic!("missing config")
    };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

/// Same as in KV demo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KVCommand {
    Put(KeyValue),
    Delete(String),
    Get(String),
}

/// Same as in KV demo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIResponse {
    Decided(u64),
    Get(String, Option<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Message {
    APIRequest(KVCommand),
    APIResponse(APIResponse),
}

#[tokio::main]
async fn main() {
    // TODO: setup dashboard
    network::run().await;
}
