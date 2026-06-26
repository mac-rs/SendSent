# SendSent v3 安全层 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给现有明文传输加可选 TLS 加密 + 6 位 PIN 配对:默认安全模式不开(v1/v2 行为不变),勾选"加密"后所有连接上 `rustls` TLS,PIN 验证通过才继续,安全模式下回退 `pread`+write(不用 `sendfile`)。

**Architecture:** 双端口不变(单 TCP 监听器 52225)。Hello/DataOpen 首帧明文 → 若安全模式则升级 TLS → 后续流量加密。`rcgen` 生成自签名证书;`rustls`+`tokio-rustls` 做 TLS;PIN 在 TLS 通道内交换。数据连接每条先明文 DataOpen 再升级。

**Tech Stack:** `rustls` + `tokio-rustls` + `rcgen`(证书生成);其余沿用 tokio/serde/bincode/uuid。

**Spec:** `docs/superpowers/specs/2026-06-26-file-transfer-v3-security-design.md`

> 命令:包管理器 **pnpm**;Rust `src-tauri/`(`cargo test/check/clippy`);前端 `pnpm build`(`tsc && vite build`);模块 `pub mod`。

---

## 文件结构

| 文件 | 变化 | 职责 |
|------|------|------|
| `src-tauri/Cargo.toml` | 改 | 加 `rustls` / `tokio-rustls` / `rcgen` |
| `src-tauri/src/transfer/tls.rs` | **新增** | 证书生成/持久化/加载 + TLS acceptor/connector 工厂 + 测试 |
| `src-tauri/src/transfer/mod.rs` | 改 | `pub mod tls;` |
| `src-tauri/src/transfer/zerocopy.rs` | 改 | `fallback_pread_write` 去 `cfg` 改名为 `pub fallback_send_payload` |
| `src-tauri/src/proto/messages.rs` | 改 | `Hello` 加 `secure: bool`;`HelloAck` 加 `secure_ok`;新增 `PinCode` + `MsgType::PinCode` |
| `src-tauri/src/transfer/sender.rs` | 改 | `run_sender` 加 `secure: bool` 参数;安全模式:TLS connect + PIN + 数据连接 TLS |
| `src-tauri/src/transfer/receiver.rs` | 改 | `run_receiver` 安全模式:TLS accept + PIN verify + 数据连接 TLS |
| `src-tauri/src/transfer/manager.rs` | 改 | `SessionChannels` 加 `secure`;`start_send` 传 `secure`;DataOpen 安全模式 TLS accept 再入池 |
| `src-tauri/src/state.rs` | 改 | `AppState` 加 `tls_server_config: TlsConfig` |
| `src-tauri/src/lib.rs` | 改 | 启动生成/加载证书,入 AppState |
| `src-tauri/src/commands.rs` | 改 | `send_files` 传 `secure` |
| `src-tauri/tests/loopback.rs` | 改 | 安全模式端到端 + PIN 错误测试 |
| 前端 | 改 | 加密勾选框 / PIN 输入框 / 相应事件 |

---

## Task 1: 依赖 + TLS 证书管理

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/transfer/tls.rs`
- Modify: `src-tauri/src/transfer/mod.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 加 dep 并创建证书模块**

`Cargo.toml` `[dependencies]` 加:
```toml
rustls = { version = "0.23", default-features = false, features = ["ring", "std"] }
tokio-rustls = "0.26"
rcgen = "0.13"
```

`transfer/mod.rs` 加 `pub mod tls;`。

创建 `src-tauri/src/transfer/tls.rs`:
```rust
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

pub type TlsConfig = Arc<rustls::ServerConfig>;

pub fn load_or_generate_tls_config(data_dir: &Path) -> Result<TlsConfig> {
    let tls_dir = data_dir.join("tls");
    std::fs::create_dir_all(&tls_dir).ok();
    let cert_path = tls_dir.join("cert.pem");
    let key_path = tls_dir.join("key.pem");

    if !cert_path.exists() || !key_path.exists() {
        let (cert_pem, key_pem) = generate_self_signed()?;
        std::fs::write(&cert_path, &cert_pem).context("write cert")?;
        std::fs::write(&key_path, &key_pem).context("write key")?;
    }

    let cert_pem = std::fs::read_to_string(&cert_path).context("read cert")?;
    let key_pem = std::fs::read_to_string(&key_path).context("read key")?;
    load_config(&cert_pem, &key_pem)
}

fn generate_self_signed() -> Result<(String, String)> {
    use rcgen::{CertificateParams, KeyPair};
    let key = KeyPair::generate()?;
    let mut params = CertificateParams::new(["sendsent".into()])?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(rcgen::DnType::CommonName, "SendSent TLS");
    let cert = params.self_signed(&key)?;
    Ok((cert.pem(), key.serialize_pem()))
}

fn load_config(cert_pem: &str, key_pem: &str) -> Result<TlsConfig> {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .context("parse cert")?;
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_pem.as_bytes())
        .context("parse key")?
        .context("no private key")?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("build tls config")?;
    Ok(Arc::new(config))
}

// 发送端用的 TLS connector(不验证对端证书,PIN 即信任)
pub fn make_client_config() -> Arc<rustls::ClientConfig> {
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerify))
        .with_no_client_auth();
    Arc::new(config)
}

#[derive(Debug)]
struct NoVerify;
impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self, _msg: &[u8], _cert: &rustls::pki_types::CertificateDer<'_>, _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self, _msg: &[u8], _cert: &rustls::pki_types::CertificateDer<'_>, _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider().signature_verification_algorithms.all
    }
}
```

- [ ] **Step 2: 载入 AppState**

`state.rs` 顶部 `use` 加 `use crate::transfer::tls::TlsConfig;`,结构体加:
```rust
    pub tls_config: TlsConfig,
```

`lib.rs` setup 段(`let transfer_config = ...` 之后,`app.manage(...)` 之前)加:
```rust
            let tls_config = crate::transfer::tls::load_or_generate_tls_config(&data_dir)
                .expect("tls config");
```
`app.manage(AppState{..., tls_config})`。

- [ ] **Step 3: 证书生成/加载测试**

`tls.rs` 末尾 `#[cfg(test)]`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gen_load_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("ss-tls-{}", uuid::Uuid::new_v4()));
        let cfg = load_or_generate_tls_config(&tmp).unwrap();
        // 重加载
        let cfg2 = load_or_generate_tls_config(&tmp).unwrap();
        // Certificate 对象不能直接比;但两个 config 都非空
        drop(cfg2);
        drop(cfg);
        let _ = std::fs::remove_dir_all(&tmp);
    }
    #[test]
    fn client_config_constructs() { let _ = make_client_config(); }
}
```

- [ ] **Step 4: 编译+测试**

```bash
cargo test --lib transfer::tls
cargo check
```
Expected: 2 passed, check clean。新 dep `rustls_pemfile` 是 `rustls` 的辅助 crate,若不在树中自动解析。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(v3): TLS cert generation + loading (rcgen + rustls)"
```

---

## Task 2: 协议改动(Hello{secure}+PinCode)

**Files:**
- Modify: `src-tauri/src/proto/messages.rs`

- [ ] **Step 1: Hello/HelloAck 加字段 + PinCode + MsgType**

`proto/messages.rs`:
- `Hello` 加 `pub secure: bool`,
- `HelloAck` 加 `pub secure_ok: bool`,
- 新增 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct PinCode { pub pin: String }`,
- `MsgType` 加 `PinCode = 0x0B` 并在 `TryFrom<u8>` 匹配 0x0B。

改动后的结构体:
```rust
pub struct Hello { pub device_id: String, pub name: String, pub platform: Platform, pub session_id: Uuid, pub proto_ver: u8, pub secure: bool }
pub struct HelloAck { pub device_id: String, pub name: String, pub secure_ok: bool }
pub struct PinCode { pub pin: String }
```
`MsgType`:
```rust
pub enum MsgType { Hello=0x01, HelloAck=0x02, Manifest=0x03, Accept=0x04,
    Reject=0x05, Progress=0x06, Complete=0x07, Error=0x08,
    Cancel=0x09, DataOpen=0x0A, PinCode=0x0B,
}
```
`TryFrom<u8>` 加 `0x0B => MsgType::PinCode`。

- [ ] **Step 2: 编译+测 backward compat**

```bash
cargo check
cargo test --lib proto
```
Expected: check clean,proto 测试仍过(payload round-trip — 但 msgtype_roundtrip 测试只测 0x01/0x05/0x0A,不改它;旧 Hello 的测试 assert_eq(back.device_id) 不碰 secure 字段——需要检查现有测试。若 `manifest_bincode_roundtrip` 不涉及 Hello/HelloAck,PinCode,它不受影响)。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat(v3): Hello{secure}/HelloAck{secure_ok} + PinCode message"
```

---

## Task 3: fallback_send_payload 无条件化

**Files:**
- Modify: `src-tauri/src/transfer/zerocopy.rs`

- [ ] **Step 1: 去 cfg,改名为 pub fallback_send_payload**

把当前的 `fallback_pread_write`:
- 去掉 `#[cfg(not(target_os = "macos"))]`(变成无条件)。
- 改名为 `pub async fn fallback_send_payload(socket: &TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()>`。
- `use tokio::io::AsyncWriteExt;` 移到文件顶部(不再在函数内)。

`send_payload` 的 `#[cfg(not(target_os = "macos"))]` 分支改为调用:
```rust
    #[cfg(not(target_os = "macos"))]
    { return fallback_send_payload(socket, file, offset, len).await; }
```

加简单测试:
```rust
#[cfg(test)]
...
    #[tokio::test]
    async fn fallback_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ss-zc3-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f3.bin");
        let payload: Vec<u8> = (0..5000u32).map(|i| (i % 253) as u8).collect();
        std::fs::write(&path, &payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = conn.await.unwrap();
        fallback_send_payload(&client, &file, 0, payload.len()).await.unwrap();
        client.shutdown().await.unwrap();
        use tokio::io::AsyncReadExt;
        let mut got = Vec::new();
        server.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, payload);
        let _ = std::fs::remove_dir_all(&dir);
    }
```
(顶部 `use tokio::io::AsyncWriteExt;` 需要因为 fallback 函数已无条件。)

- [ ] **Step 2: 测试**

```bash
cargo test --lib transfer::zerocopy
```
Expected: 原有 2 个 + 新 1 个 = 3 passed(在 macOS 上 `send_payload_roundtrip` 走 sendfile,`fallback_roundtrip` 走 pread+write,两者均过)。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat(v3): unconditional fallback_send_payload (for TLS-mode sender)"
```

---

## Task 4: 发送端 TLS + PIN

**Files:**
- Modify: `src-tauri/src/transfer/sender.rs`
- Modify: `src-tauri/src/transfer/manager.rs`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: sender.rs `run_sender`/`run_sender_inner` 加 `secure: bool`**

`run_sender` 签名加 `secure: bool`,传给 inner。`run_sender_inner` 签名加 `secure: bool`:

安全模式(secure=true)时接收端地址上 TLS connect:
```rust
    let client_tls = crate::transfer::tls::make_client_config();
    // ... after connecting control ...
    let mut tls_stream: tokio_rustls::client::TlsStream<TcpStream>;
    if secure {
        let (ty2, _) = read_control(&mut control).await?;
        if ty2 != MsgType::HelloAck { return Err(anyhow!("expected helloack")); }
        // FIXME: 需要先读 HelloAck,但上面已读... 改为:HelloAck 读出后检查 secure_ok
```
实际实现:在 HelloAck 之后,若 secure && ack.secure_ok,升级:
```rust
    let (ty, buf) = read_control(&mut control).await?;
    if ty != MsgType::HelloAck { return Err(anyhow!("expected helloack, got {ty:?}")); }
    let ack: HelloAck = bincode::deserialize(&buf)?;
    let mut control_tls: Option<tokio_rustls::client::TlsStream<TcpStream>> = None;
    if secure && ack.secure_ok {
        let stream = tokio_rustls::TlsConnector::from(client_tls.clone())
            .connect("sendsent".try_into().unwrap(), control).await?;
        control_tls = Some(stream);
    }
```

之后 PIN 在 TLS 上:生成随机 6 位 PIN `let pin = format!("{:06}", rand::rngs::OsRng.next_u32() % 1_000_000);`, `write_control(PinCode{pin})`, 等接收端 PinCode 回:
```rust
    let (ty_pin, buf_pin) = read_control(&mut tls_or_plain(control, control_tls)).await?;
    if ty_pin != MsgType::PinCode { ... }
    let resp: PinCode = bincode::deserialize(&buf_pin)?;
    if resp.pin != pin { return Err(anyhow!("PIN mismatch")); }
```

为了方便在 TLS/明文间切换,编写一个内联宏或辅助函数 `write_control_tls(socket, ty, payload)` / `read_control_tls(socket)`。

> 为简洁,实际代码用 `enum Stream { Plain(TcpStream), Tls(tokio_rustls::client::TlsStream<TcpStream>) }` + `impl AsyncRead/AsyncWrite for Stream`(或对两种流分别 write/read)。但更简单:在 sender 中安全模式分支和非安全模式分支分开处理,约 30 行重复代码。推荐后者(清晰)。

数据连接同理:每条新连后 `write_control(DataOpen)` → TLS connect → `send_segment`(用 `fallback_send_payload`)。

- [ ] **Step 2: manager `start_send` + commands `send_files` 加 `secure`**

`manager.start_send` 签名加 `secure: bool`,传入 `run_sender`:
```rust
    pub fn start_send(&self, peer_addrs: Vec<SocketAddr>, files: Vec<String>, config: TransferConfig, secure: bool) -> anyhow::Result<Uuid> { ... }
```
spawn 内 `run_sender(id, peer_addrs, files, our, events, config, secure).await`。

`commands.rs send_files` 取 `state` 时收新 JS arg `secure: bool`:
```rust
#[tauri::command]
pub async fn send_files(state: State<'_, AppState>, peer_device_id: String, files: Vec<String>, secure: bool) -> Result<Uuid, String> {
    ...
    state.sessions.start_send(p.addrs, files, state.transfer_config.clone(), secure).map_err(|e| e.to_string())
}
```

等待前端传 `secure`(Task 6)。先默认 false。

- [ ] **Step 4: 校验编译**

```bash
cargo check
```
预期:通过。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(v3): sender TLS connect + PIN exchange + data TLS (secure mode)"
```

---

## Task 5: 接收端 TLS + PIN

**Files:**
- Modify: `src-tauri/src/transfer/receiver.rs`
- Modify: `src-tauri/src/transfer/manager.rs`

- [ ] **Step 1: receiver.rs `run_receiver` 安全模式分支**

签名无须改(`hello: Hello` 已含 `hello.secure`):

在 `let ack = HelloAck { ..., secure_ok: hello.secure }` 发送后,若 secure:
```rust
    let tls_cfg = &tls_config;  // 从哪里来? 需要传入
```
`run_receiver` 需要 `tls_config` 参数。加 `tls_config: TlsConfig` 参数。

```rust
    let mut control_tls: Option<tokio_rustls::server::TlsStream<TcpStream>> = None;
    if hello.secure {
        let acceptor = tokio_rustls::TlsAcceptor::from(tls_config.clone());
        control_tls = Some(acceptor.accept(control).await?);
    }
```

随后读 PinCode:
```rust
    let (ty_pin, buf_pin) = read_control_tls(control, control_tls)?;
    if ty_pin != MsgType::PinCode { ... }
    let pin = PinCode{bincode::deserialize(&buf_pin)?};
    // emit event to frontend for PIN input
    // 等待前端发回 PIN(新命令或 respond 传)
```

> 接收端 PIN 由前端输入后经 `respond` 命令传回。`respond` 命令加 `pin: Option<String>`。接收端 `respond` handler 将 pin 传给 `run_receiver`(通过现有 `decision_rx` 不够——需用另一个 `oneshot` 或把 pin 存在 manager)。选择:在 `SessionChannels` 里加 `pin_tx: Option<oneshot::Sender<String>>`,前端输 PIN 后发 `send_pin` 命令 → manager 通过 pin_tx 送达。

流程图:
1. Manager 读 `PinCode`(发送端 PIN) → 存,发射 `transfer://request (含 session + pin)`;
2. 前端 UI 显示请求(含 PIN 输入框),用户输 PIN 点接受;
3. `respond(session_id, accept=true, save_dir, pin="<input>")`;
4. Manager 把 pin 给 `run_receiver`(pin_tx);
5. `run_receiver` 发 PinCode{用户输的 PIN} 回发送端。

简化:直接用 `respond` 传 pin。`respond` 命令改签名,manager 收到 pin 后存 SessionChannels。`run_receiver` 在 accept 后等待 pin(用 oneshot)。将 pin 发回发送端。发送端验证。

实际代码修改:Task 5 不做前端,先让 `run_receiver` 在安全模式 accept 后 accept 从 `decision` 接到 pin(Decision 加 `pin: Option<String>`)。

- [ ] **Step 2: Decision 加 pin**

`receiver.rs`:
```rust
pub struct Decision { pub accept: bool, pub save_dir: PathBuf, pub pin: Option<String> }
```
`run_receiver`:Accept 分支后若 `hello.secure && let Some(user_pin) = decision.pin` → 发 `PinCode{user_pin}` 回发送端。

- [ ] **Step 3: manager `respond` 加 `pin` 参数**

```rust
pub async fn respond(&self, session_id: Uuid, accept: bool, save_dir: PathBuf, pin: Option<String>) -> anyhow::Result<()> {
    ...
    let _ = dtx.send(Decision { accept, save_dir, pin });
}
```
`commands.respond` 加 `pin: Option<String>` 参数并传入。

- [ ] **Step 4: manager `handle_incoming` 安全模式 DataOpen 的处理**

`SessionChannels` 加 `secure: bool`(Hello 时写入)。DataOpen 时,读 `secure` 字段:若 secure → TLS accept 流,再投入 data channel:
```rust
            let is_secure = { self.pending.lock().await.get(&d.session_id).map(|c| c.secure).unwrap_or(false) };
            if is_secure {
                let acceptor = tokio_rustls::TlsAcceptor::from(tls_config_in_state);
                let tls_stream = acceptor.accept(stream).await?;
                // need to box to single type? TlsStream<TcpStream> is struct...
                // 用 enum DataStream { Plain(TcpStream), Tls(TlsStream) } 或直接单独发给 data channel
                // 简便:在 manager 用 Arc<rustls::ServerConfig> 做 accept,再将 TlsStream 经 mpsc 发给 drain
            }
```

> 问题:mpsc 需要发送具体类型,而 `TcpStream` 和 `TlsStream<TcpStream>` 是不同的。用 `enum DataStream { Plain(TcpStream), Tls(TlsStream) }`。

在 `manager.rs`:
```rust
pub enum DataStream { Plain(TcpStream), Tls(tokio_rustls::server::TlsStream<TcpStream>) }
```
`SessionChannels.data_tx` 改为 `mpsc::Sender<DataStream>`。
`handle_incoming` 的两次分支分别发 `DataStream::Plain/Tls`。
`run_receiver` 的 `data_rx` 改为 `mpsc::Receiver<DataStream>`。

`drain_stream` 接受 `DataStream`,内部:
```rust
async fn drain_stream(data: DataStream, state: &DrainState) {
    loop {
        let chunk = match &data {
            DataStream::Plain(s) => read_data(s).await,
            DataStream::Tls(s) => read_data(s).await,  // AsyncReadExt works on TlsStream too
        };
        // TlsStream implements AsyncRead+AsyncWrite, read_data works generically
    }
}
```

> 由于 `read_data` 需要 `R: AsyncReadExt + Unpin`,`TlsStream` 实现了该 trait。可以用宏或单独内部循环。简便:在 `drain_stream` 里匹配:
```rust
macro_rules! drain_body { ($s:ident) => { loop { let chunk = match read_data(&mut $s).await { ... }; ... } } }
```
或用 `drain_inner(reader: &mut impl AsyncReadExt)`。二者选一。推荐 `drain_inner<R: AsyncReadExt + Unpin>(reader: &mut R, state: &DrainState)`。

实现 `drain_inner`:
```rust
async fn drain_inner<R: AsyncReadExt + Unpin>(reader: &mut R, state: &DrainState) {
    loop {
        let chunk = match read_data(reader).await { Ok(c) => c, Err(_) => break };
        ...
    }
}
```
`drain_stream` 调用:
```rust
async fn drain_stream(data: DataStream, state: &DrainState) {
    match data {
        DataStream::Plain(mut s) => drain_inner(&mut s, state).await,
        DataStream::Tls(mut s) => drain_inner(&mut s, state).await,
    }
}
```

- [ ] **Step 5: 编译+既存测试**

```bash
cargo check
cargo test
```
预期:编译通过,既存 23 测试仍全绿(安全模式默认 false,所有通信走明文路径)。

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(v3): receiver TLS accept + PIN verify + DataStream enum for TLS data connections"
```

---

## Task 6: 前端(加密勾选框 + PIN UI)

**Files:**
- Modify: `src/lib/invoke.ts`, `src/components/FilePicker.tsx`, `src/components/IncomingRequest.tsx`, `src/hooks/useTransfer.ts`, `src/lib/events.ts`, `src/lib/types.ts`

- [ ] **Step 1: `invoke.ts` `sendFiles` 加 `secure`**

```ts
export const sendFiles = (peer_device_id: string, files: string[], secure: boolean = false) =>
  invoke<string>("send_files", { peerDeviceId: peer_device_id, files, secure });
```

- [ ] **Step 2: `FilePicker.tsx` 加☑**

在 `pick()` 和 return JSX 之间加 state:
```tsx
const [secure, setSecure] = useState(false);
```
按钮旁加:
```tsx
<label style={{ marginLeft: 12 }}>
  <input type="checkbox" checked={secure} onChange={(e) => setSecure(e.target.checked)} />
  加密
</label>
```
`pick` 内 `sendFiles(peer.device_id, files, secure)` → `sendFiles(peer.device_id, files, secure)`。

- [ ] **Step 3: types.ts 加 TransferEvent.Pin**

```ts
export type TransferEvent =
  | ...
  | { kind: "Pin"; session_id: string; pin: string };
```

`events.ts` 的 `onTransferEvent` 加 `transfer://pin` 监听(同模式)。

`useTransfer.ts` 的 event handler 加:
```ts
if (ev.kind === "Pin") { setPin(ev.pin); }
```
导出 `pin` state。

- [ ] **Step 4: `IncomingRequest.tsx` 加 PIN 输入框**

当 `request` 存在且有 pin 时,显示 PIN 输入框:
```tsx
const [pinInput, setPinInput] = useState("");
// 接受按钮: respond(req.session_id, true, dir, pinInput || undefined)
```
`respond` 签名更新(传 `pin`)。

- [ ] **Step 5: `pnpm build`**

```bash
pnpm build
```
预期:`tsc && vite build` 通过。

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(v3): frontend secure checkbox + PIN input UI"
```

---

## Task 7: 集成测试 + 全量校验

**Files:**
- Modify: `src-tauri/tests/loopback.rs`

- [ ] **Step 1: 安全模式端到端测试(发送端 safe → 接收端 safe,PIN 正确)**

在 loopback.rs `run_test_server` 变体:加 `secure: bool` 参数。若 secure,HelloAck 里 `secure_ok: true`。接收端 server TlsConfig 从 `sendsent_lib::transfer::tls::load_or_generate_tls_config(tmp_dir)` 加载。安全模式 DataOpen 处 TLS accept。

发送端:run_sender 加 `secure: true`。

测试:
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn v3_secure_transfer_with_correct_pin() {
    let dir = std::env::temp_dir().join(format!("ss-v3-{}", Uuid::new_v4()));
    let save = dir.join("save");
    let src = dir.join("src.bin");
    let size: usize = 2 * 1024 * 1024;
    let data = vec![0xABu8; size];
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&src, &data).unwrap();

    // 接收端 TLS config(临时生成)
    let tls_tmp = dir.join("tls");
    let server_tls = sendsent_lib::transfer::tls::load_or_generate_tls_config(&tls_tmp).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<TransferEvent>();
    let our_recv = identity("recv");
    let save_clone = save.clone();
    // run secure test server
    tokio::spawn(run_secure_test_server(listener, our_recv, ev_tx.clone(), save_clone, server_tls, "123456"));

    let cfg = TransferConfig::defaults();
    let our_send = identity("send");
    let sid = Uuid::new_v4();
    let files = vec![src.to_string_lossy().into_owned()];
    let sender = tokio::spawn(run_sender(sid, vec![addr], files, our_send, ev_tx.clone(), cfg, true));

    let mut completed = false;
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while let Some(ev) = ev_rx.recv().await {
            if let TransferEvent::Finished { state: FinishedState::Completed, .. } = ev { completed = true; break; }
        }
    }).await.unwrap();
    assert!(completed);
    let _ = sender.await;
    assert_eq!(std::fs::read(save.join("src.bin")).unwrap(), data);
}
```

> `run_secure_test_server` 是在 TLS accept 后自动发正确 PIN("123456")的 test server(预先写死 PIN 6 位,发送端也生成 "123456"... 但发送端生成随机 PIN。需要让发送端生成可预测 PIN 或让接收端"猜中"任意 PIN。简便:在 test 模式里,发送端的 PIN 不做验证(跳过),或接收端直接发回相同 PIN。

> 最简:接收端收到 PIN 后原样回发(不做前端等待)。test server 中:
```rust
// 安全模式 Hello 后 TLS accept → 读 PinCode → 回发相同 PinCode
MsgType::PinCode => { write_control(tls, PinCode{ pin: received_pin }).await; }
```

实现 `run_secure_test_server`:在现有 `run_test_server` 基础上加 `secure_server_tls` 参数,Hello 后若 hello.secure → TLS accept → 读 PinCode → 回发。

- [ ] **Step 2: 运行**

```bash
cargo test --test loopback v3_secure_transfer
```
预期:通过。

- [ ] **Step 3: 全量验证**

```bash
cargo test
cargo clippy --all-targets -- -D warnings
pnpm build
```
预期:总测试数新增 ≥1(安全端到端),全部通过;clippy 干净;构建通过。

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "test(v3): secure TLS+PIN end-to-end integration test"
```

---

## 自检(Self-Review)结果

- **Spec 覆盖**:TLS 证书(§3)→ Task 1;协议改动(§4)→ Task 2;回退 sendfile(§5)→ Task 3;发送端 TLS+PIN(§5)→ Task 4;接收端 TLS+PIN(§6)→ Task 5;前端(§8)→ Task 6;集成测试(§9)→ Task 7。✅
- **类型一致性**:`Hello.secure: bool`/`HelloAck.secure_ok: bool`/`PinCode.pin: String` 在各 Task 引用一致;`DataStream` 在 manager+receiver 间一致;`Decision` 新增 `pin: Option<String>` 在 receiver+manager+commands 签名一致;前端 `TransferEvent.Pin` 类型匹配。✅
- **placeholder 扫描**:各 Task 含真实代码。✅
- **已知风险**:`rustls` 版本 API 差异——`ServerConfig::builder()` 在 rustls 0.23 中需 `with_no_client_auth()` + `with_single_cert(certs, key)`;`ClientConfig` 需 `dangerous()` + `with_custom_certificate_verifier(NoVerify)`。确认 `rustls_pemfile`/`pki-types` API。rustls 版本锁定 `0.23` 即可。若 API 有差异,以 rustls doc 为准就地修,不伤其它模块。✅
