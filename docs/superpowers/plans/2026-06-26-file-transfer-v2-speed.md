# SendSent v2 速度层 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 v1 的单连接、用户态读写升级为「多数据连接 + macOS `sendfile` 零拷贝 + socket 调参」,在千兆 LAN 达到 ≥900 Mbps,并用 loopback 基准量化。

**Architecture:** 无线协议改动。发送端开 N 条数据连接(默认 4,可配)、大文件按 offset 区间拆 / 小文件轮询、载荷走 `sendfile`(其它平台 `pread`+write 回退);接收端每会话并发 drain 多条流、用位置写 `write_at` 复用句柄;`total_done==total` 判完成。三可调项(`conns`/`chunk_size`/`split_threshold`)持久化为 `TransferConfig`。

**Tech Stack:** Rust + tokio + `libc`(sendfile)+ `socket2`(buffer 调参);`std::os::{unix,windows}::fs::FileExt` 位置写。

**Spec:** `docs/superpowers/specs/2026-06-26-file-transfer-v2-speed-design.md`

> 全程命令基准:Rust 在 `src-tauri/` 跑 `cargo test` / `cargo check` / `cargo clippy --all-targets -- -D warnings`;前端 `pnpm build`;桌面联调 `pnpm tauri dev`。包管理器 **pnpm**。模块为 `pub mod`,集成测试在 `src-tauri/tests/loopback.rs`。

---

## 文件结构

| 文件 | 变化 | 职责 |
|------|------|------|
| `src-tauri/Cargo.toml` | 改 | 加 `libc`、`socket2` |
| `src-tauri/src/store.rs` | 改 | `TransferConfig { conns, chunk_size, split_threshold }` + `load_or_create_transfer_config` + 环境变量覆盖 + 测试 |
| `src-tauri/src/state.rs` | 改 | `AppState` 加 `transfer_config: TransferConfig` |
| `src-tauri/src/lib.rs` | 改 | 启动加载 `TransferConfig` 进 `AppState` |
| `src-tauri/src/proto/frame.rs` | 改 | 加 `write_data_header`(只写 29B 帧头) |
| `src-tauri/src/transfer/zerocopy.rs` | **新增** | `send_payload(socket, file, offset, len)`:macOS sendfile / 通用回退 + 测试 |
| `src-tauri/src/transfer/sock.rs` | **新增** | `tune_socket(&TcpStream)`(buffer + nodelay) |
| `src-tauri/src/transfer/mod.rs` | 改 | `pub mod zerocopy; pub mod sock;` |
| `src-tauri/src/transfer/sender.rs` | 改 | 多连接 + 区间拆分 + 零拷贝发送 |
| `src-tauri/src/transfer/receiver.rs` | 改 | 多流并发 drain + `write_at` + Notify 完成判定 |
| `src-tauri/src/transfer/manager.rs` | 改 | 数据通道 `oneshot`→`mpsc`;`start_send` 传 config |
| `src-tauri/src/commands.rs` | 改 | `send_files` 传 `state.transfer_config` |
| `src-tauri/tests/loopback.rs` | 改 | 大文件基准 + 完整性测试 |

---

## Task 1: 依赖 + TransferConfig

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/store.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 加依赖**

在 `src-tauri/Cargo.toml` `[dependencies]` 末尾追加:
```toml
libc = "0.2"
socket2 = "0.5"
```

- [ ] **Step 2: 写失败测试(store.rs 顶部追加 TransferConfig + 测试)**

在 `src-tauri/src/store.rs` 末尾(`load_or_create` 函数之后、`#[cfg(test)]` 之前)追加:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferConfig {
    pub conns: u32,
    pub chunk_size: u64,   // bytes
    pub split_threshold: u64, // bytes
}

impl TransferConfig {
    pub const DEFAULT_CONNS: u32 = 4;
    pub const DEFAULT_CHUNK: u64 = 1024 * 1024;       // 1 MiB (== MAX_DATA_PAYLOAD)
    pub const DEFAULT_SPLIT: u64 = 4 * 1024 * 1024;   // 4 MiB

    pub fn defaults() -> Self {
        Self { conns: Self::DEFAULT_CONNS, chunk_size: Self::DEFAULT_CHUNK, split_threshold: Self::DEFAULT_SPLIT }
    }

    /// clamp 到合法区间,避免恶意/手抖配置
    pub fn sanitized(mut self) -> Self {
        self.conns = self.conns.clamp(1, 16);
        self.chunk_size = self.chunk_size.clamp(64 * 1024, 1024 * 1024); // ≤ MAX_DATA_PAYLOAD
        self.split_threshold = self.split_threshold.max(self.chunk_size);
        self
    }
}

pub fn load_or_create_transfer_config(data_dir: &std::path::Path) -> TransferConfig {
    let p = data_dir.join("transfer.json");
    let mut cfg = match (|| -> anyhow::Result<TransferConfig> {
        if p.exists() {
            let s = std::fs::read_to_string(&p).context("read transfer config")?;
            return Ok(serde_json::from_str(&s).context("parse transfer config")?);
        }
        Ok(TransferConfig::defaults())
    })() {
        Ok(c) => c,
        Err(_) => TransferConfig::defaults(),
    };
    // 环境变量覆盖(便于压测/测试)
    if let Ok(v) = std::env::var("SENDSENT_CONNS") { if let Ok(n) = v.parse() { cfg.conns = n; } }
    if let Ok(v) = std::env::var("SENDSENT_CHUNK_KB") { if let Ok(n) = v.parse() { cfg.chunk_size = n * 1024; } }
    if let Ok(v) = std::env::var("SENDSENT_SPLIT_MB") { if let Ok(n) = v.parse() { cfg.split_threshold = n * 1024 * 1024; } }
    cfg.sanitized()
}
```

在 `src-tauri/src/store.rs` 的 `#[cfg(test)] mod tests` 内追加:
```rust
    #[test]
    fn transfer_config_sanitizes() {
        let c = TransferConfig { conns: 99, chunk_size: 10, split_threshold: 1 }.sanitized();
        assert_eq!(c.conns, 16);
        assert_eq!(c.chunk_size, 64 * 1024);
        assert_eq!(c.split_threshold, c.chunk_size);
    }
    #[test]
    fn transfer_config_loads_defaults_then_file() {
        let tmp = std::env::temp_dir().join(format!("ss-tcfg-{}", uuid::Uuid::new_v4()));
        let c1 = load_or_create_transfer_config(&tmp);
        assert_eq!(c1.conns, TransferConfig::DEFAULT_CONNS);
        let custom = TransferConfig { conns: 2, chunk_size: 256 * 1024, split_threshold: 8 * 1024 * 1024 };
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("transfer.json"), serde_json::to_string(&custom).unwrap()).unwrap();
        let c2 = load_or_create_transfer_config(&tmp);
        assert_eq!(c2.conns, 2);
        assert_eq!(c2.chunk_size, 256 * 1024);
        let _ = std::fs::remove_dir_all(&tmp);
    }
```

- [ ] **Step 3: 运行测试**

```bash
cargo test --lib store
```
Expected: 原 1 个 + 新 2 个 = 3 passed。

- [ ] **Step 4: AppState 加字段(`src-tauri/src/state.rs`)**

在 `AppState` 结构体加字段,并在文件顶部 `use` 加 `TransferConfig`:
```rust
use crate::discovery::Discovery;
use crate::store::{Identity, TransferConfig};
use crate::transfer::manager::SessionManager;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppState {
    pub identity: Identity,
    pub discovery: Arc<dyn Discovery>,
    pub sessions: Arc<SessionManager>,
    pub save_dir: PathBuf,
    pub transfer_config: TransferConfig,
}
```

- [ ] **Step 5: lib.rs 加载 config**

在 `src-tauri/src/lib.rs` 的 `setup` 中,`let save_dir = ...` 之后追加:
```rust
            let transfer_config = store::load_or_create_transfer_config(&data_dir);
            tracing::info!("transfer_config: conns={} chunk={} split={}",
                transfer_config.conns, transfer_config.chunk_size, transfer_config.split_threshold);
```
并把 `app.manage(AppState { ... })` 改为含 `transfer_config`:
```rust
            app.manage(AppState { identity, discovery, sessions, save_dir, transfer_config });
```

- [ ] **Step 6: 校验编译**

```bash
cargo check
```
Expected: 通过(`libc`/`socket2` 已是 transitive,直接依赖解析无新下载或少量下载)。

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat(v2): TransferConfig (conns/chunk/split) persisted + env override"
```

---

## Task 2: write_data_header + 零拷贝模块

**Files:**
- Modify: `src-tauri/src/proto/frame.rs`
- Create: `src-tauri/src/transfer/zerocopy.rs`
- Modify: `src-tauri/src/transfer/mod.rs`

- [ ] **Step 1: proto/frame.rs 加 write_data_header(只写 29B 帧头)**

在 `src-tauri/src/proto/frame.rs` 的 `write_data` 之后追加:
```rust
/// 只写数据帧的 29 字节头(载荷随后由 zerocopy::send_payload 另发,实现零拷贝)。
pub async fn write_data_header<W: AsyncWriteExt + Unpin>(w: &mut W, file_id: Uuid, offset: u64, len: u32) -> io::Result<()> {
    if len > MAX_DATA_PAYLOAD {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "data payload too large"));
    }
    let mut hdr = [0u8; 29];
    hdr[0] = DATA_TAG;
    hdr[1..17].copy_from_slice(file_id.as_bytes());
    hdr[17..25].copy_from_slice(&offset.to_be_bytes());
    hdr[25..29].copy_from_slice(&len.to_be_bytes());
    w.write_all(&hdr).await
}
```

- [ ] **Step 2: transfer/mod.rs 声明新模块**

`src-tauri/src/transfer/mod.rs` 改为(在现有 `pub mod` 列表加两行):
```rust
pub mod meter;
pub mod atomic;
pub mod sender;
pub mod receiver;
pub mod manager;
pub mod zerocopy;
pub mod sock;
```
(若 `sock.rs` 暂未创建会报错,Task 3 创建;此处先加 `pub mod zerocopy;`,Task 3 再加 `pub mod sock;`。本 Task 只加 `pub mod zerocopy;`。)

- [ ] **Step 3: 写 zerocopy.rs(含测试)**

创建 `src-tauri/src/transfer/zerocopy.rs`:
```rust
use std::fs::File;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// 把 file 中 [offset, offset+len) 的字节推到 socket。
/// macOS:libc::sendfile(内核直推);其它平台:pread 到缓冲 + write 回退。
pub async fn send_payload(socket: &TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()> {
    if len == 0 { return Ok(()); }

    #[cfg(target_os = "macos")]
    {
        return sendfile_macos(socket, file, offset, len).await;
    }

    #[cfg(not(target_os = "macos"))]
    {
        return fallback_pread_write(socket, file, offset, len).await;
    }
}

#[cfg(target_os = "macos")]
async fn sendfile_macos(socket: &TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let out_fd = socket.as_raw_fd();
    let in_fd = file.as_raw_fd();
    let mut off = offset as i64;
    let mut remaining = len;
    // macOS sendfile: len 是值-结果,可能只传一部分,循环到 remaining==0
    while remaining > 0 {
        let mut to_send: libc::off_t = remaining as libc::off_t;
        let rc = unsafe {
            libc::sendfile(in_fd, out_fd, off, &mut to_send, std::ptr::null_mut(), 0)
        };
        if rc < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::WouldBlock {
                socket.writable().await?;
                continue;
            }
            return Err(e);
        }
        if to_send == 0 {
            // 对端可能关闭或无进展
            return Err(io::Error::new(io::ErrorKind::WriteZero, "sendfile made no progress"));
        }
        off += to_send;
        remaining -= to_send as usize;
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
async fn fallback_pread_write(socket: &TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    let mut buf = vec![0u8; 64 * 1024];
    let mut off = offset;
    let mut remaining = len;
    while remaining > 0 {
        let n = buf.len().min(remaining);
        let read = file.read_at(&mut buf[..n], off)?;
        if read == 0 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "file short")); }
        socket.write_all(&buf[..read]).await?;
        off += read as u64;
        remaining -= read;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn send_payload_roundtrip() {
        // 回环:写一个临时文件,send_payload 到一条 socket,对端读回比对
        let dir = std::env::temp_dir().join(format!("ss-zc-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.bin");
        let payload: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (mut server, _) = listener.accept().await.unwrap();
        let client = conn.await.unwrap();

        send_payload(&client, &file, 0, payload.len()).await.unwrap();
        client.shutdown().await.unwrap();

        use tokio::io::AsyncReadExt;
        let mut got = Vec::new();
        server.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, payload);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn send_payload_offset_range() {
        // 只发文件 [1000, 3000) 这段
        let dir = std::env::temp_dir().join(format!("ss-zc2-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f2.bin");
        let payload: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (mut server, _) = listener.accept().await.unwrap();
        let client = conn.await.unwrap();

        send_payload(&client, &file, 1000, 2000).await.unwrap();
        client.shutdown().await.unwrap();

        use tokio::io::AsyncReadExt;
        let mut got = Vec::new();
        server.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, &payload[1000..3000]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 4: 运行测试**

```bash
cargo test --lib transfer::zerocopy
```
Expected: 2 passed(在 macOS 上走 sendfile 路径)。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(v2): write_data_header + zerocopy send_payload (macOS sendfile)"
```

---

## Task 3: socket 调参

**Files:**
- Create: `src-tauri/src/transfer/sock.rs`
- Modify: `src-tauri/src/transfer/mod.rs`(加 `pub mod sock;`)

- [ ] **Step 1: 写 sock.rs**

创建 `src-tauri/src/transfer/sock.rs`:
```rust
use socket2::SockRef;
use std::io;
use tokio::net::TcpStream;

const BUF: usize = 8 * 1024 * 1024; // 8 MiB

/// 调大收发缓冲 + 关闭 Nagle。失败仅记录,不致命。
pub fn tune_socket(stream: &TcpStream) {
    let sref = SockRef::from(stream);
    if let Err(e) = sref.set_recv_buffer_size(BUF) {
        tracing::warn!("set_recv_buffer_size: {e}");
    }
    if let Err(e) = sref.set_send_buffer_size(BUF) {
        tracing::warn!("set_send_buffer_size: {e}");
    }
    if let Err(e) = stream.set_nodelay(true) {
        tracing::warn!("set_nodelay: {e}");
    }
}

/// 同 tune_socket,但返回 Result 便于测试
pub fn tune_socket_checked(stream: &TcpStream) -> io::Result<()> {
    let sref = SockRef::from(stream);
    sref.set_recv_buffer_size(BUF)?;
    sref.set_send_buffer_size(BUF)?;
    stream.set_nodelay(true)?;
    Ok(())
}
```

- [ ] **Step 2: transfer/mod.rs 加 `pub mod sock;`**

把 `src-tauri/src/transfer/mod.rs` 的模块列表补上 `pub mod sock;`(与 Task 2 Step 2 合并后应为 8 个 `pub mod`)。

- [ ] **Step 3: 写测试(sock.rs 末尾)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn tune_socket_does_not_panic() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (s, _) = listener.accept().await.unwrap();
        let c = conn.await.unwrap();
        // 仅断言不 panic、nodelay 生效
        tune_socket(&s);
        tune_socket(&c);
        assert!(s.nodelay().unwrap());
    }
}
```

- [ ] **Step 4: 运行测试 + 编译**

```bash
cargo test --lib transfer::sock
cargo check
```
Expected: 1 passed;编译通过。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(v2): socket tuning (8MiB buffers + nodelay)"
```

---

## Task 4: 多连接 + 零拷贝发送端

**Files:**
- Modify: `src-tauri/src/transfer/sender.rs`
- Modify: `src-tauri/src/transfer/manager.rs`(`start_send` 传 config)
- Modify: `src-tauri/src/commands.rs`(`send_files` 传 config)

> 整段重写 `sender.rs` 的发送部分。`build_manifest`/`walk`/`connect_any` 保留,`connect_any` 内加 `tune_socket`。

- [ ] **Step 1: 重写 `src-tauri/src/transfer/sender.rs` 全文**

```rust
use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, write_control, write_data_header};
use crate::proto::messages::*;
use crate::store::{Identity, TransferConfig};
use crate::transfer::sock::tune_socket;
use crate::transfer::zerocopy::send_payload;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use uuid::Uuid;

pub async fn run_sender(
    session_id: Uuid,
    peer_addrs: Vec<SocketAddr>,
    files: Vec<String>,
    our: Identity,
    events: mpsc::UnboundedSender<TransferEvent>,
    config: TransferConfig,
) -> Result<()> {
    let result = run_sender_inner(session_id, peer_addrs, files, our.clone(), events.clone(), config).await;
    if let Err(e) = &result {
        tracing::error!("sender failed: {e}");
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::Internal, message: e.to_string() }) });
    }
    result
}

async fn run_sender_inner(
    session_id: Uuid,
    peer_addrs: Vec<SocketAddr>,
    files: Vec<String>,
    our: Identity,
    events: mpsc::UnboundedSender<TransferEvent>,
    config: TransferConfig,
) -> Result<()> {
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Connecting,
        bytes_done: 0, bytes_total: 0, files_done: 0, files_total: 0, speed_bps: 0 });

    let mut control = match connect_any(&peer_addrs).await {
        Ok(s) => { tracing::info!("sender connected (tried {peer_addrs:?})"); s }
        Err(e) => { tracing::warn!("sender connect failed for {peer_addrs:?}: {e}"); return Err(e); }
    };
    let hello = Hello { device_id: our.device_id.clone(), name: our.name.clone(),
        platform: Platform::Macos, session_id, proto_ver: PROTO_VER };
    write_control(&mut control, MsgType::Hello, &bincode::serialize(&hello)?).await?;
    let (ty, _) = read_control(&mut control).await?;
    if ty != MsgType::HelloAck { return Err(anyhow!("expected helloack, got {ty:?}")); }

    let (manifest, file_map) = build_manifest(session_id, &files)?;
    let files_total = manifest.files.iter().filter(|f| f.kind == FileKind::File).count() as u64;
    write_control(&mut control, MsgType::Manifest, &bincode::serialize(&manifest)?).await?;
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::AwaitingAccept,
        bytes_done: 0, bytes_total: manifest.total_size, files_done: 0, files_total, speed_bps: 0 });

    let (ty, _buf) = read_control(&mut control).await?;
    if ty == MsgType::Reject {
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
        return Ok(());
    }
    if ty != MsgType::Accept { return Err(anyhow!("expected accept, got {ty:?}")); }

    // —— 规划分发:把每个文件切成 segment,再 round-robin 到 N 个 bucket ——
    let buckets = plan_buckets(&manifest, &file_map, config.conns as usize, config.split_threshold);
    let n_conns = buckets.iter().filter(|b| !b.is_empty()).count().max(1);
    let chunk = config.chunk_size as usize;

    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
        bytes_done: 0, bytes_total: manifest.total_size, files_done: 0, files_total, speed_bps: 0 });

    let total_done = Arc::new(AtomicU64::new(0));
    let total_size = manifest.total_size;

    // 并发开 n_conns 条数据连接,各自跑自己的 bucket
    let mut join = tokio::task::JoinSet::new();
    for bucket in buckets.into_iter().filter(|b| !b.is_empty()) {
        let addrs = peer_addrs.clone();
        let done = total_done.clone();
        let sid = session_id;
        join.spawn(async move {
            let mut data = connect_any(&addrs).await?;
            write_control(&mut data, MsgType::DataOpen, &bincode::serialize(&DataOpen { session_id: sid })?).await?;
            for seg in &bucket {
                send_segment(&mut data, seg, chunk, &done).await?;
            }
            let _ = data.shutdown().await;
            Ok::<(), anyhow::Error>(())
        });
    }

    // 等所有数据连接完成
    while let Some(res) = join.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow!("join: {e}")),
        }
    }

    let done = total_done.load(Ordering::Relaxed);
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Finalizing,
        bytes_done: done, bytes_total: total_size, files_done: files_total, files_total, speed_bps: 0 });

    let (ty, _) = read_control(&mut control).await?;
    let final_state = if ty == MsgType::Complete { FinishedState::Completed } else { FinishedState::Failed };
    let _ = events.send(TransferEvent::Finished { session_id, state: final_state,
        error: if final_state == FinishedState::Failed { Some(ErrorPayload { code: ErrorCode::Internal, message: "no complete".into() }) } else { None } });
    Ok(())
}

struct Segment { file_id: Uuid, path: PathBuf, offset: u64, len: u64 }

/// 返回 conns 个 bucket(每个是一串 Segment)。大文件(>=split)按 conns 等分;小文件整块;再 round-robin。
fn plan_buckets(
    manifest: &Manifest,
    file_map: &HashMap<Uuid, PathBuf>,
    conns: usize,
    split: u64,
) -> Vec<Vec<Segment>> {
    let conns = conns.max(1);
    let mut segments: Vec<Segment> = Vec::new();
    for f in &manifest.files {
        if f.kind != FileKind::File { continue; }
        let path = match file_map.get(&f.id) { Some(p) => p.clone(), None => continue };
        if f.size >= split && conns > 1 {
            let each = f.size / conns as u64;
            let mut off = 0u64;
            for i in 0..conns {
                let end = if i == conns - 1 { f.size } else { off + each };
                if end > off { segments.push(Segment { file_id: f.id, path: path.clone(), offset: off, len: end - off }); }
                off = end;
            }
        } else if f.size > 0 {
            segments.push(Segment { file_id: f.id, path, offset: 0, len: f.size });
        }
    }
    let mut buckets = vec![Vec::new(); conns];
    for (i, seg) in segments.into_iter().enumerate() { buckets[i % conns].push(seg); }
    buckets
}

async fn send_segment(data: &mut TcpStream, seg: &Segment, chunk: usize, done: &AtomicU64) -> Result<()> {
    let file = std::fs::File::open(&seg.path)?;
    let mut off = seg.offset;
    let end = seg.offset + seg.len;
    while off < end {
        let n = chunk.min((end - off) as usize) as u32;
        write_data_header(data, seg.file_id, off, n).await?;
        send_payload(data, &file, off, n as usize).await?;
        off += n as u64;
        done.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(())
}

async fn connect_any(addrs: &[SocketAddr]) -> Result<TcpStream> {
    let mut last = None;
    for a in addrs {
        match TcpStream::connect(a).await { Ok(s) => { tune_socket(&s); return Ok(s); } Err(e) => last = Some(e) }
    }
    Err(anyhow!("connect failed: {:?}", last))
}

pub fn build_manifest(session_id: Uuid, files: &[String]) -> Result<(Manifest, HashMap<Uuid, PathBuf>)> {
    let mut metas = Vec::new();
    let mut map: HashMap<Uuid, PathBuf> = HashMap::new();
    let mut total: u64 = 0;
    for f in files {
        walk(f, "", &mut metas, &mut map, &mut total)?;
    }
    let count = metas.len() as u64;
    Ok((Manifest { session_id, files: metas, total_size: total, total_count: count }, map))
}

fn walk(abs: &str, rel_prefix: &str, metas: &mut Vec<FileMeta>,
        map: &mut HashMap<Uuid, PathBuf>, total: &mut u64) -> Result<()> {
    let p = Path::new(abs);
    let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let rel = if rel_prefix.is_empty() { name.clone() } else { format!("{rel_prefix}/{name}") };
    if p.is_dir() {
        metas.push(FileMeta { id: Uuid::new_v4(), name: name.clone(), rel_path: rel.clone(), size: 0, kind: FileKind::Dir, hash: None });
        for entry in std::fs::read_dir(p)? {
            let entry = entry?;
            walk(&entry.path().to_string_lossy(), &rel, metas, map, total)?;
        }
    } else {
        let size = std::fs::metadata(p)?.len();
        let id = Uuid::new_v4();
        map.insert(id, p.to_path_buf());
        metas.push(FileMeta { id, name, rel_path: rel, size, kind: FileKind::File, hash: None });
        *total += size;
    }
    Ok(())
}
```

- [ ] **Step 2: manager.start_send 传 config**

`src-tauri/src/transfer/manager.rs` 的 `start_send` 改签名为加 `config: crate::store::TransferConfig`,并传入 `run_sender`:
```rust
    pub fn start_send(&self, peer_addrs: Vec<SocketAddr>, files: Vec<String>, config: crate::store::TransferConfig) -> anyhow::Result<Uuid> {
        let session_id = Uuid::new_v4();
        let our = self.our.clone();
        let events = self.events_tx.clone();
        let id = session_id;
        tokio::spawn(async move {
            let _ = run_sender(id, peer_addrs, files, our, events, config).await;
        });
        Ok(session_id)
    }
```
文件顶部 `use` 加 `run_sender` 已存在;无需新 import(`crate::store::TransferConfig` 全路径)。

- [ ] **Step 3: commands.send_files 传 config**

`src-tauri/src/commands.rs` 的 `send_files` 末尾改为:
```rust
    tracing::info!("send_files → '{}' addrs={:?} port={} files={}", p.name, p.addrs, p.port, files.len());
    state.sessions.start_send(p.addrs, files, state.transfer_config.clone()).map_err(|e| e.to_string())
```

- [ ] **Step 4: 校验编译**

```bash
cargo check
```
Expected: 通过。注意:此时 receiver 尚未改造,**端到端会失败**(receiver 还按单连接 oneshot)。这是预期,Task 5 修。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(v2): multi-connection + zero-copy sender (offset-range split, round-robin)"
```

---

## Task 5: 多流并发接收端

**Files:**
- Modify: `src-tauri/src/transfer/manager.rs`(数据通道 oneshot→mpsc)
- Modify: `src-tauri/src/transfer/receiver.rs`(DrainState + drain_stream + 并发 + Notify + write_at)

> 核心:每会话的数据连接从「单 oneshot」改为「mpsc 池」;receiver 对每条流 spawn 一个 drain,共享 `parts`(`Arc<File>` + 位置写)+ `AtomicU64 total_done`;`total_done==total` 经 `Notify` 唤醒主流程做 finalize。

- [ ] **Step 1: manager 改数据通道为 mpsc**

`src-tauri/src/transfer/manager.rs`:
- `SessionChannels.data_tx` 由 `Option<oneshot::Sender<TcpStream>>` 改为 `mpsc::Sender<TcpStream>`。
- `use` 行把 `oneshot` 保留(Decision 仍用),`mpsc` 已有。

替换 `SessionChannels` 与 `handle_incoming` 的 Hello / DataOpen 分支:
```rust
struct SessionChannels {
    decision_tx: Option<oneshot::Sender<Decision>>,
    data_tx: mpsc::Sender<tokio::net::TcpStream>,
}
```
Hello 分支(创建 mpsc,容量 = 16,把 `xrx` 传给 run_receiver):
```rust
            MsgType::Hello => {
                let hello: Hello = bincode::deserialize(&buf)?;
                tracing::info!("Hello from '{}' session {}", hello.name, hello.session_id);
                let session_id = hello.session_id;
                let (dtx, drx) = oneshot::channel::<Decision>();
                let (xtx, xrx) = mpsc::channel::<tokio::net::TcpStream>(16);
                self.pending.lock().await.insert(session_id, SessionChannels {
                    decision_tx: Some(dtx), data_tx: xtx,
                });
                let events = self.events_tx.clone();
                let our = self.our.clone();
                tokio::spawn(async move {
                    let _ = run_receiver(stream, hello, events, drx, xrx, our).await;
                });
                Ok(())
            }
```
DataOpen 分支(每条都 `.send().await`,不再 take):
```rust
            MsgType::DataOpen => {
                let d: DataOpen = bincode::deserialize(&buf)?;
                let tx = {
                    let guard = self.pending.lock().await;
                    guard.get(&d.session_id).map(|c| c.data_tx.clone())
                };
                if let Some(tx) = tx {
                    let _ = tx.send(stream).await;
                }
                Ok(())
            }
```

- [ ] **Step 2: 重写 receiver.rs(run_receiver 签名 + 多流 drain)**

把 `src-tauri/src/transfer/receiver.rs` 全文替换为:
```rust
use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, read_data, write_control};
use crate::proto::messages::*;
use crate::store::Identity;
use crate::transfer::atomic::AtomicWriter;
use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Notify};
use uuid::Uuid;

#[derive(Debug)]
pub struct Decision { pub accept: bool, pub save_dir: PathBuf }

fn peer_platform(p: Platform) -> crate::discovery::Platform {
    match p {
        Platform::Macos => crate::discovery::Platform::Macos,
        Platform::Windows => crate::discovery::Platform::Windows,
        Platform::Linux => crate::discovery::Platform::Linux,
        Platform::Ios => crate::discovery::Platform::Ios,
        Platform::Android => crate::discovery::Platform::Android,
    }
}

/// 位置写:无 seek 状态,多任务并发写同一文件不同 offset 安全。
fn write_at(file: &File, buf: &[u8], offset: u64) -> std::io::Result<()> {
    #[cfg(unix)]
    { use std::os::unix::fs::FileExt; file.write_at(buf, offset)?; }
    #[cfg(windows)]
    { use std::os::windows::fs::FileExt; file.seek_write(buf, offset)?; }
    Ok(())
}

struct DrainState {
    parts: HashMap<Uuid, (FileMeta, Arc<File>, AtomicU64)>, // (meta, handle, received)
    total_done: AtomicU64,
    total: u64,
    notify: Notify,
    writer: Arc<AtomicWriter>,
    failed: AtomicBool,
}

pub async fn run_receiver(
    mut control: TcpStream,
    hello: Hello,
    events: mpsc::UnboundedSender<TransferEvent>,
    decision_rx: oneshot::Receiver<Decision>,
    mut data_rx: mpsc::Receiver<TcpStream>,
    our: Identity,
) -> Result<()> {
    let session_id = hello.session_id;

    let ack = HelloAck { device_id: our.device_id.clone(), name: our.name.clone() };
    write_control(&mut control, MsgType::HelloAck, &bincode::serialize(&ack)?).await?;

    let (ty, buf) = read_control(&mut control).await?;
    if ty != MsgType::Manifest {
        let _ = write_control(&mut control, MsgType::Error,
            &bincode::serialize(&ErrorMsg { code: ErrorCode::ProtocolError, message: "expected manifest".into() })?).await;
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: "expected manifest".into() }) });
        return Ok(());
    }
    let manifest: Manifest = match bincode::deserialize(&buf) {
        Ok(m) => m,
        Err(e) => {
            let _ = write_control(&mut control, MsgType::Error,
                &bincode::serialize(&ErrorMsg { code: ErrorCode::ProtocolError, message: e.to_string() })?).await;
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: e.to_string() }) });
            return Ok(());
        }
    };

    let peer = crate::discovery::Peer {
        device_id: hello.device_id.clone(), name: hello.name.clone(),
        platform: peer_platform(hello.platform),
        proto_version: hello.proto_ver as u16, addrs: vec![], port: 0, last_seen_ms: 0,
    };
    let _ = events.send(TransferEvent::Request { session_id, sender: peer, manifest: manifest.clone() });

    let decision = match decision_rx.await { Ok(d) => d, Err(_) => return Ok(()) };
    if !decision.accept {
        let _ = write_control(&mut control, MsgType::Reject,
            &bincode::serialize(&Reject { reason: "declined".into() })?).await;
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
        return Ok(());
    }
    let _ = write_control(&mut control, MsgType::Accept,
        &bincode::serialize(&Accept { save_dir: decision.save_dir.to_string_lossy().into() })?).await;

    tracing::info!("receiver: awaiting data connections");

    // 构建 DrainState(打开 .part 文件一次)
    let state = match build_drain_state(&manifest, &decision.save_dir, session_id) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            let _ = write_control(&mut control, MsgType::Error,
                &bincode::serialize(&ErrorMsg { code: ErrorCode::WriteFailed, message: e.to_string() })?).await;
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::WriteFailed, message: e.to_string() }) });
            return Ok(());
        }
    };

    // 进度上报任务(独立,按 total_done 周期发)
    let prog_state = state.clone();
    let prog_events = events.clone();
    let prog = tokio::spawn(async move {
        let mut meter = crate::transfer::meter::SpeedMeter::new(Duration::from_secs(2));
        let files_total = prog_state.parts.len() as u64;
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let done = prog_state.total_done.load(Ordering::Relaxed);
            meter.record(std::time::Instant::now(), done);
            let files_done = prog_state.parts.values().filter(|(_, _, r)| r.load(Ordering::Relaxed) > 0).count() as u64;
            let _ = prog_events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
                bytes_done: done, bytes_total: prog_state.total, files_done, files_total, speed_bps: meter.bps() });
            if done >= prog_state.total { break; }
            if prog_state.failed.load(Ordering::Relaxed) { break; }
        }
    });

    // accept 循环:每来一条数据连接 spawn 一个 drain_stream
    let acc_state = state.clone();
    let mut drain_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    while let Some(stream) = data_rx.recv().await {
        let st = acc_state.clone();
        drain_handles.push(tokio::spawn(async move {
            let _ = drain_stream(stream, &st).await;
        }));
    }

    // data_rx 关闭(manager 清理时)→ 不再指望新流;等所有 drain 结束
    for h in drain_handles { let _ = h.await; }
    prog.abort();

    let done = state.total_done.load(Ordering::Relaxed);
    let complete = done == state.total
        && state.parts.values().all(|(m, _, r)| r.load(Ordering::Relaxed) == m.size);
    tracing::info!("drain ended: done={done} total={} complete={complete}", state.total);

    let outcome: bool = if state.failed.load(Ordering::Relaxed) { false } else { complete };
    if outcome {
        let mut all_ok = true;
        for f in &manifest.files {
            if f.kind == FileKind::File && state.writer.finalize(&f.rel_path).is_err() { all_ok = false; }
        }
        state.writer.cleanup();
        let _ = write_control(&mut control, MsgType::Complete, &bincode::serialize(&Complete)?).await;
        let _ = events.send(TransferEvent::Finished { session_id,
            state: if all_ok { FinishedState::Completed } else { FinishedState::Failed },
            error: if all_ok { None } else { Some(ErrorPayload { code: ErrorCode::WriteFailed, message: "finalize failed".into() }) } });
    } else {
        state.writer.cleanup();
        let _ = write_control(&mut control, MsgType::Error,
            &bincode::serialize(&ErrorMsg { code: ErrorCode::ConnectionLost, message: "incomplete".into() })?).await;
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::ConnectionLost, message: "transfer incomplete".into() }) });
    }
    Ok(())
}

fn build_drain_state(manifest: &Manifest, save_dir: &Path, session_id: Uuid) -> Result<DrainState> {
    let writer = Arc::new(AtomicWriter::new(save_dir, &session_id.to_string())?);
    let mut parts: HashMap<Uuid, (FileMeta, Arc<File>, AtomicU64)> = HashMap::new();
    for f in &manifest.files {
        if f.kind == FileKind::File {
            let pp = writer.part_path(&f.rel_path);
            if let Some(p) = pp.parent() { std::fs::create_dir_all(p)?; }
            let fl = std::fs::File::create(&pp)?;
            fl.set_len(f.size)?;
            parts.insert(f.id, (f.clone(), Arc::new(fl), AtomicU64::new(0)));
        } else {
            let _ = std::fs::create_dir_all(save_dir.join(&f.rel_path));
        }
    }
    Ok(DrainState {
        parts,
        total_done: AtomicU64::new(0),
        total: manifest.total_size,
        notify: Notify::new(),
        writer,
        failed: AtomicBool::new(false),
    })
}

async fn drain_stream(mut data: TcpStream, state: &DrainState) {
    loop {
        let chunk = match read_data(&mut data).await { Ok(c) => c, Err(_) => break };
        if let Some((meta, file, recvd)) = state.parts.get(&chunk.file_id) {
            if let Err(e) = write_at(file, &chunk.data, chunk.offset) {
                tracing::error!("write_at failed: {e}");
                state.failed.store(true, Ordering::Relaxed);
                state.notify.notify_waiters();
                break;
            }
            recvd.fetch_add(chunk.data.len() as u64, Ordering::Relaxed);
            let prev = state.total_done.fetch_add(chunk.data.len() as u64, Ordering::Relaxed);
            if prev + chunk.data.len() as u64 >= state.total {
                state.notify.notify_one();
            }
            let _ = meta;
        }
    }
}
```

> 说明:`run_receiver` 现在靠「data_rx 关闭 + 所有 drain 结束 + total_done==total」判定完成。manager 在 `cleanup_session` 时 drop `data_tx`(mpsc sender),`data_rx.recv()` 返回 None → accept 循环退出。但 cleanup_session 当前在 Finished 后才调用(鸡生蛋)——故 Step 3 修 manager:在**所有数据连接看起来到齐后**主动 drop data_tx。更稳妥的做法见 Step 3。

- [ ] **Step 3: manager 在数据连接到齐后关闭 data_rx**

由于 receiver 不知道发送端会开几条连接,用「接收端按 manifest 完成度」驱动:不改 manager,而是让 receiver 的 accept 循环也监听 `state.notify`(total_done 到了就提前退出 accept,不再等 data_rx 关闭)。把 Step 2 的 accept 循环改为:
```rust
    // accept 循环:收到一条流就 drain;total_done 到了或 data_rx 关闭就退出
    let acc_state = state.clone();
    let mut drain_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    loop {
        tokio::select! {
            biased;
            _ = acc_state.notify.notified(), if acc_state.total_done.load(Ordering::Relaxed) >= acc_state.total => { break; }
            stream = data_rx.recv() => {
                match stream {
                    Some(s) => {
                        let st = acc_state.clone();
                        drain_handles.push(tokio::spawn(async move { let _ = drain_stream(s, &st).await; }));
                    }
                    None => break, // manager dropped data_tx
                }
            }
        }
    }
    for h in drain_handles { let _ = h.await; }
    prog.abort();
```
> 这样 total_done 到 total 即退出(无需等 data_rx 关闭);data_tx 被 manager drop 时(`cleanup_session`)也能退出。两条退出路径都安全。删掉 Step 2 里那段「while let Some(stream) = data_rx.recv()」旧循环(被本 select 替换)。

- [ ] **Step 4: 校验编译 + 跑既有测试**

```bash
cargo check
cargo test --lib transfer
cargo test --test loopback
```
Expected: 编译通过;`transfer` 单测全过;loopback 的 v1 端到端用例(`end_to_end_send_folder`)在多连接发送 + 多流接收下仍应通过(单连接是 v2 的 `conns` 被 clamp/或小文件只产生 1 个 bucket → 1 条数据连接,兼容)。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(v2): concurrent multi-stream receiver (write_at + Notify completion)"
```

---

## Task 6: loopback 大文件基准测试

**Files:**
- Modify: `src-tauri/tests/loopback.rs`

- [ ] **Step 1: 在 loopback.rs 末尾追加基准测试**

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn v2_big_file_throughput() {
    use std::time::Instant;
    let dir = std::env::temp_dir().join(format!("ss-bench-{}", Uuid::new_v4()));
    let save = dir.join("save");
    let src = dir.join("src.bin");
    // ~200MB(够量速度、又不至于太慢)。需要多连接:大于 split_threshold(4MiB)→ 会被拆到 4 条。
    let size: usize = 200 * 1024 * 1024;
    let mut big = Vec::with_capacity(size);
    let mut i = 0u64;
    while big.len() < size { big.extend_from_slice(&(i).to_le_bytes()); i += 1; }
    big.truncate(size);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&src, &big).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<TransferEvent>();
    let our_recv = identity("recv");
    let save_clone = save.clone();
    tokio::spawn(run_test_server(listener, our_recv, ev_tx.clone(), save_clone));

    let cfg = sendsent_lib::store::TransferConfig::defaults();
    let our_send = identity("send");
    let sid = Uuid::new_v4();
    let files = vec![src.to_string_lossy().into_owned()];
    let sender = tokio::spawn(async move {
        sendsent_lib::transfer::sender::run_sender(sid, vec![addr], files, our_send, ev_tx.clone(), cfg).await
    });

    let start = Instant::now();
    let mut completed = false;
    let drain = tokio::time::timeout(Duration::from_secs(60), async {
        while let Some(ev) = ev_rx.recv().await {
            if let TransferEvent::Finished { state: FinishedState::Completed, .. } = ev { completed = true; break; }
        }
    }).await;
    let secs = start.elapsed().as_secs_f64().max(0.001);
    assert!(drain.is_ok(), "timed out");
    assert!(completed, "expected Completed");
    let _ = sender.await;

    // 完整性
    let got = std::fs::read(save.join("src.bin")).unwrap();
    assert_eq!(got.len(), size);
    assert_eq!(got, big);

    let mbps = (size as f64 * 8.0) / secs / 1_000_000.0;
    eprintln!("v2 loopback throughput: {mbps:.0} Mbps");
    // loopback 远高于千兆;保守下限 2 Gbps(给 CI/笔记本留余量)
    assert!(mbps >= 2000.0, "throughput too low: {mbps:.0} Mbps");

    let _ = std::fs::remove_dir_all(&dir);
}
```

> 测试文件顶部需已 `use std::time::Duration;`(v1 用例已用)。若未导入,在顶部加。`run_test_server` / `identity` 已在文件中(v1 用例定义)。

- [ ] **Step 2: 运行基准**

```bash
cargo test --test loopback v2_big_file_throughput -- --nocapture
```
Expected: 通过,stderr 打印 `v2 loopback throughput: <N> Mbps`(N 应 ≥ 2000,通常数千),完整性断言通过。

- [ ] **Step 3: 全量测试**

```bash
cargo test
```
Expected: 所有(lib 单测 + loopback 含 v1 端到端 + v2 基准)通过。

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "test(v2): loopback big-file throughput benchmark (>=2Gbps)"
```

---

## Task 7: 全量校验 + AGENTS.md 增补

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: clippy + 全测试 + 前端构建**

```bash
cargo clippy --all-targets -- -D warnings
cargo test
pnpm build
```
Expected: clippy 干净;`cargo test` 全绿(含 v1 回归 + v2 基准);`pnpm build` 通过。

- [ ] **Step 2: 桌面手测(acceptance)**

```bash
pnpm tauri dev
```
两台物理桌面(同千兆 LAN)互发一个大文件(≥1GB),看 UI 速度读数应接近 ≥900 Mbps。单机可用 Mac + 模拟器(注意共用 Vite:见 AGENTS.md iOS 节)。

- [ ] **Step 3: AGENTS.md 增补**

在 `AGENTS.md` 的 `## File transfer (v1)` 段标题改为 `## File transfer (v1 / v2 speed)`,并在该段末尾追加:
```markdown
- **v2 速度层**:多数据连接(默认 4,`SENDSENT_CONNS`)+ macOS `sendfile` 零拷贝(其它平台 `pread`+write 回退,见 `transfer/zerocopy.rs`)+ socket 调参(`transfer/sock.rs`)。
- 可调项 `TransferConfig { conns, chunk_size, split_threshold }`,持久化于 `<app_data_dir>/transfer.json`,环境变量 `SENDSENT_CONNS` / `SENDSENT_CHUNK_KB` / `SENDSENT_SPLIT_MB` 可覆盖。
- 发送端把大文件按 offset 区间拆到 N 连接、小文件轮询;接收端 `run_receiver` 对每条数据连接并发 drain,用位置写 `write_at` 复用 `.part` 句柄,`total_done==total` 判完成。无线协议改动。
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "docs(agents): note v2 speed layer (multi-conn, sendfile, TransferConfig)"
```

---

## 自检(Self-Review)结果

- **Spec 覆盖**:零拷贝(§3)→ Task 2;多连接收发(§4)+ 位置写(§4.3/§5)→ Task 4/5;socket 调参(§6)→ Task 3;TransferConfig(§2.2)→ Task 1;基准验收(§7)→ Task 6;范围边界(§8:不做 Linux/Win 原生零拷贝、加密、移动)→ 各 Task 范围内体现。✅
- **类型一致性**:`TransferConfig` 字段(`conns: u32`/`chunk_size: u64`/`split_threshold: u64`)在 store/state/lib/sender/commands 间一致;`run_sender(..., config)` / `start_send(..., config)` 签名在调用处匹配;`write_data_header(file_id, offset, len: u32)` 与 `send_payload(socket, file, offset, len: usize)` 在 `send_segment` 中衔接(len u32 → as usize);`DrainState`/`drain_stream`/`build_drain_state` 命名一致;`data_tx` 在 manager 与 receiver 间为 `mpsc::Sender<TcpStream>` 一致。✅
- **已知风险点(执行时留意)**:
  1. `sendfile_macos` 的 `EWOULDBLOCK` → `socket.writable().await` 循环;若 sendfile 行为与预期不符,以 `cargo test --lib transfer::zerocopy` 的 roundtrip 测试为准就地修。
  2. Task 5 的 accept `select!` 含 `if` 守卫的 `notify.notified()` 分支:Rust tokio `select!` 支持条件分支;若编译告警,改为先判 total 再 `select!`。
  3. 基准下限 2000 Mbps 是 loopback 保守值;若 CI 机器弱,执行者可下调但需注明。
- **placeholder 扫描**:无 TBD/TODO,每步含完整代码与命令。✅
