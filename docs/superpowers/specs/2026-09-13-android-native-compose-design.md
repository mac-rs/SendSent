# Android 原生客户端（Kotlin + Compose + Rust JNI）设计

日期：2026-09-13
状态：待评审
范围：`src-tauri`（共享 engine + Android JNI + 平台桥）+ Android 原生 UI（Kotlin/Jetpack Compose）；不改桌面/iOS 行为。

## 1. 目标

- Android 端**移除 WebView / Tauri 运行时**，改用原生 Kotlin + Jetpack Compose（Material 3），获得原生观感、手势与性能。
- 复用现有 Rust 核心（发现、TCP 传输、SessionManager、历史、身份、二维码），业务逻辑零重写。
- 功能对齐当前 Android/iOS：设备发现、发送（SAF）、接收、进度、历史、我的（身份+二维码+IP）、设置、扫码添加。

## 2. 非目标

- 不改桌面（Tauri）与 iOS（已原生）实现。
- 后台传输的前台服务（foreground service）本期不做（记为后续）。
- 不重写协议 / 加密 / 发现逻辑。

## 3. 现状（Android）

- Tauri 生成工程 `gen/android/`：`MainActivity : TauriActivity`；`generated/`（`Rust.kt` 加载 `libsendsent_lib.so`、`WryActivity`、`TauriActivity`、`Ipc` 等）。
- 两个 Tauri 插件桥：
  - `NsdPlugin.kt`：`NsdManager` 注册/浏览（`register`/`browse`/`poll`），Rust `discovery::nsd` 通过 Tauri IPC 每 2s 轮询。
  - `ContentPlugin.kt`：`openFd(uri)`/`getDisplayName(uri)`，Rust `commands::send_files` 用 fd 建 `/proc/self/fd/<fd>` 符号链接零拷贝发送。
- Rust `.so` 由 `id("rust")` + `tauri.build.gradle.kts` 构建；`jniLibs/<abi>/libsendsent_lib.so` 指向 cargo 产物。
- 桌面/Android 共享 `tauri-shell` feature（默认开）。

## 4. 架构总览

```
Kotlin/Compose (Android)             Rust core (libsendsent_lib.so, Android)
  Native.kt (external fun)  ──JNI──▶  jni_bridge.rs
     ▲  nativeOnService(json)           └─ engine::Core (OnceLock)
     │  NsdBridge (NsdManager)            ├ discovery::android_native (Kotlin push)
     └ SafPicker (SAF → fd)               ├ SessionManager / history / tls
                                          └ 事件: 回调? → 见下
```

- **事件回传**：Android 无 C 回调那么简单。方案：Rust 维护一个「事件队列 + JNI 通知」——Rust 侧把 JSON 事件压入一个全局队列，并通过 JNI 调用 Kotlin 的静态方法 `Native.onEvent(json)`（Rust→Kotlin 需要 JVM 引用：在 `nativeInit` 时缓存 `JavaVM` + 全局 class ref 实现）。Kotlin 再分发到 Compose 状态。
  - 简化备选：Kotlin 侧定时 `nativePollEvents()` 拉取（100ms 轮询），避免 Rust→JVM 附着复杂度和线程问题。**采用轮询**（可靠、简单；UI 事件非硬实时）。
- **发现**：Kotlin `NsdBridge` 拥有 `NsdManager`；解析到服务后调用 `nativeOnService(name, host, port, txtJson)`；丢失调用 `nativeOnServiceLost(name)`。Rust `discovery::android_native` 实现 `Discovery`（`start` 仅记录，实际由 Kotlin 驱动）。
- **文件**：Kotlin `SafPicker` 用 `OpenMultipleDocuments` → 每个 URI `openFileDescriptor` + `DISPLAY_NAME` → `nativeSend(peerId, filesJson)`（`filesJson=[{"fd":N,"name":"..."}]`）；Rust 建符号链接交给 SessionManager。
- **去 Tauri**：`tauri-shell` 对 Android 关闭；删 `generated/`、`NsdPlugin`、`ContentPlugin`、`tauri.build.gradle.kts`、Tauri `rust` 插件；改用自定义 Gradle `Exec` 任务直接 `cargo build`。

## 5. Rust 侧设计

### 5.1 抽出共享 engine

- 新增 `src/engine.rs`（平台无关）：把 iOS `ffi.rs` 的 `Core` + 全局 `OnceLock` + 各操作（identity/peers/add/send/respond/history/config/rename/addresses/qr + 事件队列 + `Throttle`）搬进来，逻辑与现 `ffi.rs` 一致。
- `ffi.rs`（iOS）变薄：仅 C ABI 包装 + `CString` 转换 + 事件回调（内部订阅 engine 事件队列）。
- `jni_bridge.rs`（Android）：JNI 包装。
- 事件模型：`engine` 持有一个 `Mutex<VecDeque<String>>`（JSON 事件，progress 节流），Android `nativePollEvents()` 取出并清空；iOS 由 engine 主动调用回调（保留 iOS 现有回调路径：engine 暴露一个可选的 `EventSink`）。

### 5.2 JNI 桥（Android）

- 依赖：`[target.'cfg(target_os="android")'.dependencies] jni = "0.21"`。
- 约定：所有方法为 `#[no_mangle] pub extern "system" fn Java_com_mankong_sendsent_Native_<name>(env: JNIEnv, _class: JClass, ...) -> jstring/void`。字符串入参 `JString`；返回 JSON `jstring`（Kotlin `String`）。
- 方法：`nativeInit(dataDir, saveDir, port)`、`nativeIdentity`、`nativePeers`、`nativeAddPeer(addr)`、`nativeSend(peerId, filesJson)`、`nativeRespond(sessionId, accept)`、`nativeHistory`、`nativeDeleteHistory(id)`、`nativeClearHistory`、`nativeGetConfig`、`nativeSetConfig(conns, chunkKb, splitMb)`、`nativeSetDisplayName(name)`、`nativeAddresses`、`nativeQr(size)`、`nativePollEvents`、`nativeOnService(name, host, port, txtJson)`、`nativeOnServiceLost(name)`。
- 错误：返回 `{"error":...}` JSON（同 iOS 约定），Kotlin 统一解包。

### 5.3 Android 原生发现

- 新 `src/discovery/android_native.rs`：`Discovery` 实现，持 `Arc<Mutex<PeerRegistry>>` + `tx`。
  - `start()` no-op（Kotlin 驱动）。
  - `add_service(name, host, port, txt)`：从 TXT 解析 `id/name/plat/port/ip` 构造 `Peer`、`upsert` 并 emit `PeerEvent::Found`。
  - `remove_service(name)`：按发现的映射移除并 emit `Lost`。
  - `add_manual_peer`：同 iOS。
  - `set_display_name`：no-op（改名由 Kotlin 重新注册实现；Rust 侧保留接口）。
- 匹配规则：Kotlin 传来的 `name`（serviceName）→ 需要用 TXT 的 `id` 作为 `device_id`（与其它平台一致）。若 TXT 缺失则回退 serviceName。

### 5.4 构建（Rust）

- 与 iOS 一致，Android 原生构建 `cargo build --lib --target <abi> --no-default-features`。
- `tauri-shell` 仍为桌面默认；Android 原生关闭。

## 6. Kotlin / Compose 设计

### 6.1 文件结构（`gen/android/app/src/main/java/com/mankong/sendsent/`）

```
MainActivity.kt          // ComponentActivity + setContent { App() }
Native.kt                // object + external fun + JSON 解析 + 事件轮询
Models.kt                // Peer/Identity/HistoryRecord/TransferProgress/Config (kotlinx.serialization 或 org.json)
Core.kt                  // Android 单例状态(StateFlow), 包装 Native
NsdBridge.kt             // NsdManager 注册/浏览 → nativeOnService/Lost
SafPicker.kt             // SAF OpenMultipleDocuments → fd/name → nativeSend
ui/App.kt                // Scaffold + NavigationBar(设备/传输/我的/设置)
ui/DevicesScreen.kt      // LazyColumn + SwipeToDismissBox + 底部发送栏
ui/TransfersScreen.kt    // TabRow 进行中/历史 + 滑动删除
ui/ProfileScreen.kt      // 身份 + 二维码 + IP
ui/SettingsScreen.kt     // 列表 + Stepper
ui/AddDeviceDialog.kt    // 手输 IP + 扫码入口
ui/ScannerScreen.kt      // CameraX + ZXing
```

- JSON：用 `org.json`（Android 内置，零依赖）解析，避免引入 kotlinx.serialization 插件与网络依赖。
- 状态：`Core` 用 `MutableStateFlow`（peers/progress/history/request/toast/identity/addresses）；`MainActivity` 内 `lifecycleScope` 启动 100ms 轮询 `nativePollEvents()` 并分发。

### 6.2 交互（与 iOS 对齐）

- 底部 `NavigationBar` 四项：设备/传输/我的/设置（Material 3）。
- 设备：`LazyColumn` 卡片行（头像色块+名称+平台·地址+选择圈）；`SwipeToDismissBox` 左滑详情/右滑删除（`DismissDirection`）；顶部活跃传输条；多选后底部发送栏（加密/SHA-256 开关 + “选择文件发送”按钮）；空态。
- 传输：`TabRow`（进行中/历史）；进行中环形/线性进度 + 速度；历史行状态图标/方向/对端/大小/相对时间，滑动删除，菜单清空。
- 我的：身份卡 + 二维码 `Bitmap`（`nativeQr` base64 → `BitmapFactory`）+ IP 列表。
- 设置：显示名、端口、传输参数（`Stepper` 用 `IconButton`）。
- 接收：`AlertDialog`（发送者/文件数/大小，接受/拒绝）。
- 扫码：CameraX `ImageAnalysis` + ZXing 解码 `sendsent://`。

### 6.3 视觉

Material 3 原生组件（`Scaffold`/`NavigationBar`/`TopAppBar`/`ListItem`/`Card`/`SwipeToDismissBox`/`AlertDialog`/`Switch`）；动态取色（`dynamicColorScheme`，可选）；边缘到边缘 + insets。

## 7. 构建系统改动（Gradle）

- `app/build.gradle.kts`：
  - 删 `id("rust")` 与 `apply(from = "tauri.build.gradle.kts")`；删 `tauri.properties` 依赖（版本改硬编码或 `version.properties`）。
  - 新增 Compose：`buildFeatures { compose = true }`、`composeOptions`/Kotlin 2 compose 插件、`androidx.activity:activity-compose`、`androidx.compose.material3`、`androidx.lifecycle:lifecycle-runtime-compose`、`androidx.camera:*`、`com.google.zxing:core`（网络可用时；离线则用 ML Kit/自带解码——见风险）。
  - 新增 `cargoBuild` Gradle 任务（`Exec`）：对每个 ABI 运行 `cargo build --lib --release --target <abi> --no-default-features`，把 `libsendsent_lib.so` 拷到 `src/main/jniLibs/<abi>/`；`preBuild` 依赖它。
- `AndroidManifest.xml`：`MainActivity` 为普通 `android:name=".MainActivity"`（Compose），删 Tauri/Wry 相关；权限：`INTERNET`、`ACCESS_NETWORK_STATE`、`ACCESS_WIFI_STATE`、`CAMERA`；`android:usesCleartextTraffic`（局域网明文 TCP）按现状。
- 删除 `generated/`、`NsdPlugin.kt`、`ContentPlugin.kt`、`tauri.build.gradle.kts`（若存在）。

> 注意：`gen/android` 由 Tauri 生成；改动需写入 `gen/android` 并在 `AGENTS.md` 记录（Tauri 的 `android init` 会覆盖，本项目不再使用 Tauri Android）。

## 8. 迁移与删除

- 删 Tauri Android 运行时（`generated/`、`TauriActivity`、插件、gradle 桥）。
- Web 端 `src/platforms/android/*`（`AndroidApp.tsx`/`android.css`）不再打包；随后清理（同 iOS：从 `platformModules` 移除，`main.tsx` 不再引入 `android.css`）。
- `AGENTS.md` 增补 Android 原生架构、JNI 约定、Gradle/cargo 构建、已移除的 Tauri 部分。

## 9. 错误处理

- Rust 返回 err-json；Kotlin 统一转 `Toast`/`Snackbar`。
- 初始化失败（端口/TLS）阻断式提示。
- SAF fd 失效（权限/进程回收）在发送前校验并提示。
- NsdManager 失败降级为手动 IP 添加。

## 10. 测试与验收

- Rust：`cargo build --lib --target aarch64-linux-android --no-default-features`；`cargo clippy --all-targets -- -D warnings`（桌面）；`cargo test`（核心不回归）。
- 桌面/iOS：不得回归。
- 真机验收（PJA110）：冷启动不崩溃；发现同网设备；SAF 选文件发送到 Mac/iOS；反向接收落盘；进度/历史/清空/单条删除；我的二维码可被扫；设置改名/参数生效。

## 11. 风险与开放问题

- **JNI 串/线程**：`JNIEnv` 仅在调用线程有效；Rust 事件不直接回调 Kotlin，改用 `nativePollEvents` 轮询规避 attach 复杂度。
- **Rust→Kotlin 调用**：本期避免（用轮询），故不需要缓存 JavaVM/class ref。
- **ZXing 依赖**：扫码**采用 ZXing**（`com.google.zxing:core` + CameraX `ImageAnalysis` 解码）。若构建环境无法访问 Maven，回退到已在设备上的方案（系统能力或 ML Kit）。
- **Kotlin / Compose 编译器版本**：需与 `gen/android` 现有 Kotlin 版本匹配（Kotlin 2.x 用 `org.jetbrains.kotlin.plugin.compose`）；实现前先确认版本并统一。
- **后台传输**：不做前台服务，息屏/切后台可能被限速或暂停（与现状相当）。
- **Tauri Android init 覆盖**：不再使用 `tauri android init`；所有 Android 定制固化在 `gen/android` 并在文档说明。
- **动态取色/主题**：Material You 动态色可选，默认仍跟随深浅色。
