# iOS 原生客户端（SwiftUI + Rust FFI）设计

日期：2026-09-13
状态：待评审
范围：`src-tauri`（Rust 核心 + iOS FFI）+ iOS 原生 UI（SwiftUI）；不改动桌面 / Android 行为。

## 1. 目标

- iOS 端**彻底移除 WKWebView / Tauri 运行时**，改用原生 SwiftUI 呈现，获得真正的 iOS 观感、手势、材质与性能。
- 复用现有 Rust 核心（mDNS/Bonjour 发现、TCP 传输、SessionManager、历史、身份、二维码），**业务逻辑零重写**。
- 功能与当前 iOS 版本对齐：设备发现、发送（原生文档选择器）、接收（接受/拒绝）、实时进度、历史、我的（身份 + 二维码 + IP）、设置、扫码添加。

## 2. 非目标

- 不改桌面（macOS/Windows/Linux）与 Android 的 Tauri 实现。
- 不引入新的业务能力（v1 只做功能对齐）。
- 不重写传输协议 / 加密 / 发现逻辑。

## 3. 现状

- `sendsent_lib`（`crate-type = ["staticlib","cdylib","rlib"]`）已能产出静态库，被 Xcode 链接为 `libapp.a`。
- 核心模块与 Tauri 基本解耦：
  - `discovery`（`Discovery` trait；iOS 用 `ios_bonjour::BonjourDiscovery`）
  - `transfer::manager::SessionManager`（`run_listener` / `start_send` / `respond` / `cleanup_session`）
  - `store`（`Identity` / `TransferConfig`）、`history`、`transfer::tls`
- Tauri 只在 `lib.rs::run()`、`commands.rs`、`content_plugin.rs` / `nsd_plugin.rs`（Android）中出现。
- 事件模型：`PeerEvent`（Found/Lost）→ Tauri `peer://*`；`TransferEvent`（Request/Progress/Finished/Recorded）→ Tauri `transfer://*`，payload 均为 serde JSON。
- 已存在 C ABI 先例：`sendsent_pick_files`（`commands::ios_picker` ↔ `Picker.swift`）。

## 4. 架构总览（A1：彻底解耦）

```
SwiftUI App (iOS)                Rust core (sendsent_lib, iOS)
  Core: ObservableObject  ──FFI──▶ ffi::Core (global OnceLock)
   ├ events  ◀─ callback ─────────  discovery / SessionManager / history
   └ commands ─ extern "C" ───────▶ peers/add/send/respond/…
```

- Rust 侧新增 `src/ffi.rs`（`#[cfg(target_os="ios")]`），持有一个全局 `Core`，内含 `Identity`、`Arc<dyn Discovery>`、`Arc<SessionManager>`、`Arc<Mutex<HistoryStore>>`、`TransferConfig`、`TlsConfig`、`save_dir`、`port`、以及一个专用 tokio `Runtime`。
- Tauri shell 用 Cargo feature（`tauri-shell`）隔离；iOS 原生构建 `--no-default-features`，**不编译 Tauri/tao**，从而移除 `[patch.crates-io] tao` 与 `Info.plist` 的 `TaoSceneDelegate` scene hack。
- 桌面 / Android 构建保持默认 feature（含 Tauri），行为不变。

## 5. Rust 侧设计

### 5.1 Feature gate

- `Cargo.toml`：
  - `[features] default = ["tauri-shell"]`；`tauri-shell = ["dep:tauri", "dep:tauri-plugin-opener", "dep:tauri-plugin-dialog", "dep:tauri-plugin-fs"]`。
  - 将上述 4 个 tauri 依赖改为 `optional = true`；`tauri-build` 仅在构建 bin/shell 时需要（build-dependencies 保留，iOS 原生构建时不调用其生成逻辑——见构建章节）。
- `lib.rs`：`run()`、`#[cfg_attr(mobile, tauri::mobile_entry_point)]`、`commands` 模块、`state::AppState` 均在 `#[cfg(feature = "tauri-shell")]` 下。
- `main.rs`（bin）仅 `tauri-shell` 下编译。
- `ffi.rs` 在 `#[cfg(target_os="ios")]` 下编译（与 feature 无关；iOS 原生构建默认关闭 tauri-shell）。
- 删除 `commands::ios_picker` 与 `Picker.swift`（原生侧改用 SwiftUI `.fileImporter`）。
- 移除 `[patch.crates-io] tao`（仅 tauri iOS 需要）。

### 5.2 全局 Core

`ffi::Core` 字段：
```
identity: Identity
data_dir: PathBuf        // Application Support/sendsent
save_dir: PathBuf        // Documents/sendsent（接收落地目录）
port: u16
discovery: Arc<dyn Discovery>
sessions: Arc<SessionManager>
history: Arc<Mutex<HistoryStore>>
config: TransferConfig
tls: TlsConfig
runtime: tokio::runtime::Runtime
event_cb: Mutex<Option<extern "C" fn(*const c_char)>>
```

- `sendsent_ios_init(data_dir, save_dir, port, event_cb) -> i32`：幂等；`OnceLock<Core>` 初始化一次。内部：加载/创建 identity、transfer config、TLS、history；构造 `BonjourDiscovery` 并用 runtime 启动；spawn `SessionManager::run_listener(port)`；spawn 事件转发任务（见 5.4）。
- 所有 FFI 调用通过全局 `Core`；未初始化时返回错误。

### 5.3 FFI 函数清单

约定：字符串入参 `*const c_char`（UTF-8）；返回值为 `*mut c_char`（UTF-8 JSON，调用方负责 `sendsent_ios_free_string` 释放），无内容返回 `null`。

| 函数 | 说明 | 返回 |
| --- | --- | --- |
| `sendsent_ios_init(data_dir, save_dir, port, cb) -> i32` | 初始化核心 | 0 成功, 非 0 失败 |
| `sendsent_ios_peers() -> *mut c_char` | 当前 peer 列表（JSON 数组，复用 `Peer` serde） | JSON |
| `sendsent_ios_add_peer(addr)` | 手动加入 `ip:port` | null / err-json |
| `sendsent_ios_send(peer_id, files_json, secure, verify)` | 发送；`files_json` 为字符串数组 | `{"session_id":"..."}` / err |
| `sendsent_ios_respond(session_id, accept)` | 接受/拒绝接收 | null / err |
| `sendsent_ios_history() -> *mut c_char` | 历史记录（复用 `HistoryRecord` serde） | JSON |
| `sendsent_ios_clear_history()` | 清空历史 | null / err |
| `sendsent_ios_identity() -> *mut c_char` | `Identity` | JSON |
| `sendsent_ios_set_display_name(name)` | 改名（持久化 + 更新发现广播） | null / err |
| `sendsent_ios_addresses() -> *mut c_char` | 本机 IPv4 列表 `[{interface,ip}]` | JSON |
| `sendsent_ios_qr(size, ip) -> *mut c_char` | 二维码 PNG base64 | JSON string |
| `sendsent_ios_get_transfer_config() -> *mut c_char` | 传输配置 | JSON |
| `sendsent_ios_set_transfer_config(conns, chunk_kb, split_mb)` | 写配置 | null / err |
| `sendsent_ios_free_string(ptr)` | 释放返回字符串 | — |

- JSON 契约**直接复用现有 serde 类型**（`Peer`、`Identity`、`HistoryRecord`、`TransferEvent`、`Manifest`），不新增 schema；`SocketAddr` 序列化为 `"ip:port"` 字符串。
- 错误统一 `{"error":"..."}`，成功为 `null` 或结果对象。

### 5.4 事件回调

- `event_cb` 由 Swift 注册，签名 `extern "C" fn(*const c_char)`。
- Rust 复用现有 serde：`TransferEvent` 直接 `serde_json::to_string(&ev)`，其内部 tag 为 `kind`（`request` / `progress` / `finished` / `recorded`）；peer 事件用同构信封：
  - `{"kind":"peer_found","peer":{…}}`
  - `{"kind":"peer_lost","device_id":"…"}`
  - `{"kind":"recorded", …HistoryRecord 字段}`（来自 `TransferEvent::Recorded`）
- 事件回调在 Rust 工作线程调用；Swift 侧负责切主线程。回调期间不持锁，避免死锁。
- **进度节流**：`kind == "progress"` 在 Rust 侧按 session 做最小间隔（默认 100ms）+ 最终态必发，避免高频回调。
- 字符串以 `CString` 传给回调，回调返回后由 Rust 释放。

### 5.5 线程 / 内存

- 全局 `OnceLock<Core>`；`Core` 内全部 `Arc`/`Mutex`。
- FFI 返回字符串：`CString::into_raw`，Swift 用 `String(cString:)` 后调 `sendsent_ios_free_string` 归还。
- 路径字符串统一 UTF-8；文件路径由 Swift 传入（已做 security-scoped 访问）。

## 6. Swift 侧设计

### 6.1 文件结构（`src-tauri/gen/apple/Sources/sendsent/`）

```
SendSentApp.swift        // @main App + RootView(TabView)
Core.swift               // ObservableObject：FFI 桥 + 状态
FFI.swift                // @_silgen_name 声明 + JSON 解码辅助
Models.swift             // Codable：Peer/Identity/HistoryRecord/TransferEvent…
Views/
  DevicesView.swift      // List + 多选 + 发送按钮
  TransfersView.swift    // 进行中 / 历史(segmented)
  ProfileView.swift      // 身份 + 二维码 + IP
  SettingsView.swift     // Form：改名/保存位置/传输参数/关于
  AddDeviceView.swift    // 手动 IP + 扫码
  IncomingRequestView.swift // UIAlertController 风格（用 .alert）
  QrSheet.swift          // 二维码全屏
  ScanView.swift         // AVFoundation 扫码
```

### 6.2 Core（ObservableObject, @MainActor）

- 属性：`peers: [Peer]`、`progress: [String: ProgressView]`、`history: [HistoryRecord]`、`request: RequestView?`、`identity`、`addresses`、`toast`。
- `init()`：计算 `dataDir`/`saveDir`，调 `sendsent_ios_init`；注册事件回调（`Unmanaged.passUnretained(self)` 或全局 C 函数 + 单例），回调 `DispatchQueue.main.async` 更新。
- 方法：`refresh/addPeer/send(files:secure:verify:)/respond/clearHistory/rename/setTransferConfig`。
- 生命周期：单例，随 App 存活。

### 6.3 界面与交互

- **RootView**：原生 `TabView`，4 个 tab：设备 / 传输 / 我的 / 设置（SF Symbols：`dot.radiowaves.left.and.right`、`arrow.left.arrow.right`、`person.crop.circle`、`gearshape`）。
- **设备**：`NavigationStack` + 大标题；`List` 多选（`EditButton` 或自绘选择圈）；顶部 `.searchable`；选中后工具栏/底部 `Button("发送文件")` 调 `.fileImporter`；空态 `ContentUnavailableView`（iOS 17）。导航栏 `+` 打开 AddDevice。
- **传输**：`Picker(.segmented)`（进行中/历史）+ `List`；进度用 `ProgressView(value:)`；历史 swipe 删除 / 工具栏清空。
- **我的**：身份头 + 二维码（Rust PNG，白底）+ IP 列表（可复制）+ 端口。
- **设置**：`Form`/`List`：显示名（TextField）、保存位置、传输参数（Stepper）、关于、系统主题跟随。
- **收到请求**：`.alert`（标题=发送者，消息=文件数/大小，动作 接受/拒绝），触觉反馈。
- **扫码添加**：`AVCaptureSession` + `VNDetectBarcodesRequest`（QR），解析 `sendsent://` payload → `addPeer`。
- **发送**：`.fileImporter(isPresented:allowedContentTypes:allowsMultipleSelection:)` 返回 `[URL]`；`startAccessingSecurityScopedResource()`，把 `url.path` 传给 `sendsent_ios_send`，传输期间保持访问，结束后释放。

### 6.4 视觉原则（原生）

- 只用系统组件与 SF Symbols：`List`/`insetGrouped`、`NavigationStack`、`Form`、`Toggle`、`Picker`、`ProgressView`、`.alert`、`.sheet`、`.searchable`、`.contextMenu`。
- 颜色用语义色（`.label`/`.secondaryLabel`/`.systemGroupedBackground`/`.systemBlue`…），自动深浅色；不做自造卡片、渐变、假状态栏、假 Home Indicator。
- 48pt 点击区、系统间距、`ContentUnavailableView` 空态、`sensoryFeedback`/`UIImpactFeedbackGenerator` 触觉。

## 7. 构建系统改动

- `project.yml`：
  - `deploymentTarget.iOS`：15.0 → **17.0**（见开放问题）。
  - “Build Rust Code” preBuildScript 改为直接 cargo：
    ```
    export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
    cargo build --manifest-path "$SRCROOT/../../../Cargo.toml" \
      --target aarch64-apple-ios --no-default-features --release
    cp "$SRCROOT/../../../target/aarch64-apple-ios/release/libsendsent_lib.a" \
       "$SRCROOT/Externals/arm64/release/libapp.a"
    ```
    （debug 配置映射到 `--debug` 与 `.../debug/libapp.a`；保留 `outputFiles`。）
  - 删除 `globalize_symbols.sh` 调用（Tauri/swift-rs 专用）。
  - `Info.plist`：移除 `UIApplicationSceneManifest` 的 `TaoSceneDelegate` 配置，改用 SwiftUI 标准生命周期；保留 `NSBonjourServices` / `NSLocalNetworkUsageDescription`。
  - 移除对 `WebKit.framework` 的依赖；新增无需（AVFoundation 系统即可）。`libapp.a` 依赖保留。
  - 新增 `Sources/sendsent` 下的 Swift 文件（`sources: Sources` 已覆盖）。
- 仍用 `pnpm tauri ios build -t aarch64` 编排 xcodebuild（CLI 仅当构建器），或后续直接 `xcodebuild`。
- 前端：iOS 不再加载 React UI；`src/` 与平台 CSS 保留给其它平台。构建 iOS 时可不跑 `beforeBuildCommand`（`-c '{"build":{"beforeDevCommand":""}}'` 或相应配置），避免无谓前端构建。

## 8. 迁移与删除

- 删除 `Picker.swift`、`commands::ios_picker`、`globalize_symbols.sh`、`[patch.crates-io] tao`、`Info.plist` scene hack。
- `Cargo.toml` feature 化 Tauri 依赖。
- iOS 不再引用 `src/` React UI 与 `platforms/ios/*` CSS（保留文件，供未来其它用途/回退，但构建不打包）。
- `AGENTS.md` 增补：iOS 原生架构、FFI 约定、构建命令与注意事项。

## 9. 错误处理

- Rust FFI 返回 err-json，Swift 统一解包为 `NSError`/`CoreError` 并 `toast` 提示。
- 初始化失败（端口占用、TLS 失败）在 RootView 显示阻断式提示。
- 文件访问失败（security scope 失效）在发送前校验并提示。

## 10. 测试与验收

- Rust：`cargo build --target aarch64-apple-ios --no-default-features`；`cargo clippy --all-targets -- -D warnings`（桌面默认 feature 下）；`cargo test`（核心不回归）。
- 桌面/Android：`cargo check` + 既有 loopback 测试保持通过（feature 化后不得回归）。
- 真机验收（iOS 27 / iPhone 16 Pro）：
  1. 冷启动不崩溃；真机状态栏由系统提供（无自绘）。
  2. 发现同网设备并展示；手动 IP 与扫码可添加。
  3. 选中设备 → 原生选择器选文件 → 发送，Mac/Android 收到。
  4. 反向接收：收到请求 → 原生弹窗接受/拒绝 → 文件落到 Documents/sendsent，历史可见。
  5. 我的页二维码可被另一台设备扫到并成功连接。
  6. 设置改名/传输参数生效；深浅色正常。

## 11. 风险与开放问题

- **构建链路**：iOS 改直连 cargo 后，需确认 `tauri ios build` 仍能编排；否则改用 `xcodebuild` 脚本。
- **FFI 安全**：字符串所有权/线程；回调重入与锁；进度节流。
- **security-scoped URL 生命周期**：发送期间必须持续持有 access。
- **iOS 15 vs 17**：本设计使用 `NavigationStack`/`ContentUnavailableView` 等；建议 deploymentTarget 升到 **17.0**（真机是 iOS 27，无历史包袱）。若需兼容 16，需替换部分 API。
- **包体积/符号**：去掉 Tauri/WebKit 后应显著减小；确认无 tauri 残留符号。
- **回退**：是否保留“Web iOS”构建开关作为回退？默认不保留（用户要求尽量少用网页）。
- **`set_display_name` 现状**：`commands::set_display_name` 目前是空实现（no-op）。原生 FFI 必须真正持久化 `identity.json` 并调用 `Discovery::set_display_name` 更新广播，否则改名不生效。
