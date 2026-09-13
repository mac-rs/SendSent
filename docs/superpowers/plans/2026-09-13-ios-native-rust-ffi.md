# iOS 原生客户端 — Plan 1: Rust FFI 与构建解耦

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Rust 核心（`sendsent_lib`）在 iOS 上以 `--no-default-features` 编译为静态库，并通过一层 `extern "C"` FFI（init/peers/send/respond/history/identity/addresses/qr/config + JSON 事件回调）暴露给 SwiftUI，彻底不依赖 Tauri。

**Architecture:** 用 Cargo feature `tauri-shell`（默认开启）隔离 Tauri 相关模块（`run()`、`commands`、`state`、Android 插件、`main`）。新增 iOS-only 的 `src/ffi.rs`，持有全局 `OnceLock<Core>`（identity/discovery/SessionManager/history/tls/config/port + 专用 tokio runtime + 事件回调）。事件复用现有 `TransferEvent` 的 serde（内部 tag `kind`），peer 事件用 `{"kind":"peer_found"|"peer_lost",...}`，进度按 session 节流。iOS 构建时 Xcode 直接 `cargo build --lib --target aarch64-apple-ios --no-default-features` 并把 `libsendsent_lib.a` 拷成 `libapp.a`。

**Tech Stack:** Rust 2024、tokio、serde/serde_json、zeroconf、rustls、qrcode/image/base64、get_ifaddrs；构建用 xcodegen(`project.yml`)+`tauri ios build`(仅编排 xcodebuild)。

**范围：** 本计划只做 Rust + 构建。SwiftUI 视图层见 Plan 2（`2026-09-13-ios-native-swiftui.md`）。桌面/Android 行为必须零回归。

---

## 文件结构

- Modify `src-tauri/Cargo.toml` — 新增 `[features] tauri-shell`；4 个 tauri 依赖改 `optional`；删除 `[patch.crates-io] tao`。
- Modify `src-tauri/build.rs` — 仅在 `feature="tauri-shell"` 时调用 `tauri_build::build()`。
- Modify `src-tauri/src/lib.rs` — 模块与 `run()` 用 feature gate；新增 `pub mod misc; pub mod ffi;`。
- Create `src-tauri/src/misc.rs` — 平台无关纯函数：`MyAddress`、`get_my_addresses`、`urlencoding`、`render_qr_png`、`my_qr_base64`（从 `commands.rs` 迁出，含单测）。
- Create `src-tauri/src/ffi.rs` — `#[cfg(target_os="ios")]`；`Core`、FFI 函数、事件桥、节流。
- Modify `src-tauri/src/commands.rs` — 删除 QR/地址实现，改为调用 `crate::misc`；删除 `ios_picker` 模块与 `normalize_input_path` 的 iOS 分支。
- Modify `src-tauri/src/state.rs` — 保持在 `tauri-shell` 下（`AppState` 只被 commands 使用）。
- Delete `src-tauri/gen/apple/Sources/sendsent/Picker.swift`。
- Delete `src-tauri/gen/apple/globalize_symbols.sh`。
- Modify `src-tauri/gen/apple/project.yml` — 构建脚本直连 cargo；deploymentTarget→17.0；删除 globalize 调用；移除 WebKit 依赖。
- Modify `src-tauri/gen/apple/sendsent_iOS/Info.plist` — 移除 `TaoSceneDelegate` scene 配置（沿用 SwiftUI 生命周期；Swift 侧在 Plan 2 加 `@main`）。
- Modify `AGENTS.md` — 记录原生架构、构建命令、FFI 约定。

---

### Task 1: Cargo feature 隔离 Tauri

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/build.rs`
- Modify: `src-tauri/src/lib.rs:1-56`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/state.rs`（加 gate）

- [ ] **Step 1: 改 `Cargo.toml`**

在 `[lib]` 之后加入 features；把 4 个 tauri 依赖改为可选：

```toml
[features]
default = ["tauri-shell"]
tauri-shell = ["dep:tauri", "dep:tauri-plugin-opener", "dep:tauri-plugin-dialog", "dep:tauri-plugin-fs"]

[dependencies]
tauri = { version = "2", features = [], optional = true }
tauri-plugin-opener = { version = "2", optional = true }
tauri-plugin-dialog = { version = "2", optional = true }
tauri-plugin-fs = { version = "2", optional = true }
```

删除整个 `[patch.crates-io]` 段（tao 补丁）。

- [ ] **Step 2: 改 `build.rs`**

```rust
fn main() {
    #[cfg(feature = "tauri-shell")]
    tauri_build::build();

    // 原生 iOS 构建不再链接 Swift 侧符号（Picker 已移除），无需 dynamic_lookup。
}
```

- [ ] **Step 3: gate `lib.rs` 的 shell 部分**

把文件头部模块声明改为：

```rust
pub mod proto;
pub mod discovery;
pub mod transfer;
pub mod store;
pub mod events;
pub mod history;

#[cfg(feature = "tauri-shell")]
pub mod state;
#[cfg(feature = "tauri-shell")]
pub mod commands;
#[cfg(all(feature = "tauri-shell", target_os = "android"))]
pub mod content_plugin;
#[cfg(all(feature = "tauri-shell", target_os = "android"))]
pub mod nsd_plugin;
```

把 `run()` 整段（含 `use tauri...`、`use state::AppState`、`use transfer::manager::SessionManager` 等仅供 run 使用的 import）用 `#[cfg(feature = "tauri-shell")]` 包住。具体做法：

```rust
#[cfg(feature = "tauri-shell")]
use tauri::{Emitter, Manager};
#[cfg(feature = "tauri-shell")]
use state::AppState;
// ... 其它 run-only imports 同样加 #[cfg(feature = "tauri-shell")]

#[cfg(feature = "tauri-shell")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() { /* 原内容不变 */ }
```

注意：`use discovery::{Discovery, PeerEvent};`、`use events::{name, TransferEvent};`、`use store::...`、`use transfer::manager::SessionManager;`、`use events::name;` 等只有 `run()` 用到 → 全部移入 `run()` 函数体内（`fn run() { use ...; ... }`）或加 gate。推荐移入函数体，避免未使用告警。

- [ ] **Step 4: gate `main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "tauri-shell")]
fn main() {
    sendsent_lib::run()
}

#[cfg(not(feature = "tauri-shell"))]
fn main() {}
```

- [ ] **Step 5: gate `state.rs`**

文件首行加 `#![cfg(feature = "tauri-shell")]`（或在 `lib.rs` 已 gate 模块即可；此处仅保险）。

- [ ] **Step 6: 验证默认构建不回归**

Run: `cd src-tauri && cargo check`
Expected: PASS（桌面 shell 正常）。

Run: `cd src-tauri && cargo build --lib --no-default-features`
Expected: PASS（不编译 tauri；`misc`/`ffi` 分别在 Task 2/3 引入并声明到 `lib.rs`，本任务不声明，避免缺文件）。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/build.rs src-tauri/src/lib.rs src-tauri/src/main.rs src-tauri/src/state.rs
git commit -m "refactor(ios): gate tauri shell behind feature"
```

---

### Task 2: 把纯函数迁到 `misc.rs`（含单测）

**Files:**
- Create: `src-tauri/src/misc.rs`
- Modify: `src-tauri/src/lib.rs`（新增 `pub mod misc;`）
- Modify: `src-tauri/src/commands.rs`（复用 misc；删除 `ios_picker` 模块与 `normalize_input_path`）
- Modify: `src-tauri/src/lib.rs`（从 `generate_handler!` 删除 `commands::ios_picker::pick_files_ios`）

- [ ] **Step 1: 写 `misc.rs` 与失败测试**

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MyAddress {
    pub interface: String,
    pub ip: String,
}

/// 枚举所有非 loopback / 非 link-local 的 IPv4 接口。
pub fn get_my_addresses() -> Result<Vec<MyAddress>, String> {
    let mut addrs: Vec<MyAddress> = Vec::new();
    let ifaces = get_if_addrs::get_if_addrs().map_err(|e| format!("enum ifaces: {e}"))?;
    for iface in ifaces {
        if iface.is_loopback() {
            continue;
        }
        if let get_if_addrs::IfAddr::V4(v4) = iface.addr {
            let ip = v4.ip;
            if ip.is_link_local() || ip.is_unspecified() {
                continue;
            }
            addrs.push(MyAddress { interface: iface.name.clone(), ip: ip.to_string() });
        }
    }
    addrs.sort_by(|a, b| a.ip.cmp(&b.ip));
    Ok(addrs)
}

/// 最小 percent-encoding（RFC 3986 unreserved 之外全部转义）。
pub fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

/// 把 QrCode 渲染为 PNG 字节。
pub fn render_qr_png(code: &qrcode::QrCode, size: u32) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Luma};
    let modules = code.width() as u32;
    let quiet = 4u32;
    let total = (modules + quiet * 2) * 10;
    let _ = size;
    let scale = (size as f32 / total as f32).clamp(0.5, 4.0) as u32;
    let scale = scale.max(1);
    let img_size = (modules + quiet * 2) * scale;
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::from_pixel(img_size, img_size, Luma([255u8]));
    let colors = code.to_colors();
    for y in 0..modules {
        for x in 0..modules {
            if colors[(y * modules + x) as usize] == qrcode::Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = (x + quiet) * scale + dx;
                        let py = (y + quiet) * scale + dy;
                        if px < img_size && py < img_size {
                            img.put_pixel(px, py, Luma([0u8]));
                        }
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(out)
}

/// 生成 sendsent:// 二维码 PNG 的 base64。
pub fn my_qr_base64(
    name: &str,
    port: u16,
    save_dir: &str,
    size: Option<u32>,
    ip: Option<String>,
) -> Result<String, String> {
    use base64::Engine;
    let sz = size.unwrap_or(300).clamp(128, 1024);
    let addrs = get_my_addresses().unwrap_or_default();
    let chosen = ip
        .filter(|s| addrs.iter().any(|a| a.ip == *s))
        .or_else(|| addrs.first().map(|a| a.ip.clone()))
        .unwrap_or_default();
    let payload = format!(
        "sendsent://{}?addr={}:{}&dir={}",
        urlencoding(name),
        chosen,
        port,
        urlencoding(save_dir)
    );
    let code = qrcode::QrCode::new(payload.as_bytes()).map_err(|e| format!("qr: {e}"))?;
    let png = render_qr_png(&code, sz)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(png))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoding_escapes_reserved() {
        assert_eq!(urlencoding("a b/c?d=1"), "a%20b%2Fc%3Fd%3D1");
        assert_eq!(urlencoding("safe-._~"), "safe-._~");
    }

    #[test]
    fn render_qr_png_has_png_magic() {
        let code = qrcode::QrCode::new(b"sendsent://x?addr=1.2.3.4:52225").unwrap();
        let png = render_qr_png(&code, 300).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}
```

- [ ] **Step 2: 运行测试确认通过**

Run: `cd src-tauri && cargo test --lib misc::`
Expected: 2 passed。

- [ ] **Step 3: 让 `commands.rs` 复用**

删除 `commands.rs` 里的 `get_my_addresses` 实体、`MyAddress`、`urlencoding`、`render_qr_png`，改为：

```rust
#[tauri::command]
pub fn get_my_addresses() -> Result<Vec<crate::misc::MyAddress>, String> {
    crate::misc::get_my_addresses()
}

#[tauri::command]
pub fn get_my_qr(state: State<'_, AppState>, size: Option<u32>, ip: Option<String>) -> Result<String, String> {
    crate::misc::my_qr_base64(
        &state.identity.name,
        state.port,
        &state.save_dir.to_string_lossy(),
        size,
        ip,
    )
}
```

同时删除 `commands.rs:297-324` 的 `normalize_input_path` / `hex_val`（Picker 已移除），并把 `send_files` 里 `.map(normalize_input_path)` 去掉（iOS 传纯路径，Android 传 content://）：

```rust
// send_files 顶部：iOS 直接是路径；Android content:// 分支保留
let files: Vec<String> = files;
```

（若 Android 需要 URL 规范化，则把 `normalize_input_path` 保留但仅在 `#[cfg(not(any(target_os="ios")))]` 下使用；保持桌面/Android 行为不变。）

- [ ] **Step 4: 删除 `ios_picker` 模块与 handler 条目**

删除 `commands.rs` 末尾的 `pub(crate) mod ios_picker { ... }`（`#[cfg(target_os="ios")]` 与 `#[cfg(not(target_os="ios"))]` 两个分支）。

在 `lib.rs` 的 `generate_handler![...]` 列表删除这一行：

```rust
commands::ios_picker::pick_files_ios,
```

（原生 iOS 改用 Swift `.fileImporter` 发送；桌面/Android 不使用该命令，故可整段移除。）

- [ ] **Step 5: 验证**

Run: `cd src-tauri && cargo check && cargo test --lib`
Expected: PASS。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/misc.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "refactor: extract helpers into misc; drop ios picker bridge"
```

---

### Task 3: `ffi.rs` 骨架 + init/identity/peers/addresses/qr

**Files:**
- Create: `src-tauri/src/ffi.rs`
- Modify: `src-tauri/src/lib.rs`（新增 `#[cfg(target_os = "ios")] pub mod ffi;`）

- [ ] **Step 1: 写 `ffi.rs`（Core + 只读 FFI）**

```rust
//! iOS 原生 FFI：SwiftUI 通过 C ABI 调用 Rust 核心。
#![cfg(target_os = "ios")]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;

use crate::discovery::{Discovery, PeerEvent};
use crate::discovery::ios_bonjour::BonjourDiscovery;
use crate::events::TransferEvent;
use crate::history::HistoryStore;
use crate::store::{self, Identity, TransferConfig};
use crate::transfer::manager::SessionManager;
use crate::transfer::tls::TlsConfig;

type EventCb = extern "C" fn(*const c_char);

pub struct Core {
    pub identity: Mutex<Identity>,
    pub data_dir: PathBuf,
    pub save_dir: PathBuf,
    pub port: u16,
    pub discovery: std::sync::Arc<dyn Discovery>,
    pub sessions: std::sync::Arc<SessionManager>,
    pub history: std::sync::Arc<AsyncMutex<HistoryStore>>,
    pub config: Mutex<TransferConfig>,
    pub runtime: tokio::runtime::Runtime,
    pub event_cb: Mutex<Option<EventCb>>,
}

static CORE: OnceLock<Core> = OnceLock::new();

fn core() -> Option<&'static Core> {
    CORE.get()
}

fn emit(core: &Core, json: String) {
    let cb = *core.event_cb.lock().unwrap();
    if let Some(cb) = cb {
        if let Ok(c) = CString::new(json) {
            cb(c.as_ptr());
        }
    }
}

unsafe fn cstr(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("null pointer".into());
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| e.to_string())
}

fn to_c(s: String) -> *mut c_char {
    CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn err(msg: impl Into<String>) -> *mut c_char {
    to_c(serde_json::json!({ "error": msg.into() }).to_string())
}

/// 统一释放 FFI 返回的字符串。
#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_init(
    data_dir: *const c_char,
    save_dir: *const c_char,
    port: u16,
    event_cb: Option<EventCb>,
) -> i32 {
    if CORE.get().is_some() {
        return 0;
    }
    let data_dir = match unsafe { cstr(data_dir) } {
        Ok(s) => PathBuf::from(s),
        Err(_) => return 1,
    };
    let save_dir = match unsafe { cstr(save_dir) } {
        Ok(s) => PathBuf::from(s),
        Err(_) => return 2,
    };
    if std::fs::create_dir_all(&data_dir).is_err() {
        return 3;
    }
    let _ = std::fs::create_dir_all(&save_dir);

    let _ = rustls::crypto::ring::default_provider().install_default();
    let platform = store::current_platform();
    let identity = match store::load_or_create(&data_dir, platform, "iPhone") {
        Ok(i) => i,
        Err(_) => return 4,
    };
    let config = store::load_or_create_transfer_config(&data_dir);
    let tls: TlsConfig = match crate::transfer::tls::load_or_generate_tls_config(&data_dir) {
        Ok(t) => t,
        Err(_) => return 5,
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(r) => r,
        Err(_) => return 6,
    };

    let (ptx, prx) = mpsc::unbounded_channel::<PeerEvent>();
    let discovery: std::sync::Arc<dyn Discovery> =
        std::sync::Arc::new(BonjourDiscovery::new(identity.clone(), port, ptx));
    let (ttx, trx) = mpsc::unbounded_channel::<TransferEvent>();
    let sessions = SessionManager::new(identity.clone(), ttx, tls);
    let history = std::sync::Arc::new(AsyncMutex::new(HistoryStore::load(&data_dir)));

    let core = Core {
        identity: Mutex::new(identity),
        data_dir,
        save_dir,
        port,
        discovery: discovery.clone(),
        sessions: sessions.clone(),
        history: history.clone(),
        config: Mutex::new(config),
        runtime,
        event_cb: Mutex::new(event_cb),
    };
    if CORE.set(core).is_err() {
        return 0;
    }
    let core = CORE.get().unwrap();

    crate::ffi::spawn_tasks(core, discovery, sessions, prx, trx, port);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_identity() -> *mut c_char {
    match core() {
        Some(c) => to_c(serde_json::to_string(&*c.identity.lock().unwrap()).unwrap()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_peers() -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let peers = c.runtime.block_on(c.discovery.peers());
    to_c(serde_json::to_string(&peers).unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_addresses() -> *mut c_char {
    match crate::misc::get_my_addresses() {
        Ok(a) => to_c(serde_json::to_string(&a).unwrap()),
        Err(e) => err(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_qr(size: u32, ip: *const c_char) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let ip = if ip.is_null() {
        None
    } else {
        unsafe { cstr(ip) }.ok()
    };
    let name = c.identity.lock().unwrap().name.clone();
    let save = c.save_dir.to_string_lossy().to_string();
    match crate::misc::my_qr_base64(&name, c.port, &save, Some(size), ip) {
        Ok(b64) => to_c(serde_json::json!(b64).to_string()),
        Err(e) => err(e),
    }
}

// spawn_tasks 在 Task 5 实现；此处先放占位以通过编译
pub(crate) fn spawn_tasks(
    _core: &'static Core,
    _discovery: std::sync::Arc<dyn Discovery>,
    _sessions: std::sync::Arc<SessionManager>,
    _prx: mpsc::UnboundedReceiver<PeerEvent>,
    _trx: mpsc::UnboundedReceiver<TransferEvent>,
    _port: u16,
) {
}
```

- [ ] **Step 2: 编译 iOS 目标（不链接）**

Run: `cd src-tauri && cargo build --lib --target aarch64-apple-ios --no-default-features`
Expected: PASS（`target/aarch64-apple-ios/debug/libsendsent_lib.a` 生成）。

- [ ] **Step 3: 默认构建不回归**

Run: `cd src-tauri && cargo check`
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ffi.rs
git commit -m "feat(ios): add ffi core skeleton (init/identity/peers/addresses/qr)"
```

---

### Task 4: 写操作 FFI（add/send/respond/history/config/rename）

**Files:**
- Modify: `src-tauri/src/ffi.rs`

- [ ] **Step 1: 追加 FFI 函数**

```rust
#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_add_peer(addr: *const c_char) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let addr = match unsafe { cstr(addr) } { Ok(s) => s, Err(e) => return err(e) };
    let sock: std::net::SocketAddr = match addr.parse() {
        Ok(s) => s,
        Err(e) => return err(format!("invalid address: {e}")),
    };
    match c.runtime.block_on(c.discovery.add_manual_peer(sock)) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_send(
    peer_id: *const c_char,
    files_json: *const c_char,
    secure: bool,
    verify: bool,
) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let peer_id = match unsafe { cstr(peer_id) } { Ok(s) => s, Err(e) => return err(e) };
    let files_json = match unsafe { cstr(files_json) } { Ok(s) => s, Err(e) => return err(e) };
    let files: Vec<String> = match serde_json::from_str(&files_json) {
        Ok(f) => f,
        Err(e) => return err(format!("bad files json: {e}")),
    };
    let peers = c.runtime.block_on(c.discovery.peers());
    let Some(peer) = peers.into_iter().find(|p| p.device_id == peer_id) else {
        return err("peer not found");
    };
    let cfg = c.config.lock().unwrap().clone();
    match c.sessions.start_send(peer, files, cfg, secure, verify) {
        Ok(id) => to_c(serde_json::json!({ "session_id": id.to_string() }).to_string()),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_respond(session_id: *const c_char, accept: bool) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let sid = match unsafe { cstr(session_id) } {
        Ok(s) => s,
        Err(e) => return err(e),
    };
    let uuid = match uuid::Uuid::parse_str(&sid) {
        Ok(u) => u,
        Err(e) => return err(format!("bad session id: {e}")),
    };
    let save = c.save_dir.clone();
    match c.runtime.block_on(c.sessions.respond(uuid, accept, save, None)) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_history() -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let items = c.runtime.block_on(async { c.history.lock().await.list() });
    to_c(serde_json::to_string(&items).unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_clear_history() -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    c.runtime.block_on(async { c.history.lock().await.clear() });
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_get_transfer_config() -> *mut c_char {
    match core() {
        Some(c) => to_c(serde_json::to_string(&*c.config.lock().unwrap()).unwrap()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_set_transfer_config(conns: u32, chunk_kb: u64, split_mb: u64) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let mut cfg = c.config.lock().unwrap();
    cfg.conns = conns;
    cfg.chunk_size = chunk_kb * 1024;
    cfg.split_threshold = split_mb * 1024 * 1024;
    let sanitized = cfg.clone().sanitized();
    *cfg = sanitized.clone();
    drop(cfg);
    let path = c.data_dir.join("transfer.json");
    match serde_json::to_string_pretty(&sanitized) {
        Ok(s) => {
            if let Err(e) = std::fs::write(&path, s) {
                return err(format!("write transfer.json: {e}"));
            }
            std::ptr::null_mut()
        }
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_set_display_name(name: *const c_char) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let name = match unsafe { cstr(name) } { Ok(s) => s, Err(e) => return err(e) };
    let name = name.trim().to_string();
    if name.is_empty() {
        return err("empty name");
    }
    {
        let mut id = c.identity.lock().unwrap();
        id.name = name.clone();
        let path = c.data_dir.join("identity.json");
        if let Ok(s) = serde_json::to_string_pretty(&*id) {
            let _ = std::fs::write(&path, s);
        }
    }
    match c.runtime.block_on(c.discovery.set_display_name(&name)) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(e.to_string()),
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo build --lib --target aarch64-apple-ios --no-default-features && cargo check`
Expected: PASS。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ffi.rs
git commit -m "feat(ios): add mutating ffi (send/respond/history/config/rename)"
```

---

### Task 5: 事件桥 + 进度节流

**Files:**
- Modify: `src-tauri/src/ffi.rs`

- [ ] **Step 1: 在 `misc.rs` 写节流单测（先失败）**

`Throttle` 放在平台无关的 `misc.rs`，这样 macOS 上可测。追加到 `misc.rs`（测试并入已有 `tests` 模块）：

```rust
#[derive(Default)]
pub struct Throttle {
    min: std::time::Duration,
    last: std::collections::HashMap<uuid::Uuid, std::time::Instant>,
}

impl Throttle {
    pub fn new(min: std::time::Duration) -> Self {
        Self { min, last: Default::default() }
    }
    /// 给定“当前时刻”判断是否放行（显式传入 now 以便测试）。
    pub fn allow_at(&mut self, id: uuid::Uuid, now: std::time::Instant) -> bool {
        match self.last.get(&id) {
            Some(t) if now.duration_since(*t) < self.min => false,
            _ => {
                self.last.insert(id, now);
                true
            }
        }
    }
}
```

在 `misc.rs` 的 `#[cfg(test)] mod tests` 中追加：

```rust
    #[test]
    fn throttle_limits_frequency_but_allows_first() {
        use std::time::{Duration, Instant};
        let id = uuid::Uuid::new_v4();
        let t0 = Instant::now();
        let mut th = Throttle::new(Duration::from_millis(100));
        assert!(th.allow_at(id, t0), "首次放行");
        assert!(!th.allow_at(id, t0 + Duration::from_millis(50)), "50ms 内拦截");
        assert!(th.allow_at(id, t0 + Duration::from_millis(100)), "到 100ms 放行");
    }
```

Run: `cd src-tauri && cargo test --lib misc::throttle`
Expected: PASS（Throttle 已定义）。

- [ ] **Step 2: 实现 `spawn_tasks`（改用 `crate::misc::Throttle`）**

```rust
pub(crate) fn spawn_tasks(
    core: &'static Core,
    discovery: std::sync::Arc<dyn Discovery>,
    sessions: std::sync::Arc<SessionManager>,
    mut prx: mpsc::UnboundedReceiver<PeerEvent>,
    mut trx: mpsc::UnboundedReceiver<TransferEvent>,
    port: u16,
) {
    // discovery
    core.runtime.spawn(async move {
        if let Err(e) = discovery.start().await {
            tracing::error!("discovery start failed: {e}");
        }
    });

    // peer events → callback
    core.runtime.spawn(async move {
        while let Some(ev) = prx.recv().await {
            let json = match ev {
                PeerEvent::Found(p) => serde_json::json!({ "kind": "peer_found", "peer": p }).to_string(),
                PeerEvent::Lost(id) => serde_json::json!({ "kind": "peer_lost", "device_id": id }).to_string(),
            };
            emit(core, json);
        }
    });

    // listener
    core.runtime.spawn(async move {
        if let Err(e) = sessions.clone().run_listener(port).await {
            tracing::error!("listener failed: {e}");
        }
    });

    // transfer events → callback（progress 节流）
    core.runtime.spawn(async move {
        let mut throttle = Throttle::new(Duration::from_millis(100));
        while let Some(ev) = trx.recv().await {
            match &ev {
                TransferEvent::Progress { session_id, .. } => {
                    if !throttle.allow_at(*session_id, Instant::now()) {
                        continue;
                    }
                }
                _ => {}
            }
            if let TransferEvent::Finished { session_id, .. } = &ev {
                sessions.cleanup_session(*session_id).await;
            }
            let json = serde_json::to_string(&ev).unwrap_or_else(|e| {
                serde_json::json!({ "kind": "error", "message": e.to_string() }).to_string()
            });
            emit(core, json);
        }
    });
}
```

- [ ] **Step 3: 处理 `Finished` 有终态必发**

`Finished` 不参与节流（上面只对 `Progress` 节流），因此终态必发；`Recorded` 也必发。确认无误。

- [ ] **Step 4: 测试 + 编译**

Run: `cd src-tauri && cargo test --lib ffi:: && cargo build --lib --target aarch64-apple-ios --no-default-features && cargo check`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ffi.rs
git commit -m "feat(ios): bridge core events to ffi callback with progress throttle"
```

---

### Task 6: 构建解耦（project.yml / Info.plist / 删 hack）

**Files:**
- Modify: `src-tauri/gen/apple/project.yml`
- Modify: `src-tauri/gen/apple/sendsent_iOS/Info.plist`
- Delete: `src-tauri/gen/apple/Sources/sendsent/Picker.swift`
- Delete: `src-tauri/gen/apple/globalize_symbols.sh`
- Modify: `src-tauri/gen/apple/sendsent.xcodeproj/project.pbxproj`（删 Picker.swift 引用 + globalize 调用；或重跑 `pnpm tauri ios init` 重新生成）

- [ ] **Step 1: 改 `project.yml`**

- `deploymentTarget.iOS: 15.0` → `17.0`。
- 替换 `preBuildScripts` 中 “Build Rust Code”：

```yaml
    preBuildScripts:
      - script: |
          export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
          CONFIG_FLAG="--release"
          PROFILE_DIR="release"
          if [ "${CONFIGURATION}" = "debug" ]; then
            CONFIG_FLAG=""
            PROFILE_DIR="debug"
          fi
          cargo build --manifest-path "$SRCROOT/../../Cargo.toml" \
            --lib --target aarch64-apple-ios --no-default-features $CONFIG_FLAG
          mkdir -p "$SRCROOT/Externals/arm64/${CONFIGURATION}"
          cp "$SRCROOT/../../target/aarch64-apple-ios/${PROFILE_DIR}/libsendsent_lib.a" \
             "$SRCROOT/Externals/arm64/${CONFIGURATION}/libapp.a"
        name: Build Rust Code
        basedOnDependencyAnalysis: false
        outputFiles:
          - $(SRCROOT)/Externals/arm64/${CONFIGURATION}/libapp.a
```

- 删除 `dependencies` 里的 `- sdk: WebKit.framework`（不再用 WebView）。
- 保留 `libapp.a` 依赖、UIKit/Metal 等。

- [ ] **Step 2: 改 `Info.plist`**

删除 `UIApplicationSceneManifest` 段（`TaoSceneDelegate` 配置）。SwiftUI `@main App`（Plan 2）自带场景。保留 `NSBonjourServices`、`NSLocalNetworkUsageDescription`、`UIFileSharingEnabled`、`LSSupportsOpeningDocumentsInPlace`。

- [ ] **Step 3: 删除文件**

```bash
rm src-tauri/gen/apple/Sources/sendsent/Picker.swift
rm src-tauri/gen/apple/globalize_symbols.sh
```

- [ ] **Step 4: 重新生成 Xcode 工程**

Run: `pnpm tauri ios init`
（用 `project.yml` 重新生成 `project.pbxproj`，从而移除 Picker.swift 引用与 globalize 脚本调用；若 init 不覆盖已有工程，则手动删除 pbxproj 中 `Picker.swift` 与 `globalize_symbols.sh` 相关条目。）

Expected: `src-tauri/gen/apple/sendsent.xcodeproj` 与 `project.yml` 一致，无 Picker/globalize 残留。

- [ ] **Step 5: 构建静态库（不经 xcodebuild）验证**

Run: `cd src-tauri && cargo build --lib --release --target aarch64-apple-ios --no-default-features`
Expected: PASS，产物 `target/aarch64-apple-ios/release/libsendsent_lib.a`。

- [ ] **Step 6: Commit**

```bash
git add -A src-tauri/gen/apple
git commit -m "build(ios): decouple from tauri; build rust staticlib directly"
```

> 注意：本任务后 iOS 应用尚无原生入口（SwiftUI 在 Plan 2 提供），因此**此时不要** `pnpm tauri ios build` 运行完整 app；只验证静态库产出。Plan 2 完成后才能整包运行。

---

### Task 7: 文档 + 全量验证

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: 更新 `AGENTS.md`**

在 iOS 开发段落追加：原生 SwiftUI 架构、`--no-default-features` 构建、FFI 函数与事件约定、`libapp.a` 来源（`libsendsent_lib.a`）、已移除 tao patch/scene hack/Picker.swift、Plan 2 说明。

- [ ] **Step 2: 全量验证**

Run: `cd src-tauri && cargo clippy --all-targets -- -D warnings`
Expected: PASS（桌面默认 feature）。

Run: `cd src-tauri && cargo test`
Expected: PASS（含 `tests/loopback.rs`）。

Run: `cd src-tauri && cargo build --lib --target aarch64-apple-ios --no-default-features`
Expected: PASS。

Run: `cd src-tauri && cargo build --lib --target aarch64-linux-android`（若配置了 Android target）
Expected: PASS（默认 feature，Android 行为不变）。

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "docs: document native iOS architecture and ffi"
```

---

## Self-Review 记录

- **Spec 覆盖**：spec §5（FFI/事件/线程）→ Task 3/4/5；§5.1（feature gate）→ Task 1；§7（构建）→ Task 6；§8（迁移删除）→ Task 2/6；§10（验证）→ Task 6/7。SwiftUI（§6）→ Plan 2。
- **无占位**：本计划所有代码步骤均给出完整代码与命令。
- **类型一致**：`Core`/`Throttle`/`spawn_tasks`/`my_qr_base64`/`get_my_addresses` 在定义与调用处签名一致；事件 tag 统一为 `kind`。
- **已知风险**：`#[unsafe(no_mangle)]` 需 edition 2024（已确认 `edition = "2024"`）；`cargo test` 会编译 `ffi.rs` 的 `#[cfg(test)]`（仅 iOS target 编译 `ffi.rs`，桌面测试不受影响）。
