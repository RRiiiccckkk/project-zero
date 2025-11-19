use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub type NodeId = [u8; 32];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZeroPacket {
    pub payload: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecureEnvelope {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
    pub sender_pubkey: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Signal {
    Ping,
    Pong,
    
    // DHT 路由
    FindNode(NodeId), 
    Neighbors(Vec<(NodeId, SocketAddr)>), 

    // DHT 存储
    Store(NodeId, String),
    FindValue(NodeId),
    Value(NodeId, String),

    // Phase 7: NAT 穿透
    // Bootnode 命令接收者(Target)主动向 requester_addr 发送 Ping
    Punch(NodeId, SocketAddr), // (Requester_ID, Requester_IP)

    // 聊天
    Message(String),
}