code
Markdown
# Project Zero 🌐

> **Rebuilding the Internet, one packet at a time.**
> *Trust Math, Not Servers.*

![Status](https://img.shields.io/badge/Status-v0.4%20Alpha-green)
![Build](https://img.shields.io/badge/Build-Rust-orange)
![Protocol](https://img.shields.io/badge/Protocol-UDP%20Gossip-blue)

Project Zero 是下一代去中心化互联网基础设施的原型。它旨在彻底颠覆现有的 Web 2.0 架构（HTTP/DNS/TCP/HTML），构建一个基于**内容寻址**、**原生渲染**且**密码学安全**的全新网络。

---

## 🚀 Core Philosophies (核心哲学)

1.  **The Client is a Projector (客户端即投影仪)**:
    客户端严禁执行任何非用户授权的代码（无 JavaScript，无 WASM）。它只负责将接收到的 JSON 蓝图投影为原生 UI 像素。
2.  **Content Addressable (内容寻址)**:
    数据通过哈希 (CID) 索引，而非 IP 或域名。位置无关，内容即一切。
3.  **Pixel Streaming for Complexity (复杂即流媒体)**:
    任何高负载计算都在网络侧完成。
4.  **Trust Math (信任数学)**:
    所有数据包均经过 Ed25519 签名。任何篡改都会导致哈希校验失败并被系统自动隔离。

---

## ✨ Key Features (v0.4 Alpha)

*   **🕸️ ZeroNet Protocol**: 基于 UDP + Tokio 的异步 Gossip 协议，内置风暴抑制与去重机制。
*   **🧩 UDP Fragmentation**: 支持 MB 级别大文件传输，自动分片与重组，突破 MTU 限制。
*   **🎨 ZeroUI Engine**: 基于 `egui` 的即时模式渲染器，支持声明式 UI 蓝图。
*   **🧠 No-Code Logic**: 支持声明式状态机（计数器、开关），无需脚本即可实现交互。
*   **🛡️ Self-Healing Storage**: 自动检测磁盘数据腐败（篡改），并从网络节点自动获取副本进行修复。
*   **📦 Smart Storage**: 采用 Git 风格的哈希前缀分片存储 (`/objects/ab/xxxx...`)。

---

## 🛠️ Architecture

| Layer | Component | Description |
| :--- | :--- | :--- |
| **L4** | **Presentation** | ZeroUI, Projector, Creator Studio |
| **L3** | **Storage** | ZeroStore (CAS), Manifest Log, Deduplication |
| **L2** | **Security** | Ed25519 Signatures, SHA256 Integrity Check |
| **L1** | **Network** | Async UDP, Fragmentation, Gossip Protocol |

---

## 🚦 Getting Started

Project Zero 是一个 P2P 节点。要体验其功能，建议在本地启动两个节点进行组网。

### Prerequisites
*   Rust (latest stable)

### 1. Start Node A (The Creator)
此节点作为发布者，监听 9000 端口。
```bash
cargo run -- 9000 ./storage_a
2. Start Node B (The Consumer)
此节点作为浏览者，监听 9001 端口，并连接到 Node A。

code
Bash
cargo run -- 9001 ./storage_b 127.0.0.1:9000
🎮 User Guide

Creator Studio (左侧面板)
Upload Image: 点击按钮选择本地图片。系统会自动分片并存储，生成 CID。
Authoring: 编写 JSON 蓝图，将图片 CID 填入 "src" 字段。
Publish: 点击发布，生成 Page CID。
Projector / Browser (右侧面板)
Fetch: 输入 Page CID。
Experience:
文字与布局瞬间加载。
大图片通过 UDP 分片流式传输并在本地重组。
点击按钮体验无代码交互（计数器、开关）。
🔒 Security & Resilience Test
Project Zero 具有自我修复能力。

关闭 Node A。
恶意篡改: 手动修改 Node B ./storage_b/objects/ 下的图片文件内容。
重启 Node B 并加载页面。
结果: 系统会报警 ⚠️ SECURITY ALERT: Data Corruption 并拒绝加载坏图。
启动 Node A。Node B 会自动发现错误并从 Node A 重新下载正确数据进行修复。
🗺️ Roadmap

Phase 14: Content Addressable Storage (ZeroStore)

Phase 15: Remote Declarative UI (ZeroUI)

Phase 19: UDP Fragmentation (Large File Support)

Phase 20: Interactive State (No-Code)

Phase 21: Distributed Hash Table (Kademlia DHT)

Phase 25: End-to-End Encryption (Private Chat)