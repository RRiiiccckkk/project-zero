// src/main.rs
// Phase 13: Intruder Simulation & Security Verification

mod protocol;
mod transport; 
mod peers;     
mod zeroui;

use eframe::egui;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
use rand::rngs::OsRng;

use protocol::{ZeroPacket, Payload};
use transport::UdpTransport;
use peers::RoutingTable;
use zeroui::{Blueprint, Widget, Direction};

enum NetEvent {
    Log(String),
    NewBlueprint(Blueprint, String), 
    PeerUpdate(usize),
}

enum GuiCommand {
    Bootstrap { target_addr: SocketAddr },
    BroadcastUI,
    // 新增：攻击指令
    Attack { target_addr: SocketAddr },
    SendAction { op: String, param: String },
}

struct ZeroApp {
    blueprint: Blueprint,
    verified_source: String,
    input_state: HashMap<String, String>,
    logs: Vec<String>,
    peer_count: usize,
    rx_net: std::sync::mpsc::Receiver<NetEvent>,
    tx_gui: mpsc::Sender<GuiCommand>,
}

impl ZeroApp {
    fn new(
        cc: &eframe::CreationContext, 
        rx_net: std::sync::mpsc::Receiver<NetEvent>,
        tx_gui: mpsc::Sender<GuiCommand>,
        local_port: u16
    ) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self {
            blueprint: zeroui::splash_screen(local_port),
            verified_source: "System (Local)".into(),
            input_state: HashMap::new(),
            logs: vec![format!("Secure Node started on port {}.", local_port)],
            peer_count: 0,
            rx_net,
            tx_gui,
        }
    }

    fn render_widget(&mut self, ui: &mut egui::Ui, widget: &Widget) {
        match widget {
            Widget::Container { direction, children, padding, spacing } => {
                let layout = match direction {
                    Direction::Horizontal => egui::Layout::left_to_right(egui::Align::Center),
                    Direction::Vertical => egui::Layout::top_down(egui::Align::Min),
                };
                egui::Frame::none().inner_margin(padding.unwrap_or(0.0)).show(ui, |ui| {
                    ui.with_layout(layout, |ui| {
                        if let Some(s) = spacing { ui.spacing_mut().item_spacing = egui::vec2(*s, *s); }
                        for child in children { self.render_widget(ui, child); }
                    });
                });
            }
            Widget::Text { content, size, color } => {
                let mut txt = egui::RichText::new(content);
                if let Some(s) = size { txt = txt.size(*s); }
                if let Some(c) = color { if let Ok(col) = parse_color(c) { txt = txt.color(col); } }
                ui.label(txt);
            }
            Widget::Input { id, placeholder } => {
                let val = self.input_state.entry(id.clone()).or_insert(String::new());
                ui.add(egui::TextEdit::singleline(val).hint_text(placeholder));
            }
            Widget::Button { label, action_op, action_param } => {
                if ui.button(label).clicked() {
                    let val = self.input_state.get(action_param).cloned().unwrap_or(action_param.clone());
                    let _ = self.tx_gui.try_send(GuiCommand::SendAction {
                        op: action_op.clone(),
                        param: val,
                    });
                }
            }
        }
    }
}

impl eframe::App for ZeroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.rx_net.try_recv() {
            match event {
                NetEvent::Log(msg) => self.logs.push(msg),
                NetEvent::NewBlueprint(bp, source) => {
                    self.blueprint = bp;
                    self.verified_source = source;
                },
                NetEvent::PeerUpdate(count) => self.peer_count = count,
            }
        }

        egui::TopBottomPanel::top("net_controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("🔌 Peers: {}", self.peer_count));
                
                // 正常的 Bootstrap
                let boot_val = self.input_state.entry("bootstrap_ip".into()).or_insert("127.0.0.1:8000".into());
                ui.add(egui::TextEdit::singleline(boot_val).desired_width(100.0).hint_text("IP"));
                if ui.button("Join").clicked() {
                    if let Ok(addr) = boot_val.parse::<SocketAddr>() {
                        let _ = self.tx_gui.try_send(GuiCommand::Bootstrap { target_addr: addr });
                    }
                }
                
                ui.separator();
                
                // 广播
                if ui.button("📡 Broadcast").clicked() {
                    let _ = self.tx_gui.try_send(GuiCommand::BroadcastUI);
                    self.logs.push("Broadcasting verified UI...".into());
                }

                ui.separator();

                // --- INTRUDER CONTROLS ---
                // 攻击特定 IP
                let attack_val = self.input_state.entry("attack_ip".into()).or_insert("127.0.0.1:8000".into());
                ui.add(egui::TextEdit::singleline(attack_val).desired_width(100.0).hint_text("Target IP"));
                
                // 红色按钮：模拟攻击
                if ui.add(egui::Button::new("😈 Attack").fill(egui::Color32::from_rgb(200, 50, 50))).clicked() {
                    if let Ok(addr) = attack_val.parse::<SocketAddr>() {
                        let _ = self.tx_gui.try_send(GuiCommand::Attack { target_addr: addr });
                        self.logs.push(format!("🚀 Launching SPOOFED packet to {}...", addr));
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // 显示当前验证状态
                ui.horizontal(|ui| {
                    ui.label("Current View Source:");
                    ui.colored_label(egui::Color32::GREEN, &self.verified_source);
                });
                ui.separator();
                
                let root = self.blueprint.root.clone();
                self.render_widget(ui, &root);
            });
        });

        egui::TopBottomPanel::bottom("logs").max_height(150.0).show(ctx, |ui| {
            ui.separator();
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for log in &self.logs {
                    ui.monospace(log);
                }
            });
        });
        
        ctx.request_repaint();
    }
}

async fn run_network_layer(
    local_port: u16,
    tx_net: std::sync::mpsc::Sender<NetEvent>,
    mut rx_gui: mpsc::Receiver<GuiCommand>
) {
    let addr = SocketAddr::from(([0, 0, 0, 0], local_port));
    println!("\n=== SECURE ZERONET NODE ON PORT {} ===\n", local_port);

    let transport = match UdpTransport::new(addr).await {
        Ok(t) => Arc::new(t),
        Err(e) => { println!("[FATAL] Bind Error: {}", e); return; }
    };

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let local_pubkey = signing_key.verifying_key();
    let local_id = local_pubkey.to_bytes();

    let routing_table = Arc::new(Mutex::new(RoutingTable::new(local_id)));
    
    let short_id = hex::encode(&local_id[0..4]);
    let _ = tx_net.send(NetEvent::Log(format!("Identity: {}... (Ed25519)", short_id)));

    let mut seen_packets: Vec<([u8;32], u64)> = Vec::new();

    let my_dashboard = Blueprint {
        title: format!("Secure Space: {}", short_id),
        root: Widget::Container {
            direction: Direction::Vertical,
            padding: Some(20.0),
            spacing: Some(10.0),
            children: vec![
                Widget::Text { 
                    content: format!("Identity: Node {}", short_id), 
                    size: Some(24.0), 
                    color: Some("#00FF88".into()) 
                },
                Widget::Text { 
                    content: "This content is trusted.".into(), 
                    size: None, color: None 
                },
                Widget::Button { label: "Ping".into(), action_op: "ping".into(), action_param: "".into() }
            ]
        }
    };

    loop {
        tokio::select! {
            res = transport.recv() => {
                match res {
                    Ok((packet, src_addr)) => {
                        // --- SECURITY LAYER ---
                        let sender_pubkey = match VerifyingKey::from_bytes(&packet.sender_id) {
                            Ok(pk) => pk,
                            Err(_) => { continue; }
                        };

                        if packet.signature.len() != 64 { continue; }
                        let mut sig_bytes = [0u8; 64];
                        sig_bytes.copy_from_slice(&packet.signature);
                        let signature = Signature::from_bytes(&sig_bytes);

                        let signable_data = ZeroPacket::get_signable_bytes(packet.nonce, &packet.payload);
                        
                        // 核心验证逻辑：如果这里失败，直接丢弃
                        if sender_pubkey.verify(&signable_data, &signature).is_err() {
                            let _ = tx_net.send(NetEvent::Log(format!("🛡️ BLOCKED: Fake/Tampered packet from {}", src_addr)));
                            println!("[SEC] BLOCKED ATTACK from {}", src_addr);
                            continue; // DROP THE PACKET
                        }
                        // ----------------------

                        let msg_key = (packet.sender_id, packet.nonce);
                        if seen_packets.contains(&msg_key) {
                             match packet.payload {
                                Payload::Ping | Payload::Pong | Payload::FindNode(_) | Payload::Neighbors(_) => {}, 
                                _ => continue 
                            }
                        } else {
                            seen_packets.push(msg_key);
                            if seen_packets.len() > 100 { seen_packets.remove(0); }
                        }

                        let peer_count = {
                            let mut rt = routing_table.lock().unwrap();
                            rt.update(packet.sender_id, src_addr);
                            rt.count()
                        };
                        let _ = tx_net.send(NetEvent::PeerUpdate(peer_count));

                        match packet.payload {
                            Payload::Ping => {
                                send_signed_packet(&transport, &signing_key, local_id, packet.nonce, Payload::Pong, src_addr).await;
                            },
                            Payload::Pong => {},
                            Payload::FindNode(_) => {
                                let peers = routing_table.lock().unwrap().known_peers();
                                send_signed_packet(&transport, &signing_key, local_id, 0, Payload::Neighbors(peers), src_addr).await;
                            },
                            Payload::Neighbors(peers) => {
                                {
                                    let mut rt = routing_table.lock().unwrap();
                                    for p in peers { rt.update(p.id, p.addr); }
                                }
                                let count = routing_table.lock().unwrap().count();
                                let _ = tx_net.send(NetEvent::PeerUpdate(count));
                            },
                            Payload::UiBlueprint(ref bp) => {
                                let source_id = hex::encode(&packet.sender_id[0..4]);
                                let _ = tx_net.send(NetEvent::NewBlueprint(bp.clone(), source_id.clone()));
                                let _ = tx_net.send(NetEvent::Log(format!("Verified UI from {}", source_id)));

                                let neighbors = routing_table.lock().unwrap().known_peers();
                                for peer in neighbors {
                                    if peer.addr != src_addr && peer.id != local_id {
                                        let _ = transport.send(&packet, peer.addr).await;
                                    }
                                }
                            },
                            Payload::UiAction { .. } => {}
                        }
                    }
                    Err(e) => println!("Recv Err: {}", e),
                }
            }

            cmd = rx_gui.recv() => {
                if let Some(command) = cmd {
                    match command {
                        GuiCommand::Bootstrap { target_addr } => {
                            send_signed_packet(&transport, &signing_key, local_id, 0, Payload::FindNode(local_id), target_addr).await;
                        },
                        GuiCommand::BroadcastUI => {
                            let targets = routing_table.lock().unwrap().known_peers();
                            let nonce = rand::random::<u64>();
                            seen_packets.push((local_id, nonce));
                            for peer in &targets {
                                send_signed_packet(&transport, &signing_key, local_id, nonce, Payload::UiBlueprint(my_dashboard.clone()), peer.addr).await;
                            }
                            let _ = tx_net.send(NetEvent::Log(format!("Signed & Broadcasted to {} peers.", targets.len())));
                        },
                        // --- INTRUDER LOGIC ---
                        GuiCommand::Attack { target_addr } => {
                             send_malicious_packet(&transport, &signing_key, local_id, target_addr).await;
                        },
                        // ----------------------
                        GuiCommand::SendAction { op, param } => {
                             if op == "connect_ip" {
                                if let Ok(addr) = param.parse::<SocketAddr>() {
                                     send_signed_packet(&transport, &signing_key, local_id, 0, Payload::FindNode(local_id), addr).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn send_signed_packet(
    transport: &Arc<UdpTransport>,
    key: &SigningKey,
    local_id: [u8; 32],
    nonce: u64,
    payload: Payload,
    target: SocketAddr
) {
    let signable_data = ZeroPacket::get_signable_bytes(nonce, &payload);
    let signature: Signature = key.sign(&signable_data);
    
    let packet = ZeroPacket {
        sender_id: local_id,
        nonce,
        signature: signature.to_bytes().to_vec(),
        payload,
    };
    let _ = transport.send(&packet, target).await;
}

// --- THE ATTACK FUNCTION ---
async fn send_malicious_packet(
    transport: &Arc<UdpTransport>,
    key: &SigningKey,
    local_id: [u8; 32],
    target: SocketAddr
) {
    // 1. 构造一个恶意的 UI Payload
    let evil_blueprint = Blueprint {
        title: "HACKED SYSTEM".into(),
        root: Widget::Container {
            direction: Direction::Vertical,
            padding: Some(50.0),
            spacing: Some(20.0),
            children: vec![
                Widget::Text { 
                    content: "⚠️ YOU HAVE BEEN HACKED ⚠️".into(), 
                    size: Some(32.0), 
                    color: Some("#FF0000".into()) 
                },
                Widget::Text { 
                    content: "Your cryptographic layer failed.".into(), 
                    size: Some(16.0), color: Some("#FFFFFF".into()) 
                }
            ]
        }
    };
    
    let nonce = rand::random::<u64>();
    let payload = Payload::UiBlueprint(evil_blueprint);
    
    // 2. 进行正常签名
    let signable_data = ZeroPacket::get_signable_bytes(nonce, &payload);
    let signature: Signature = key.sign(&signable_data);
    let mut corrupted_sig = signature.to_bytes().to_vec();
    
    // 3. 【关键步骤】破坏签名数据
    // 我们反转签名的最后一个字节。
    // 这模拟了：篡改数据、伪造密钥、或中间人攻击。
    if let Some(last) = corrupted_sig.last_mut() {
        *last ^= 0xFF; 
    }

    // 4. 发送带有“坏签名”的包
    let packet = ZeroPacket {
        sender_id: local_id,
        nonce,
        signature: corrupted_sig, // <--- 无效的签名
        payload,
    };
    
    let _ = transport.send(&packet, target).await;
    println!("[ATTACK] Sent corrupted packet to {}", target);
}

fn parse_color(hex: &str) -> Result<egui::Color32, ()> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ())?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ())?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ())?;
        Ok(egui::Color32::from_rgb(r, g, b))
    } else {
        Err(())
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let port = 8000 + (rand::random::<u16>() % 1000);
    let (tx_net, rx_net) = std::sync::mpsc::channel::<NetEvent>();
    let (tx_gui, rx_gui) = mpsc::channel::<GuiCommand>(32);

    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(run_network_layer(port, tx_net, rx_gui));
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 750.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Project Zero: Secure Node (With Intruder Mode)",
        options,
        Box::new(move |cc| Ok(Box::new(ZeroApp::new(cc, rx_net, tx_gui, port)))),
    )
}