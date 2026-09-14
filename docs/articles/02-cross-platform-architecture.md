# 02 · 跨平台架构:一套内核,三种外壳

## 为什么不是"一个 Tauri 应用跑三端"

最初的桌面端是标准的 Tauri 2 应用:React 前端 + Rust 后端,WebView 里跑 UI。
把这个模式推到 iOS/Android 会遇到几个硬问题:

- 移动端 WebView 的交互质感与系统不一致(滚动、手势、转场都要自己模拟);
- 文件访问要走系统的选择器/SAF,WebView 拿不到可长期使用的句柄;
- iOS 的 mDNS、后台生命周期,WebView 层很难精细控制;
- 体积与启动成本。

于是我们做了一个关键重构:**把 Tauri 变成"桌面专属外壳",把内核抽离出来。**
iOS 用 SwiftUI,Android 用 Compose,都直接调用同一个 Rust 内核。

```
                 ┌───────────────────────────────────────────┐
                 │                Rust 内核                    │
                 │  engine.rs  proto/  transfer/  discovery/   │
                 │  store/  history/  misc/  tls/              │
                 └───────┬───────────────┬───────────────┬────┘
             desktop     │        iOS     │      Android  │
        ┌────────────────┴──┐  ┌──────────┴────────┐  ┌───┴──────────────┐
        │ tauri-shell       │  │ C ABI (ffi.rs)     │  │ JNI (jni_bridge) │
        │ commands.rs/lib.rs│  │ sendsent_ios_*     │  │ Java_..._Native_*│
        │ + React UI        │  │ + SwiftUI          │  │ + Compose        │
        └───────────────────┘  └────────────────────┘  └──────────────────┘
```

## 用 Cargo feature 做编译期解耦

`src-tauri/Cargo.toml`:

```toml
[features]
default = ["tauri-shell"]
tauri-shell = ["dep:tauri", "dep:tauri-plugin-opener",
               "dep:tauri-plugin-dialog", "dep:tauri-plugin-fs"]
```

- 桌面:默认 feature,编译 Tauri 外壳;
- iOS / Android:`--no-default-features` 构建,`tauri`/`tao` 完全不参与编译,
  只产出内核静态库/动态库。

```bash
# iOS:静态库,链接进 Xcode 工程
cargo build --lib --target aarch64-apple-ios --no-default-features

# Android:.so,JNI 加载
cargo build --lib --target aarch64-linux-android --no-default-features
```

gated 的代码用 `#[cfg(feature = "tauri-shell")]` 标注(`state.rs`、`commands.rs`、
`nsd.rs` 等只在桌面构建时存在)。

## 内核:平台无关的 `engine.rs`

内核暴露一组与 UI 无关的操作,并统一处理**事件**与**运行时**:

```rust
pub struct Core {
    pub identity: Mutex<Identity>,
    pub data_dir: PathBuf, pub save_dir: PathBuf, pub port: u16,
    pub discovery: Arc<dyn Discovery>,
    pub sessions: Arc<SessionManager>,
    pub history: Arc<AsyncMutex<HistoryStore>>,
    pub config: Mutex<TransferConfig>,
    pub runtime: tokio::runtime::Runtime,   // 自带 tokio 运行时
    pub sink: Mutex<Option<EventSink>>,      // iOS: C 回调
    pub queue: Mutex<VecDeque<String>>,      // Android: 轮询队列
}
```

操作(`send` / `respond` / `add_peer` / `set_display_name` / …)是普通函数,
三端共用。它们通过 `push_event` 把 `TransferEvent`(进度、完成、请求、历史)
投递出去。

## 三端唯一的真正差异:事件怎么"送出去"

这是内核里最关键的一处平台适配——UI 需要一个从 Rust 到前端的**异步事件通道**,
三端机制完全不同:

| 平台 | 机制 | 代码 |
|------|------|------|
| 桌面 | Tauri `emit`(`peer://found`、`transfer://progress`…) | `lib.rs` |
| iOS | C 函数指针回调(`EventCallback`) | `ffi.rs` |
| Android | 写入队列,由 Kotlin 每 100ms 轮询 | `engine.rs::poll_events` |

```rust
pub fn push_event(core: &Core, json: String) {
    if let Some(f) = core.sink.lock().unwrap().as_ref() { f(json); return; } // iOS
    let mut q = core.queue.lock().unwrap();                                // Android
    if q.len() >= QUEUE_MAX { q.pop_front(); }
    q.push_back(json);
}
```

iOS 侧注册回调时把 C 回调包成 `EventSink`;Android 侧传 `None`,事件进队列,
由 `Native.nativePollEvents()` 批量取走。

## 桥接层:iOS 的 C ABI

iOS 是"静态库 + 无 WebView",桥接面用最稳的 C 约定——**输入/输出都是 JSON 字符串**,
避免手写结构的 ABI 兼容问题:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_send(
    peer_id: *const c_char, files_json: *const c_char,
    secure: bool, verify: bool,
) -> *mut c_char { /* ... */ }

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_free_string(ptr: *mut c_char) { /* CString::from_raw */ }
```

Swift 侧用 `@_silgen_name` 声明这些符号,再包一层解出 JSON、把错误抛成 Swift `Error`:

```swift
@_silgen_name("sendsent_ios_send")
func sendsent_ios_send(_ peerId: UnsafePointer<CChar>?, _ filesJson: UnsafePointer<CChar>?,
                       _ secure: Bool, _ verify: Bool) -> UnsafeMutablePointer<CChar>?
```

内存约定很简单:**返回的指针由 Rust 分配,Swift 读完必须调用 `sendsent_ios_free_string` 释放**,
在 Swift 里封装成 `take(_:)` 一劳永逸。

## 桥接层:Android 的 JNI

Android 用 `.so` + JNI,同样是 JSON 字符串。每个入口都套 `catch_unwind`,
避免 Rust panic 直接把 App 崩掉:

```rust
#[no_mangle]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSend(
    mut env: JNIEnv, _this: JObject,
    peer_id: JString, files_json: JString, secure: jboolean, verify: jboolean,
) -> jstring { /* catch_unwind → JSON 字符串 */ }
```

Kotlin 侧声明在 `Native.kt`,业务状态在 `Core.kt`(`StateFlow`),UI 用 Compose 观察。

## 各平台的"数据放在哪 / 文件从哪来"

内核只认路径与端口,但三端的沙盒规则不同,差异被收敛到这里:

| 平台 | 数据目录 | 接收目录 | 发送文件来源 |
|------|----------|----------|--------------|
| macOS | `~/Library/Application Support/…` | `~/Downloads/sendsent` | 系统文件面板 |
| iOS | App 容器 `Application Support/sendsent` | 容器 `Documents/sendsent` | 文件选择器 / **相册** |
| Android | `filesDir/sendsent` | `getExternalFilesDir/sendsent` | SAF(`content://`) |

Android 有一个绕不开的坑:**SAF 返回的 `content://` 无法用路径 `open`**
(而且 SELinux 拒绝按路径打开 `/proc/self/fd/N`)。做法是取出底层 fd,建一个指向
`/proc/self/fd/<fd>` 的符号链接,并把 fd 登记到内核,发送时直接 `dup(fd)` 读:

```rust
crate::misc::register_fd(link_str, fd);   // 发送时:dup 后直接读,零拷贝友好
```

端口方面:默认 **52225**;iOS 模拟器与 Mac 共享网络栈,自动改用 **52226**
(`cfg!(target_abi = "sim")`),也可用 `SENDSENT_PORT` 覆盖。

## 小结

跨平台的关键不是"写一份代码到处跑 UI",而是:

1. 把不可移植的部分(协议、传输、发现)**收敛进内核**;
2. 把真正不可共享的部分(事件通道、文件系统、发现框架)做成**薄适配层**;
3. 用编译期 feature 切断桌面专属依赖,让移动端只携带内核。

结果是三端共享 90% 以上的逻辑,UI 则各自"原生"。
