use anyhow::{Result, anyhow, Context};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng as CryptoOsRng},
    ChaCha20Poly1305, Nonce
};
use std::fs;
use std::path::Path;

// ---------------------------------------------------------
// 发送方工具 (一次性) - 保持不变
// ---------------------------------------------------------
pub struct CryptoBox {
    secret: EphemeralSecret,
    public: PublicKey,
}

impl CryptoBox {
    pub fn new() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn get_public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    pub fn encrypt(self, peer_public_bytes: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let peer_public = PublicKey::from(*peer_public_bytes);
        let shared_secret = self.secret.diffie_hellman(&peer_public);
        
        let cipher = ChaCha20Poly1305::new_from_slice(shared_secret.as_bytes())
            .map_err(|_| anyhow!("密钥长度错误"))?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut CryptoOsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext)
            .map_err(|_| anyhow!("加密失败"))?;

        Ok((nonce.to_vec(), ciphertext))
    }
}

// ---------------------------------------------------------
// 接收方工具 (长期持有) - 升级版
// ---------------------------------------------------------
pub struct Decryptor {
    secret: StaticSecret,
}

impl Decryptor {
    // [新功能] 从磁盘加载加密私钥，没有就创建
    pub fn load_or_create() -> Result<Self> {
        let path = Path::new("crypto.secret");
        if path.exists() {
            // 读取现有
            let bytes = fs::read(path).context("无法读取加密私钥")?;
            let bytes_arr: [u8; 32] = bytes.as_slice().try_into()
                .map_err(|_| anyhow!("加密私钥文件损坏"))?;
            Ok(Self { secret: StaticSecret::from(bytes_arr) })
        } else {
            // 创建新的
            let secret = StaticSecret::random_from_rng(OsRng);
            let bytes = secret.to_bytes();
            fs::write(path, bytes).context("无法保存加密私钥")?;
            Ok(Self { secret })
        }
    }

    pub fn get_public_key(&self) -> PublicKey {
        PublicKey::from(&self.secret)
    }

    // 获取公钥的 bytes 形式，方便传输
    pub fn get_public_key_bytes(&self) -> [u8; 32] {
        *self.get_public_key().as_bytes()
    }

    pub fn decrypt(&self, sender_ephemeral_pk: &[u8; 32], nonce_bytes: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let sender_pk = PublicKey::from(*sender_ephemeral_pk);
        let shared_secret = self.secret.diffie_hellman(&sender_pk);
        
        let cipher = ChaCha20Poly1305::new_from_slice(shared_secret.as_bytes())
            .map_err(|_| anyhow!("密钥长度错误"))?;
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("解密失败或校验不通过"))?;
            
        Ok(plaintext)
    }
}