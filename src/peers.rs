use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use crate::protocol::PeerId;

/// PeerManager: 简单的内存路由表
/// 记录 PeerId -> SocketAddr 的映射
#[derive(Debug)]
pub struct PeerManager {
    // 记录 ID 到 地址的映射
    peers: HashMap<PeerId, SocketAddr>,
    // 记录我们尝试连接过的地址 (去重用)
    known_addrs: HashSet<SocketAddr>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            known_addrs: HashSet::new(),
        }
    }

    /// 添加或更新节点信息
    pub fn add_peer(&mut self, id: PeerId, addr: SocketAddr) {
        // 如果是新节点或地址变更，记录日志
        if let Some(old_addr) = self.peers.insert(id.clone(), addr) {
            if old_addr != addr {
                log::info!("Peer {} moved: {} -> {}", id.short(), old_addr, addr);
            }
        } else {
            log::info!("New Peer Discovered: {} at {}", id.short(), addr);
        }
        self.known_addrs.insert(addr);
    }

    /// 添加一个引导节点地址 (尚未知晓 ID)
    pub fn add_bootstrap_addr(&mut self, addr: SocketAddr) {
        self.known_addrs.insert(addr);
    }

    /// 获取所有已知节点的地址 (用于广播)
    pub fn get_all_addrs(&self) -> Vec<SocketAddr> {
        self.known_addrs.iter().cloned().collect()
    }
}