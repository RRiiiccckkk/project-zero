use anyhow::{Context, Result};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey, Signature}; // 这里只保留这一行引入
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------
// 核心结构：NodeIdentity
// ---------------------------------------------------------
pub struct NodeIdentity {
    keypair: SigningKey,
}

impl NodeIdentity {
    // [功能 1] 加载身份，如果不存在则创建一个新的
    pub fn load_or_create() -> Result<Self> {
        let path = Path::new("identity.secret");

        if path.exists() {
            println!(">> 正在读取现有身份文件...");
            let bytes = fs::read(path).context("无法读取身份文件")?;

            // 必须把 Vec<u8> 转成 &[u8; 32]
            let bytes_ref: &[u8; 32] = bytes.as_slice().try_into().map_err(|_| {
                anyhow::anyhow!("身份文件损坏：长度不正确，必须是 32 字节")
            })?;

            let keypair = SigningKey::from_bytes(bytes_ref);
            Ok(Self { keypair })
        } else {
            println!(">> 未找到身份，正在创建全新的数字身份...");
            Self::create_new(path)
        }
    }

    // [功能 2] 生成新身份并保存到磁盘
    fn create_new(path: &Path) -> Result<Self> {
        let mut csprng = OsRng;
        let keypair = SigningKey::generate(&mut csprng);

        let bytes = keypair.to_bytes();
        fs::write(path, bytes).context("无法写入身份文件")?;

        println!(">> 新身份已创建并保存至 {:?}", path);
        Ok(Self { keypair })
    }

    // [功能 3] 获取我的公钥
    pub fn get_public_key(&self) -> VerifyingKey {
        self.keypair.verifying_key()
    }

    // [功能 4] 获取人类可读的 ID
    pub fn get_id_string(&self) -> String {
        hex::encode(self.get_public_key().as_bytes())
    }

    // [功能 5] 对数据进行签名 (新增)
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.keypair.sign(data)
    }
}