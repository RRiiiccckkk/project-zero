use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet {
    // 1. 发现信标 (每秒广播)
    Beacon {
        id: String,      // 节点唯一标识 (如 "Node_A")
        listen_port: u16 // 我正在监听哪个端口
    },
    // 2. 握手确认 (收到信标后的回礼)
    Hello {
        id: String,
    },
    // 3. 数据传输 (现有的)
    Chunk { 
        file_hash: String, 
        index: u32, 
        total: u32, 
        data: Vec<u8> 
    },
}

// UI 事件定义 (用于通知界面更新)
#[derive(Debug)]
pub enum GuiEvent {
    Log(String),
    BindSuccess { port: u16 },
    PeerDiscovered { ip: String, id: String }, // 新增：发现节点
    AssetReady { data: Vec<u8> },
}

// UI 命令定义
#[derive(Debug)]
pub enum Command {
    SendTo { ip: String, port: String, path: String },
    BroadcastBeacon, // 触发一次广播 (通常由定时器自动触发)
}