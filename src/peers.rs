use std::net::SocketAddr;
use crate::protocol::NodeId;

pub const K_BUCKET_SIZE: usize = 20;
const BUCKET_COUNT: usize = 256; 

#[derive(Debug, Clone, Copy)]
pub struct PeerInfo {
    pub id: NodeId,
    pub addr: SocketAddr,
}

pub struct RoutingTable {
    local_id: NodeId,
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
        // FIXED: 使用 _i 忽略未使用变量
        for (_i, (a, b)) in self.local_id.iter().zip(other.iter()).enumerate() {
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
            peer.addr = addr;
            bucket.push(peer);
        } else {
            if bucket.len() < K_BUCKET_SIZE {
                bucket.push(PeerInfo { id, addr });
            }
        }
    }

    pub fn closest_nodes(&self, target: &NodeId) -> Vec<(NodeId, SocketAddr)> {
        let mut all_peers = Vec::new();
        for bucket in &self.buckets {
            for peer in bucket {
                all_peers.push(*peer);
            }
        }

        all_peers.sort_by(|a, b| {
            let dist_a = xor_distance(&a.id, target);
            let dist_b = xor_distance(&b.id, target);
            dist_a.cmp(&dist_b)
        });

        all_peers.into_iter()
            .take(K_BUCKET_SIZE)
            .map(|p| (p.id, p.addr))
            .collect()
    }

    pub fn get_addr(&self, id: &NodeId) -> Option<SocketAddr> {
        let idx = self.distance_bucket_index(id);
        self.buckets[idx].iter().find(|p| p.id == *id).map(|p| p.addr)
    }

    pub fn list_all(&self) -> Vec<(NodeId, SocketAddr)> {
        self.buckets.iter().flat_map(|b| b.clone()).map(|p| (p.id, p.addr)).collect()
    }
}

fn xor_distance(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut res = [0u8; 32];
    for i in 0..32 {
        res[i] = a[i] ^ b[i];
    }
    res
}