
```markdown
# Project Zero v0.3

> **下一代去中心化原生 GUI 网络协议原型。**
> A next-gen decentralized native GUI network protocol prototype.

Project Zero 旨在彻底颠覆现有的 Web 架构。我们摒弃了 HTML/CSS/JS 和 HTTP，构建了一个基于 **UDP + Ed25519 + Native GUI** 的全栈网络。

## 🧠 核心哲学 (Philosophical Core)

1.  **The Client is a Projector (客户端即投影仪):**
    客户端严禁执行任何非用户授权的代码。没有 JavaScript，没有虚拟机。客户端只负责将接收到的数据结构（Blueprints）“投影”为原生 UI 像素。
2.  **Remote Declarative UI (远程声明式 UI):**
    UI 是数据，而非代码。节点之间传输 JSON 描述的 UI 树，支持动态更新和流式传输。
3.  **Trust No One (零信任):**
    网络是不可信的。所有数据包必须经过数字签名。任何签名验证失败的数据包都会在底层被物理拦截，无法触及 UI 层。

## 🌟 核心特性 (Features)

### 1. ZeroUI 渲染引擎
*   **Native Performance:** 基于 `eframe` / `egui` (Rust Immediate Mode GUI)，利用 GPU 加速渲染，启动速度毫秒级。
*   **Hot-Swappable UI:** 界面由远程数据包驱动，可实时更新，无须重新编译客户端。

### 2. 弹性网络层 (Resilient Networking)
*   **Gossip Protocol (流言协议):** 实现了基于传染病算法的消息传播，支持全网广播。
*   **Storm Suppression (风暴抑制):** 内置去重缓存 (Deduplication Cache) 和回环检测，防止广播风暴。
*   **Kademlia DHT:** (底层集成) 用于节点发现和路由维护。

### 3. 军事级安全 (Military-Grade Security)
*   **Ed25519 Identity:** 节点 ID 即公钥。
*   **Signed Packets:** 所有 UI 蓝图和信令均由发送者私钥签名。
*   **Intruder Detection:** 自动识别并丢弃伪造、篡改或中间人攻击的数据包。

## 🛠 技术栈 (Tech Stack)

*   **Core Language:** Rust (Edition 2021)
*   **GUI Framework:** `eframe`, `egui`
*   **Async Runtime:** `tokio`
*   **Cryptography:** `ed25519-dalek`, `rand`
*   **Serialization:** `serde`, `serde_json`

## 🚀 快速开始 (Quick Start)

### 环境要求
*   Rust (最新 Stable 版本)
*   支持 GPU 的操作系统 (Windows/macOS/Linux)

### 运行节点
Project Zero v0.3 是一个图形化程序。建议开启多个终端运行多个实例以模拟网络。

```bash
# 启动节点 (默认随机端口)
cargo run