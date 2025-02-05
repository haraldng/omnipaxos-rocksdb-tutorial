use crate::database::Database;
use crate::kv::KVCommand;
use crate::{
    network::{Message, Network},
    OmniPaxosKV,
};
use omnipaxos::util::LogEntry;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIResponse {
    Decided(u64),
    Get(String, Option<String>),
}

pub struct Server {
    pub omni_paxos: OmniPaxosKV,
    pub network: Network,
    pub database: Database,
    pub last_decided_idx: u64,
}

impl Server {
    async fn process_incoming_msgs(&mut self) {
        let messages = self.network.get_received().await;
        for msg in messages {
            match msg {
                Message::APIRequest(kv_cmd) => match kv_cmd {
                    KVCommand::Get(key) => {
                        let value = self.database.handle_command(KVCommand::Get(key.clone()));
                        let msg = Message::APIResponse(APIResponse::Get(key, value));
                        self.network.send(0, msg).await;
                    }
                    cmd => {
                        self.omni_paxos.append(cmd).unwrap();
                    }
                },
                Message::OmniPaxosMsg(msg) => {
                    self.omni_paxos.handle_incoming(msg);
                }
                _ => unimplemented!(),
            }
        }
    }

    async fn send_outgoing_msgs(&mut self) {
        let messages = self.omni_paxos.outgoing_messages();
        for msg in messages {
            let receiver = msg.get_receiver();
            self.network
                .send(receiver, Message::OmniPaxosMsg(msg))
                .await;
        }
    }

    async fn handle_decided_entries(&mut self) {
        let new_decided_idx = self.omni_paxos.get_decided_idx();
        if self.last_decided_idx < new_decided_idx as u64 {
            let decided_entries = self
                .omni_paxos
                .read_decided_suffix(self.last_decided_idx as usize)
                .unwrap();
            self.update_database(decided_entries);
            self.last_decided_idx = new_decided_idx as u64;
            /*** reply client ***/
            let msg = Message::APIResponse(APIResponse::Decided(new_decided_idx as u64));
            self.network.send(0, msg).await;
            // snapshotting
            if new_decided_idx % 5 == 0 {
                println!(
                    "Log before: {:?}",
                    self.omni_paxos.read_decided_suffix(0).unwrap()
                );
                self.omni_paxos
                    .snapshot(Some(new_decided_idx), true)
                    .expect("Failed to snapshot");
                println!(
                    "Log after: {:?}\n",
                    self.omni_paxos.read_decided_suffix(0).unwrap()
                );
            }
        }
    }

    fn update_database(&self, decided_entries: Vec<LogEntry<KVCommand>>) {
        for entry in decided_entries {
            match entry {
                LogEntry::Decided(cmd) => {
                    self.database.handle_command(cmd);
                }
                _ => {}
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        let mut msg_interval = time::interval(Duration::from_millis(1));
        let mut tick_interval = time::interval(Duration::from_millis(10));
        loop {
            tokio::select! {
                biased;
                _ = msg_interval.tick() => {
                    self.process_incoming_msgs().await;
                    self.send_outgoing_msgs().await;
                    self.handle_decided_entries().await;
                },
                _ = tick_interval.tick() => {
                    self.omni_paxos.tick();
                },
                else => (),
            }
        }
    }
}
