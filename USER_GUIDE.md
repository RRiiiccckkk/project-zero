```markdown
这是基于 **Phase 20** 完成后的最新版用户手册。它涵盖了从存储分片、大文件传输、到交互式 UI 和自我修复机制的所有新特性。

--- START OF FILE USER_GUIDE.md ---

```markdown
# Project Zero v0.4 User Guide / 用户使用手册

**Version:** 0.4 Alpha (The "ZeroNet" Release)
**Architecture:** ZeroStore + ZeroUI + UDP Fragmentation + Declarative State

---

## 📖 目录 (Table of Contents)
1.  [项目概述 (Overview)](#1-项目概述-overview)
2.  [快速启动 (Quick Start)](#2-快速启动-quick-start)
3.  [界面概览 (Interface Overview)](#3-界面概览-interface-overview)
4.  [创作者指南 (Creator's Guide)](#4-创作者指南-creators-guide)
    *   [上传多媒体资源 (Assets)](#41-上传多媒体资源-assets)
    *   [编写交互式蓝图 (Blueprints)](#42-编写交互式蓝图-blueprints)
    *   [蓝图语法参考 (Syntax Cheat Sheet)](#43-蓝图语法参考-syntax-cheat-sheet)
5.  [浏览与交互 (Browsing & Interactivity)](#5-浏览与交互-browsing--interactivity)
6.  [高级特性与安全 (Advanced & Security)](#6-高级特性与安全-advanced--security)
    *   [UDP 分片传输 (Fragmentation)](#61-udp-分片传输-fragmentation)
    *   [自我修复机制 (Self-Healing)](#62-自我修复机制-self-healing)
    *   [存储结构 (Storage Layout)](#63-存储结构-storage-layout)

---

## 1. 项目概述 (Overview)
Project Zero v0.4 是一个完全去中心化的互联网基础设施原型。它移除了 HTTP、TCP、DNS 和 Web 服务器，实现了：
*   **内容寻址 (Content Addressing)**: 通过 CID (Hash) 访问数据。
*   **原生渲染 (Native Rendering)**: 传输 JSON 蓝图，由客户端直接 GPU 渲染，无 HTML/CSS。
*   **无代码交互 (No-Code Logic)**: 安全的声明式状态机，杜绝脚本注入攻击。
*   **抗毁性 (Resilience)**: 数据篡改自动检测与网络自我修复。

---

## 2. 快速启动 (Quick Start)

为了模拟 P2P 网络，建议在同一台机器上启动两个不同的节点（终端窗口）。

### 启动节点 A (发布者/Creator)
```bash
# 端口 9000，存储目录 ./storage_a
cargo run -- 9000 ./storage_a
```

### 启动节点 B (浏览者/Consumer)
```bash
# 端口 9001，存储目录 ./storage_b，连接到节点 A
cargo run -- 9001 ./storage_b 127.0.0.1:9000
```

---

## 3. 界面概览 (Interface Overview)

v0.4 采用了 **双栏布局 (Split View)**：

*   **左侧: Creator Studio (创作者工作室)**
    *   用于上传图片、编写 JSON 代码、发布页面。
    *   包含 "Upload Image" 和 "Publish App Blueprint" 按钮。
*   **右侧: Projector / Browser (投影仪/浏览器)**
    *   用于输入 CID 并渲染远程内容。
    *   包含 CID 输入框、"Fetch" 按钮和渲染画布。
*   **底部: System Logs (系统日志)**
    *   显示传输进度、分片重组状态、安全警告等。
*   **顶部: Status Bar (状态栏)**
    *   显示当前节点 ID 和最近一次操作的反馈（如 "Asset Uploaded!"）。

---

## 4. 创作者指南 (Creator's Guide)

### 4.1 上传多媒体资源 (Assets)
ZeroUI 支持显示图片，甚至是大尺寸图片（支持自动分片）。

1.  在左侧点击 **📂 Upload Image**。
2.  选择本地的 `.png` 或 `.jpg` 文件。
3.  观察日志，等待上传完成（大文件会显示 `Fragmenting...`）。
4.  成功后，点击出现的 **📋 Asset CID** 按钮复制哈希值。

### 4.2 编写交互式蓝图 (Blueprints)
Project Zero 不使用 HTML，而是使用 JSON 描述 UI。

1.  在左侧编辑框编写 JSON。
2.  若要引用刚才上传的图片，将 `src` 字段的值替换为刚才复制的 Asset CID。
3.  点击 **🚀 Publish App Blueprint**。
4.  成功后，点击 **📋 Page CID** 获取该页面的访问地址。

### 4.3 蓝图语法参考 (Syntax Cheat Sheet)

**基础组件:**
```json
{ "type": "VStack", "spacing": 10.0, "children": [...] }  // 垂直布局
{ "type": "HStack", "spacing": 10.0, "children": [...] }  // 水平布局
{ "type": "Text", "value": "Hello", "size": 20.0 }        // 文本
{ "type": "Image", "src": "<CID>", "width": 300.0 }       // 图片
```

**交互组件 (按钮与链接):**
```json
// 页面跳转
{ 
  "type": "Button", 
  "label": "Go to Page 2", 
  "on_click": { "type": "Navigate", "cid": "<Target_CID>" } 
}
```

**状态绑定 (State & Logic):**
*   **显示变量**: 在 Text value 中使用 `$` 前缀，如 `"$counter"`。
*   **修改变量**: 使用 `Increment` (自增) 或 `Toggle` (切换 0/1)。

```json
// 显示状态
{ "type": "Text", "value": "Count: $counter", "size": 20.0 }

// 触发逻辑
{ 
  "type": "Button", 
  "label": "+1", 
  "on_click": { "type": "Increment", "key": "counter" } 
}
```

---

## 5. 浏览与交互 (Browsing & Interactivity)

在右侧的 **Projector** 面板：

1.  **访问**: 在地址栏输入 CID（或从左侧复制 Page CID），点击 **Fetch**。
2.  **加载**:
    *   如果是纯文本页面，瞬间显示。
    *   如果是富媒体页面，你会看到日志滚动 `Req Resource...`，图片随后自动加载。
    *   如果是大文件，你会看到 `Received Chunk X/100`，随后 `Reassembly Complete`。
3.  **交互**:
    *   点击按钮体验无代码逻辑（如计数器增加、开关切换）。
    *   这些状态仅存在于当前会话内存中，重启节点会重置。

---

## 6. 高级特性与安全 (Advanced & Security)

### 6.1 UDP 分片传输 (Fragmentation)
Project Zero 突破了 UDP 的 MTU 限制（通常 <1500 字节）。
*   **机制**: 大文件会被自动切割成 4KB 的小块 (Chunks)。
*   **现象**: 当传输大图时，日志会显示大量的分片接收记录。
*   **可靠性**: 接收端包含重组缓冲区 (Assembler)，只有收齐所有碎片才会还原文件。

### 6.2 自我修复机制 (Self-Healing)
系统假设磁盘是不可信的。
*   **完整性校验**: 每次从磁盘读取文件（Fetch）时，系统都会重新计算哈希。
*   **篡改测试**:
    1.  关闭 Node A。
    2.  手动修改 Node B `./storage_b/objects/` 下的某个图片文件（破坏其内容）。
    3.  启动 Node B 并 Fetch 该页面。
    4.  **结果**: 界面显示红色警告 `⚠️ SECURITY ALERT: Data Corruption`，并拒绝显示坏图。
    5.  **修复**: 启动 Node A，Node B 会自动向 A 请求正确的数据副本，并覆盖坏文件，图片恢复显示。

### 6.3 存储结构 (Storage Layout)
为了优化性能，ZeroStore 采用了前缀分片策略：

*   **物理层 (`/objects/`)**:
    *   路径: `./storage_x/objects/ab/abcdef123...`
    *   说明: 文件名即哈希。前两位字符 (`ab`) 为子目录，防止单目录文件过多。
*   **逻辑层 (`manifest.log`)**:
    *   说明: 人类可读的日志文件，记录了何时(`Time`)、以何种操作(`NEW/DUP`)、存入了哪个 CID。

---

> **Project Zero** - *Trust Math, Not Servers.*
```