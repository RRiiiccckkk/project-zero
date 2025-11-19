use serde::{Serialize, Deserialize};
use crate::identity::NodeIdentity;
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use x25519_dalek::{EphemeralSecret, PublicKey as XPublicKey};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use chacha20poly1305::aead::{Aead, AeadCore};
use rand::rngs::OsRng;
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Request,  // 请求页面
    Response, // 返回页面
    Signal,   // 网络信令 (Discovery)
}

// === 新增：网络信令协议 ===
#[derive(Serialize, Deserialize, Debug)]
pub enum Signal {
    Hello,                              // "我上线了，请记住我"
    Query(String),                      // "请告诉我 ID xxxx 的 IP 是多少？"
    Found { id: String, addr: String }, // "你要找的 ID xxxx 在 IP yyyy"
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZeroPacket {
    pub sender_id: VerifyingKey,
    pub signature: Signature,
    pub msg_type: MessageType,
    pub payload: Vec<u8>, 
}

impl ZeroPacket {
    pub fn new(identity: &NodeIdentity, payload: Vec<u8>, msg_type: MessageType) -> Result<Self> {
        let signature = identity.sign(&payload);
        Ok(Self {
            sender_id: identity.get_public_key(),
            signature,
            msg_type,
            payload,
        })
    }

    pub fn verify(&self, expected_sender: &VerifyingKey) -> bool {
        if &self.sender_id != expected_sender {
            return false;
        }
        self.sender_id.verify(&self.payload, &self.signature).is_ok()
    }
}

#[derive(Serialize, Deserialize)]
pub struct SecureEnvelope {
    pub ephemeral_pubkey: [u8; 32], 
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

impl SecureEnvelope {
    pub fn seal(
        _sender_identity: &NodeIdentity, 
        _receiver_id: &VerifyingKey, 
        plaintext: &[u8]
    ) -> Result<Self> {
        let mut rng = OsRng;
        
        let ephemeral_secret = EphemeralSecret::random_from_rng(&mut rng);
        let ephemeral_public = XPublicKey::from(&ephemeral_secret);
        
        // HACK: Phase 4 仍然使用固定密钥，真实 P2P 应在此处做 ECDH
        let key = [42u8; 32]; 
        let cipher = ChaCha20Poly1305::new(&key.into());
        
        let nonce = ChaCha20Poly1305::generate_nonce(&mut rng);
        let ciphertext = cipher.encrypt(&nonce, plaintext)
            .map_err(|e| anyhow!("Encrypt err: {}", e))?;
        
        Ok(Self {
            ephemeral_pubkey: *ephemeral_public.as_bytes(),
            nonce: nonce.into(),
            ciphertext,
        })
    }

    pub fn open(&self, _receiver_identity: &NodeIdentity, _sender_id: &VerifyingKey) -> Result<Vec<u8>> {
        let key = [42u8; 32];
        let cipher = ChaCha20Poly1305::new(&key.into());
        
        let plaintext = cipher.decrypt(
            &self.nonce.into(), 
            self.ciphertext.as_ref()
        ).map_err(|e| anyhow!("Decrypt error: {}", e))?;
        
        Ok(plaintext)
    }
}