use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey, Signature, SignatureError};
use sha2::{Sha256, Digest};
use std::fmt;
use uuid::Uuid;

// --- Type Definitions ---

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub [u8; 32]);

impl PeerId {
    pub fn from_public_key(pk: &VerifyingKey) -> Self { Self(pk.to_bytes()) }
    pub fn to_verifying_key(&self) -> Result<VerifyingKey, SignatureError> { VerifyingKey::from_bytes(&self.0) }
    pub fn short(&self) -> String {
        let hex = hex::encode(&self.0);
        format!("{}...", &hex[0..6])
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", hex::encode(&self.0)) }
}

// --- Protocol Payload ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Payload {
    Heartbeat { timestamp: u64 },
    BlueprintGossip { ui_tree_json: String, timestamp: u64 },

    Store { cid: String, data: Vec<u8> },
    Fetch { cid: String },
    Data { cid: String, data: Vec<u8> },

    // --- Phase 19: Fragmentation ---
    /// 数据分片
    /// 用于传输超过 MTU 的大包
    Chunk {
        group_id: String, // 原始包的唯一 ID (用于将分片归类)
        index: u32,       // 当前分片序号 (0, 1, 2...)
        total: u32,       // 总分片数
        data: Vec<u8>,    // 分片数据
    },
}

// --- ZeroPacket (Envelope) ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZeroPacket {
    pub id: String,
    pub sender: PeerId,
    pub payload: Payload,
    pub signature: Vec<u8>,
}

impl ZeroPacket {
    pub fn new(payload: Payload, signing_key: &SigningKey) -> Self {
        let sender = PeerId::from_public_key(&signing_key.verifying_key());
        let id = Uuid::new_v4().to_string();
        
        let mut packet = Self {
            id,
            sender,
            payload,
            signature: vec![],
        };

        let signature = packet.sign_payload(signing_key);
        packet.signature = signature.to_vec();

        packet
    }

    fn sign_payload(&self, key: &SigningKey) -> Signature {
        let bytes = self.serialize_for_signing();
        key.sign(&bytes)
    }

    pub fn verify(&self) -> bool {
        let Ok(vk) = self.sender.to_verifying_key() else { return false; };
        let bytes = self.serialize_for_signing();
        if let Ok(sig) = Signature::from_slice(&self.signature) {
            return vk.verify_strict(&bytes, &sig).is_ok();
        }
        false
    }

    fn serialize_for_signing(&self) -> Vec<u8> {
        let temp = (&self.id, &self.sender, &self.payload);
        bincode::serialize(&temp).unwrap_or_default()
    }
    
    pub fn calculate_cid(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}