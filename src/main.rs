mod protocol;
mod transport;
mod peers;
mod zeroui;
mod storage;
mod discovery; 

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, HashMap};
use std::net::{SocketAddr, UdpSocket};
use tokio::sync::mpsc;
use eframe::egui;
use log::{info, error};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;

// 强制链接
#[cfg(target_os = "android")]
use winit as _;
#[cfg(target_os = "android")]
use android_activity as _;

use protocol::{ZeroPacket, Payload, PeerId};
use transport::Transport;
use peers::PeerManager;
use storage::ZeroStore;
use discovery::DiscoveryEngine;

// --- Enum Definitions ---
#[derive(Debug)]
pub enum Command { Publish { content: Vec<u8> }, Fetch { cid: String }, ConnectPeer { ip: String } }
#[derive(Debug)]
pub enum GuiEvent { Log(String), Error(String), ContentArrived { cid: String, data: Vec<u8> }, PublishSuccess { cid: String, is_json: bool }, PeerFound(String), SelfIpFound(String) }

// ==========================================
// ENTRY POINTS
// ==========================================

// 1. PC Entry
#[cfg(not(target_os = "android"))]
fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    let (tx, rx) = start_background_node();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Project Zero",
        options,
        // [Fix] 加上 Ok()
        Box::new(|cc| Ok(Box::new(GuiApp::new(cc, tx, rx)))),
    ).map_err(|e| anyhow::anyhow!(e.to_string()))
}

// Dummy for Android bin
#[cfg(target_os = "android")]
fn main() {}

// 2. Android NDK Entry
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug)
    );

    let (tx, rx) = start_background_node();

    let mut options = eframe::NativeOptions::default();
    options.event_loop_builder = Some(Box::new(move |builder| {
        builder.with_android_app(app);
    }));

    let _ = eframe::run_native(
        "Project Zero Mobile",
        options,
        // [Fix] 加上 Ok()
        Box::new(|cc| Ok(Box::new(GuiApp::new(cc, tx, rx)))),
    ).map_err(|e| {
        error!("EFRAME FATAL: {}", e);
    });
}

// ==========================================
// BACKGROUND NODE
// ==========================================

fn start_background_node() -> (mpsc::Sender<Command>, mpsc::Receiver<GuiEvent>) {
    let (tx_cmd, rx_cmd) = mpsc::channel::<Command>(100);
    let (tx_event, rx_event) = mpsc::channel::<GuiEvent>(100);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if let Err(e) = run_node_logic(rx_cmd, tx_event).await {
                error!("NODE CRASHED: {}", e);
            }
        });
    });

    (tx_cmd, rx_event)
}

async fn run_node_logic(mut rx_cmd: mpsc::Receiver<Command>, tx_event: mpsc::Sender<GuiEvent>) -> Result<()> {
    let config = Config::parse();
    
    let my_ip = get_local_ip();
    let _ = tx_event.send(GuiEvent::SelfIpFound(my_ip)).await;
    
    let mut csprng = OsRng;
    let mut secret_bytes = [0u8; 32];
    csprng.fill_bytes(&mut secret_bytes);
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let my_id = PeerId::from_public_key(&signing_key.verifying_key());
    
    let _ = std::fs::create_dir_all(&config.storage_path);
    let store = Arc::new(ZeroStore::new(&config.storage_path).await?);

    let transport = match Transport::bind(config.port).await {
        Ok(t) => t,
        Err(e) => {
            let _ = tx_event.send(GuiEvent::Error(format!("Bind Fail: {}", e))).await;
            Transport::bind(0).await?
        }
    };
    
    let peers = Arc::new(Mutex::new(PeerManager::new()));
    if let Some(addr) = config.bootstrap_peer { peers.lock().unwrap().add_bootstrap_addr(addr); }

    let discovery = DiscoveryEngine::new();
    discovery.start_beacon();
    let tx_d = tx_event.clone();
    let (tx_i, rx_i) = std::sync::mpsc::channel();
    discovery.start_listener(tx_i);
    std::thread::spawn(move || { while let Ok(_) = rx_i.recv() { let _ = tx_d.blocking_send(GuiEvent::PeerFound("New".into())); }});

    let net_key = signing_key.clone();
    let net_store = store.clone();
    let net_peers = peers.clone();
    
    let _ = tx_event.send(GuiEvent::Log(format!("ID: {}", my_id.short()))).await;

    let mut seen = HashSet::new();
    loop {
        tokio::select! {
            Some(cmd) = rx_cmd.recv() => {
                match cmd {
                    Command::Publish { content } => {
                        if let Ok(cid) = net_store.store(&content).await {
                            let _ = tx_event.send(GuiEvent::PublishSuccess{cid:cid.clone(), is_json:false}).await;
                            smart_broadcast(Payload::Store{cid, data:content}, &net_key, &transport, &net_peers, &mut seen).await;
                        }
                    },
                    Command::Fetch { cid } => {
                        if let Ok(data) = net_store.fetch(&cid).await {
                            let _ = tx_event.send(GuiEvent::ContentArrived{cid, data}).await;
                        } else {
                            smart_broadcast(Payload::Fetch{cid}, &net_key, &transport, &net_peers, &mut seen).await;
                        }
                    },
                    Command::ConnectPeer { ip } => {
                        if let Ok(addr) = format!("{}:9000", ip).parse::<SocketAddr>() {
                             let _ = tx_event.send(GuiEvent::Log(format!("-> {}", addr))).await;
                             let _ = transport.send_to(ZeroPacket::new(Payload::Heartbeat{timestamp:0}, &net_key), addr).await;
                             net_peers.lock().unwrap().add_bootstrap_addr(addr);
                        } else {
                             let _ = tx_event.send(GuiEvent::Error("Invalid IP".into())).await;
                        }
                    }
                }
            },
            Ok((pkt, addr)) = transport.recv() => {
                if pkt.verify() && !seen.contains(&pkt.id) {
                    seen.insert(pkt.id.clone());
                    net_peers.lock().unwrap().add_peer(pkt.sender.clone(), addr);
                    process_payload(pkt.payload, &net_store, &tx_event).await;
                }
            }
        }
    }
}

// --- Utils ---

fn get_local_ip() -> String {
    let socket = match UdpSocket::bind("0.0.0.0:0") { Ok(s) => s, Err(_) => return "BindErr".into() };
    match socket.connect("8.8.8.8:80") {
        Ok(_) => socket.local_addr().map(|a| a.ip().to_string()).unwrap_or("AddrErr".into()),
        Err(_) => "Offline".into()
    }
}

async fn smart_broadcast(p: Payload, k: &SigningKey, t: &Transport, pm: &Arc<Mutex<PeerManager>>, seen: &mut HashSet<String>) {
    if let Ok(_) = bincode::serialize(&p) {
        let pkt = ZeroPacket::new(p, k);
        seen.insert(pkt.id.clone());
        let addrs = pm.lock().unwrap().get_all_addrs();
        for addr in addrs { let _ = t.send_to(pkt.clone(), addr).await; }
    }
}

async fn process_payload(p: Payload, store: &Arc<ZeroStore>, tx: &mpsc::Sender<GuiEvent>) {
    match p {
        Payload::Store { cid, data } => { let _ = store.store(&data).await; },
        Payload::Data { cid, data } => { 
            if store.store(&data).await.is_ok() { let _ = tx.send(GuiEvent::ContentArrived{cid, data}).await; } 
        },
        _ => {}
    }
}

struct Config { port: u16, storage_path: String, bootstrap_peer: Option<SocketAddr> }
impl Config {
    fn parse() -> Self {
        #[cfg(target_os = "android")] { Self { port: 0, storage_path: "/data/data/com.projectzero.node/files".to_string(), bootstrap_peer: None } }
        #[cfg(not(target_os = "android"))] { Self { port: 9000, storage_path: "./storage".to_string(), bootstrap_peer: None } }
    }
}

struct GuiApp { tx: mpsc::Sender<Command>, rx: mpsc::Receiver<GuiEvent>, discovery: DiscoveryEngine, local_ip: String, logs: Vec<String>, target_ip: String }
impl GuiApp {
    fn new(_cc: &eframe::CreationContext<'_>, tx: mpsc::Sender<Command>, rx: mpsc::Receiver<GuiEvent>) -> Self {
        Self { tx, rx, discovery: DiscoveryEngine::new(), local_ip: "Init...".into(), logs: vec![], target_ip: String::new() }
    }
}
impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                GuiEvent::Log(s) => self.logs.push(s),
                GuiEvent::Error(s) => self.logs.push(format!("ERR: {}", s)),
                GuiEvent::PeerFound(_) => ctx.request_repaint(),
                GuiEvent::SelfIpFound(ip) => self.local_ip = ip,
                _ => {}
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Zero v0.5");
            ui.colored_label(egui::Color32::GREEN, format!("IP: {}", self.local_ip));
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Connect:");
                ui.text_edit_singleline(&mut self.target_ip);
                if ui.button("Go").clicked() { let _ = self.tx.try_send(Command::ConnectPeer{ip: self.target_ip.clone()}); }
            });
            
            ui.collapsing("Nearby Nodes", |ui| {
                for peer in self.discovery.get_peers() {
                    ui.horizontal(|ui| {
                        ui.label(&peer.ip);
                        if ui.button("Connect").clicked() { let _ = self.tx.try_send(Command::ConnectPeer{ip: peer.ip.clone()}); }
                    });
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| { for log in &self.logs { ui.label(log); } });
        });
    }
}