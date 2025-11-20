use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};
use anyhow::{Context, Result, bail};
use chrono::Local; // 需要在 Cargo.toml 添加 chrono

/// ZeroStore: 双层存储架构
/// 
/// Physical Layout (Machine Optimized):
/// ./storage/objects/
///   ├── 49/
///   │    └── 49a272e5... (Content)
/// 
/// Logical Layout (Human Readable):
/// ./storage/manifest.log
///   └── [2023-11-20 17:30:00] PUT 49a272e5... (Size: 374b)
/// 
#[derive(Clone, Debug)]
pub struct ZeroStore {
    base_path: PathBuf,
    objects_path: PathBuf,
    manifest_path: PathBuf,
}

impl ZeroStore {
    pub async fn new(path: &str) -> Result<Self> {
        let base_path = PathBuf::from(path);
        let objects_path = base_path.join("objects");
        let manifest_path = base_path.join("manifest.log");

        // 初始化目录结构
        if !objects_path.exists() {
            fs::create_dir_all(&objects_path).await
                .context("Failed to create objects directory")?;
        }

        Ok(Self { base_path, objects_path, manifest_path })
    }

    pub fn calculate_cid(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    fn get_paths(&self, cid: &str) -> (PathBuf, PathBuf) {
        let shard = &cid[0..2]; 
        let dir = self.objects_path.join(shard);
        let file = dir.join(cid);
        (dir, file)
    }

    /// 记录人类可读的日志 (Append Only)
    async fn append_manifest(&self, action: &str, cid: &str, size: usize) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_entry = format!("[{}] {} | CID: {} | Size: {}b\n", timestamp, action, cid, size);
        
        // 忽略日志写入错误，不应阻塞核心流程
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.manifest_path)
            .await 
        {
            let _ = file.write_all(log_entry.as_bytes()).await;
        }
    }

    pub async fn store(&self, data: &[u8]) -> Result<String> {
        let cid = Self::calculate_cid(data);
        let (dir_path, file_path) = self.get_paths(&cid);

        // 1. 物理存储 (Physical Store)
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path).await?;
        }

        if !file_path.exists() {
            let temp_name = format!("temp_{}.tmp", cid);
            let temp_path = self.objects_path.join(&temp_name);

            let mut file = fs::File::create(&temp_path).await?;
            file.write_all(data).await?;
            file.flush().await?;
            fs::rename(&temp_path, &file_path).await?;
            
            // 2. 逻辑记录 (Logical Log) - 仅在第一次写入时记录
            self.append_manifest("NEW", &cid, data.len()).await;
            log::info!("[ZeroStore] New Object: {}/{}", &cid[0..2], &cid[0..8]);
        } else {
            self.append_manifest("DUP", &cid, data.len()).await;
            // log::info!("[ZeroStore] Dedup: {}", &cid[0..8]);
        }

        Ok(cid)
    }

    pub async fn fetch(&self, cid: &str) -> Result<Vec<u8>> {
        let (_, file_path) = self.get_paths(cid);

        if !file_path.exists() {
            bail!("Content not found: {}", cid);
        }

        let data = fs::read(&file_path).await?;

        let calculated_cid = Self::calculate_cid(&data);
        if calculated_cid != cid {
            let _ = fs::remove_file(file_path).await;
            bail!("Data corruption detected");
        }
        
        // 也就是 Access Log
        // self.append_manifest("GET", cid, data.len()).await;

        Ok(data)
    }
}