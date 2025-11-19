mod protocol;
mod transport;
mod peers;
mod safedoc;

use clap::Parser;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use x25519_dalek::StaticSecret;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, AeadCore};
use chacha20poly1305::aead::Aead;
use hex;
use sha2::{Sha256, Digest};

use protocol::{Signal, ZeroPacket, SecureEnvelope, NodeId};
use transport::UdpTransport;
use peers::RoutingTable;
use safedoc::SafePage;

#[derive(Parser)]
#[command(name = "project-zero")]
struct Cli {
    #[arg(short, long, default_value = "0.0.0.0:0")]
    bind: String,
}

struct KeyStore {
    sign_key: SigningKey,
    #[allow(dead_code)]
    ecdh_secret: StaticSecret,
}

impl KeyStore {
    fn new() -> Self {
        let mut csprng = OsRng;
        let sign_key = SigningKey::generate(&mut csprng);
        let ecdh_secret = StaticSecret::random_from_rng(&mut csprng);
        Self { sign_key, ecdh_secret }
    }
    
    fn node_id(&self) -> NodeId {
        self.sign_key.verifying_key().to_bytes()
    }
}

type Storage = Arc<Mutex<HashMap<[u8; 32], String>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let socket_addr: SocketAddr = cli.bind.parse()?;
    
    let keys = Arc::new(KeyStore::new());
    let my_id = keys.node_id();
    println!("Node Started. ID: {}", hex::encode(my_id));

    let transport: Arc<UdpTransport> = Arc::new(UdpTransport::new(socket_addr).await?);
    println!("Listening on: {}", transport.local_addr()?);

    let routing_table = Arc::new(Mutex::new(RoutingTable::new(my_id)));
    let storage: Storage = Arc::new(Mutex::new(HashMap::new()));

    let (tx_cmd, mut rx_cmd) = mpsc::channel::<String>(32);

    let t_recv = transport.clone();
    let keys_recv = keys.clone();
    let rt_recv = routing_table.clone();
    let store_recv = storage.clone();

    tokio::spawn(async move {
        loop {
            match t_recv.recv().await {
                Ok((packet, addr)) => {
                    if let Ok(envelope) = serde_json::from_slice::<SecureEnvelope>(&packet.payload) {
                        let sender_id = envelope.sender_pubkey;
                        if let Ok(signal) = decrypt_signal(&keys_recv, &envelope) {
                            handle_signal(signal, sender_id, addr, &rt_recv, &store_recv, &t_recv, &keys_recv).await;
                        }
                    }
                }
                Err(e) => eprintln!("Receive error: {}", e),
            }
        }
    });

    tokio::spawn(async move {
        let stdin = io::stdin();
        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut line = String::new();
            if stdin.read_line(&mut line).is_ok() {
                let _ = tx_cmd.send(line.trim().to_string()).await;
            }
        }
    });

    while let Some(cmd) = rx_cmd.recv().await {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "id" => println!("My ID: {}", hex::encode(my_id)),
            "peers" => {
                let rt = routing_table.lock().unwrap();
                let peers = rt.list_all();
                println!("Known Peers: {}", peers.len());
                for (id, addr) in peers {
                    println!(" - {} @ {}", hex::encode(id), addr);
                }
            },
            "bootstrap" => {
                if parts.len() < 2 { println!("Usage: bootstrap <ip:port>"); continue; }
                if let Ok(addr) = parts[1].parse::<SocketAddr>() {
                    println!("Bootstrapping via {}", addr);
                    send_signal(&transport, &keys, addr, Signal::FindNode(keys.node_id())).await;
                }
            },
            // Phase 8: 发布页面
            "publish" => {
                // 创建一个示例页面并发布
                let page = SafePage::example();
                let json = serde_json::to_string(&page).unwrap();
                
                let key = hash_content(&json);
                println!("Publishing Page...");
                println!("Page Key: \x1b[32m{}\x1b[0m", hex::encode(key)); // Green Key
                
                let closest = { routing_table.lock().unwrap().closest_nodes(&key) };
                if closest.is_empty() {
                    storage.lock().unwrap().insert(key, json);
                    println!("Stored locally.");
                } else {
                    for (_, addr) in closest {
                        send_signal(&transport, &keys, addr, Signal::Store(key, json.clone())).await;
                    }
                    println!("Sent to network.");
                }
            },
            // Phase 8: 浏览 (渲染) 页面
            "browse" => {
                if parts.len() < 2 { println!("Usage: browse <key_hex>"); continue; }
                if let Ok(key_bytes) = hex::decode(parts[1]) {
                    if key_bytes.len() == 32 {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&key_bytes);
                        
                        // 1. 先查本地
                        let local_val = { storage.lock().unwrap().get(&key).cloned() };
                        if let Some(val) = local_val {
                            render_json(&val);
                        } else {
                            // 2. 查网络 (注意：这里简化了逻辑，真实浏览器应该挂起等待结果)
                            // 这里的 browse 只是发送请求，结果会在 handle_signal 里异步打印。
                            // 为了演示，我们通过特殊的 Signal::FindValue 触发，但 handle_signal 需要知道如何处理渲染。
                            // 简化方案：Handle Signal 收到 Value 后，尝试解析为 SafePage，如果成功则渲染。
                            println!("Fetching page from DHT...");
                            let closest = { routing_table.lock().unwrap().closest_nodes(&key) };
                            for (_, addr) in closest {
                                send_signal(&transport, &keys, addr, Signal::FindValue(key)).await;
                            }
                        }
                    }
                }
            },
            "put" => {
                 if parts.len() < 2 { println!("Usage: put <content>"); continue; }
                 let content = parts[1..].join(" ");
                 let key = hash_content(&content);
                 println!("Key: {}", hex::encode(key));
                 let closest = { routing_table.lock().unwrap().closest_nodes(&key) };
                 for (_, addr) in closest {
                     send_signal(&transport, &keys, addr, Signal::Store(key, content.clone())).await;
                 }
            },
            "get" => {
                if parts.len() < 2 { println!("Usage: get <key>"); continue; }
                // ... (保留旧的 get 逻辑用于原始数据)
                if let Ok(bytes) = hex::decode(parts[1]) {
                    let mut key = [0u8;32]; key.copy_from_slice(&bytes);
                    let closest = { routing_table.lock().unwrap().closest_nodes(&key) };
                    for (_, addr) in closest { send_signal(&transport, &keys, addr, Signal::FindValue(key)).await; }
                }
            },
            "quit" | "exit" => break,
            _ => println!("Unknown command."),
        }
    }
    Ok(())
}

fn render_json(json: &str) {
    if let Ok(page) = serde_json::from_str::<SafePage>(json) {
        page.render();
    } else {
        println!("Raw Data: {}", json);
    }
}

fn hash_content(s: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hasher.finalize().into()
}

async fn send_signal(transport: &Arc<UdpTransport>, keys: &Arc<KeyStore>, addr: SocketAddr, signal: Signal) {
    let payload_bytes = serde_json::to_vec(&signal).unwrap();
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let key = [0u8; 32]; 
    let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
    let ciphertext = cipher.encrypt(&nonce, payload_bytes.as_ref()).unwrap();
    let envelope = SecureEnvelope { nonce: nonce.into(), ciphertext, sender_pubkey: keys.node_id() };
    let packet = ZeroPacket { payload: serde_json::to_vec(&envelope).unwrap() };
    transport.send(&packet, addr).await.ok();
}

fn decrypt_signal(_keys: &Arc<KeyStore>, envelope: &SecureEnvelope) -> Result<Signal, String> {
    let key = [0u8; 32]; 
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|_| "Key err")?;
    let plaintext = cipher.decrypt(&envelope.nonce.into(), envelope.ciphertext.as_ref()).map_err(|_| "Decrypt fail")?;
    serde_json::from_slice(&plaintext).map_err(|e| e.to_string())
}

async fn handle_signal(
    signal: Signal,
    sender_id: NodeId,
    addr: SocketAddr, 
    rt: &Arc<Mutex<RoutingTable>>,
    store: &Storage,
    transport: &Arc<UdpTransport>,
    keys: &Arc<KeyStore>
) {
    { let mut table = rt.lock().unwrap(); table.update(sender_id, addr); }

    match signal {
        Signal::Ping => { send_signal(transport, keys, addr, Signal::Pong).await; },
        Signal::Pong => {},
        Signal::FindNode(target) => {
            let (closest, target_addr_opt) = {
                let table = rt.lock().unwrap();
                (table.closest_nodes(&target), table.get_addr(&target))
            };
            send_signal(transport, keys, addr, Signal::Neighbors(closest)).await;
            if let Some(target_addr) = target_addr_opt {
                if target != sender_id {
                    send_signal(transport, keys, target_addr, Signal::Punch(sender_id, addr)).await;
                }
            }
        },
        Signal::Punch(req_id, req_addr) => {
            send_signal(transport, keys, req_addr, Signal::Ping).await;
            { let mut table = rt.lock().unwrap(); table.update(req_id, req_addr); }
        },
        Signal::Neighbors(peers) => {
            let mut table = rt.lock().unwrap();
            for (id, p_addr) in peers { table.update(id, p_addr); }
        },
        Signal::Store(key, val) => {
            // 收到数据时，如果不显示太吵了，可以简化日志
            // println!("DHT Stored Key: {}", hex::encode(key));
            store.lock().unwrap().insert(key, val);
        },
        Signal::FindValue(key) => {
            let val_opt = { store.lock().unwrap().get(&key).cloned() };
            if let Some(val) = val_opt {
                send_signal(transport, keys, addr, Signal::Value(key, val)).await;
            } else {
                let nodes = { rt.lock().unwrap().closest_nodes(&key) };
                send_signal(transport, keys, addr, Signal::Neighbors(nodes)).await;
            }
        },
        // Phase 8 Logic: 收到 Value 后尝试渲染
        Signal::Value(_key, val) => {
            // 如果是 JSON 页面结构，就渲染；否则打印原始内容
            render_json(&val);
        },
        Signal::Message(txt) => { println!("MSG from {}: {}", addr, txt); }
    }
}