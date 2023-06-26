use std::time::Duration;
use crate::database::Database;
use crate::kv::KVCommand;
use crate::{
    network::{Message, Network},
};
use serde::{Deserialize, Serialize};
use tokio::time;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIResponse {
    Get(String, Option<String>),
}

pub struct Server {
    pub network: Network,
    pub database: Database,
}

impl Server {
    async fn process_incoming_msgs(&mut self) {
        let messages = self.network.get_received().await;
        for msg in messages {
            match msg {
                Message::APIRequest(kv_cmd) => {
                    match kv_cmd {
                        KVCommand::Get(key) => {
                            let value = self.database.handle_command(KVCommand::Get(key.clone()));
                            let msg = Message::APIResponse(APIResponse::Get(key, value));
                            self.network.send(0, msg).await;
                        },
                        cmd => {
                            self.database.handle_command(cmd);
                        },
                    }
                }
                _ => unimplemented!(),
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        let mut msg_interval = time::interval(Duration::from_millis(1));
        loop {
            tokio::select! {
                biased;
                _ = msg_interval.tick() => {
                    self.process_incoming_msgs().await;
                },
                else => (),
            }
        }
    }
}
