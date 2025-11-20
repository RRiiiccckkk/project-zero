use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;
use crate::protocol::ZeroPacket;

/// Transport: 异步 UDP 传输层封装
pub struct Transport {
    socket: Arc<UdpSocket>,
}

impl Transport {
    /// 绑定到指定端口
    pub async fn bind(port: u16) -> Result<Self> {
        // 绑定到 0.0.0.0 以接收所有接口流量
        let addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(&addr).await?;
        log::info!("UDP Transport bound to {}", socket.local_addr()?);
        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    /// 发送数据包
    pub async fn send_to(&self, packet: ZeroPacket, target: SocketAddr) -> Result<()> {
        // 序列化 (Bincode)
        let data = bincode::serialize(&packet)?;
        self.socket.send_to(&data, target).await?;
        Ok(())
    }

    /// 接收数据包
    pub async fn recv(&self) -> Result<(ZeroPacket, SocketAddr)> {
        let mut buf = [0u8; 65535]; // 最大 UDP 包大小
        loop {
            // 等待数据
            let (len, addr) = self.socket.recv_from(&mut buf).await?;
            
            // 尝试反序列化
            match bincode::deserialize::<ZeroPacket>(&buf[..len]) {
                Ok(packet) => return Ok((packet, addr)),
                Err(e) => {
                    // 忽略损坏的包，继续监听
                    log::debug!("Failed to deserialize packet from {}: {}", addr, e);
                    continue;
                }
            }
        }
    }
}