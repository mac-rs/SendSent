# 设计文档:文件传输 v2 速度层

- 日期:2026-06-26
- 项目:SendSent(Tauri 2 + React 19 + TS)
- 范围:**v2 速度层**。v1 已打通管线并能量速度(见 `2026-06-25-high-speed-file-transfer-design.md`);v2 把"打满带宽"砸实。
- 关系:本设计是 v1 的增量,不改线协议、不改前后端契约骨架。v3(加密/移动发送/续传)与体验完善为后续独立子项目。

---

## 1. 目标与约束

### 1.1 目标
让 SendSent 在局域网内**真正饱和链路**:两台桌面 peer 在千兆 LAN 上达到 **≥900 Mbps**,并用量化基准证明零拷贝路径生效。

### 1.2 已确认约束(brainstorming 结论)
| # | 约束 | 取值 |
|---|------|------|
| 1 | 零拷贝平台覆盖 | **macOS `sendfile(2)` 优先**;Linux/Windows 本期用 `pread`+write 通用回退(原生 splice/io_uring、TransmitFile 后续加) |
| 2 | 并发模型 | **固定连接池**,默认 4(可配);大文件按 offset 区间拆、小文件轮询 |
| 3 | 成功标准 | 千兆 LAN **≥900 Mbps**;loopback 基准验证零拷贝路径通 + 量化提升;UI 实时速度读数(已有) |

### 1.3 关键原则:无协议改动
Hello/控制帧/数据帧/DataOpen 全部不变(数据帧本就带 `offset`,多连接天然兼容)。v2 纯属**收发两端内部升级**:
- 发送端:开 N 条数据连接 + 零拷贝推流;
- 接收端:每会话并发 drain N 条 + 复用文件句柄。

> 不兼容点:v2 发送端会开多条数据连接,而 v1 接收端每会话只收 1 条(单 `oneshot`),故 **v2 发送端 → v1 接收端不兼容**。全员升级即可;接收端对旧发送端仍兼容(只来 1 条照样能收)。

---

## 2. 架构总览

### 2.1 新增/改动模块
| 文件 | 变化 | 职责 |
|------|------|------|
| `transfer/zerocopy.rs` | **新增** | `send_payload(socket, file, offset, len)`:macOS 走 `sendfile`;其它平台走 `pread`+write 回退 |
| `transfer/sender.rs` | 改 | 开 N 条数据连接;按策略分发 chunk;载荷走 `zerocopy` |
| `transfer/receiver.rs` | 改 | 数据通道 `oneshot`→`mpsc<TcpStream>`;每流 spawn 并发 drain;复用 .part 句柄 |
| `transfer/manager.rs` | 改 | 每会话数据连接改为 `mpsc::Sender<TcpStream>`(池),DataOpen 把流投入 |
| `proto/messages.rs` | 不变 | — |
| `lib.rs` | 微调 | 读 `SENDSENT_CONNS`(默认 4)放进 `AppState` |
| `store.rs` | 改 | 加 `TransferConfig { conns, chunk_size, split_threshold }` + `load_or_create`(<app_data_dir>/transfer.json) |

### 2.2 可配置项(`TransferConfig`)
这三个调参不硬编码,做成持久化配置,启动加载进 `AppState`,环境变量可覆盖(便于测试/压榨):

| 字段 | 含义 | 默认 | 约束 | 环境变量覆盖 |
|------|------|------|------|--------------|
| `conns` | 每会话数据连接数 | `4` | 1–16 | `SENDSENT_CONNS` |
| `chunk_size` | 数据块大小 | `1 MiB` | ≤ `MAX_DATA_PAYLOAD`(1 MiB) | `SENDSENT_CHUNK_KB`(KB 为单位) |
| `split_threshold` | 大文件按区间拆的阈值;小于此则按文件轮询 | `4 MiB` | ≥ `chunk_size` | `SENDSENT_SPLIT_MB`(MB 为单位) |

- 持久化:`<app_data_dir>/transfer.json`(与 `identity.json` 同目录),启动时 `load_or_create`;解析失败回退默认。
- 加载顺序:文件默认值 ← 文件值 ← 环境变量覆盖。
- `TransferConfig` 经 `AppState` 传入 sender;UI 设置页(可视化编辑)归入后续"体验完善"子项目,不在 v2。

### 2.3 数据流(v2)
```
发送端                              接收端
 建立 N 条数据连接(各自 DataOpen)  → 监听 accept,每条 DataOpen 投入该 session 的 mpsc 池
 大文件按 offset 区间拆到 N 条       → run_receiver:对池里每条流 spawn drain 任务
 小文件轮询分配到 N 条               → 并发写同一组 .part 文件(offset 安全),共享 total_done
 每 chunk:write 帧头(29B)+          → 完成判定不变:total_done==total_size 且各文件齐全
   send_payload(载荷,零拷贝)
```

---

## 3. 零拷贝设计(发送端)

### 3.1 抽象
`transfer/zerocopy.rs`:
```rust
/// 把 file 中 [offset, offset+len) 的字节推到 socket。
/// macOS:libc::sendfile(内核直推,不经用户态);其它平台:pread 到小缓冲 + write。
pub async fn send_payload(socket: &TcpStream, file: &std::fs::File, offset: u64, len: usize) -> std::io::Result<()>;
```

### 3.2 macOS 实现
- 用 `libc::sendfile(fd, s, offset, len, header, trailer, flags)`。
- `fd`=文件、`s`=socket 底层 fd、`offset` 指定读取起点(不改文件自身 offset,天然支持并发区间)、`len`=本块字节数、`header/trailer`=NULL。
- sendfile 可能只传一部分(len 是值-结果),需循环直到传完 `len`。
- **不与 TLS 叠加**:v2 明文,sendfile 直推成立;v3 上 TLS 后此路径需改回加密 write(届时再加 TLS-aware 路径)。

### 3.3 通用回退(Linux/Windows 及测试)
- `pread` 读到 ~64KiB 栈/堆缓冲,`write` 到 socket,循环到 `len`。
- 仍享受多连接 + socket 调参收益;原生 splice/io_uring、TransmitFile 留待后续按平台加(届时只改本文件)。

### 3.4 帧头与块大小
- 每个 chunk:**先 `write_data_header(socket, file_id, offset, len)`(29B 普通写),再 `send_payload(...)`**。
- 默认块大小 **256KiB → 1MiB**(取自 `TransferConfig.chunk_size`,默认 1MiB),受 `MAX_DATA_PAYLOAD`(1MiB)上限制约,减少每块帧头/系统调用开销。

---

## 4. 多连接设计

### 4.1 发送端
- 控制连接握手、Manifest、等 Accept 不变。
- Accept 后:**开 `conns` 条数据连接**(`conns` 来自 `TransferConfig`,默认 4),每条先发 `DataOpen{session_id}`。
- `conns` 由 `TransferConfig` 提供(启动从 transfer.json 读,环境变量 `SENDSENT_CONNS` 可覆盖),经 `AppState` 传入 sender。
- 分发:
  - **大文件**(`size ≥ split_threshold`,默认 4MiB,可配):按 `conns` 把 `[0,size)` 均分为区间,每个区间由一条连接顺序发(每条内部仍按 `chunk_size` 分块)。各区间并行。
  - **小文件**(`size < split_threshold`):按文件轮询(round-robin)分配到各连接。
  - 目录条目:仍只在 Manifest 里建结构,不发数据。
- 全部发完后**所有数据连接 shutdown**,等 `Complete`(不变)。

### 4.2 接收端
- `manager`:`SessionChannels.data_tx` 由 `Option<oneshot::Sender<TcpStream>>` 改为 `mpsc::Sender<TcpStream>`(容量如 `conns`)。每次 `DataOpen` 到来 → `data_tx.send(stream)`,不再 `take()`。
- `run_receiver`:`data_rx` 改为 `mpsc::Receiver<TcpStream>`;**每收到一条流就 `tokio::spawn` 一个 drain 任务**,共享该会话的:
  - `Arc<HashMap<Uuid, (FileMeta, Arc<File>, AtomicU64 received)>>>`(句柄复用;用**位置写** `write_at`,多任务并发安全,无需 Mutex),
  - `Arc<AtomicU64> total_done`,
  - 完成判定(见下)。
- 所有 drain 任务结束(所有流 EOF)+ `total_done==total_size` → finalize → 发 `Complete`(不变)。用一个 `JoinSet`/计数在所有 drain 完成后做最终判定。
- 错误:任一 drain 写失败 → 记录错误,整体判 `Failed` 并清 temp(沿用 v1 的原子落盘 + Drop 清理)。

### 4.3 并发安全
- 多条连接可能并发写**同一文件的不同 offset**(大文件被拆分)。用**位置写**(`std::os::unix::fs::FileExt::write_at` / Windows `seek_write`):无 seek 状态、不同 offset 互不干扰 → 并发安全。
- `Arc<File>` 可被多任务共享(`write_at` 取 `&self`);`received` 用每文件 `AtomicU64`,`total_done` 用 `AtomicU64`。

---

## 5. 接收端写优化(必要)

v1 每 chunk `open+seek+write`(评审 M8),在零拷贝高速下磁盘写必成瓶颈。v2:
- 每个 .part 文件在 drain 开始时**打开一次**,存入共享 `parts` 表(`Arc<File>`),后续 chunk 复用同一句柄。
- 写入用**位置写** `FileExt::write_at(buf, offset)`(Unix)/ `seek_write`(Windows)——无 seek 状态,多连接并发写同一文件不同 offset 安全。
- 大文件预先 `set_len` 稀疏分配(已有);1MiB 块直接写,不加 `BufWriter`(避免与位置写/并发冲突)。
- 目录项仍 `create_dir_all` 建空目录。

---

## 6. socket 调参

发送端每条 TCP 连接(控制 + N 条数据):
- `SO_SNDBUF` / `SO_RCVBUF` 调到 **4–8 MiB**(用 `socket2` 设置;已在依赖树中)。
- `TCP_NODELAY` 置位(控制帧不被 nagel 延迟)。
- 集中在一个 `tune_socket(&TcpStream)` 辅助函数,发送端 connect 后、接收端 accept 后各调一次。

> 新增依赖:`socket2`(如未在树中)。

---

## 7. 基准与验收

### 7.1 loopback 集成测试(`tests/loopback.rs` 扩展)
- 新增用例:生成 ~500MB 临时文件,通过**真实 TCP 路径 + N 连接 + 零拷贝**发送,计时,断言:
  - 文件字节完整(`total_done==size`);
  - loopback 吞吐 ≥ 保守下限(如 **2 Gbps**),证明零拷贝/多连接路径生效(loopback 远高于千兆)。
- 复用现有 `run_test_server` + 自动接受。

### 7.2 真实 LAN 验收
- 由开发者用两台物理桌面 peer 在千兆 LAN 验证 ≥900 Mbps(UI `speed_bps` 读数)。模拟器/同机不算(共享网络栈)。

### 7.3 回归
- 既有 18 个测试保持全绿(含 v1 的回环端到端、截断回归)。
- `cargo clippy --all-targets -- -D warnings` 保持干净。

---

## 8. 范围边界(v2 不做)

- Linux `splice`/`io_uring`、Windows `TransmitFile`(后续按平台加,仅改 `zerocopy.rs`)。
- 加密 / PIN 配对(v3);移动端发送 / mDNS fallback / 断线续传(v3)。
- 不改线协议(与 v1 帧级兼容;v2 发送端→v1 接收端不兼容,见 1.3)。
- sha256 校验、群发、文本传输、UI 打磨(更后)。

---

## 9. v2 完成定义(acceptance)
1. macOS 上发送端经 `sendfile` 推流(代码 + 日志可证);其它平台走通用回退;
2. 两端默认 4 条数据连接,大文件按区间拆、小文件轮询;
3. loopback 基准测试通过且吞吐达标;
4. 接收端 .part 句柄复用,无每 chunk 重开;
5. socket buffer 调大、NODELAY 置位;
6. 既有测试全绿、clippy 干净;
7. (开发者)两台物理机千兆 LAN 实测 ≥900 Mbps。
