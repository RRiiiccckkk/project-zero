code
Markdown
# Project Zero (v0.9.99 "Super-Nova")

> **The Client is a Projector.**
> Next-Gen P2P Infrastructure built on Rust.

## 🌌 Vision
Project Zero 旨在重构互联网基础设施。我们抛弃了 Web 栈，采用 **"Remote Declarative UI"** 和 **"UDP Pixel Streaming"**。客户端不再计算逻辑，只负责像投影仪一样渲染接收到的蓝图。

## 🏗 Architecture (v0.9.99)
- **Core:** Rust (No-std logic, Sync/Thread-based)
- **Transport:** UDP + Custom Fragmentation + **Multi-Socket Hydra** (Anti-VPN/Proxy)
- **Discovery:** 
    - **Omni-Radar:** 自动识别本机物理网段。
    - **Smart Bomb:** 针对性单播扫描 (穿透 AP 隔离)。
    - **Super-Nova:** 跨网段 (±50 Subnets) 深度扫描。
- **Platform:** macOS / Linux / Windows / **Android (Native Activity)**

## 📱 Android Support
v0.9.99 彻底解决了 Android 权限与构建痛点：
- **Permission Injection:** 通过 `Cargo.toml` TOML 注入绕过 Manifest 缓存。
- **Structure:** 采用 `lib.rs` (Core) + `examples/` (Desktop) 分离架构。
- **Resilience:** 在 VPN/5G 环境下仍能通过物理网卡锁定局域网目标。

## 🚀 Getting Started

### Prerequisites
1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Install Android Tools (NDK API 33+) & `cargo-apk`.

### Running on Desktop
```bash
# 启动 PC 端控制台
cargo run --example desktop

Running on Android
code
Bash
# 1. 清理 (重要)
rm -rf target

# 2. 编译库并安装 (必须加 --lib 以避免冲突)
cargo apk build --lib
adb install -r target/debug/apk/project_zero.apk

🎮 How to Use
Launch: 启动手机端和电脑端。
Scan:
电脑端点击 "SCAN LOCAL NETWORK" (自动) 或 "SUPER SCAN" (跨网段)。
Connect:
在 "DETECTED NODES" 列表中点击发现的目标。
Fire:
点击 "SEND FILE"。
🗺 Roadmap

Phase 22: Visual Link (UDP Image Transport)

Phase 23: Omni-Radar Discovery (Anti-AP-Isolation)

Phase 24: Remote UI Control (JSON Blueprints)

Phase 25: Global DHT Identity