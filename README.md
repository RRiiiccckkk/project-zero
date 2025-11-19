
```markdown
# Project Zero

> **从零构建的 Rust 去中心化 P2P 网络栈原型。**
> A prototype decentralized P2P network stack built from scratch in Rust.

Project Zero 旨在探索 Web 3.0 的底层实现。它摒弃了传统的 HTTP/TCP/WebSockets 架构，完全基于 **UDP** 构建了一个包含可靠传输、加密、路由、存储和渲染的全栈网络。

## 🌟 核心特性 (Features)

### 1. 网络层 (Networking)
*   **异步 UDP:** 基于 `Tokio` 实现高并发非阻塞 IO。
*   **ARQ 可靠传输:** 实现了应用层重传机制，解决 UDP 丢包问题。
*   **NAT 穿透 (Hole Punching):** 支持 Rendezvous 打洞，允许内网节点直连。

### 2. 安全层 (Security)
*   **零信任架构:** 默认不信任任何对等节点。
*   **身份认证:** 基于 `Ed25519` 签名验证身份。
*   **加密通信:** 所有数据包使用 `ChaCha20Poly1305` 进行 AEAD 加密。

### 3. 路由与存储 (Routing & Storage)
*   **Kademlia DHT:** 实现了 XOR 距离度量和 K-Bucket 路由表，消除单点故障。
*   **内容寻址 (CAS):** 数据 Key 由内容哈希生成 (`SHA256`)，类似 IPFS。
*   **分布式 KV:** 数据分散存储在网络节点中。

### 4. 应用层 (Application)
*   **SafeDoc 协议:** 自定义的去中心化文档格式。
*   **TUI 浏览器:** 内置终端渲染引擎，可浏览加密的去中心化网页。

## 🛠 技术栈 (Tech Stack)

*   **Language:** Rust (Edition 2021)
*   **Async Runtime:** `tokio`
*   **Cryptography:** `ed25519-dalek`, `x25519-dalek`, `chacha20poly1305`, `sha2`
*   **Serialization:** `serde`, `serde_json`
*   **CLI:** `clap`

## 🚀 快速开始 (Quick Start)

### 1. 运行引导节点 (Bootnode)
```bash
cargo run -- --bind 0.0.0.0:9999