mod protocol;
mod transport;
mod peers;
mod zeroui;
mod storage;

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, HashMap};
use std::env;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use eframe::egui;
use log::{info, error, debug};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use uuid::Uuid;

use protocol::{ZeroPacket, Payload, PeerId};
use transport::Transport;
use peers::PeerManager;
use storage::ZeroStore;
use zeroui::{Component, Action, RenderContext, StateMutation};

const CHUNK_SIZE: usize = 4096; 

// --- Assembler State ---
struct Assembler {
    buffers: HashMap<String, (u32, u32, HashMap<u32, Vec<u8>>)>,
}

impl Assembler {
    fn new() -> Self { Self { buffers: HashMap::new() } }
    fn add_chunk(&mut self, group_id: String, index: u32, total: u32, data: Vec<u8>) -> Option<Vec<u8>> {
        let entry = self.buffers.entry(group_id.clone()).or_insert((0, total, HashMap::new()));
        if entry.2.contains_key(&index) { return None; }
        entry.2.insert(index, data);
        entry.0 += 1;
        if entry.0 == total {
            let mut full_data = Vec::new();
            for i in 0..total {
                if let Some(chunk_data) = entry.2.get(&i) { full_data.extend_from_slice(chunk_data); } 
                else { return None; }
            }
            self.buffers.remove(&group_id);
            return Some(full_data);
        }
        None
    }
}

// --- CLI Configuration ---
struct Config {
    port: u16,
    storage_path: String,
    bootstrap_peer: Option<SocketAddr>,
}

impl Config {
    fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let port = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(9000);
        let storage_path = args.get(2).cloned().unwrap_or_else(|| "./storage".to_string());
        let bootstrap_peer = args.get(3).and_then(|s| s.parse().ok());
        Self { port, storage_path, bootstrap_peer }
    }
}

#[derive(Debug)]
pub enum Command {
    Publish { content: Vec<u8> },
    Fetch { cid: String },
}

#[derive(Debug)]
pub enum GuiEvent {
    Log(String),
    Error(String),
    ContentArrived { cid: String, data: Vec<u8> },
    PublishSuccess { cid: String, is_json: bool },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let config = Config::parse();
    info!("Starting Node on Port: {}", config.port);

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let my_id = PeerId::from_public_key(&signing_key.verifying_key());
    
    let store = Arc::new(ZeroStore::new(&config.storage_path).await?);
    let transport = Transport::bind(config.port).await.unwrap_or_else(|e| panic!("{}", e));
    let peers = Arc::new(Mutex::new(PeerManager::new()));
    if let Some(addr) = config.bootstrap_peer { peers.lock().unwrap().add_bootstrap_addr(addr); }

    let (tx_cmd, mut rx_cmd) = mpsc::channel::<Command>(100);
    let (tx_event, rx_event) = mpsc::channel::<GuiEvent>(100);

    // Network Reactor
    let net_key = signing_key.clone();
    let net_store = store.clone();
    let net_peers = peers.clone();
    let tx_event_net = tx_event.clone();

    tokio::spawn(async move {
        let mut seen_packets = HashSet::new();
        let mut assembler = Assembler::new(); 
        if let Some(addr) = config.bootstrap_peer {
             let _ = transport.send_to(ZeroPacket::new(Payload::Heartbeat { timestamp: 0 }, &net_key), addr).await;
        }

        loop {
            tokio::select! {
                Some(cmd) = rx_cmd.recv() => {
                    match cmd {
                        Command::Publish { content } => {
                            let is_json = if let Ok(s) = String::from_utf8(content.clone()) { s.trim().starts_with('{') } else { false };
                            match net_store.store(&content).await {
                                Ok(cid) => {
                                    let _ = tx_event_net.send(GuiEvent::PublishSuccess { cid: cid.clone(), is_json }).await;
                                    let _ = tx_event_net.send(GuiEvent::Log(format!("Saved: {}...", &cid[0..8]))).await;
                                    let payload = Payload::Store { cid, data: content };
                                    smart_broadcast(payload, &net_key, &transport, &net_peers, &mut seen_packets).await;
                                }
                                Err(e) => { error!("Store failed: {}", e); }
                            }
                        }
                        // --- 核心修改区：Command::Fetch ---
                        Command::Fetch { cid } => {
                            // 不再使用 if let Ok... 而是使用 match 捕获具体错误
                            match net_store.fetch(&cid).await {
                                Ok(data) => {
                                    let _ = tx_event_net.send(GuiEvent::ContentArrived { cid, data }).await;
                                }
                                Err(e) => {
                                    // 如果错误信息包含 "corruption"，说明是哈希校验失败
                                    if e.to_string().to_lowercase().contains("corruption") {
                                        let _ = tx_event_net.send(GuiEvent::Error(format!("⚠️ SECURITY ALERT: Data Corruption Detected in {}! File quarantined.", &cid[0..8]))).await;
                                    } else {
                                        // 只是普通的文件未找到
                                        // let _ = tx_event_net.send(GuiEvent::Log(format!("Cache Miss: {}", &cid[0..8]))).await;
                                    }
                                    
                                    // 无论什么错误，都尝试从网络获取 (Self-Healing)
                                    let _ = tx_event_net.send(GuiEvent::Log(format!("Fetching Net: {}...", &cid[0..8]))).await;
                                    let payload = Payload::Fetch { cid };
                                    smart_broadcast(payload, &net_key, &transport, &net_peers, &mut seen_packets).await;
                                }
                            }
                        }
                        // -----------------------------------
                    }
                }
                Ok((packet, addr)) = transport.recv() => {
                    if !packet.verify() { continue; }
                    if seen_packets.contains(&packet.id) { continue; }
                    seen_packets.insert(packet.id.clone());
                    net_peers.lock().unwrap().add_peer(packet.sender.clone(), addr);
                    process_inbound_payload(packet.payload, &net_key, &transport, &net_peers, &net_store, &tx_event_net, &mut assembler, &mut seen_packets, addr).await;
                }
            }
        }
    });

    let title = format!("Project Zero (Port {})", config.port);
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        &title,
        options,
        Box::new(|cc| Ok(Box::new(GuiApp::new(cc, tx_cmd, rx_event, my_id)))),
    ).map_err(|e| anyhow::anyhow!(e.to_string()))
}

async fn smart_broadcast(payload: Payload, key: &SigningKey, transport: &Transport, peers: &Arc<Mutex<PeerManager>>, seen_packets: &mut HashSet<String>) {
    let payload_bytes = bincode::serialize(&payload).unwrap();
    if payload_bytes.len() <= CHUNK_SIZE {
        let packet = ZeroPacket::new(payload, key);
        seen_packets.insert(packet.id.clone());
        broadcast_packet(&packet, transport, peers).await;
    } else {
        let total_chunks = (payload_bytes.len() as f32 / CHUNK_SIZE as f32).ceil() as u32;
        let group_id = Uuid::new_v4().to_string();
        for i in 0..total_chunks {
            let start = (i as usize) * CHUNK_SIZE;
            let end = std::cmp::min(start + CHUNK_SIZE, payload_bytes.len());
            let chunk_data = payload_bytes[start..end].to_vec();
            let chunk_payload = Payload::Chunk { group_id: group_id.clone(), index: i, total: total_chunks, data: chunk_data };
            let packet = ZeroPacket::new(chunk_payload, key);
            seen_packets.insert(packet.id.clone());
            broadcast_packet(&packet, transport, peers).await;
            tokio::task::yield_now().await; 
        }
    }
}

async fn broadcast_packet(packet: &ZeroPacket, transport: &Transport, peers: &Arc<Mutex<PeerManager>>) {
    let all_peers = peers.lock().unwrap().get_all_addrs();
    for addr in all_peers { let _ = transport.send_to(packet.clone(), addr).await; }
}

async fn process_inbound_payload(
    payload: Payload,
    key: &SigningKey,
    transport: &Transport,
    peers: &Arc<Mutex<PeerManager>>,
    store: &Arc<ZeroStore>,
    event_tx: &mpsc::Sender<GuiEvent>,
    assembler: &mut Assembler,
    seen_packets: &mut HashSet<String>,
    sender_addr: SocketAddr,
) {
    match payload {
        Payload::Chunk { group_id, index, total, data } => {
            debug!("Received Chunk {}/{} for {}", index + 1, total, &group_id[0..8]);
            if let Some(full_bytes) = assembler.add_chunk(group_id.clone(), index, total, data.clone()) {
                if let Ok(original_payload) = bincode::deserialize::<Payload>(&full_bytes) {
                    Box::pin(process_inbound_payload(original_payload, key, transport, peers, store, event_tx, assembler, seen_packets, sender_addr)).await;
                }
            }
            let new_payload = Payload::Chunk { group_id, index, total, data };
            let new_packet = ZeroPacket::new(new_payload, key);
            if !seen_packets.contains(&new_packet.id) {
                seen_packets.insert(new_packet.id.clone());
                broadcast_packet(&new_packet, transport, peers).await;
            }
        }
        Payload::Store { cid, data } => {
            let _ = store.store(&data).await;
            smart_broadcast(Payload::Store { cid, data }, key, transport, peers, seen_packets).await;
        }
        Payload::Fetch { cid } => {
            if let Ok(data) = store.fetch(&cid).await {
                let reply = Payload::Data { cid: cid.clone(), data };
                smart_broadcast(reply, key, transport, peers, seen_packets).await;
            } else {
                smart_broadcast(Payload::Fetch { cid }, key, transport, peers, seen_packets).await;
            }
        }
        Payload::Data { cid, data } => {
            if let Ok(_) = store.store(&data).await {
                let _ = event_tx.send(GuiEvent::ContentArrived { cid, data }).await;
            }
        }
        _ => { smart_broadcast(payload, key, transport, peers, seen_packets).await; }
    }
}

struct GuiApp {
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<GuiEvent>,
    my_id: PeerId,
    logs: Vec<String>,
    editor_text: String,
    search_cid: String,
    
    last_asset_cid: Option<String>,
    last_page_cid: Option<String>,
    status_message: Option<String>,
    
    current_cid: Option<String>,
    view_root: Option<Component>,
    view_raw_error: Option<String>,
    texture_cache: HashMap<String, egui::TextureHandle>,
    requested_resources: HashSet<String>,

    app_state: HashMap<String, i32>,
}

impl GuiApp {
    fn new(_cc: &eframe::CreationContext<'_>, tx: mpsc::Sender<Command>, rx: mpsc::Receiver<GuiEvent>, my_id: PeerId) -> Self {
        Self {
            tx, rx, my_id,
            logs: vec![],
            editor_text: r#"{
  "type": "VStack",
  "spacing": 20.0,
  "children": [
    { "type": "Text", "value": "Interactive ZeroApp", "size": 30.0 },
    { "type": "HStack", "spacing": 20.0, "children": [
        { "type": "Text", "value": "Counter:", "size": 20.0 },
        { "type": "Text", "value": "$counter", "size": 20.0 }
    ]},
    { "type": "Button", "label": "Increment (+1)", "on_click": { "type": "Increment", "key": "counter" } },
    
    { "type": "HStack", "spacing": 20.0, "children": [
         { "type": "Text", "value": "Light Switch:", "size": 20.0 },
         { "type": "Text", "value": "$light", "size": 20.0 }
    ]},
    { "type": "Button", "label": "Toggle Switch", "on_click": { "type": "Toggle", "key": "light" } }
  ]
}"#.to_string(),
            search_cid: String::new(),
            last_asset_cid: None,
            last_page_cid: None,
            status_message: None,
            current_cid: None,
            view_root: None,
            view_raw_error: None,
            texture_cache: HashMap::new(),
            requested_resources: HashSet::new(),
            app_state: HashMap::new(),
        }
    }

    fn load_texture(&self, ctx: &egui::Context, name: &str, data: &[u8]) -> Option<egui::TextureHandle> {
        if let Ok(image) = image::load_from_memory(data) {
            let size = [image.width() as _, image.height() as _];
            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            return Some(ctx.load_texture(name, color_image, Default::default()));
        }
        None
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                GuiEvent::Log(s) => self.logs.push(s),
                GuiEvent::Error(s) => self.logs.push(format!("❌ {}", s)),
                GuiEvent::PublishSuccess { cid, is_json } => {
                    if is_json {
                        self.last_page_cid = Some(cid.clone());
                        self.status_message = Some(format!("Page Published! {}", &cid[0..6]));
                    } else {
                        self.last_asset_cid = Some(cid.clone());
                        self.status_message = Some(format!("Asset Uploaded! {}", &cid[0..6]));
                    }
                },
                GuiEvent::ContentArrived { cid, data } => {
                    if let Ok(json_str) = String::from_utf8(data.clone()) {
                         if json_str.trim().starts_with('{') {
                             if let Some(root) = zeroui::parse_blueprint(&json_str) {
                                 self.view_root = Some(root);
                                 self.current_cid = Some(cid.clone());
                                 self.search_cid = cid.clone();
                                 self.view_raw_error = None;
                                 self.logs.push(format!("App Loaded: {}", &cid[0..8]));
                                 self.app_state.clear();
                                 continue; 
                             }
                         }
                    }
                    if let Some(texture) = self.load_texture(ctx, &cid, &data) {
                        self.texture_cache.insert(cid.clone(), texture);
                        self.logs.push(format!("Img Loaded: {}", &cid[0..8]));
                    } else {
                        if self.current_cid.as_ref() == Some(&cid) {
                             self.view_raw_error = Some(format!("Unknown Data ({}b)", data.len()));
                        }
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| { ui.heading(format!("Node: {}", self.my_id.short())); });
            if let Some(msg) = &self.status_message { ui.colored_label(egui::Color32::GREEN, msg); }

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.heading("Creator Studio");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("📂 Upload Image").clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg"]).pick_file() {
                                if let Ok(bytes) = std::fs::read(&path) {
                                    self.logs.push(format!("Uploading {}kb...", bytes.len() / 1024));
                                    let _ = self.tx.try_send(Command::Publish { content: bytes });
                                }
                            }
                        }
                        if let Some(cid) = &self.last_asset_cid {
                            if ui.button("📋 Asset CID").clicked() { ui.output_mut(|o| o.copied_text = cid.clone()); }
                        }
                    });

                    ui.separator();
                    ui.add(egui::TextEdit::multiline(&mut self.editor_text).code_editor().desired_rows(15));
                    
                    ui.horizontal(|ui| {
                        if ui.button("🚀 Publish App Blueprint").clicked() {
                            let bytes = self.editor_text.as_bytes().to_vec();
                            let _ = self.tx.try_send(Command::Publish { content: bytes });
                        }
                        if let Some(cid) = &self.last_page_cid {
                            if ui.button("📋 Page CID").clicked() { ui.output_mut(|o| o.copied_text = cid.clone()); }
                        }
                    });
                });

                cols[1].group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("CID:");
                        ui.text_edit_singleline(&mut self.search_cid);
                        if ui.button("Fetch").clicked() {
                            let _ = self.tx.try_send(Command::Fetch { cid: self.search_cid.clone() });
                        }
                    });
                    ui.separator();
                    
                    egui::ScrollArea::vertical().id_salt("projector").show(ui, |ui| {
                        if let Some(root) = &self.view_root {
                            let render_ctx = RenderContext { textures: &self.texture_cache, state: &self.app_state };
                            
                            if let Some(action) = zeroui::render(ui, &render_ctx, root) {
                                match action {
                                    Action::Navigate(target) => { let _ = self.tx.try_send(Command::Fetch { cid: target }); }
                                    Action::LoadResource(res_cid) => {
                                        if !self.requested_resources.contains(&res_cid) {
                                            self.requested_resources.insert(res_cid.clone());
                                            let _ = self.tx.try_send(Command::Fetch { cid: res_cid });
                                        }
                                    }
                                    Action::MutateState(mutation) => {
                                        match mutation {
                                            StateMutation::Increment { key } => {
                                                let val = self.app_state.entry(key).or_insert(0);
                                                *val += 1;
                                            }
                                            StateMutation::Toggle { key } => {
                                                let val = self.app_state.entry(key).or_insert(0);
                                                *val = if *val == 0 { 1 } else { 0 };
                                            }
                                        }
                                    }
                                }
                            }
                        } else if let Some(err) = &self.view_raw_error {
                            ui.colored_label(egui::Color32::RED, err);
                        } else {
                            ui.label("Idle.");
                        }
                    });
                });
            });
            
            ui.separator();
            egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                for log in self.logs.iter().rev() { 
                    if log.starts_with("❌") { ui.colored_label(egui::Color32::RED, log); } else { ui.label(log); }
                }
            });
        });
        ctx.request_repaint();
    }
}