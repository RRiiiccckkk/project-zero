use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "android")]
use android_activity::AndroidApp;
#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;

const DISCOVERY_PORT: u16 = 44444;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet {
    Ping { id: String },
    Pong { id: String },
    Chunk { file_hash: String, index: u32, total: u32, data: Vec<u8> },
}

#[derive(Debug)]
pub enum GuiEvent {
    Log(String),
    LocalAddresses(Vec<String>),
    BindSuccess { port: u16 },
    PeerDiscovered { ip: String, id: String },
    ScanComplete,
    AssetReady { data: Vec<u8> },
}

#[derive(Debug)]
pub enum Command {
    SendTo { ip: String, port: String, path: String },
    ScanStandard { prefix: String }, // 扫描 x.y.z.1~254
    ScanSuper { prefix: String },    // 扫描相邻网段 (x.y.z-5 ~ z+5)
}

// --- Logic ---

fn run_background_thread(tx: mpsc::Sender<GuiEvent>, rx: mpsc::Receiver<Command>) {
    let _ = tx.send(GuiEvent::Log("🟢 System Online.".to_string()));

    let mut sockets: Vec<UdpSocket> = Vec::new();
    let mut my_ips: Vec<String> = Vec::new();

    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        for iface in interfaces {
            if !iface.is_loopback() && iface.ip().is_ipv4() {
                let ip = iface.ip().to_string();
                if !ip.starts_with("198.18") { 
                    my_ips.push(ip.clone());
                    let bind_addr = format!("{}:{}", ip, DISCOVERY_PORT);
                    let fallback_addr = format!("{}:0", ip);
                    if let Ok(s) = UdpSocket::bind(&bind_addr).or_else(|_| UdpSocket::bind(&fallback_addr)) {
                        s.set_broadcast(true).ok();
                        s.set_read_timeout(Some(Duration::from_millis(2))).ok(); // 更快的超时
                        sockets.push(s);
                    }
                }
            }
        }
    }
    
    if sockets.is_empty() {
        if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
            s.set_broadcast(true).ok();
            sockets.push(s);
        }
    }

    let _ = tx.send(GuiEvent::LocalAddresses(my_ips.clone()));

    let my_id = format!("Node_{}", (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() % 1000));
    let mut known_peers: HashSet<String> = HashSet::new();
    let mut assembler_buffers: HashMap<String, Vec<Option<Vec<u8>>>> = HashMap::new();
    let mut buf = [0u8; 65535];

    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Command::ScanStandard { prefix } => {
                    let _ = tx.send(GuiEvent::Log(format!("📡 Standard Scan: {}.x", prefix)));
                    let ping = Packet::Ping { id: my_id.clone() };
                    if let Ok(bytes) = bincode::serialize(&ping) {
                        for i in 1..255 {
                            let target = format!("{}.{}:{}", prefix, i, DISCOVERY_PORT);
                            for s in &sockets { let _ = s.send_to(&bytes, &target); }
                            if i % 50 == 0 { thread::sleep(Duration::from_millis(1)); }
                        }
                        // 广播补刀
                        for s in &sockets { let _ = s.send_to(&bytes, format!("255.255.255.255:{}", DISCOVERY_PORT)); }
                    }
                    let _ = tx.send(GuiEvent::ScanComplete);
                },
                Command::ScanSuper { prefix } => {
                    // 解析前缀，例如 "10.7.131"
                    let parts: Vec<&str> = prefix.split('.').collect();
                    if parts.len() == 3 {
                        if let (Ok(p1), Ok(p2), Ok(p3)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>(), parts[2].parse::<u8>()) {
                            let _ = tx.send(GuiEvent::Log(format!("💥 SUPER NOVA: Scanning {}.{}.{} ±5 subnets", p1, p2, p3)));
                            let ping = Packet::Ping { id: my_id.clone() };
                            if let Ok(bytes) = bincode::serialize(&ping) {
                                // 扫描当前网段的前后 50 个网段 (例如 100 ~200)
                                let start = p3.saturating_sub(50);
                                let end = p3.saturating_add(50);
                                
                                for subnet in start..=end {
                                    let current_prefix = format!("{}.{}.{}", p1, p2, subnet);
                                    // 快速扫每个网段
                                    for i in 1..255 {
                                        let target = format!("{}.{}:{}", current_prefix, i, DISCOVERY_PORT);
                                        for s in &sockets { let _ = s.send_to(&bytes, &target); }
                                    }
                                    // 稍微喘口气
                                    thread::sleep(Duration::from_millis(10));
                                }
                            }
                        }
                    }
                    let _ = tx.send(GuiEvent::ScanComplete);
                },
                Command::SendTo { ip, port, path } => {
                    let target = format!("{}:{}", ip, port);
                    if let Ok(data) = std::fs::read(&path) {
                        let _ = tx.send(GuiEvent::Log(format!("🚀 Sending -> {}", target)));
                        let hash = format!("{:x}", md5::compute(&data));
                        let chunks: Vec<_> = data.chunks(1024).map(|c| c.to_vec()).collect();
                        let total = chunks.len() as u32;
                        for (i, chunk) in chunks.into_iter().enumerate() {
                            let p = Packet::Chunk { file_hash: hash.clone(), index: i as u32, total, data: chunk };
                            if let Ok(bytes) = bincode::serialize(&p) {
                                for s in &sockets { let _ = s.send_to(&bytes, &target); }
                                thread::sleep(Duration::from_millis(1));
                            }
                        }
                    } else {
                        let _ = tx.send(GuiEvent::Log(format!("❌ File error: {}", path)));
                    }
                }
            }
        }

        for socket in &sockets {
            if let Ok((amt, src)) = socket.recv_from(&mut buf) {
                if let Ok(pkt) = bincode::deserialize::<Packet>(&buf[..amt]) {
                    match pkt {
                        Packet::Ping { id } => {
                            if id != my_id {
                                let pong = Packet::Pong { id: my_id.clone() };
                                if let Ok(bytes) = bincode::serialize(&pong) {
                                    let _ = socket.send_to(&bytes, src);
                                }
                                let peer_ip = src.ip().to_string();
                                if !known_peers.contains(&peer_ip) {
                                    known_peers.insert(peer_ip.clone());
                                    let _ = tx.send(GuiEvent::PeerDiscovered { ip: peer_ip, id });
                                }
                            }
                        },
                        Packet::Pong { id } => {
                            let peer_ip = src.ip().to_string();
                            if !known_peers.contains(&peer_ip) {
                                known_peers.insert(peer_ip.clone());
                                let _ = tx.send(GuiEvent::Log(format!("👋 FOUND: {}", id)));
                                let _ = tx.send(GuiEvent::PeerDiscovered { ip: peer_ip, id });
                            }
                        },
                        Packet::Chunk { file_hash, index, total, data } => {
                            let buffer = assembler_buffers.entry(file_hash.clone()).or_insert_with(|| vec![None; total as usize]);
                            if (index as usize) < buffer.len() && buffer[index as usize].is_none() {
                                buffer[index as usize] = Some(data);
                                if buffer.iter().all(|c| c.is_some()) {
                                    let full_data: Vec<u8> = buffer.iter().flat_map(|c| c.as_ref().unwrap().clone()).collect();
                                    assembler_buffers.remove(&file_hash);
                                    let _ = tx.send(GuiEvent::Log("📥 Received!".to_string()));
                                    let _ = tx.send(GuiEvent::AssetReady { data: full_data });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// --- GUI ---
struct GuiApp {
    logs: Vec<String>,
    rx: mpsc::Receiver<GuiEvent>,
    tx_cmd: mpsc::Sender<Command>,
    
    my_ips: Vec<String>,
    scan_prefix: String,
    
    discovered_peers: Vec<(String, String)>,
    is_scanning: bool,
    
    target_ip: String,
    target_port: String,
    input_path: String,
    last_image: Option<Vec<u8>>,
}

impl GuiApp {
    fn new(cc: &eframe::CreationContext, rx: mpsc::Receiver<GuiEvent>, tx: mpsc::Sender<Command>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            logs: vec!["Init...".to_string()],
            rx, tx_cmd: tx,
            my_ips: Vec::new(),
            scan_prefix: "10.7.131".to_string(),
            discovered_peers: Vec::new(),
            is_scanning: false,
            target_ip: "".to_string(),
            target_port: DISCOVERY_PORT.to_string(),
            input_path: "test.jpg".to_string(),
            last_image: None,
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                GuiEvent::Log(s) => self.logs.insert(0, s),
                GuiEvent::LocalAddresses(ips) => {
                    self.my_ips = ips.clone();
                    for ip in ips {
                        if !ip.starts_with("127.") && !ip.starts_with("198.18") {
                            let parts: Vec<&str> = ip.split('.').collect();
                            if parts.len() == 4 {
                                self.scan_prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
                                break;
                            }
                        }
                    }
                },
                GuiEvent::BindSuccess { port: _ } => {}, 
                GuiEvent::PeerDiscovered { ip, id } => {
                    if !self.discovered_peers.iter().any(|(x, _)| *x == ip) {
                        self.discovered_peers.push((ip.clone(), id));
                    }
                },
                GuiEvent::ScanComplete => self.is_scanning = false,
                GuiEvent::AssetReady { data } => self.last_image = Some(data),
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Zero v0.9.99: Super-Nova");
            
            ui.horizontal(|ui| {
                ui.label("My IPs:");
                for ip in &self.my_ips {
                    ui.code(ip);
                }
            });
            ui.separator();

            ui.group(|ui| {
                ui.label(egui::RichText::new("🔭 NETWORK SCANNER").strong());
                
                ui.horizontal(|ui| {
                    ui.label("Prefix:");
                    ui.text_edit_singleline(&mut self.scan_prefix);
                    ui.label(".x");
                });
                
                if self.is_scanning {
                    ui.spinner();
                    ui.label("Scanning...");
                } else {
                    // 普通扫描
                    if ui.button("🔍 Scan This Subnet").clicked() {
                        self.is_scanning = true;
                        self.discovered_peers.clear();
                        let _ = self.tx_cmd.send(Command::ScanStandard { prefix: self.scan_prefix.clone() });
                    }
                    // 🔥 超级扫描 🔥
                    if ui.button("💥 SUPER SCAN (±5 Subnets)").clicked() {
                        self.is_scanning = true;
                        self.discovered_peers.clear();
                        let _ = self.tx_cmd.send(Command::ScanSuper { prefix: self.scan_prefix.clone() });
                    }
                }
            });

            if !self.discovered_peers.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new("🎯 TARGETS:").color(egui::Color32::GREEN));
                for (ip, id) in &self.discovered_peers {
                    if ui.button(format!("🔗 CONNECT: {} ({})", id, ip)).clicked() {
                        self.target_ip = ip.clone();
                    }
                }
            }

            if cfg!(not(target_os = "android")) {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("IP:"); ui.text_edit_singleline(&mut self.target_ip);
                });
                ui.text_edit_singleline(&mut self.input_path);
                if ui.button("🚀 SEND").clicked() {
                    let _ = self.tx_cmd.send(Command::SendTo { 
                        ip: self.target_ip.clone(), 
                        port: self.target_port.clone(), 
                        path: self.input_path.clone() 
                    });
                }
            }

            if let Some(data) = &self.last_image { 
                ui.add(egui::Image::from_bytes("bytes://img", data.clone()).fit_to_original_size(0.5)); 
            }
            
            ui.separator();
            egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| { for log in &self.logs { ui.monospace(log); } });
        });
        ctx.request_repaint();
    }
}

#[cfg(not(target_os = "android"))]
pub fn run_desktop() {
    let (tx, rx) = mpsc::channel();
    let (tx_cmd, rx_node) = mpsc::channel();
    thread::spawn(move || run_background_thread(tx, rx_node));
    eframe::run_native("Project Zero", eframe::NativeOptions::default(), Box::new(|cc| Ok(Box::new(GuiApp::new(cc, rx, tx_cmd))))).unwrap();
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) { 
    let (tx, rx) = mpsc::channel();
    let (tx_cmd, rx_node) = mpsc::channel();
    thread::spawn(move || run_background_thread(tx, rx_node));
    let mut options = eframe::NativeOptions::default();
    options.event_loop_builder = Some(Box::new(move |builder| { builder.with_android_app(app); }));
    eframe::run_native("Project Zero", options, Box::new(|cc| Ok(Box::new(GuiApp::new(cc, rx, tx_cmd))))).unwrap();
}