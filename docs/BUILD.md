# 构建与真机部署速查

本项目为 Tauri 2 应用（前端 React 19 + TypeScript，后端 Rust）。包管理器统一用 **pnpm**。

- App 标识：`com.mankong.sendsent`
- 默认传输端口：**52225**（iOS 模拟器自动用 52226 避免与本机冲突）
- Vite 开发端口：**1420**（`strictPort`，Tauri 要求）
- iOS 签名 team：`F8JZTX6J52`（baked 在 `src-tauri/gen/apple/project.yml`）

> 通用铁律：不要直接用 Xcode GUI 构建 iOS。Xcode 的 "Build Rust Code" 阶段会回调 `tauri ios xcode-script`，只有通过 `pnpm tauri ios ...` 启动才有对应 IPC 服务。始终用 `tauri` CLI。

---

## 0. 前置环境

```bash
# 前端依赖
pnpm install

# Rust 目标（按需安装）
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

# iOS
brew install cocoapods            # 首次 init 需要
pnpm tauri ios init               # 由 project.yml 重新生成 Xcode 工程

# Android（需要 ANDROID_HOME / NDK_HOME，NDK 27 / JDK 21）
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/<version>"
export PATH="$ANDROID_HOME/platform-tools:$PATH"   # 让 adb 可用
```

Rust 检查（桌面即可）：

```bash
cargo check                    # 在 src-tauri/ 下
cargo test                     # 内联单测 + tests/loopback.rs
cargo clippy --all-targets -- -D warnings
```

---

## 1. 桌面（macOS）

```bash
pnpm tauri dev                 # 主开发命令（前端 + Rust shell）
pnpm tauri build               # 生产打包
pnpm dev                       # 仅 Vite（端口 1420），只调前端时用
```

---

## 2. iOS

### 2.1 模拟器

```bash
# 开发模式（热重载）
pnpm tauri ios dev "iPhone 17"

# 生产构建（模拟器产物是 .app）
pnpm tauri ios build -t aarch64-sim
# -> src-tauri/gen/apple/build/arm64-sim/sendsent.app
```

### 2.2 真机

当前真机：**WenQiang的iPhone**（iPhone 16 Pro，iOS 27.0，UDID `00008140-000849C60CD2801C`）。

```bash
# 开发模式（自动构建 + 安装 + 启动 + 热重载）
# 真机占用 52225，调试时不要再开 Mac 桌面 app，否则端口冲突
pnpm tauri ios dev "WenQiang的iPhone"

# 生产构建（release，导出已签名 IPA）
pnpm tauri ios build -t aarch64
# -> src-tauri/gen/apple/build/arm64/sendsent.ipa
```

用 `devicectl` 安装 / 启动已构建的 release 包：

```bash
DEVICE=00008140-000849C60CD2801C
APP=$(ls -d "$HOME"/Library/Developer/Xcode/DerivedData/sendsent-*/Build/Products/release-iphoneos/sendsent.app)

xcrun devicectl device install app --device "$DEVICE" "$APP"
xcrun devicectl device process launch --device "$DEVICE" com.mankong.sendsent
```

### 2.3 真机日志

```bash
idevicesyslog -u 00008140-000849C60CD2801C
```

`tauri ios dev` 若提示 `idevicesyslog: No device found`，通常是 iPhone 走 Wi‑Fi 而非 USB，不影响安装启动，用上面的命令或 Console.app 看日志。

### 2.4 网络受限时（拉不到 crates / swift-rs）

```bash
export http_proxy=http://127.0.0.1:10809
export https_proxy=http://127.0.0.1:10809
export all_proxy=socks5://127.0.0.1:10808
export no_proxy=localhost,127.0.0.1
```

设备 target 首次构建时 `swift-rs` 会从 GitHub 拉取，没代理会卡在 `github.com:443`；模拟器复用缓存所以一般不受影响。

### 2.5 原生文档选择器（Picker.swift）

`Picker.swift` 提供 `@_cdecl("sendsent_pick_files")`，Rust 侧在 `commands.rs::ios_picker` 以 `extern "C"` 引用。该符号由 Xcode 目标在 Rust 之后编译链接，因此 `build.rs` 仅对 iOS 给 cdylib 链接放行未定义符号（该 dylib 在 iOS 上不使用，Xcode 实际链的是静态库 `libapp.a`）：

```rust
if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
    println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
}
```

若改动 `project.yml`，注意重新 `pnpm tauri ios init` 后要确认 `project.pbxproj` 里仍包含 `Picker.swift`，否则 Xcode 链接会报 `_sendsent_pick_files` 未定义。

---

## 3. Android

```bash
# 开发模式（需连接设备 / 模拟器）
pnpm tauri android dev

# 生产构建
pnpm tauri android build
# -> src-tauri/gen/android/app/build/outputs/apk/.../app-*-release-unsigned.apk
```

安装到设备：

```bash
# 真机调试最省事：直接构建 + 安装 debug 版
pnpm tauri android dev

# 或手动安装（release 产物默认 unsigned，需先签名才能 adb install）
adb devices
adb install -r <apk 路径>
```

只编译 Rust 库（Android 二进制 crate 不参与，属正常）：

```bash
cargo build --lib --target aarch64-linux-android
```

---

## 4. 真机互传注意事项

- 两台设备连**同一 Wi‑Fi**，且路由器未开启客户端/AP 隔离，否则 mDNS 发现不到。
- iPhone 首次启动会弹**本地网络权限**，必须点“允许”。
- 方向：Android → iOS 用系统选择器；iOS → Android 走原生 Picker。
- 同机同时跑 Mac 桌面 app 与 iOS 模拟器会争 52225 / 1420；模拟器已自动用 52226，Vite 可复用（见 AGENTS.md）。

---

## 5. 常见问题

| 现象 | 处理 |
| --- | --- |
| iOS 链接 `_sendsent_pick_files` 未定义 | 确认 `build.rs` 的 iOS link-arg 存在，且 pbxproj 含 `Picker.swift` |
| `tauri ios build` 真机拉 swift-rs 失败 | 设代理（2.4） |
| 真机部署静默回退到 "My Mac" | 真机 iOS 版本高于 Xcode 支持范围（如 iOS 27 需 Xcode 27），升级 Xcode 或降真机系统 |
| 签名 team 不符 | `project.yml` 与 pbxproj 的 `DEVELOPMENT_TEAM` 保持一致（`F8JZTX6J52`） |
| 命令参数报缺 key | JS 侧 `invoke` 参数用 camelCase（`peerDeviceId`，非 `peer_device_id`） |
