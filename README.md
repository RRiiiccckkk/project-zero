code
Markdown
# Project Zero (v0.5)

> **The Client is a Projector.**  
> Next-Gen P2P Infrastructure built on Rust.

## 🌌 Vision
Project Zero 旨在重构互联网基础设施。我们抛弃了臃肿的 Web 栈 (HTML/JS/DOM)，采用 **"Remote Declarative UI" (远程声明式 UI)** 和 **"Pixel Streaming"** 理念。客户端不再计算逻辑，只负责像投影仪一样渲染接收到的蓝图。

## 🏗 Architecture (v0.5 "Mobile Link")
- **Core:** Rust + Tokio (Async Runtime)
- **UI Engine:** Egui (Immediate Mode GUI)
- **Transport:** UDP + Custom Fragmentation Protocol (Break MTU limits)
- **Discovery:** UDP Broadcast Beacon (LAN Zero-Config)
- **Platform:** macOS / Linux / Windows / **Android (Native Activity)**

## 📱 Android Support
v0.5 版本正式打通了 Android NDK 交叉编译链路。
- **No Java/Kotlin:** 纯 Rust 实现，通过 `android-activity` 直接对接 Native 窗口。
- **Thread Isolation:** 独立的后台逻辑线程，解决 UI 线程阻塞导致的 ANR/Crash 问题。
- **P2P Enabled:** 手机端作为完整节点，支持局域网自动发现与互联。

## 🚀 Getting Started

### Prerequisites
1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Install Android Tools (for mobile build):
   - Android Studio & NDK (API 33+)
   - `cargo install cargo-apk`
   - `rustup target add aarch64-linux-android`

### Running on Desktop
```bash
cargo run
Running on Android
See USER_MANUAL.md for detailed cross-compilation guide.

🗺 Roadmap

Phase 1-20: Core Protocol, Fragmentation, ZeroStore.

Phase 21: Android Cross-compilation & LAN Discovery. (Current)

Phase 22: Visual Content Transmission (Camera/Blueprints).

Phase 23: Global DHT Identity.