# Android 原生 — Plan 1：Rust engine + JNI + Nsd/SAF 桥 + 构建解耦

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rust 核心在 Android 以 `--no-default-features` 编译为 `libsendsent_lib.so`，通过 JNI 暴露给 Kotlin；发现由 Kotlin `NsdManager` 推给 Rust；文件用 SAF fd 零拷贝；去掉 Tauri Android 运行时。

**Architecture:** 把 iOS `ffi.rs` 的 `Core` 抽到平台无关的 `engine.rs`（全局 `OnceLock<Core>` + 事件队列/`EventSink`）。iOS `ffi.rs` 与 Android `jni_bridge.rs` 都是薄包装。发现新增 `discovery::android_native`（由 Kotlin push 驱动）。事件：iOS 用 `EventSink` 回调；Android 不用 sink → 入队，Kotlin 轮询 `nativePollEvents`。

**Tech Stack:** Rust (`jni` crate)、Kotlin、Jetpack Compose(Material3)、Android Gradle + 自定义 cargo `Exec` 任务、`NsdManager`、SAF、CameraX/ZXing（扫码在 Plan 2）。

**范围：** Plan 1 只做 Rust + JNI + 平台桥 + 构建 + 最小 Compose 壳（能显示设备列表证明桥通）。完整界面在 Plan 2。桌面/iOS 不得回归。

---

## 文件结构

- Create `src-tauri/src/engine.rs` — 平台无关 Core + ops + 事件队列/sink。
- Modify `src-tauri/src/ffi.rs` — 变薄：C ABI 包装 engine，提供 iOS event sink。
- Create `src-tauri/src/jni_bridge.rs` — `#[cfg(target_os="android")]` JNI 导出。
- Create `src-tauri/src/discovery/android_native.rs` — Kotlin push 驱动的 Discovery。
- Modify `src-tauri/src/discovery/mod.rs` — 注册模块 + trait 加 `on_service`/`on_service_lost` 默认方法。
- Modify `src-tauri/Cargo.toml` — `jni` 依赖（android）。
- Modify `src-tauri/src/lib.rs` — 声明 `engine`/`jni_bridge`。
- Android：`gen/android/app/src/main/java/com/mankong/sendsent/{Native.kt,NsdBridge.kt,SafPicker.kt,Core.kt,MainActivity.kt}`、`ui/App.kt`（最小）。
- Android：`app/build.gradle.kts`、`AndroidManifest.xml`、`settings.gradle.kts`（去 Tauri）。
- Delete：`gen/android/app/src/main/java/com/mankong/sendsent/{NsdPlugin.kt,ContentPlugin.kt}`、`generated/`、`tauri.build.gradle.kts`（若存在）。

---

### Task 1: 抽出 `engine.rs`（保持 iOS 行为）

**Files:** Create `src-tauri/src/engine.rs`; Modify `src-tauri/src/ffi.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: 新建 `engine.rs`**，把 `ffi.rs` 的 `Core`/`OnceLock`/各 ops 迁移过来，改成平台无关：

```rust
#![allow(dead_code)]
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;

use crate::discovery::{Discovery, PeerEvent, Peer};
use crate::events::TransferEvent;
use crate::history::HistoryStore;
use crate::store::{self, Identity, TransferConfig};
use crate::transfer::manager::SessionManager;
use crate::transfer::tls::TlsConfig;

pub type EventSink = Box<dyn Fn(String) + Send + Sync>;

pub struct Core {
    pub identity: Mutex<Identity>,
    pub data_dir: PathBuf,
    pub save_dir: PathBuf,
    pub port: u16,
    pub discovery: Arc<dyn Discovery>,
    pub sessions: Arc<SessionManager>,
    pub history: Arc<AsyncMutex<HistoryStore>>,
    pub config: Mutex<TransferConfig>,
    pub runtime: tokio::runtime::Runtime,
    pub sink: Mutex<Option<EventSink>>,
    pub queue: Mutex<VecDeque<String>>,
}

static CORE: OnceLock<Core> = OnceLock::new();
const QUEUE_MAX: usize = 512;

pub fn core() -> Option<&'static Core> { CORE.get() }

pub fn push_event(core: &Core, json: String) {
    let sink = core.sink.lock().unwrap();
    if let Some(f) = sink.as_ref() { f(json); return; }
    drop(sink);
    let mut q = core.queue.lock().unwrap();
    if q.len() >= QUEUE_MAX { q.pop_front(); }
    q.push_back(json);
}

/// Kotlin 轮询：取出并清空事件（JSON 字符串数组）。
pub fn poll_events() -> String {
    let Some(c) = core() else { return "[]".into() };
    let mut q = c.queue.lock().unwrap();
    let v: Vec<String> = q.drain(..).collect();
    serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
}

pub fn init(data_dir: PathBuf, save_dir: PathBuf, port: u16, sink: Option<EventSink>) -> i32 {
    if CORE.get().is_some() { return 0; }
    if std::fs::create_dir_all(&data_dir).is_err() { return 3; }
    let _ = std::fs::create_dir_all(&save_dir);
    let _ = rustls::crypto::ring::default_provider().install_default();

    let identity = match store::load_or_create(&data_dir, store::current_platform(), "device") {
        Ok(i) => i, Err(_) => return 4,
    };
    let config = store::load_or_create_transfer_config(&data_dir);
    let tls: TlsConfig = match crate::transfer::tls::load_or_generate_tls_config(&data_dir) {
        Ok(t) => t, Err(_) => return 5,
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(r) => r, Err(_) => return 6,
    };

    let (ptx, prx) = mpsc::unbounded_channel::<PeerEvent>();
    let discovery: Arc<dyn Discovery> = make_discovery(identity.clone(), port, ptx);
    let (ttx, trx) = mpsc::unbounded_channel::<TransferEvent>();
    let sessions = SessionManager::new(identity.clone(), ttx, tls);
    let history = Arc::new(AsyncMutex::new(HistoryStore::load(&data_dir)));

    let core = Core {
        identity: Mutex::new(identity), data_dir, save_dir, port,
        discovery: discovery.clone(), sessions: sessions.clone(), history,
        config: Mutex::new(config), runtime, sink: Mutex::new(sink),
        queue: Mutex::new(VecDeque::new()),
    };
    if CORE.set(core).is_err() { return 0; }
    let core = CORE.get().unwrap();
    spawn_tasks(core, discovery, sessions, prx, trx, port);
    0
}

#[cfg(target_os = "ios")]
fn make_discovery(id: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Arc<dyn Discovery> {
    Arc::new(crate::discovery::ios_bonjour::BonjourDiscovery::new(id, port, tx))
}
#[cfg(target_os = "android")]
fn make_discovery(id: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Arc<dyn Discovery> {
    Arc::new(crate::discovery::android_native::AndroidNativeDiscovery::new(id, port, tx))
}
#[cfg(all(not(target_os = "ios"), not(target_os = "android")))]
fn make_discovery(id: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Arc<dyn Discovery> {
    Arc::new(crate::discovery::mdns::MdnsDiscovery::new(id, port, tx))
}

fn spawn_tasks(core: &'static Core, discovery: Arc<dyn Discovery>, sessions: Arc<SessionManager>,
               mut prx: mpsc::UnboundedReceiver<PeerEvent>, mut trx: mpsc::UnboundedReceiver<TransferEvent>, port: u16) {
    core.runtime.spawn(async move { let _ = discovery.start().await; });
    core.runtime.spawn(async move {
        while let Some(ev) = prx.recv().await {
            let json = match ev {
                PeerEvent::Found(p) => serde_json::json!({"kind":"peer_found","peer":p}).to_string(),
                PeerEvent::Lost(id) => serde_json::json!({"kind":"peer_lost","device_id":id}).to_string(),
            };
            push_event(core, json);
        }
    });
    let ls = sessions.clone();
    core.runtime.spawn(async move { let _ = ls.run_listener(port).await; });
    core.runtime.spawn(async move {
        let mut throttle = crate::misc::Throttle::new(std::time::Duration::from_millis(100));
        while let Some(ev) = trx.recv().await {
            if let TransferEvent::Progress { session_id, .. } = &ev {
                if !throttle.allow_at(*session_id, std::time::Instant::now()) { continue; }
            }
            if let TransferEvent::Finished { session_id, .. } = &ev { sessions.cleanup_session(*session_id).await; }
            push_event(core, serde_json::to_string(&ev).unwrap_or_default());
        }
    });
}

// ── ops（从 ffi.rs 迁移，返回 JSON/Result，不再关心 C/JNI）──
pub fn identity_json() -> String { serde_json::to_string(&*core().unwrap().identity.lock().unwrap()).unwrap() }
pub fn peers_json() -> String { let c = core().unwrap(); serde_json::to_string(&c.runtime.block_on(c.discovery.peers())).unwrap() }
pub fn addresses_json() -> Result<String, String> { crate::misc::get_my_addresses().map(|a| serde_json::to_string(&a).unwrap()) }
pub fn qr_json(size: u32, ip: Option<String>) -> Result<String, String> {
    let c = core().unwrap();
    let name = c.identity.lock().unwrap().name.clone();
    let save = c.save_dir.to_string_lossy().to_string();
    crate::misc::my_qr_base64(&name, c.port, &save, Some(size), ip).map(|b| serde_json::json!(b).to_string())
}
pub fn add_peer(addr: &str) -> Result<(), String> {
    let c = core().unwrap();
    let sock: std::net::SocketAddr = addr.parse().map_err(|e| format!("invalid address: {e}"))?;
    c.runtime.block_on(c.discovery.add_manual_peer(sock)).map_err(|e| e.to_string())
}
pub fn send(peer_id: &str, files_json: &str) -> Result<String, String> {
    let c = core().unwrap();
    let files: Vec<String> = serde_json::from_str(files_json).map_err(|e| e.to_string())?;
    let peers = c.runtime.block_on(c.discovery.peers());
    let peer = peers.into_iter().find(|p| p.device_id == peer_id).ok_or("peer not found")?;
    let cfg = c.config.lock().unwrap().clone();
    c.sessions.start_send(peer, files, cfg, true, false).map(|id| serde_json::json!({"session_id": id.to_string()}).to_string()).map_err(|e| e.to_string())
}
pub fn respond(session_id: &str, accept: bool) -> Result<(), String> {
    let c = core().unwrap();
    let id = uuid::Uuid::parse_str(session_id).map_err(|e| e.to_string())?;
    let save = c.save_dir.clone();
    c.runtime.block_on(c.sessions.respond(id, accept, save, None)).map_err(|e| e.to_string())
}
pub fn history_json() -> String { let c = core().unwrap(); serde_json::to_string(&c.runtime.block_on(async { c.history.lock().await.list() })).unwrap() }
pub fn delete_history(id: &str) { let c = core().unwrap(); c.runtime.block_on(async { c.history.lock().await.remove(id) }); }
pub fn clear_history() { let c = core().unwrap(); c.runtime.block_on(async { c.history.lock().await.clear() }); }
pub fn get_config_json() -> String { serde_json::to_string(&*core().unwrap().config.lock().unwrap()).unwrap() }
pub fn set_config(conns: u32, chunk_kb: u64, split_mb: u64) -> Result<(), String> {
    let c = core().unwrap();
    let sanitized = { let mut cfg = c.config.lock().unwrap(); cfg.conns = conns; cfg.chunk_size = chunk_kb*1024; cfg.split_threshold = split_mb*1024*1024; let s = cfg.clone().sanitized(); *cfg = s.clone(); s };
    std::fs::write(c.data_dir.join("transfer.json"), serde_json::to_string_pretty(&sanitized).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
pub fn set_display_name(name: &str) -> Result<(), String> {
    let c = core().unwrap();
    let name = name.trim();
    if name.is_empty() { return Err("empty name".into()); }
    { let mut id = c.identity.lock().unwrap(); id.name = name.to_string(); let _ = std::fs::write(c.data_dir.join("identity.json"), serde_json::to_string_pretty(&*id).unwrap()); }
    c.runtime.block_on(c.discovery.set_display_name(name)).map_err(|e| e.to_string())
}
/// Kotlin NsdManager 推来的服务（Android）。
pub fn on_service(name: &str, host: &str, port: u16, txt_json: &str) {
    let Some(c) = core() else { return };
    let txt: std::collections::HashMap<String, String> = serde_json::from_str(txt_json).unwrap_or_default();
    c.runtime.block_on(c.discovery.on_service(name, host, port, &txt));
}
pub fn on_service_lost(name: &str) {
    let Some(c) = core() else { return };
    c.runtime.block_on(c.discovery.on_service_lost(name));
}
```

- [ ] **Step 2: 改 `ffi.rs` 为薄包装**，删除 `Core`/`spawn_tasks`/ops，改为调用 `engine`：

```rust
// 例：init
#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_init(data_dir: *const c_char, save_dir: *const c_char, port: u16, event_cb: Option<EventCb>) -> i32 {
    let d = match unsafe { cstr(data_dir) } { Ok(s)=>s, Err(_)=>return 1 };
    let s = match unsafe { cstr(save_dir) } { Ok(s)=>s, Err(_)=>return 2 };
    let sink: Option<crate::engine::EventSink> = event_cb.map(|cb| {
        Box::new(move |json: String| { if let Ok(c) = CString::new(json) { cb(c.as_ptr()); } }) as crate::engine::EventSink
    });
    crate::engine::init(PathBuf::from(d), PathBuf::from(s), port, sink)
}
// 其余各函数：`to_c(engine::peers_json())`、`ffi_try(engine::add_peer(&addr))` 等。
```
`ffi_try`：`Result<(),String>` → null 或 err-json；`ffi_res`：`Result<String,String>` → JSON 或 err-json。

- [ ] **Step 3: `lib.rs` 声明**

```rust
pub mod engine;
#[cfg(target_os = "ios")] pub mod ffi;
#[cfg(target_os = "android")] pub mod jni_bridge;
```

- [ ] **Step 4: 验证（iOS + 桌面不回归）**

Run: `cd src-tauri && cargo check && cargo build --lib --target aarch64-apple-ios --no-default-features`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/engine.rs src-tauri/src/ffi.rs src-tauri/src/lib.rs
git commit -m "refactor: extract platform-agnostic engine from ios ffi"
```

---

### Task 2: `discovery::android_native` + trait 方法

**Files:** Create `src-tauri/src/discovery/android_native.rs`; Modify `src-tauri/src/discovery/mod.rs`

- [ ] **Step 1: trait 加方法（默认 no-op）**

```rust
// discovery/mod.rs, in trait Discovery
async fn on_service(&self, name: &str, host: &str, port: u16, txt: &std::collections::HashMap<String, String>) {
    let _ = (name, host, port, txt);
}
async fn on_service_lost(&self, name: &str) { let _ = name; }
```

- [ ] **Step 2: `android_native.rs`**

```rust
//! Android: 发现由 Kotlin NsdManager 驱动,这里只维护注册表。
use crate::discovery::{platform_from_str, Discovery, Peer, PeerEvent, PeerRegistry, Platform};
use crate::store::Identity;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;

pub struct AndroidNativeDiscovery {
    registry: Arc<Mutex<PeerRegistry>>,
    by_name: Arc<Mutex<HashMap<String, String>>>, // serviceName -> device_id
    tx: mpsc::UnboundedSender<PeerEvent>,
}

impl AndroidNativeDiscovery {
    pub fn new(_identity: Identity, _port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Self {
        Self { registry: Arc::new(Mutex::new(PeerRegistry::new())), by_name: Arc::new(Mutex::new(HashMap::new())), tx }
    }
}

#[async_trait]
impl Discovery for AndroidNativeDiscovery {
    async fn start(&self) -> Result<()> { Ok(()) } // Kotlin 驱动
    async fn peers(&self) -> Vec<Peer> { self.registry.lock().unwrap().list() }
    async fn set_display_name(&self, _n: &str) -> Result<()> { Ok(()) }
    async fn add_manual_peer(&self, addr: SocketAddr) -> Result<()> {
        let id = format!("manual-{}", uuid::Uuid::new_v4());
        let p = Peer { device_id: id, name: format!("{}:{}", addr.ip(), addr.port()),
            platform: Platform::Unknown, proto_version: 1, addrs: vec![addr], port: addr.port(), last_seen_ms: 0 };
        if let Some(ev) = self.registry.lock().unwrap().upsert(Instant::now(), p) { let _ = self.tx.send(ev); }
        Ok(())
    }
    async fn on_service(&self, sname: &str, host: &str, port: u16, txt: &HashMap<String, String>) {
        let id = txt.get("id").cloned().unwrap_or_else(|| sname.to_string());
        let name = txt.get("name").cloned().unwrap_or_else(|| sname.to_string());
        let platform = txt.get("plat").map(|p| platform_from_str(p)).unwrap_or(Platform::Unknown);
        let addr: SocketAddr = match format!("{host}:{port}").parse() { Ok(a) => a, Err(_) => return };
        let p = Peer { device_id: id.clone(), name, platform, proto_version: 1, addrs: vec![addr], port, last_seen_ms: 0 };
        self.by_name.lock().unwrap().insert(sname.to_string(), id);
        if let Some(ev) = self.registry.lock().unwrap().upsert(Instant::now(), p) { let _ = self.tx.send(ev); }
    }
    async fn on_service_lost(&self, sname: &str) {
        let id = self.by_name.lock().unwrap().remove(sname);
        if let Some(id) = id {
            if let Some(ev) = self.registry.lock().unwrap().remove(&id) { let _ = self.tx.send(ev); }
        }
    }
}
```

- [ ] **Step 3: `mod.rs` 注册**

```rust
#[cfg(target_os = "android")]
pub mod android_native;
```

- [ ] **Step 4: 验证**

Run: `cd src-tauri && cargo check`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/discovery/mod.rs src-tauri/src/discovery/android_native.rs src-tauri/src/engine.rs
git commit -m "feat(android): discovery driven by kotlin NsdManager push"
```

---

### Task 3: `jni_bridge.rs`（JNI 导出）

**Files:** Create `src-tauri/src/jni_bridge.rs`; Modify `src-tauri/Cargo.toml`

- [ ] **Step 1: Cargo 依赖**

```toml
[target.'cfg(target_os = "android")'.dependencies]
android_logger = "0.14"
jni = "0.21"
```

- [ ] **Step 2: `jni_bridge.rs`**

```rust
#![cfg(target_os = "android")]
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jint, jstring};
use jni::JNIEnv;

use crate::engine;

fn js(env: &mut JNIEnv, s: String) -> jstring {
    env.new_string(s).map(|j| j.into_raw()).unwrap_or(std::ptr::null_mut())
}
fn s_arg(env: &mut JNIEnv, s: &JString) -> Option<String> { env.get_string(s).ok().map(|g| g.into()) }
fn err(env: &mut JNIEnv, msg: impl Into<String>) -> jstring {
    js(env, serde_json::json!({"error": msg.into()}).to_string())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeInit(
    mut env: JNIEnv, _this: JObject, data_dir: JString, save_dir: JString, port: jint) -> jint {
    let (Some(d), Some(s)) = (s_arg(&mut env, &data_dir), s_arg(&mut env, &save_dir)) else { return 1 };
    engine::init(std::path::PathBuf::from(d), std::path::PathBuf::from(s), port as u16, None)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativePollEvents(mut env: JNIEnv, _this: JObject) -> jstring {
    js(&mut env, engine::poll_events())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeIdentity(mut env: JNIEnv, _this: JObject) -> jstring {
    js(&mut env, engine::identity_json())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativePeers(mut env: JNIEnv, _this: JObject) -> jstring {
    js(&mut env, engine::peers_json())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeHistory(mut env: JNIEnv, _this: JObject) -> jstring {
    js(&mut env, engine::history_json())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeAddresses(mut env: JNIEnv, _this: JObject) -> jstring {
    match engine::addresses_json() { Ok(s) => js(&mut env, s), Err(e) => err(&mut env, e) }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeQr(mut env: JNIEnv, _this: JObject, size: jint) -> jstring {
    match engine::qr_json(size as u32, None) { Ok(s) => js(&mut env, s), Err(e) => err(&mut env, e) }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeGetConfig(mut env: JNIEnv, _this: JObject) -> jstring {
    js(&mut env, engine::get_config_json())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeAddPeer(mut env: JNIEnv, _this: JObject, addr: JString) -> jstring {
    let Some(a) = s_arg(&mut env, &addr) else { return err(&mut env, "bad addr") };
    match engine::add_peer(&a) { Ok(()) => std::ptr::null_mut(), Err(e) => err(&mut env, e) }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSend(mut env: JNIEnv, _this: JObject, peer: JString, files: JString) -> jstring {
    let (Some(p), Some(f)) = (s_arg(&mut env, &peer), s_arg(&mut env, &files)) else { return err(&mut env, "bad args") };
    match engine::send(&p, &f) { Ok(s) => js(&mut env, s), Err(e) => err(&mut env, e) }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeRespond(mut env: JNIEnv, _this: JObject, sid: JString, accept: jboolean) -> jstring {
    let Some(s) = s_arg(&mut env, &sid) else { return err(&mut env, "bad sid") };
    match engine::respond(&s, accept != 0) { Ok(()) => std::ptr::null_mut(), Err(e) => err(&mut env, e) }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeDeleteHistory(mut env: JNIEnv, _this: JObject, sid: JString) -> jstring {
    if let Some(s) = s_arg(&mut env, &sid) { engine::delete_history(&s); }
    std::ptr::null_mut()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeClearHistory(_env: JNIEnv, _this: JObject) {
    engine::clear_history();
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSetConfig(mut env: JNIEnv, _this: JObject, conns: jint, chunk_kb: jlong, split_mb: jlong) -> jstring {
    match engine::set_config(conns as u32, chunk_kb as u64, split_mb as u64) { Ok(()) => std::ptr::null_mut(), Err(e) => err(&mut env, e) }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSetDisplayName(mut env: JNIEnv, _this: JObject, name: JString) -> jstring {
    let Some(n) = s_arg(&mut env, &name) else { return err(&mut env, "bad name") };
    match engine::set_display_name(&n) { Ok(()) => std::ptr::null_mut(), Err(e) => err(&mut env, e) }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeOnService(mut env: JNIEnv, _this: JObject, name: JString, host: JString, port: jint, txt: JString) {
    let (Some(n), Some(h), Some(t)) = (s_arg(&mut env, &name), s_arg(&mut env, &host), s_arg(&mut env, &txt)) else { return };
    engine::on_service(&n, &h, port as u16, &t);
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeOnServiceLost(mut env: JNIEnv, _this: JObject, name: JString) {
    if let Some(n) = s_arg(&mut env, &name) { engine::on_service_lost(&n); }
}
```

- [ ] **Step 3: 验证**

Run: `cargo build --lib --target aarch64-linux-android --no-default-features`（需 NDK linker env，见 AGENTS.md）
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/jni_bridge.rs
git commit -m "feat(android): jni bridge over shared engine"
```

---

### Task 4: Kotlin 桥（Native / NsdBridge / SafPicker / Core / 最小 App）

**Files:** Create `gen/android/app/src/main/java/com/mankong/sendsent/{Native.kt,NsdBridge.kt,SafPicker.kt,Core.kt,MainActivity.kt}`, `ui/App.kt`

- [ ] **Step 1: `Native.kt`**

```kotlin
package com.mankong.sendsent

object Native {
    init { System.loadLibrary("sendsent_lib") }
    external fun nativeInit(dataDir: String, saveDir: String, port: Int): Int
    external fun nativePollEvents(): String
    external fun nativeIdentity(): String
    external fun nativePeers(): String
    external fun nativeHistory(): String
    external fun nativeAddresses(): String
    external fun nativeQr(size: Int): String
    external fun nativeGetConfig(): String
    external fun nativeAddPeer(addr: String): String?
    external fun nativeSend(peerId: String, filesJson: String): String?
    external fun nativeRespond(sessionId: String, accept: Boolean): String?
    external fun nativeDeleteHistory(sessionId: String): String?
    external fun nativeClearHistory()
    external fun nativeSetConfig(conns: Int, chunkKb: Long, splitMb: Long): String?
    external fun nativeSetDisplayName(name: String): String?
    external fun nativeOnService(name: String, host: String, port: Int, txtJson: String)
    external fun nativeOnServiceLost(name: String)
}
```
> Kotlin `object` 的方法是实例方法；Rust 第二参数为 `JObject`（已按此声明）。

- [ ] **Step 2: `NsdBridge.kt`**（NsdManager 注册 + 浏览 → 推给 Rust）

```kotlin
package com.mankong.sendsent

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import org.json.JSONObject

class NsdBridge(private val ctx: Context) {
    private val nsd = ctx.getSystemService(Context.NSD_SERVICE) as NsdManager
    private var registration: NsdManager.RegistrationListener? = null
    private var discovery: NsdManager.DiscoveryListener? = null

    fun register(name: String, id: String, plat: String, port: Int, ip: String) {
        val info = NsdServiceInfo().apply {
            serviceName = name; serviceType = SERVICE_TYPE; this.port = port
            setAttribute("id", id); setAttribute("name", name); setAttribute("plat", plat)
            setAttribute("v", "1"); setAttribute("port", port.toString())
            if (ip.isNotEmpty()) setAttribute("ip", ip)
        }
        registration?.let { runCatching { nsd.unregisterService(it) } }
        val l = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(i: NsdServiceInfo) {}
            override fun onRegistrationFailed(i: NsdServiceInfo, e: Int) {}
            override fun onServiceUnregistered(i: NsdServiceInfo) {}
            override fun onUnregistrationFailed(i: NsdServiceInfo, e: Int) {}
        }
        registration = l
        nsd.registerService(info, NsdManager.PROTOCOL_DNS_SD, l)
    }

    fun browse() {
        if (discovery != null) return
        val l = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(t: String) {}
            override fun onDiscoveryStopped(t: String) {}
            override fun onStartDiscoveryFailed(t: String, e: Int) {}
            override fun onStopDiscoveryFailed(t: String, e: Int) {}
            override fun onServiceFound(s: NsdServiceInfo) {
                runCatching {
                    nsd.resolveService(s, object : NsdManager.ResolveListener {
                        override fun onResolveFailed(i: NsdServiceInfo, e: Int) {}
                        override fun onServiceResolved(i: NsdServiceInfo) {
                            val host = i.host?.hostAddress ?: return
                            val txt = JSONObject()
                            for ((k, v) in i.attributes) txt.put(k, String(v, Charsets.UTF_8))
                            Native.nativeOnService(i.serviceName, host, i.port, txt.toString())
                        }
                    })
                }
            }
            override fun onServiceLost(s: NsdServiceInfo) { Native.nativeOnServiceLost(s.serviceName) }
        }
        discovery = l
        nsd.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, l)
    }

    companion object { private const val SERVICE_TYPE = "_sendsent._tcp" }
}
```

- [ ] **Step 3: `SafPicker.kt`**（打开 URI → 保留 PFD → 组 filesJson）

```kotlin
package com.mankong.sendsent

import android.content.Context
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap

class SafPicker(private val ctx: Context) {
    private val keep = ConcurrentHashMap<Int, ParcelFileDescriptor>()

    /** 由 Activity 的 OpenMultipleDocuments 回调调用;返回 filesJson=[{"fd":N,"name":"..."}] */
    fun filesJson(uris: List<Uri>): String {
        val arr = JSONArray()
        for (u in uris) {
            val pfd = ctx.contentResolver.openFileDescriptor(u, "r") ?: continue
            keep[pfd.fd] = pfd
            val name = displayName(u) ?: "file"
            arr.put(JSONObject().put("fd", pfd.fd).put("name", name))
        }
        return arr.toString()
    }

    private fun displayName(u: Uri): String? {
        ctx.contentResolver.query(u, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
            if (c.moveToFirst()) return c.getString(0)
        }
        return null
    }
}
```
> Rust `engine::send` 需支持 `[{"fd","name"}]`：在 Android 构建下若 `files_json` 是该形状，则为每个 fd 建 `/proc/self/fd/<fd>` 符号链接（复用 `commands::send_files` 的 Android 逻辑）。**Step 4 实现。**

- [ ] **Step 4: `engine::send` 支持 fd JSON（Android）**

在 `engine.rs` 增加：
```rust
#[cfg(target_os = "android")]
pub fn resolve_files(files_json: &str) -> Result<Vec<String>, String> {
    #[derive(serde::Deserialize)]
    struct FdFile { fd: i32, name: String }
    let parsed: Vec<serde_json::Value> = serde_json::from_str(files_json).map_err(|e| e.to_string())?;
    // 若为字符串数组则原样返回
    if parsed.iter().all(|v| v.is_string()) {
        return Ok(parsed.into_iter().map(|v| v.as_str().unwrap().to_string()).collect());
    }
    let fds: Vec<FdFile> = serde_json::from_str(files_json).map_err(|e| e.to_string())?;
    let base = std::env::temp_dir().join("sendsent-fds");
    let _ = std::fs::create_dir_all(&base);
    let mut out = Vec::new();
    for f in fds {
        let safe: String = f.name.chars().map(|c| if c.is_alphanumeric() || c=='.'||c=='-'||c=='_' {c} else {'_'}).collect();
        let dir = base.join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let link = dir.join(if safe.is_empty() { "file".into() } else { safe });
        std::os::unix::fs::symlink(format!("/proc/self/fd/{}", f.fd), &link).map_err(|e| e.to_string())?;
        out.push(link.to_string_lossy().into_owned());
    }
    Ok(out)
}
```
并让 `engine::send` 在 Android 下先 `let files = resolve_files(files_json)?;` 再传给 `start_send`（非 Android 直接解析字符串数组）。

- [ ] **Step 5: `Core.kt`（StateFlow）+ `MainActivity.kt` + `ui/App.kt`（最小）**

- `Core`：`MutableStateFlow` peers/progress/history/request/identity/addresses/toast；`start()` 轮询 `Native.nativePollEvents()`；解析 `kind` 更新。
- `MainActivity : ComponentActivity`：`setContent { App(core) }`；启动时算 `dataDir`/`saveDir`（`filesDir` / `getExternalFilesDir(DIRECTORY_DOCUMENTS)`）、`Native.nativeInit(...)`、`NsdBridge.register(...)`（用 `nativeIdentity()` 的 JSON）、`NsdBridge.browse()`。
- `App`：`Scaffold` + 一个 `LazyColumn` 列出 `Native.nativePeers()` 解析出的设备（证明桥通）。

- [ ] **Step 6: 构建 `libsendsent_lib.so` + 跑起来**

（先完成 Task 5 的 Gradle；本步骤在 Task 5 后执行。）Expected：App 启动，列表显示发现到的设备。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/engine.rs src-tauri/gen/android/app/src/main/java/com/mankong/sendsent
git commit -m "feat(android): kotlin bridges (native/nsd/saf) + minimal compose shell"
```

---

### Task 5: 去 Tauri + Gradle 直连 cargo + Compose 依赖

**Files:** Modify `gen/android/app/build.gradle.kts`, `gen/android/app/src/main/AndroidManifest.xml`, `gen/android/settings.gradle.kts`（若含 tauri 引用）；Delete `gen/android/app/src/main/java/com/mankong/sendsent/{NsdPlugin.kt,ContentPlugin.kt}`、`generated/`、`tauri.build.gradle.kts`

- [ ] **Step 1: `build.gradle.kts`**

- 删 `id("rust")`、`apply(from = "tauri.build.gradle.kts")`、`tauri.properties`。
- 加 Compose（Kotlin 2.x：`id("org.jetbrains.kotlin.plugin.compose")`）：
```kotlin
android { buildFeatures { compose = true } ; composeOptions { } }
dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.09.03")
    implementation(composeBom)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
}
```
- 加 cargo 任务：
```kotlin
val abis = mapOf("arm64-v8a" to "aarch64-linux-android", "armeabi-v7a" to "armv7-linux-androideabi", "x86_64" to "x86_64-linux-android")
tasks.register("cargoBuild") {
    doLast {
        abis.forEach { (abi, target) ->
            exec { commandLine("bash", "-lc",
              "cd ${rootProject.projectDir}/../../.. && cargo build --lib --release --target $target --no-default-features") }
            val so = file("${rootProject.projectDir}/../../../target/$target/release/libsendsent_lib.so")
            val dst = file("src/main/jniLibs/$abi").apply { mkdirs() }
            so.copyTo(File(dst, "libsendsent_lib.so"), overwrite = true)
        }
    }
}
tasks.named("preBuild") { dependsOn("cargoBuild") }
```
（路径按 `gen/android` 相对仓根调整：`src-tauri` 在仓根的上一级？`gen/android` → `../../..` = `src-tauri`? 校验：`gen/android` 的 `rootProject.projectDir` = `gen/android`；`gen/android/../../..` = 仓根。`Cargo.toml` 在 `src-tauri/`，故用 `gen/android/../..` = `src-tauri`。实现时按实际修正并验证。）

- [ ] **Step 2: `AndroidManifest.xml`**：`MainActivity` 普通 activity；删 Wry 相关；权限 `INTERNET`/`ACCESS_NETWORK_STATE`/`ACCESS_WIFI_STATE`/`CAMERA`；保留 `usesCleartextTraffic`。

- [ ] **Step 3: 删除 Tauri 运行时文件**（`NsdPlugin.kt`、`ContentPlugin.kt`、`generated/`、`tauri.build.gradle.kts`）。`settings.gradle.kts` 去掉 tauri 仓库/插件（若引用）。

- [ ] **Step 4: 构建 + 安装**（仓根，带 ANDROID_HOME/NDK）
```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"; export NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"
cd src-tauri/gen/android && ./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk   # 若目录名不同以实际为准
```
Expected：BUILD SUCCESSFUL；App 启动显示设备列表。

- [ ] **Step 5: Commit**

```bash
git add -A src-tauri/gen/android
git commit -m "build(android): drop tauri runtime; cargo gradle task + compose deps"
```

---

### Task 6: 文档 + 全量验证

- [ ] **Step 1: `AGENTS.md` 增补** Android 原生架构、JNI 约定、Gradle/cargo 构建、已移除的 Tauri 部分、不再使用 `tauri android init`。

- [ ] **Step 2: 验证**
```
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test
cd src-tauri && cargo build --lib --target aarch64-apple-ios --no-default-features
cd src-tauri && cargo build --lib --target aarch64-linux-android --no-default-features
cd src-tauri/gen/android && ./gradlew assembleDebug
```
Expected：全部 PASS（loopback `v3_secure_transfer` 为既有失败，忽略）。

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "docs: android native architecture and jni"
```

---

## Self-Review 记录

- **Spec 覆盖**：spec §5（engine/JNI/discovery/构建）→ Task 1/2/3/5；§6（Kotlin 桥/最小壳）→ Task 4；§7/§8（构建/迁移）→ Task 5/6。
- **无占位**：核心代码给出；Gradle 路径明确要求“实现时按实际修正并验证”。
- **类型一致**：Kotlin `external fun` 与 Rust `Java_com_mankong_sendsent_Native_*` 名称/参数一一对应；`{fd,name}` JSON 契约在 `SafPicker` 与 `engine::resolve_files` 两端一致。
- **风险**：JNI `JObject` vs `JClass`（Kotlin `object` 用实例方法 → `JObject`）；Gradle cargo 路径；Compose/Kotlin 版本匹配；ZXing 在 Plan 2。
