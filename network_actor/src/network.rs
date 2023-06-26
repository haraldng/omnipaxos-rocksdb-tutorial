use serde_json;
use std::collections::HashMap;
use std::{
    fmt,
    io::{stdout, Write},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, mpsc, Mutex},
    time::sleep,
};

use crate::{KVCommand, KeyValue, Message, CLIENT_PORTS, PORT_MAPPINGS};

pub async fn run() {
    // setup client sockets to talk to nodes
    let api_sockets = Arc::new(Mutex::new(HashMap::new()));
    for port in CLIENT_PORTS.iter() {
        let api_sockets = api_sockets.clone();
        tokio::spawn(async move {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
                .await
                .unwrap();
            let (socket, _addr) = listener.accept().await.unwrap();
            let (reader, writer) = socket.into_split();
            api_sockets.lock().await.insert(port, writer);
            // receiver actor
            tokio::spawn(async move {
                let mut reader = BufReader::new(reader);
                loop {
                    let mut data = vec![];
                    let bytes_read = reader.read_until(b'\n', &mut data).await.unwrap();
                    if bytes_read == 0 {
                        // dropped socket EOF
                        println!("{port} disconnected");
                        api_sockets.lock().await.remove(port);
                        break;
                    }
                    if let Ok(msg) = serde_json::from_slice::<Message>(&data) {
                        println!("From {}: {:?}", port, msg); // TODO: handle APIResponse
                    }
                }
            });
        });
    }

    // Handle user input to propose values
    let api = api_sockets.clone();
    tokio::spawn(async move {
        loop {
            // Get input
            let mut input = String::new();
            print!("Type a command here <put/delete/get> <args>: ");
            let _ = stdout().flush();
            let mut reader = BufReader::new(tokio::io::stdin());
            reader
                .read_line(&mut input)
                .await
                .expect("Did not enter a string");

            // Parse and send command
            match parse_command(input) {
                Ok((command, None)) => {
                    let mut sent_command = false;
                    for port in CLIENT_PORTS.iter() {
                        if let Some(writer) = api.lock().await.get_mut(port) {
                            let cmd = Message::APIRequest(command.clone());
                            let mut data =
                                serde_json::to_vec(&cmd).expect("could not serialize cmd");
                            data.push(b'\n');
                            writer.write_all(&data).await.unwrap();
                            sent_command = true;
                            break;
                        }
                    }
                    if !sent_command {
                        println!("Couldn't send command, no node is reachable");
                    }
                }
                Ok((command, Some(port))) => {
                    if let Some(writer) = api.lock().await.get_mut(&port) {
                        let cmd = Message::APIRequest(command.clone());
                        let mut data = serde_json::to_vec(&cmd).expect("could not serialize cmd");
                        data.push(b'\n');
                        writer.write_all(&data).await.unwrap();
                    }
                }
                Err(err) => println!("{err}"),
            }
            // Wait some amount of time for cluster response
            sleep(Duration::from_millis(50)).await;
        }
    });

    // setup intra-cluster communication
    let partitions: Arc<Mutex<Vec<(u64, u64, f32)>>> = Arc::new(Mutex::new(vec![]));
    let mut out_channels = HashMap::new();
    for port in PORT_MAPPINGS.keys() {
        let (sender, _rec) = broadcast::channel::<Vec<u8>>(10000);
        let sender = Arc::new(sender);
        out_channels.insert(*port, sender.clone());
    }
    let out_channels = Arc::new(out_channels);

    let (central_sender, mut central_receiver) = mpsc::channel(10000);
    let central_sender = Arc::new(central_sender);

    for port in PORT_MAPPINGS.keys() {
        let out_chans = out_channels.clone();
        let central_sender = central_sender.clone();
        tokio::spawn(async move {
            let central_sender = central_sender.clone();
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
                .await
                .unwrap();
            let (socket, _addr) = listener.accept().await.unwrap();
            let (reader, mut writer) = socket.into_split();
            // sender actor
            let out_channels = out_chans.clone();
            tokio::spawn(async move {
                let mut receiver = out_channels.get(port).unwrap().clone().subscribe();
                while let Ok(data) = receiver.recv().await {
                    let _ = writer.write_all(&data).await;
                }
            });
            // receiver actor
            let central_sender = central_sender.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(reader);
                loop {
                    let mut data = vec![];
                    reader.read_until(b'\n', &mut data).await.unwrap();
                    if let Err(_e) = central_sender
                        .send((port, PORT_MAPPINGS.get(port).unwrap(), data))
                        .await
                    {
                        break;
                    };
                }
            });
        });
    }

    // the one central actor that sees all messages
    while let Some((from_port, to_port, msg)) = central_receiver.recv().await {
        // drop message if network is partitioned between sender and receiver
        for (from, to, _probability) in partitions.lock().await.iter() {
            if from == from_port && to == to_port {
                continue;
            }
        }
        let sender = out_channels.get(to_port).unwrap().clone();
        let _ = sender.send(msg);
    }
}

struct ParseCommandError(String);
impl fmt::Display for ParseCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
fn parse_command(line: String) -> Result<(KVCommand, Option<u64>), ParseCommandError> {
    let mut words = line.trim().split(" ");
    let command_type = words
        .next()
        .ok_or(ParseCommandError("Not enough arguments".to_string()))?;

    let command = match command_type {
        "delete" => {
            let value = words
                .next()
                .ok_or(ParseCommandError("Not enough arguments".to_string()))?;
            let port = words.next().map(|x| x.parse::<u64>().unwrap());
            (KVCommand::Delete(value.to_string()), port.into())
        }
        "get" => {
            let value = words
                .next()
                .ok_or(ParseCommandError("Not enough arguments".to_string()))?;
            let port = words.next().map(|x| x.parse::<u64>().unwrap());
            (KVCommand::Get(value.to_string()), port.into())
        }
        "put" => {
            let key = words
                .next()
                .ok_or(ParseCommandError("Not enough arguments".to_string()))?
                .to_string();
            let value = words
                .next()
                .ok_or(ParseCommandError("Not enough arguments".to_string()))?
                .to_string();
            let port = words.next().map(|x| x.parse::<u64>().unwrap());
            (KVCommand::Put(KeyValue { key, value }), port.into())
        }
        "help" => {
            return Err(ParseCommandError(
                "Commands: put <key> <value>, get <key>, delete <key> (optional <port>)".into(),
            ));
        }
        _ => Err(ParseCommandError("Invalid command type".to_string()))?,
    };
    Ok(command)
}
