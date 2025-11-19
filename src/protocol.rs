use crate::identity::NodeIdentity;
use crate::safedoc::SafePage; // 确保只出现一次
use anyhow::{anyhow, Result};
use ed25519_dalek::{Verifier, Signature};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------
// 1. 消息载荷 (Payload)
// ---------------------------------------------------------
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    // [修改] Request 现在携带发送者的加密公钥 (32字节)
    Request { 
        path: String,
        reply_key: [u8; 32] 
    },
    
    Response { page: SafePage },
    
    NotFound { reason: String },
}

// ---------------------------------------------------------
// 2. 零号数据包 (The Zero Packet)
// ---------------------------------------------------------
#[derive(Serialize, Deserialize, Debug)]
pub struct ZeroPacket {
    pub sender_id: [u8; 32],
    pub signature: Vec<u8>,
    pub content: String,
}

impl ZeroPacket {
    pub fn new(identity: &NodeIdentity, message: Message) -> Result<Self> {
        let content_json = serde_json::to_string(&message)?;
        let my_public_key = identity.get_public_key();
        let signature = identity.sign(content_json.as_bytes());

        Ok(Self {
            sender_id: my_public_key.to_bytes(),
            signature: signature.to_vec(),
            content: content_json,
        })
    }

    pub fn verify(&self) -> Result<Message> {
        let peer_public_key = ed25519_dalek::VerifyingKey::from_bytes(&self.sender_id)
            .map_err(|_| anyhow!("无效的发送者 ID"))?;

        let signature_obj = Signature::from_slice(&self.signature)
            .map_err(|_| anyhow!("无效的签名格式"))?;

        peer_public_key.verify(self.content.as_bytes(), &signature_obj)
            .map_err(|_| anyhow!("严重警告：签名验证失败！数据包可能被篡改！"))?;

        let message: Message = serde_json::from_str(&self.content)?;
        Ok(message)
    }
}

// ---------------------------------------------------------
// 3. 安全信封 (Encrypted Envelope)
// ---------------------------------------------------------
#[derive(Serialize, Deserialize, Debug)]
pub struct SecureEnvelope {
    pub ephemeral_pk: [u8; 32],
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}