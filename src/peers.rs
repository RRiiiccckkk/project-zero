use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ed25519_dalek::VerifyingKey;
use std::net::SocketAddr;
use anyhow::{Result, anyhow};

#[derive(Clone)]
pub struct PeerInfo {
    pub node_id: VerifyingKey, // Identity (Ed25519 Public Key)
    pub addr: SocketAddr,      // Physical Address (IP:Port)
}

#[derive(Clone)]
pub struct PeerManager {
    // 这里的 Key 使用 bytes 是因为 VerifyingKey 不直接支持作为 HashMap Key
    peers: Arc<Mutex<HashMap<[u8; 32], PeerInfo>>>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_peer(&self, node_id: VerifyingKey, addr: SocketAddr) {
        let mut peers = self.peers.lock().unwrap();
        peers.insert(node_id.to_bytes(), PeerInfo { node_id, addr });
    }

    /// 通过 Hex 字符串和 IP 字符串手动注册一个 Peer
    /// 用于 CLI 模式下 Client 手动指定 Server
    pub fn add_peer_from_hex(&self, node_id_hex: &str, addr_str: &str) -> Result<()> {
        let bytes = hex::decode(node_id_hex).map_err(|_| anyhow!("Invalid hex ID"))?;
        let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| anyhow!("Invalid ID length (must be 32 bytes)"))?;
        let node_id = VerifyingKey::from_bytes(&bytes_array).map_err(|_| anyhow!("Invalid Key bytes"))?;
        
        let addr: SocketAddr = addr_str.parse().map_err(|_| anyhow!("Invalid IP address"))?;
        
        self.add_peer(node_id, addr);
        // println!("[PeerManager] Manually added peer: {} @ {}", node_id_hex, addr);
        Ok(())
    }

    pub fn get_peer(&self, node_id_bytes: &[u8; 32]) -> Option<PeerInfo> {
        let peers = self.peers.lock().unwrap();
        peers.get(node_id_bytes).cloned()
    }
}