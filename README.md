```markdown
# 🌌 Project Zero: The Genesis of WWW-2.0
"Build the internet you want to see."

一个基于 Rust 从零构建的、去中心化的、无脚本执行的、端到端加密的下一代互联网原型。


## 📖 项目简介 (Introduction)
现代互联网（WWW-1.0）建立在几十年的技术债之上：脆弱的中心化基础设施（如 DNS、Cloudflare）、臃肿的协议栈、以及不安全的客户端脚本执行环境。

Project Zero 是一次彻底的**“白板设计” (Clean Slate Design)**。我们不修补旧网，我们创造新网。

- **No Central Servers**: 身份即地址 (Identity-Based Networking)。
- **No Scripting**: 彻底杜绝 XSS 和追踪，客户端只负责渲染数据，不执行逻辑。
- **Encryption Native**: 安全不是插件，而是出厂设置。


## 📂 项目目录结构 (Directory Structure)
本项目的模块化设计遵循“关注点分离”原则：

```text
project_zero/
├── Cargo.toml              # 依赖管理 (仅引入最经得起考验的加密/异步库)
├── identity.secret         # [自动生成] 你的数字身份私钥 (丢弃即死亡)
├── crypto.secret           # [自动生成] 你的长期加密私钥
└── src/
    ├── main.rs             # 🚀 程序入口：编排 Client 与 Server 的交互逻辑
    ├── identity.rs         # 🆔 身份层：基于 Ed25519 的身份生成与签名
    ├── crypto.rs           # 🔐 加密层：基于 X25519 + ChaCha20Poly1305 的 ECDH 密钥交换
    ├── protocol.rs         # 📜 协议层：定义 ZeroPacket (签名包) 和 SecureEnvelope (信封)
    ├── transport.rs        # 📡 传输层：基于 UDP 的异步收发封装
    ├── peers.rs            # 📒 路由层：去中心化节点通讯录 (Peer Manager)
    └── safedoc.rs          # 🎨 应用层：SafeDoc 标准定义与 TUI 渲染引擎
```


## 🏆 目前成果 (Current Status: MVP)
截至 Step 8，我们已经成功跑通了以下核心功能：

### 自权身份系统 (Self-Sovereign Identity)
- 基于 Ed25519 算法生成全球唯一的 NodeID。
- 完全脱离 IP 地址依赖，身份可移植。

### 军用级安全传输
- **签名验证**: 任何未经签名的数据包在反序列化前即被丢弃。
- **端到端加密 (E2EE)**: 使用 Ephemeral Keys (临时密钥) 实现前向安全性 (PFS)。ISP 或黑客无法解密流量。

### SafeDoc 协议
- 定义了一种纯数据驱动的页面描述语言（替代 HTML）。
- 实现了基于终端 (Terminal) 的渲染引擎。
- 特性: 0 JavaScript，0 Cookie，加载速度 < 10ms。

### 双向握手
- 实现了 Client 请求携带回信密钥 -> Server 加密响应的完整闭环。


## 🗺️ 后续计划 (Roadmap)
为了让 Project Zero 从“原型”进化为“生态”，后续开发者需关注以下方向：

1. **Phase 1: 网络增强 (The Network)**
   - 可靠性升级 (Reliability): 目前基于 UDP，丢包未处理。需要实现类似 TCP 的 ACK/Retransmit (重传) 机制（参考 QUIC 协议）。
   - 自动发现 (Discovery): 实现 DHT (分布式哈希表) 或 Gossip 协议，让节点不需要手动互加通讯录就能找到对方 IP。

2. **Phase 2: 体验升级 (The Experience)**
   - 交互式 TUI: 升级 main.rs，允许用户通过键盘输入 URL，而不是硬编码请求。
   - 二进制协议: 将目前的 JSON 序列化替换为 Bincode 或 Protobuf，进一步压缩流量体积，提升隐蔽性。

3. **Phase 3: 生态构建 (The Ecosystem)**
   - File Transfer: 支持大文件分片加密传输。
   - Onion Routing: 实现类似 Tor 的多跳路由，隐藏发送者的物理 IP。


## ⚔️ 开发者建议与规范 (Guidelines)
如果你想接过火种继续开发，请务必遵守以下**“宪法”**：

### Minimalism is Security (极简即安全)
- 严禁引入庞大的第三方 Web 框架。
- 严禁在 Client 端实现任何形式的通用脚本执行（如 Lua/JS/Wasm）。逻辑必须隔离在 Server 端。

### Verify, Then Trust (先验证，后信任)
- 网络层收到任何数据包，第一件事必须是验证签名和解密。解析失败直接 Drop，不要回复错误信息（防止侧信道攻击）。

### Rust Best Practices
- 生产环境代码严禁使用 unwrap()。所有错误必须通过 Result<> 向上传递并优雅处理。
- 保持 no_std 兼容性的可能性，未来我们可能会移植到嵌入式设备或微内核上。

### Privacy by Design
- 不仅要加密内容，还要思考如何隐藏元数据（如流量特征混淆）。


## 🚀 First Release Checklist

1. 在 GitHub 上创建新的仓库（如 `project-zero`），然后在本地运行下面的命令完成初次提交与推送：
   ```sh
   git remote add origin https://github.com/<your-username>/<repo>.git
   git branch -M main
   git add .
   git commit -m "chore: initial project zero"
   git push -u origin main
   ```
2. `identity.secret` 和 `crypto.secret` 会在二进制第一次运行时自动生成，且已经被 `.gitignore` 忽略，请不要把它们提交到仓库；只保留在本地安全存储。
3. 想要让新节点上线，只需克隆仓库、运行 `cargo run`（或已有的 bin），它会在当前目录生成必需的密钥文件，之后就可以像常规 Rust 项目一样构建/部署。
4. 推送完成后把 GitHub 仓库 URL 记录在文档或 README 中，方便未来协作。

> 满足以上步骤后你就拥有了 Project Zero 的“第一版”公开发行。


## 🔥 结语 (Final Words)
Project Zero 不仅仅是一堆代码，它是一种抵抗脆弱性的尝试。

当中心化的云服务崩溃时，当“守门人”决定关闭大门时，我们希望 Project Zero 能成为那个永远在线、无法被关闭的“数字避难所”。

> "The Net interprets censorship as damage and routes around it."  
> — John Gilmore

保持野心，保持纯粹。

Project Zero / Initiated 2025
```
