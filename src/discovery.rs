use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashSet;
use std::sync::mpsc::Sender;

const DISCOVERY_PORT: u16 = 44444;
const BEACON_MSG: &[u8] = b"ZERO_BEACON";

#[derive(Clone)]
pub struct Peer {
    pub ip: String,
    pub last_seen: std::time::Instant,
}

pub struct DiscoveryEngine {
    peers: Arc<Mutex<Vec<Peer>>>,
}

impl DiscoveryEngine {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // 启动广播信标 (我是谁)
    pub fn start_beacon(&self) {
        thread::spawn(move || {
            // 绑定到任意端口用于发送
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind beacon socket");
            socket.set_broadcast(true).expect("Failed to set broadcast");

            let target = format!("255.255.255.255:{}", DISCOVERY_PORT);
            
            loop {
                // 广播 "ZERO_BEACON"
                let _ = socket.send_to(BEACON_MSG, &target);
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    // 启动监听器 (谁在周围)
    // tx: 用于通知 UI 线程刷新
    pub fn start_listener(&self, tx: Sender<()>) {
        let peers = self.peers.clone();
        
        thread::spawn(move || {
            let socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT));
            
            match socket {
                Ok(socket) => {
                    let mut buf = [0u8; 32];
                    loop {
                        if let Ok((amt, src)) = socket.recv_from(&mut buf) {
                            let msg = &buf[..amt];
                            if msg == BEACON_MSG {
                                let ip = src.ip().to_string();
                                // 这里简单处理，暂时过滤掉自己需要在上层逻辑做，
                                // 但因为 UDP 广播也会发给自己，我们稍后在 UI 层去重。
                                
                                let mut list = peers.lock().unwrap();
                                let mut found = false;
                                for peer in list.iter_mut() {
                                    if peer.ip == ip {
                                        peer.last_seen = std::time::Instant::now();
                                        found = true;
                                        break;
                                    }
                                }
                                
                                if !found {
                                    list.push(Peer {
                                        ip,
                                        last_seen: std::time::Instant::now(),
                                    });
                                    println!("Found new Zero Node: {}", src);
                                }
                                
                                // 通知 UI 更新
                                let _ = tx.send(());
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Discovery bind failed: {}", e);
                }
            }
        });
    }

    pub fn get_peers(&self) -> Vec<Peer> {
        let list = self.peers.lock().unwrap();
        // 可以在这里过滤掉超时节点 (比如 > 10秒未见的)
        list.clone()
    }
}