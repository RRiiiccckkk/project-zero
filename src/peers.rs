// src/peers.rs
use std::net::SocketAddr;
use crate::protocol::{NodeId, PeerInfo};

pub const K_BUCKET_SIZE: usize = 20;
const BUCKET_COUNT: usize = 256; 

pub struct RoutingTable {
    pub local_id: NodeId,
    buckets: Vec<Vec<PeerInfo>>,
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        let mut buckets = Vec::with_capacity(BUCKET_COUNT);
        for _ in 0..BUCKET_COUNT {
            buckets.push(Vec::new());
        }
        Self {
            local_id,
            buckets,
        }
    }

    fn distance_bucket_index(&self, other: &NodeId) -> usize {
        let mut distinct_bits = 0;
        for (a, b) in self.local_id.iter().zip(other.iter()) {
            let xor = a ^ b;
            if xor == 0 {
                distinct_bits += 8;
            } else {
                distinct_bits += xor.leading_zeros() as usize;
                break;
            }
        }
        if distinct_bits >= 255 { 255 } else { distinct_bits }
    }

    pub fn update(&mut self, id: NodeId, addr: SocketAddr) {
        if id == self.local_id { return; } 

        let idx = self.distance_bucket_index(&id);
        let bucket = &mut self.buckets[idx];

        if let Some(pos) = bucket.iter().position(|p| p.id == id) {
            let mut peer = bucket.remove(pos);
            peer.addr = addr; // 更新地址
            bucket.push(peer);
        } else {
            if bucket.len() < K_BUCKET_SIZE {
                bucket.push(PeerInfo { id, addr });
            }
        }
    }

    /// 获取最近的节点（简化版：返回所有已知节点用于广播）
    pub fn known_peers(&self) -> Vec<PeerInfo> {
        self.buckets.iter().flat_map(|b| b.clone()).collect()
    }
    
    pub fn count(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }
}