mod identity;
mod protocol;
mod transport;
mod crypto;
mod peers;
mod safedoc;

use anyhow::Result;
use identity::NodeIdentity;
use protocol::{Message, ZeroPacket, SecureEnvelope};
use transport::ZeroTransport;
use crypto::{CryptoBox, Decryptor};
use safedoc::{SafePage, Element};
use std::time::Duration;
use std::net::SocketAddr;

// ============================================================================
// 服务器逻辑 (The Server)
// ============================================================================
async fn run_server() -> Result<()> {
    // 1. 初始化
    let identity = NodeIdentity::load_or_create()?;
    let decryptor = Decryptor::load_or_create()?;
    let mut transport = ZeroTransport::bind(8080).await?;
    
    println!("✅ [Server] 核心启动 | 身份: {}... | 监听: 8080", &identity.get_id_string()[0..8]);

    loop {
        // 2. 接收加密信封
        let (envelope, src_addr) = match transport.recv().await {
            Ok(res) => res,
            Err(e) => { eprintln!("   [Server] 接收错误: {}", e); continue; }
        };

        // 3. 解密
        let decrypted = match decryptor.decrypt(&envelope.ephemeral_pk, &envelope.nonce, &envelope.ciphertext) {
            Ok(d) => d,
            Err(_) => { println!("   [Server] ⚠️ 拦截到无法解密的恶意包"); continue; }
        };

        // 4. 验证签名
        let packet: ZeroPacket = serde_json::from_slice(&decrypted)?;
        let request_msg = match packet.verify() {
            Ok(m) => m,
            Err(_) => { println!("   [Server] ⚠️ 签名无效，丢弃"); continue; }
        };

        // 5. 处理请求
        if let Message::Request { path, reply_key } = request_msg {
            println!("   [Server] 收到请求: {} | 来自: {}", path, src_addr);
            
            // --- 生成页面 ---
            let resp_msg = match path.as_str() {
                "/home" => {
                    let page = SafePage::new("Project Zero 首页")
                        .add(Element::Header("欢迎来到暗网".to_string()))
                        .add(Element::Text("恭喜！你刚刚完成了一次端到端的绝密通信。".to_string()))
                        .add(Element::Text("服务器不知道你是谁，ISP看不到你在读什么。".to_string()))
                        .add(Element::Link { label: "关于我们".to_string(), target_id: identity.get_id_string() });
                    Message::Response { page }
                },
                _ => Message::NotFound { reason: "404: 虚空之中空无一物".to_string() },
            };

            // 6. 加密回信 (使用 Request 里带来的 reply_key)
            //    注意：这次 Server 是发送方，Server 生成临时密钥
            let box_server = CryptoBox::new();
            
            //    A. 制作签名包
            let signed_resp = ZeroPacket::new(&identity, resp_msg)?;
            let signed_bytes = serde_json::to_vec(&signed_resp)?;
            
            //    B. 加密
            let server_ephemeral_pk = box_server.get_public_key_bytes();
            let (nonce, ciphertext) = box_server.encrypt(&reply_key, &signed_bytes)?; // 这里消耗了 box_server
            
            //    C. 封装
            let resp_envelope = SecureEnvelope {
                ephemeral_pk: server_ephemeral_pk,
                nonce,
                ciphertext,
            };

            //    D. 发送 (复用 transport)
            transport.send(&resp_envelope, src_addr).await?;
            println!("   [Server] 响应已加密并发送 -> {}", src_addr);
        }
    }
}

// ============================================================================
// 客户端逻辑 (The Client)
// ============================================================================
async fn run_client() -> Result<()> {
    // 给 Server 一点启动时间
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 1. 初始化
    let identity = NodeIdentity::load_or_create()?;
    let decryptor = Decryptor::load_or_create()?; // Client 也有长期加密私钥
    let transport = ZeroTransport::bind(0).await?; // 随机端口
    
    // 假设我们要访问的目标 (Server)
    // 在真实网络中，这里应该是从 PeerManager 查出来的
    // 这里我们硬编码 Server 的加密公钥 (模拟已知)
    // *** 注意：实际运行中，因为是在同一台机跑，Server的key也是这一份 ***
    // 为了简单，我们直接重新加载一次 Decryptor 拿到 Key，或者你可以把 Server 的 key 打印出来填这里
    let server_target_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let server_enc_pk = decryptor.get_public_key_bytes(); // 这里偷懒了，假设 Client 知道 Server 的 Key

    println!("\n>> [Client] 浏览器启动。正在连接暗网...");

    // 2. 构造请求
    // 这里的关键是：把 decryptor.get_public_key_bytes() 塞进去，告诉 Server 怎么回信
    let req_msg = Message::Request { 
        path: "/home".to_string(),
        reply_key: decryptor.get_public_key_bytes() 
    };
    
    let signed_req = ZeroPacket::new(&identity, req_msg)?;
    let signed_bytes = serde_json::to_vec(&signed_req)?;

    // 3. 加密发送
    let box_client = CryptoBox::new();
    let client_ephemeral_pk = box_client.get_public_key_bytes();
    let (nonce, ciphertext) = box_client.encrypt(&server_enc_pk, &signed_bytes)?;

    let req_envelope = SecureEnvelope {
        ephemeral_pk: client_ephemeral_pk,
        nonce,
        ciphertext,
    };

    transport.send(&req_envelope, server_target_addr).await?;
    println!(">> [Client] 请求已发送，等待回信...");

    // 4. 接收响应 (这里用 transport.socket 因为我们要 split 出来用，或者简单点直接创建一个新的 loop)
    // 由于 ZeroTransport 的 recv 需要 &mut self，我们在单次发送后直接调用即可
    // 注意：我们要把 transport 变为可变
    let mut transport = transport; 
    
    // 等待回信 (5秒超时)
    let timeout = tokio::time::timeout(Duration::from_secs(5), transport.recv());
    
    match timeout.await {
        Ok(Ok((resp_envelope, _))) => {
            println!(">> [Client] 收到加密数据，正在渲染...");
            
            // 5. 解密响应
            let decrypted = decryptor.decrypt(&resp_envelope.ephemeral_pk, &resp_envelope.nonce, &resp_envelope.ciphertext)?;
            
            // 6. 验证签名
            let packet: ZeroPacket = serde_json::from_slice(&decrypted)?;
            let msg = packet.verify()?;
            
            // 7. 渲染页面
            if let Message::Response { page } = msg {
                // *** 调用渲染引擎 ***
                page.render();
            } else {
                println!(">> [Client] 错误：收到的不是页面");
            }
        },
        _ => println!(">> [Client] 请求超时，服务器可能已下线。"),
    }

    Ok(())
}

// ============================================================================
// 主入口
// ============================================================================
#[tokio::main]
async fn main() -> Result<()> {
    // 启动 Server 任务
    tokio::spawn(async {
        if let Err(e) = run_server().await {
            eprintln!("Server Error: {}", e);
        }
    });

    // 启动 Client 任务
    if let Err(e) = run_client().await {
        eprintln!("Client Error: {}", e);
    }

    Ok(())
}