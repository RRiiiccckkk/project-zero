用户使用手册 (USER_GUIDE.md)
code
Markdown
# Project Zero User Guide / 用户使用守则

## 1. 简介 (Introduction)
Project Zero 是一个去中心化的 P2P 原型网络，实现了底层 UDP 通信、加密、DHT 路由、分布式存储以及一个基于文本终端 (TUI) 的去中心化浏览器。本手册将指导你如何操作节点。

Project Zero is a decentralized P2P prototype network featuring UDP communication, encryption, DHT routing, distributed storage, and a terminal-based decentralized browser. This guide explains how to operate a node.

---

## 2. 启动与组网 (Startup & Networking)

### 启动节点 (Start Node)
默认情况下，节点会绑定到随机端口。
By default, the node binds to a random port.

```bash
# 普通启动 / Normal Start
cargo run

# 指定端口启动 (例如 Bootnode) / Bind specific port
cargo run -- --bind 127.0.0.1:9999
加入网络 (Bootstrap)
新节点启动后是孤立的，必须连接到一个已知节点（Bootnode）才能加入 DHT 网络。
A new node is isolated. You must connect to a known node (Bootnode) to join the DHT.

code
Bash
# 语法 / Syntax: bootstrap <ip:port>
> bootstrap 127.0.0.1:9999
查看邻居 (View Peers)
查看当前路由表（K-Buckets）中已发现的节点。
View discovered nodes in your routing table.

code
Bash
> peers
查看自身 ID (View Self ID)
显示当前节点的公钥 ID（32字节 Hex）。
Show current node's public key ID.

code
Bash
> id
3. 社交与通信 (Social & Messaging)

查找节点 (Find Node)
在网络中定位某个节点 ID。如果目标在 NAT 后面，系统会尝试进行打洞 (Hole Punching)。
Locate a node ID in the network. If behind NAT, hole punching is attempted.

code
Bash
# 语法 / Syntax: find <NodeID_Hex>
> find 56f9605182337d69...
发送私信 (Direct Message)
向指定 ID 发送端到端加密消息。
Send E2E encrypted message to a specific ID.

code
Bash
# 语法 / Syntax: msg <NodeID_Hex> <Message>
> msg 56f9605182337d69... Hello World!
4. 分布式存储与 Web 3.0 (Storage & Web 3.0)
这是 Project Zero 的核心功能：去中心化内容发布与浏览。
Core feature: Decentralized content publishing and browsing.

发布网页 (Publish Page)
将内置的 SafePage (JSON 格式) 发布到 DHT 网络中。发布成功后，网络会返回一个 Content Key。
Publish the built-in SafePage to the DHT. Upon success, the network returns a Content Key.

code
Bash
> publish

# 输出示例 / Output Example:
# Publishing Page...
# Page Key: a0e9776a0bdd1f31dfd4d4d998bfe740501d7d4c1d00f976504565eee33d4ae0
⚠️ 注意/Note: 请务必保存返回的 Page Key，这是访问该网页的唯一凭证。

浏览网页 (Browse Page)
使用 Key 从网络下载数据，并渲染为可视化页面。
Download and render the page using its Key.

code
Bash
# 语法 / Syntax: browse <Content_Key>
> browse a0e9776a0bdd1f31dfd4d4d998bfe740501d7d4c1d00f976504565eee33d4ae0
原始数据存取 (Raw Data I/O)
如果你只想存储简单的字符串数据：
For storing simple raw strings:

code
Bash
# 存储 / Put
> put MySecretData
# (返回 Key / Returns Key)

# 获取 / Get
> get <Key>
5. 常见问题 (FAQ)
Q: 为什么 bootstrap 后 peers 还是空的？
A: bootstrap 发送的是 UDP 包。如果 Bootnode 未运行或防火墙拦截，将无响应。请确认 IP 和端口正确。

Q: 什么是 NAT 穿透？
A: 当两个节点都在家用路由器后面时，直接通信会被拦截。Project Zero 使用“打洞”技术，通过 Bootnode 协调，让双方路由器允许通信。

Q: 数据存在哪里？
A: 数据存储在距离 Key 最近的 K 个节点的内存中 (HashMap)。当前版本重启节点会导致数据丢失（纯内存模式）。