use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::Arc;
use std::io;
use crate::protocol::ZeroPacket;

pub struct UdpTransport {
    socket: Arc<UdpSocket>,
}

impl UdpTransport {
    pub async fn new(addr: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    pub async fn send(&self, packet: &ZeroPacket, target: SocketAddr) -> io::Result<()> {
        let bytes = serde_json::to_vec(packet).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.socket.send_to(&bytes, target).await?;
        Ok(())
    }

    pub async fn recv(&self) -> io::Result<(ZeroPacket, SocketAddr)> {
        let mut buf = [0u8; 65535]; // UDP standard max size
        let (len, addr) = self.socket.recv_from(&mut buf).await?;
        
        let packet: ZeroPacket = serde_json::from_slice(&buf[..len])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            
        Ok((packet, addr))
    }
}