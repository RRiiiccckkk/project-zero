mod identity;
mod crypto;
mod protocol;
mod transport;
mod peers;
mod safedoc;

use clap::{Parser, Subcommand};
use anyhow::Result; // 移除了未使用的 anyhow 宏引用
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use std::io::{self, Write};
use tokio::sync::mpsc;
use std::collections::HashMap;

use crate::identity::NodeIdentity;
use crate::transport::TransportLayer;
use crate::peers::PeerManager;
use crate::protocol::{SecureEnvelope, ZeroPacket, MessageType, Signal};
use crate::safedoc::{SafePage, Element};

const DEFAULT_PORT: u16 = 9000;
const BOOTNODE_ADDR: &str = "127.0.0.1:9999"; // 硬编码的种子节点地址

#[derive(Parser)]
#[command(name = "Project Zero")]
#[command(version = "0.5.1")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    GenId,
    /// 启动种子节点 (Directory Server)
    Bootnode,
    /// 启动普通节点 (P2P Client/Server)
    Start {
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::GenId => {
            println!("{}", hex::encode(NodeIdentity::new().get_public_key().to_bytes()));
        }
        Commands::Bootnode => {
            run_bootnode().await?;
        }
        Commands::Start { port } => {
            run_node(port).await?;
        }
    }
    Ok(())
}

// ==========================================
// 1. Bootnode 逻辑 (The Phonebook)
// ==========================================
async fn run_bootnode() -> Result<()> {
    let identity = Arc::new(NodeIdentity::new());
    let socket = UdpSocket::bind(BOOTNODE_ADDR).await?;
    let transport = Arc::new(TransportLayer::new(socket));
    
    // 内存数据库：ID (Hex) -> IP:Port (String)
    let mut registry: HashMap<String, String> = HashMap::new();

    println!("=== Zero Bootnode Running ===");
    println!("ID      : {}", hex::encode(identity.get_public_key().to_bytes()));
    println!("Address : {}", BOOTNODE_ADDR);
    println!("Waiting for nodes...");

    let mut buf = [0u8; 65535];

    loop {
        if let Ok((size, src_addr)) = transport.socket.recv_from(&mut buf).await {
            let data = &buf[..size];
            if data.len() <= 32 { continue; }

            let sender_id_bytes = &data[0..32];
            let envelope_bytes = &data[32..];

            if let Ok(sender_id) = ed25519_dalek::VerifyingKey::from_bytes(sender_id_bytes.try_into().unwrap()) {
                let sender_hex = hex::encode(sender_id.to_bytes());

                if let Ok(env) = serde_json::from_slice::<SecureEnvelope>(envelope_bytes) {
                    if let Ok(decrypted) = env.open(&identity, &sender_id) {
                        if let Ok(packet) = serde_json::from_slice::<ZeroPacket>(&decrypted) {
                            if packet.verify(&sender_id) {
                                match packet.msg_type {
                                    MessageType::Signal => {
                                        if let Ok(signal) = serde_json::from_slice::<Signal>(&packet.payload) {
                                            match signal {
                                                Signal::Hello => {
                                                    // 注册节点
                                                    registry.insert(sender_hex.clone(), src_addr.to_string());
                                                    println!("[Bootnode] Registered: {} -> {}", &sender_hex[0..8], src_addr);
                                                },
                                                Signal::Query(target_id_hex) => {
                                                    // 查询节点
                                                    println!("[Bootnode] Lookup query for: {}", &target_id_hex[0..8]);
                                                    if let Some(addr) = registry.get(&target_id_hex) {
                                                        // 找到目标，发送 Found
                                                        let response = Signal::Found {
                                                            id: target_id_hex.clone(),
                                                            addr: addr.clone()
                                                        };
                                                        let payload = serde_json::to_vec(&response)?;
                                                        // 封包发回
                                                        let pkt = ZeroPacket::new(&identity, payload, MessageType::Signal)?;
                                                        let env = SecureEnvelope::seal(&identity, &sender_id, &serde_json::to_vec(&pkt)?)?;
                                                        
                                                        let mut final_data = Vec::new();
                                                        final_data.extend_from_slice(identity.get_public_key().to_bytes().as_slice());
                                                        final_data.extend_from_slice(&serde_json::to_vec(&env)?);
                                                        transport.socket.send_to(&final_data, src_addr).await?;
                                                    }
                                                },
                                                _ => {}
                                            }
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ==========================================
// 2. Regular Node 逻辑 (Auto Discovery)
// ==========================================
async fn run_node(port: u16) -> Result<()> {
    let identity = Arc::new(NodeIdentity::new());
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
    let transport = Arc::new(TransportLayer::new(socket));
    let peer_manager = Arc::new(PeerManager::new());

    println!("=== Zero P2P Node Started ===");
    println!("My ID   : {}", hex::encode(identity.get_public_key().to_bytes()));
    println!("Address : 127.0.0.1:{}", port);

    // === Auto Register: Tell Bootnode we are here ===
    let bootnode_addr = BOOTNODE_ADDR.to_string();
    // 先尝试把 Bootnode 加到本地（不一定要成功，如果地址不对会报错，但不影响逻辑）
    let _ = peer_manager.add_peer_from_hex(&hex::encode(identity.get_public_key().to_bytes()), &bootnode_addr);

    {
        let signal = Signal::Hello;
        let payload = serde_json::to_vec(&signal)?;
        let pkt = ZeroPacket::new(&identity, payload, MessageType::Signal)?;
        
        // === 修复点：直接使用 &VerifyingKey ===
        let my_pk = identity.get_public_key();
        let env = SecureEnvelope::seal(&identity, &my_pk, &serde_json::to_vec(&pkt)?)?; 
        
        let mut final_data = Vec::new();
        final_data.extend_from_slice(identity.get_public_key().to_bytes().as_slice());
        final_data.extend_from_slice(&serde_json::to_vec(&env)?);
        
        // 即使 Bootnode 没启动，这里 UDP 发送也不会报错，只是丢包
        let _ = transport.socket.send_to(&final_data, BOOTNODE_ADDR).await;
        println!("[System] Sent Hello to Bootnode.");
    }

    let (tx_response, mut rx_response) = mpsc::channel::<SafePage>(10);
    let net_transport = transport.clone();
    let net_identity = identity.clone();
    let net_peers = peer_manager.clone();
    
    tokio::spawn(async move {
        network_listener(net_transport, net_identity, net_peers, tx_response).await;
    });

    loop {
        print!("zero> ");
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "exit" => break,
            "get" => {
                if parts.len() == 3 {
                    let target_id_hex = parts[1];
                    let path = parts[2];

                    // 1. 检查本地 PeerManager
                    let target_peer_opt = if let Ok(bytes) = hex::decode(target_id_hex) {
                        if let Ok(arr) = bytes.try_into() {
                            peer_manager.get_peer(&arr)
                        } else { None }
                    } else { None };

                    if let Some(peer) = target_peer_opt {
                        // 2a. 认识这个节点 -> 直接发送请求
                        println!("[System] Fetching {} from {}...", path, &target_id_hex[0..8]);
                        send_request(&transport, &identity, &peer, path).await?;

                        match timeout(Duration::from_secs(2), rx_response.recv()).await {
                            Ok(Some(page)) => page.render(),
                            Ok(None) => println!("[Error] Channel closed."),
                            Err(_) => println!("[Error] Request timed out."),
                        }
                    } else {
                        // 2b. 不认识这个节点 -> 向 Bootnode 查询
                        println!("[Discovery] Peer unknown. Querying Bootnode...");
                        
                        let signal = Signal::Query(target_id_hex.to_string());
                        let payload = serde_json::to_vec(&signal)?;
                        let pkt = ZeroPacket::new(&identity, payload, MessageType::Signal)?;
                        
                        // === 修复点：直接使用 &VerifyingKey ===
                        let my_pk = identity.get_public_key();
                        let env = SecureEnvelope::seal(&identity, &my_pk, &serde_json::to_vec(&pkt)?)?;
                        
                        let mut final_data = Vec::new();
                        final_data.extend_from_slice(identity.get_public_key().to_bytes().as_slice());
                        final_data.extend_from_slice(&serde_json::to_vec(&env)?);
                        transport.socket.send_to(&final_data, BOOTNODE_ADDR).await?;
                        
                        println!("[Discovery] Query sent. If found, try 'get' again in a moment.");
                    }
                } else {
                    println!("Usage: get <NodeID> <Path>");
                }
            },
            _ => println!("Unknown command."),
        }
    }
    Ok(())
}

async fn send_request(transport: &Arc<TransportLayer>, identity: &Arc<NodeIdentity>, peer: &crate::peers::PeerInfo, path: &str) -> Result<()> {
    let req = format!("GET {}", path);
    let pkt = ZeroPacket::new(identity, req.into_bytes(), MessageType::Request)?;
    let env = SecureEnvelope::seal(identity, &peer.node_id, &serde_json::to_vec(&pkt)?)?;
    let mut final_data = Vec::new();
    final_data.extend_from_slice(identity.get_public_key().to_bytes().as_slice());
    final_data.extend_from_slice(&serde_json::to_vec(&env)?);
    transport.socket.send_to(&final_data, peer.addr).await?;
    Ok(())
}

async fn network_listener(
    transport: Arc<TransportLayer>,
    identity: Arc<NodeIdentity>,
    peer_manager: Arc<PeerManager>,
    tx_response: mpsc::Sender<SafePage>
) {
    let mut buf = [0u8; 65535];
    loop {
        if let Ok((size, src_addr)) = transport.socket.recv_from(&mut buf).await {
            let data = &buf[..size];
            if data.len() <= 32 { continue; }

            let sender_id_bytes = &data[0..32];
            let envelope_bytes = &data[32..];
            
            if let Ok(sender_id) = ed25519_dalek::VerifyingKey::from_bytes(sender_id_bytes.try_into().unwrap()) {
                // 自动学习 sender IP
                peer_manager.add_peer(sender_id, src_addr);

                if let Ok(env) = serde_json::from_slice::<SecureEnvelope>(envelope_bytes) {
                    if let Ok(decrypted) = env.open(&identity, &sender_id) {
                        if let Ok(packet) = serde_json::from_slice::<ZeroPacket>(&decrypted) {
                            if packet.verify(&sender_id) {
                                match packet.msg_type {
                                    MessageType::Signal => {
                                        // 处理信令 (Found)
                                        if let Ok(signal) = serde_json::from_slice::<Signal>(&packet.payload) {
                                            if let Signal::Found { id, addr } = signal {
                                                println!("\n[Discovery] SUCCESS! Found {} at {}", &id[0..8], addr);
                                                println!("zero> "); // 恢复提示符
                                                io::stdout().flush().unwrap();
                                                
                                                // 注册发现的节点到 PeerManager
                                                let _ = peer_manager.add_peer_from_hex(&id, &addr);
                                            }
                                        }
                                    },
                                    MessageType::Request => {
                                        let req_str = String::from_utf8_lossy(&packet.payload);
                                        let page = generate_page(&req_str, &identity);
                                        if let Ok(resp_payload) = serde_json::to_vec(&page) {
                                            if let Ok(resp_pkt) = ZeroPacket::new(&identity, resp_payload, MessageType::Response) {
                                                if let Ok(resp_env) = SecureEnvelope::seal(&identity, &sender_id, &serde_json::to_vec(&resp_pkt).unwrap()) {
                                                    let mut final_data = Vec::new();
                                                    final_data.extend_from_slice(identity.get_public_key().to_bytes().as_slice());
                                                    final_data.extend_from_slice(&serde_json::to_vec(&resp_env).unwrap());
                                                    let _ = transport.socket.send_to(&final_data, src_addr).await;
                                                }
                                            }
                                        }
                                    },
                                    MessageType::Response => {
                                        if let Ok(page) = serde_json::from_slice::<SafePage>(&packet.payload) {
                                            let _ = tx_response.send_timeout(page, Duration::from_millis(100)).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn generate_page(req: &str, identity: &NodeIdentity) -> SafePage {
    let req = req.trim();
    match req {
        "GET /index" | "GET /" => SafePage {
            title: "Zero Node".to_string(),
            elements: vec![
                Element::Header("Welcome to Web 3.0".to_string()),
                Element::Text(format!("Node: ...{}", &hex::encode(identity.get_public_key().to_bytes())[56..])),
                Element::Text("Discovery: Bootnode Enabled".to_string()),
            ],
        },
        _ => SafePage {
            title: "404".to_string(),
            elements: vec![Element::Text("Not Found".to_string())],
        }
    }
}