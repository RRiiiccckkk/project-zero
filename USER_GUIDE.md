```markdown
# Project Zero v0.3 User Guide / 用户使用手册

**Version:** 0.3 (Secure Gossip Release)
**Architecture:** ZeroUI + UDP + Ed25519 + DHT

---

## 📖 目录 (Table of Contents)
1.  [快速启动 (Quick Start)](#1-快速启动-quick-start)
2.  [通用界面指南 (General Interface)](#2-通用界面指南-general-interface)
3.  [开发者高级指南 (Developer's Guide)](#3-开发者高级指南-developers-guide)
    *   [构建网络拓扑 (Network Topology)](#31-构建网络拓扑)
    *   [Gossip 广播机制验证](#32-gossip-广播机制验证)
    *   [入侵与防御测试 (Security Audit)](#33-入侵与防御测试-security-audit)
4.  [故障排查 (Troubleshooting)](#4-故障排查-troubleshooting)

---

## 1. 快速启动 (Quick Start)

Project Zero v0.3 是一个独立的 P2P 节点程序。要模拟去中心化网络，你需要同时运行多个实例。

### 启动节点
打开终端，运行以下命令：
```bash
cargo run
```
*建议：打开 2~3 个独立的终端窗口，分别运行该命令，以模拟不同的网络节点。*

---

## 2. 通用界面指南 (General Interface)

启动后，你将看到 **ZeroUI** 图形界面。

### 2.1 顶部控制栏 (Top Control Bar)
这是节点的控制中枢，从左到右依次为：

*   **🔌 Peers (节点数)**:
    显示当前节点路由表中已连接的邻居数量。初始为 0。
*   **🔒 Verified (安全状态)**:
    显示当前主视图内容的来源签名。
    *   `System (Local)`: 显示的是本地默认内容。
    *   `<Hex_Key>` (绿色): 显示的是来自远程节点的、签名验证通过的安全内容。
*   **Bootstrap IP (引导入口)**:
    *   用于输入你想连接的第一个节点的地址（例如 `127.0.0.1:8000`）。
    *   点击 **[Join]** 按钮进行连接。
*   **📡 Broadcast (广播按钮)**:
    *   点击后，将把自己当前的 UI 蓝图签名并广播给所有已知节点。

### 2.2 主视图 (Main View)
屏幕中央的白色/灰色区域是 **"投影区"**。
*   **特性**: 这里不包含任何本地硬编码逻辑。它完全由接收到的 JSON 数据包驱动渲染。
*   **交互**: 如果远程蓝图中包含按钮（如 Ping），点击它会发送回执信号。

### 2.3 底部日志面板 (System Logs)
位于窗口最底部。
*   这是了解节点底层行为的窗口。
*   它会显示：数据包接收、握手状态、签名验证结果、Gossip 转发记录等。

---

## 3. 开发者高级指南 (Developer's Guide)

本章节适用于理解协议流、调试网络以及验证安全性的开发者。

### 3.1 构建网络拓扑
为了测试 Gossip 协议，你需要构建一个链式或网状结构。

**场景：构建 A -> B -> C 链路**

1.  **获取端口**: 启动三个节点，查看各自底部日志的第一行：
    *   节点 A: `Secure Node started on port 8000`
    *   节点 B: `Secure Node started on port 8001`
    *   节点 C: `Secure Node started on port 8002`
2.  **连接 B 到 A**:
    *   在节点 B 的 `Bootstrap IP` 输入 `127.0.0.1:8000`，点击 **[Join]**。
    *   *检查*: B 的 Peers 变为 1，A 的 Peers 变为 1。
3.  **连接 C 到 B**:
    *   在节点 C 的 `Bootstrap IP` 输入 `127.0.0.1:8001`，点击 **[Join]**。
    *   *检查*: C 的 Peers 变为 1 (连了 B)，B 的 Peers 变为 2 (连了 A 和 C)。

### 3.2 Gossip 广播机制验证
验证消息是否能通过中间节点转发。

1.  在 **节点 A** 上点击 **[📡 Broadcast]**。
2.  **观察节点 C**:
    *   虽然 C 没有直接连接 A，但 C 的界面应瞬间变为 A 的金色欢迎界面。
    *   顶部状态栏应显示：`🔒 Verified: <NodeA_PublicKey>`。
3.  **观察节点 B (中继者)**:
    *   检查 B 的底部日志，应包含：`>> Relayed to 1 peers.`
    *   *原理解析*: B 收到 A 的包，验证签名通过，查表发现 C，遂将包转发给 C。

### 3.3 入侵与防御测试 (Security Audit)
v0.3 内置了 **Intruder Mode (入侵模式)**，用于验证 Ed25519 签名拦截机制。

#### 第一步：确定攻击目标
假设 **节点 A** 想攻击 **节点 B**。
*   首先，你需要知道 **节点 B** 的确切监听端口（查看节点 B 的窗口标题或首行日志，例如 `8001`）。

#### 第二步：配置攻击向量
在 **节点 A** 的界面顶部，最右侧：
1.  找到红色的 **[😈 Attack]** 按钮。
2.  **关键步骤**: 在红色按钮**左侧的小输入框**中，输入受害者地址：
    `127.0.0.1:8001`
    *(注意：不要输错成 Bootstrap 的输入框，也不要输错端口)*

#### 第三步：发动攻击与取证
1.  点击 **[😈 Attack]**。
2.  **攻击者 (A) 日志**:
    `🚀 Launching SPOOFED packet...` (发送了签名最后一字节被反转的恶意包)。
3.  **受害者 (B) 取证**:
    *   **界面表现**: 界面**必须**保持原样，不能出现红色的 "HACKED" 警告。
    *   **日志取证**: 底部日志**必须**出现以下红字警告：
        ```text
        🛡️ BLOCKED: Fake/Tampered packet from 127.0.0.1:XXXX
        ```

---

## 4. 故障排查 (Troubleshooting)

**Q: 为什么点击 Attack 后，受害者的日志没有任何反应？**
*   **A**: 你可能攻击了错误的端口（例如打到了空端口）。请务必检查受害者窗口第一行日志 `Started on port XXXX`，并确保攻击者输入框里的地址与之一致。

**Q: 为什么我看不到 "Relayed" 日志？**
*   **A**: 只有**中间节点**会打印 Relayed。
    *   A -> B: 没人转发。
    *   A -> B -> C: A 广播，B 会打印 Relayed（转发给 C）。

**Q: 为什么广播一次后，再点广播没有反应？**
*   **A**: 系统内置了**去重缓存 (Deduplication)**。如果你短时间内发送完全相同的包（Nonce 相同），接收方会视为重复包直接丢弃。v0.3 的广播按钮每次点击都会生成新的随机 Nonce，所以通常都会刷新。

**Q: 这个系统真的没有 Web 服务器吗？**
*   **A**: 是的。没有 Nginx，没有 Apache，没有 HTML。你看到的每一个像素都是由 Rust 代码解析 UDP 数据包后直接绘制在 GPU 上的。

---

> **Project Zero** - *Rebuilding the Internet, one packet at a time.*