use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

// ---------------------------------------------------------
// 节点信息 (PeerInfo)
// ---------------------------------------------------------
#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub id: String,
    pub addr: SocketAddr,
    pub encryption_pk: [u8; 32],
}

// ---------------------------------------------------------
// 通讯录管理器 (PeerManager)
// ---------------------------------------------------------
#[derive(Clone)]
pub struct PeerManager {
    table: Arc<Mutex<HashMap<String, PeerInfo>>>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            table: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_peer(&self, id: String, addr: SocketAddr, enc_pk: [u8; 32]) {
        let mut table = self.table.lock().unwrap();
        let peer = PeerInfo {
            id: id.clone(),
            addr,
            encryption_pk: enc_pk,
        };
        println!(">> [通讯录] 已更新节点: {} -> {}", &id[0..8], addr);
        table.insert(id, peer);
    }

    pub fn get_peer(&self, id: &str) -> Option<PeerInfo> {
        let table = self.table.lock().unwrap();
        table.get(id).cloned()
    }

    pub fn list_peers(&self) {
        let table = self.table.lock().unwrap();
        println!("--- 当前在线节点表 ({}) ---", table.len());
        for (id, info) in table.iter() {
            println!("ID: {}... | IP: {} | CryptoKey: OK", &id[0..8], info.addr);
        }
        println!("---------------------------");
    }
}