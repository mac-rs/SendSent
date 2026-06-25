# 设计文档:跨平台高速文件传输(v1)

- 日期:2026-06-25
- 项目:SendSent(Tauri 2 + React 19 + TypeScript 桌面/移动应用)
- 范围:**v1**(打通管线 + 可度量吞吐)。v2(速度/零拷贝)、v3(移动/安全)为后续子项目,本文仅在边界处指明留口。
- 状态:已通过 brainstorming 五段评审,待用户确认后进入实现计划(writing-plans)。

---

## 1. 目标与约束

### 1.1 目标
做一个 LocalSend 式的局域网文件传输应用,**强调高速**:同一局域网内自动发现设备、零配置互传文件,并追求打满网卡带宽的极限吞吐。

### 1.2 已确认约束(来自需求澄清)
| # | 约束 | 对设计的影响 |
|---|------|--------------|
| 1 | 独立自有协议,不兼容 LocalSend | 架构自由,无 REST/端口兼容包袱 |
| 2 | 仅局域网(LAN) | mDNS 发现 + 直连,无后端服务 |
| 3 | 桌面(macOS/Win/Linux)+ 移动(iOS/Android) | mDNS 库须纯 Rust;移动多播/后台受限 |
| 4 | 混合负载:大文件 + 海量小文件都要快 | 传输层须兼顾顺序吞吐与并发多流 |
| 5 | 默认明文,可选加密 | 偏向裸 TCP(QUIC 强制 TLS 1.3、无法关闭) |
| 6 | 成功标准=打满带宽(千兆 ≥900Mbps,万兆近线速) | v2 须上 OS 级零拷贝;v1 先打通并能量速度 |

### 1.3 关键技术洞察(决定选型)
> 内核零拷贝(`sendfile`/`splice`/`io_uring`/`TransmitFile`)**只能作用于 TCP socket**:QUIC 的加密与可靠性跑在用户态,无法用内核 `sendfile` 把文件直推网卡。加上"默认明文"(QUIC 关不掉 TLS),两条硬约束同时指向**裸 TCP**,而非 QUIC。

---

## 2. 传输层方案对比与选型

| 方案 | 要点 | 与约束契合度 |
|------|------|--------------|
| **A. 裸多路复用 TCP + 零拷贝**(选用) | 自定义二进制帧;1 控制连接 + N 数据连接;大文件走内核零拷贝;可选 TLS | ✅ 明文 ✅ 零拷贝打满带宽 ✅ 大小文件皆可控;代价:帧/流控/多路复用自写,零拷贝按平台写胶水 |
| B. QUIC(quinn/quiche) | 库自带多流+TLS+0-RTT | ❌ 强制加密(冲突约束5)❌ 无法 sendfile(冲突约束6);UDP 在移动端可能受限 |
| C. 优化版 HTTP/2 over TCP | 多路复用+连接复用+下载端 sendfile | 速度天花板在 A 之下;请求/响应模型与双向高吞吐相悖 |

**决定:采用方案 A。** 它是唯一同时兑现"默认明文"与"零拷贝打满带宽"的选项。

---

## 3. 架构与模块边界

### 3.1 对等模型
每个节点既是服务端又是客户端:启动即监听一个 TCP 端口(默认 `52225`,可配置),并用 mDNS 广播自己的 `IP:port` + 设备信息;同时浏览局域网内其他节点。任意一方可发起会话。

### 3.2 依赖选型
| 用途 | 选型 | 理由 |
|------|------|------|
| 异步运行时 | `tokio` | Tauri 2 已用 |
| mDNS | `mdns-sd`(纯 Rust) | 无原生依赖,移动端可用;`zeroconf` 绑 Avahi/Bonjour 在移动端不可用 |
| 序列化 | `serde` + `bincode` | 帧紧凑;控制信令二进制,数据走裸字节 |
| ID | `uuid` | session/文件 ID |
| 文件/文件夹选择器 | `tauri-plugin-dialog`(新增) | 前端取绝对路径后传给 `send_files` |

### 3.3 Rust 模块布局(`src-tauri/src/`)
```
main.rs            # 入口(基本不动)
lib.rs             # run():注册插件/命令,启动后台服务(discovery + transfer manager)
discovery/         # mDNS 注册 + 扫描;暴露 Discovery trait(v3 可换实现)
proto/             # 帧定义 + (de)serialize + 魔数/版本
transfer/          # 会话管理器 / 发送端 / 接收端 / 状态机 / 临时落盘
store.rs           # 设备身份持久化、下载目录、设置
commands.rs        # #[tauri::command] 薄封装
events.rs          # 向前端 emit 的事件载荷 + 错误码枚举
```

### 3.4 Rust ↔ TypeScript 边界
- **所有网络 IO、文件 IO、协议解析在 Rust**;React 不碰 socket/文件。
- 前端→后端:`invoke()` 命令。
- 后端→前端:`emit()` 事件(异步更新:发现/进度)。
- 前端结构:`src/lib/invoke.ts`(类型化封装)、`src/lib/events.ts`(类型化监听)、`src/hooks/`(`usePeers`、`useTransfer`)、`src/components/`、`App.tsx`(只渲染状态)。

---

## 4. 发现协议(mDNS)

### 4.1 服务注册
- 服务类型:`_sendsent._tcp.local.`(不复用 `_localsend`)。
- 实例名:设备显示名(默认主机名,可改)。重名时 UI 用平台/后缀消歧,**身份认 `device_id`**。
- 启动时:注册自身服务 + 开始浏览。

### 4.2 TXT 记录(全字符串值)
| 字段 | 含义 | 示例 |
|------|------|------|
| `v` | 协议主版本(v1=1) | `1` |
| `id` | 设备稳定 ID(UUIDv4,持久化) | `a1b2…` |
| `name` | 显示名 | `mankong-mac` |
| `plat` | 平台 | `macos\|windows\|linux\|ios\|android` |
| `port` | 监听端口 | `52225` |

`device_id` 为唯一主键;多网卡 resolve 出多地址全部进 `addrs`,传输时逐个连到 `port`。

### 4.3 Peer 模型(Rust/前端共享语义)
```rust
struct Peer {
    device_id: String,
    name: String,
    platform: Platform,        // enum
    proto_version: u16,
    addrs: Vec<SocketAddr>,
    port: u16,
    last_seen: Instant,        // 老化依据
}
```

### 4.4 生命周期
1. `mdns-sd` 回调 add → resolve 地址 + 解析 TXT → 校验 `v` → 按 `device_id` 去重 upsert → emit `peer://found`。
2. 同 `device_id` 再现只更新 `addrs`/`last_seen`,不重复 emit。
3. remove 回调 → emit `peer://lost`。
4. 老化兜底:60s 定时任务清 `now-last_seen>90s` 的节点并 emit lost(mDNS goodbye 不保证)。

### 4.5 设备身份持久化
首次运行生成 UUIDv4,写入 App 数据目录(`store.rs`,经 Tauri `app_data_dir`)的 `identity.json`。

### 4.6 为 v3 留口
`discovery/` 暴露 `Discovery` trait(`start`/`peers`/事件流)。mDNS 是其一个实现;v3 移动多播不可用时新增"主动探测子网/手动输 IP"实现,上层无感。

---

## 5. 传输线协议

### 5.1 连接模型
每会话**两条 TCP 连接**,发起方(发送端)均为 client、接收端为 server:
- **控制连接**(持久):握手、清单、接受/拒绝、进度、取消。独立 TCP 流,不被数据洪峰阻塞。
- **数据连接**(按需):仅 `Accept` 后才建立,承载文件字节。**v1 用 1 条;v2 扩到 N 条,帧格式不变。**

接收端只开一个监听口(52225),`accept` 后读第一帧按类型分流(见 5.4)。

### 5.2 控制帧布局
```
 0      1      2               7                  7+LEN
+------+------+------+----+----+---------------------+
| MAGIC| VER  | TYPE |     LEN (u32 BE)      |   PAYLOAD[LEN]  |
| 0x53 | 0x01 | u8   |                        | bincode(message)|
+------+------+------+------------------------+-----------------+
```
- `MAGIC=0x53('S')`:帧同步;**收到错 MAGIC → 协议错误,断开**。
- `VER=0x01`:每帧带主版本。
- `LEN`:u32 BE,payload 字节数,控制帧上限 **4 MiB**;超限 → 协议错误。

控制消息类型:
| TYPE | 消息 | 关键字段 |
|------|------|----------|
| 0x01 | `Hello` | 发送方 device_id/name/platform、**session_id(新 UUID)** |
| 0x02 | `HelloAck` | 接收方 device_id/name |
| 0x03 | `Manifest` | files[]、total_size、total_count |
| 0x04 | `Accept` | save_dir 确认 |
| 0x05 | `Reject` | reason |
| 0x06 | `Progress` | file_id、bytes_done |
| 0x07 | `Complete` | 全部收妥 |
| 0x08 | `Error` | code + message |
| 0x09 | `Cancel` | 一方中止 |
| 0x0A | `DataOpen` | **session_id**(数据连接凭证) |

`Manifest` 的 `FileMeta`:
```rust
struct FileMeta {
    id: Uuid,             // 发送端生成
    name: String,         // 基名
    rel_path: String,     // 批次内相对路径(文件夹结构);单文件为 ""
    size: u64,
    kind: FileKind,       // File | Dir
    hash: Option<String>, // sha256 hex,v1 默认 None
}
struct Manifest {
    session_id: Uuid,
    files: Vec<FileMeta>,
    total_size: u64,
    total_count: u64,
}
```

### 5.3 数据帧布局
```
 0       1               17              25              29           29+LEN
+--------+-----------------+---------------+---------------+-------------+
| TAG    |   FILE_ID(16B)  | OFFSET(u64BE) | LEN(u32 BE)   | DATA[LEN]   |
| 0xD5   |   UUID bytes    | 文件内偏移     | ≤1MiB         | 裸字节       |
+--------+-----------------+---------------+---------------+-------------+
```
- `TAG=0xD5`:数据帧标记。
- 显式 `OFFSET` → 落盘直接 seek,天然支持续传(v3)与并行分块(v2),帧格式不变。
- 默认块大小 **256 KiB**(可调)。

### 5.4 数据连接归属会话
发送端 `Accept` 后才开数据连接,**第一帧先发控制帧 `DataOpen{session_id}`**,随后切换为数据帧流。接收端 `accept` 后读第一帧:
- `Hello` → 新建会话,此连接为控制通道;
- `DataOpen` → 按 session_id 绑定为该会话数据通道,随后按数据帧格式读。

### 5.5 握手时序
```
发送端                                         接收端
  │  ── connect 控制连接 ────────────────────►  │ accept
  │  ── Hello{device_id, session_id, ...} ──►  │ 校验 VER/MAGIC
  │  ◄────────────────── HelloAck ──────────   │
  │  ── Manifest{files[], total} ──────────►   │
  │                                            │  (UI 提示,等用户决策)
  │  ◄──────────────── Accept / Reject ─────   │
  │  ── connect 数据连接 ───────────────────►  │ accept → 读 DataOpen
  │  ══ 数据帧流(逐文件/分块) ═════════════►   │ 落盘(按 OFFSET seek)
  │  ◄──── Progress(周期) / 我方也发 ───────    │
  │  ◄──────────────── Complete ────────────   │ 全部写完
  │  ── 关闭 ──────────────────────────────►   │
```

### 5.6 版本与完整性
- **版本协商**:TXT `v` + 控制帧 `VER` 双重校验;不一致 → `Error{IncompatibleVersion}` + 断开。v1 不做部分兼容。
- **完整性**:TCP 保证有序不损坏,v1 默认不做校验和(避免对大文件预哈希、与极速冲突)。`FileMeta.hash` 为 `Option`,留口子;"验证模式"开启时发送端流式算 sha256、接收端 Complete 前校验。
- 任何 `LEN 超限 / MAGIC 错 / payload 反序列化失败` → 发 `Error` 帧 + 断连,后端再 emit `transfer://finished { state: failed, error }`(无独立 `transfer://error` 通道,所有终态统一走 `finished`)。

---

## 6. 会话状态机与错误处理

### 6.1 落盘策略(原子化)
chunks 先写 `<save_dir>/.sendsent-tmp/<session>/<file>.part`,**只在 `Complete` 时 rename 到最终路径**(命名冲突自动追加 ` (1)`…)。任何非正常结束 → 删除该 session 的 temp 目录。用户真实下载目录永不出现半个文件。

### 6.2 状态机(角色 S=发送 / R=接收)
```
Init ──connect+Hello──► Handshaking ──HelloAck──► AwaitingDecision
                                              (S:已发Manifest / R:已收Manifest,UI待决)
                                              │
                            ┌─────────────────┼─────────────────┐
                  Reject◄────┘                 ▼                 └──►(Cancelled)
                                            Accepted
                                              │
                                       开数据连接 + 流式分块
                                              ▼
                                         Transferring ──周期Progress──► (前端算速)
                                              │
                                        全部写完/校验
                                              ▼
                                            Completed
任意活跃态 ──Cancel帧──► Cancelled      任意态 ──错误/断连──► Failed
```

### 6.3 错误码(`events.rs`)
`IncompatibleVersion` · `ManifestTooLarge`(超 4MiB cap / 文件数上限)· `ConnectionLost` · `DiskFull` · `WriteFailed` · `ProtocolError` · `Timeout` · `Cancelled` · `Internal`

### 6.4 超时与清理
- 握手超时 10s、传输空闲超时 60s → `Timeout`→`Failed`。
- Accept 等待不设硬超时(用户可能慢);靠断连回收 session。
- **断连不自动续传**(v1),按失败处理并清 temp;续传是 v3。
- Cancel:任一方发 `Cancel` → 对方进 Cancelled → 双方清 temp。

### 6.5 吞吐度量
- 接收端周期(每 500ms 或每 4MiB)发 `Progress{file_id, bytes_done}`。
- 后端聚合成 `transfer://progress`,**节流到 ~8Hz**;`speed_bps` 在后端按滑动窗口算,前端只显示。

---

## 7. Tauri 契约(命令与事件)

### 7.1 命令面(前端 `invoke`,定义于 `commands.rs`,注册进 `lib.rs` 的 `generate_handler!`)
| 命令 | 入参 | 返回 |
|------|------|------|
| `get_identity` | — | `{device_id, name, platform}` |
| `set_display_name` | `name` | — |
| `list_peers` | — | `Vec<Peer>` |
| `send_files` | `peer_device_id, files: Vec<绝对路径>` | `session_id` |
| `respond` | `session_id, accept: bool, save_dir?` | — |
| `cancel` | `session_id` | — |
| `get_default_save_dir` | — | 路径(默认系统 Downloads) |

### 7.2 事件面(后端 `emit`,前端 `listen`)
- `peer://found { peer }` / `peer://lost { device_id }`
- `transfer://request { session_id, sender, manifest }` ← 入站请求,前端弹接受框
- `transfer://progress { session_id, state, bytes_done, bytes_total, files_done, files_total, speed_bps }`(`state ∈ connecting|awaiting_accept|transferring|finalizing`)
- `transfer://finished { session_id, state ∈ completed|rejected|cancelled|failed, error? }`

### 7.3 权限(v1 必做)
- 新增 `tauri-plugin-dialog`,并在 `src-tauri/capabilities/default.json` 加 `dialog:default`(**否则运行时静默失败**)。
- 文件读写走 Rust 标准库,**不**引入 `tauri-plugin-fs`。

---

## 8. 测试策略

仓库当前无测试 runner。v1 可测逻辑几乎都在 Rust:

- **Rust 单测(`cargo test`,零额外配置)**:帧编解码 round-trip、MAGIC/VER 校验、数据帧解析、Manifest 序列化、offset→文件重组、命名冲突消解、Progress 节流/速度滑动窗口、Peer 去重与老化。设计为纯函数。
- **回环集成测试(最高价值)**:进程内起两节点,绑 `127.0.0.1` 临时端口,**跳过 mDNS**(借 `Discovery` trait 注入假 Peer),走**真实 TCP 路径**完整收发一批文件 → 证明整条管线(含线协议)成立。纳入 `cargo test`。
- **前端**:v1 暂不引入 test runner,靠严格 `tsc` 类型把关 + 手测;`pnpm build` 通过即门槛。

### 校验命令
| 目的 | 命令 |
|------|------|
| Rust 测试 | `cargo test`(于 `src-tauri/`) |
| Rust 编译/lint | `cargo check`、`cargo clippy`(于 `src-tauri/`) |
| 前端类型/构建 | `pnpm build`(`tsc && vite build`,唯一 typecheck) |
| 手测 | `pnpm tauri dev`,双实例/双机同 LAN 发文件看速度 |

---

## 9. 范围边界(YAGNI)

### 9.1 v1 明确不做
- **v2 项**:零拷贝(`sendfile`/`splice`/`io_uring`/`TransmitFile`)、多数据连接/并行分块、`SO_SNDBUF`/背压等调参。
- **v3 项**:iOS/Android 构建、mDNS 不可用时的发现 fallback、TLS 加密 + PIN 配对、断线自动续传。
- **更后**:默认 sha256 校验(v1 可选、默认关)、群发/一对多、文本/剪贴板、UI 打磨。
- **永不做**:LocalSend 互通(已定为独立协议)。

### 9.2 v1 完成定义(acceptance)
1. 同 LAN 两台桌面实例自动互相发现;
2. 发一个**混合大小的文件夹**,接收端弹窗→接受→文件原子落入 Downloads,进度 + 实时速度正常;
3. Reject / Cancel 生效,temp 被清;
4. 传输中断连 → `Failed` 且清 temp;
5. `cargo test`(含回环集成)通过,吞吐数值如实显示。

---

## 10. 备注

- 仓库无 git(非 git repo),本 spec 暂以文件形式提交,未做 `git commit`。后续若初始化 git,建议将 `docs/` 纳入版本管理。
- 本 spec 通过后,下一步进入 writing-plans,产出 v1 的分步实现计划。
