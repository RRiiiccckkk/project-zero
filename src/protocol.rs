// src/protocol.rs
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use crate::zeroui::Blueprint;

pub type NodeId = [u8; 32];
// FIX: 使用 Vec<u8> 替代 [u8; 64] 以避免 serde 序列化错误
pub type SignatureBytes = Vec<u8>; 

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct PeerInfo {
    pub id: NodeId,
    pub addr: SocketAddr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZeroPacket {
    pub sender_id: NodeId,
    pub nonce: u64,
    pub signature: SignatureBytes, 
    pub payload: Payload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Payload {
    Ping,
    Pong,
    FindNode(NodeId), 
    Neighbors(Vec<PeerInfo>),
    UiBlueprint(Blueprint),
    UiAction {
        action_type: String,
        payload: String,
    }
}

impl ZeroPacket {
    pub fn get_signable_bytes(nonce: u64, payload: &Payload) -> Vec<u8> {
        let mut bytes = nonce.to_le_bytes().to_vec();
        let payload_bytes = serde_json::to_vec(payload).unwrap_or_default();
        bytes.extend(payload_bytes);
        bytes
    }
}