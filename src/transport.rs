use crate::protocol::SecureEnvelope; // 注意：这里改成了 SecureEnvelope
use anyhow::{Result, Context};
use tokio::net::UdpSocket;
use std::net::SocketAddr;

// ---------------------------------------------------------
// 核心结构：ZeroTransport (加密升级版)
// ---------------------------------------------------------
pub struct ZeroTransport {
    socket: UdpSocket,
    buf: [u8; 65535],
}

impl ZeroTransport {
    pub async fn bind(port: u16) -> Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        println!(">> 正在绑定网络端口: {}", addr);
        
        let socket = UdpSocket::bind(&addr).await
            .context("无法绑定端口")?;

        Ok(Self {
            socket,
            buf: [0; 65535],
        })
    }

    // [修改] 发送现在接收 SecureEnvelope
    pub async fn send(&self, envelope: &SecureEnvelope, target: SocketAddr) -> Result<()> {
        let data = serde_json::to_vec(envelope)?;
        self.socket.send_to(&data, target).await.context("发送失败")?;
        Ok(())
    }

    // [修改] 接收现在返回 (SecureEnvelope, SocketAddr)
    // 我们不再在这里解密，只负责搬运“信封”
    pub async fn recv(&mut self) -> Result<(SecureEnvelope, SocketAddr)> {
        let (len, addr) = self.socket.recv_from(&mut self.buf).await?;
        let data = &self.buf[..len];
        
        // 反序列化成信封
        let envelope: SecureEnvelope = serde_json::from_slice(data)
            .context("收到无效数据：不是标准的加密信封")?;

        Ok((envelope, addr))
    }
    
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }
}