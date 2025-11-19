use tokio::net::UdpSocket;
use anyhow::Result;
use std::net::SocketAddr;

pub struct TransportLayer {
    pub socket: UdpSocket,
}

impl TransportLayer {
    pub fn new(socket: UdpSocket) -> Self {
        Self { socket }
    }

    // 发送原始数据
    pub async fn send(&self, data: &[u8], target: SocketAddr) -> Result<()> {
        self.socket.send_to(data, target).await?;
        Ok(())
    }

    // 接收原始数据
    pub async fn recv(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let (size, addr) = self.socket.recv_from(buf).await?;
        Ok((size, addr))
    }
}