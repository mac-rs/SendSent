# 构建与真机部署速查

- **桌面（macOS/Windows/Linux）**：Tauri 2 + React（`src/`）+ Rust（`src-tauri/`）。
- **iOS**：原生 SwiftUI + Rust（无 Tauri/WebView）；Xcode 工程在 `src-tauri/gen/apple/`。
- **Android**：原生 Kotlin/Jetpack Compose + Rust JNI（无 Tauri/WebView）；工程在 `src-tauri/gen/android/`。
- 包管理器统一用 **pnpm**。
- App 标识：`com.mankong.sendsent`；默认传输端口 **52225**；Vite 开发端口 **1420**（仅桌面/Web 用）。
- iOS 签名 team：`F8JZTX6J52`（在 `src-tauri/gen/apple/project.yml`）。

---

## 0. 前置环境

```bash
pnpm install

# Rust 目标
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

# 工具
brew install xcodegen               # 生成 iOS Xcode 工程
export ANDROID_HOME="$HOME/Library/Android/sdk"   # Android SDK（NDK 27 / JDK 21）
export NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"
export PATH="$ANDROID_HOME/platform-tools:$PATH"  # adb
```

Rust 通用检查：

```bash
cd src-tauri
cargo check
cargo test                 # 内联单测 + tests/loopback.rs（见 §5 已知失败）
cargo clippy --all-targets -- -D warnings   # 必须保持干净
```

---

## 1. 桌面（macOS / Windows / Linux）

```bash
pnpm tauri dev     # 主开发命令（前端 + Rust shell）
pnpm tauri build   # 生产打包
pnpm dev           # 仅 Vite（1420），只调前端时用
```

---

## 2. iOS（原生 SwiftUI）

> iOS 不用 `tauri ios dev`。Rust 以静态库链接进原生 app；Xcode 的 “Build Rust Code” 阶段执行 `cargo build --lib --target aarch64-apple-ios --no-default-features` 并产出 `libapp.a`。

### 2.1 只编译 Rust 库

```bash
cd src-tauri
cargo build --lib --target aarch64-apple-ios --no-default-features
# -> target/aarch64-apple-ios/<profile>/libsendsent_lib.a
```

### 2.2 生成 / 更新 Xcode 工程

改过 `src-tauri/gen/apple/project.yml` 后必须重新生成（不要手改 pbxproj）：

```bash
cd src-tauri/gen/apple
xcodegen generate
```

### 2.3 真机构建 + 安装 + 启动

当前真机：**WenQiang的iPhone**（iPhone 16 Pro，iOS 27，UDID `00008140-000849C60CD2801C`）。

```bash
# 生产构建（release，导出已签名 IPA）。iOS 是原生 UI，跳过前端构建。
pnpm tauri ios build -t aarch64 -c '{"build":{"beforeBuildCommand":""}}'
# -> src-tauri/gen/apple/build/arm64/sendsent.ipa

# 安装 + 启动
DEVICE=00008140-000849C60CD2801C
rm -rf /tmp/ss-app && mkdir -p /tmp/ss-app
unzip -q src-tauri/gen/apple/build/arm64/sendsent.ipa -d /tmp/ss-app
xcrun devicectl device install app --device "$DEVICE" /tmp/ss-app/Payload/sendsent.app
xcrun devicectl device process launch --device "$DEVICE" com.mankong.sendsent
```

- **必须解锁 iPhone**：锁屏时 `process launch` 报 `device was not, or could not be, unlocked`；`install` 可能报 `error 4016 / unavailable`。
- 安装未生效（仍是旧包）时先卸载再装：
  ```bash
  xcrun devicectl device uninstall app --device "$DEVICE" com.mankong.sendsent
  ```
- 模拟器：`pnpm tauri ios build -t aarch64-sim` → `src-tauri/gen/apple/build/arm64-sim/sendsent.app`。

### 2.4 调试辅助

```bash
# 截图
xcrun devicectl device capture screenshot --device "$DEVICE" --destination /tmp/ios.png

# 查看 app 沙盒（Documents/sendsent = 接收目录；Application Support/sendsent = history.json/identity/tls）
xcrun devicectl device info files --device "$DEVICE" \
  --domain-type appDataContainer --domain-identifier com.mankong.sendsent \
  --subdirectory "Documents/sendsent"
xcrun devicectl device info files --device "$DEVICE" \
  --domain-type appDataContainer --domain-identifier com.mankong.sendsent \
  --subdirectory "Library/Application Support/sendsent"

# 系统日志
idevicesyslog -u 00008140-000849C60CD2801C
```

### 2.5 网络受限时（cargo 拉不到 crates）

```bash
export http_proxy=http://127.0.0.1:10809
export https_proxy=http://127.0.0.1:10809
export all_proxy=socks5://127.0.0.1:10808
export no_proxy=localhost,127.0.0.1
```

---

## 3. Android（原生 Kotlin + Compose + Rust JNI）

> **不要运行 `pnpm tauri android init`** —— 会重新生成 Tauri 工程并覆盖原生项目。构建走 `build-android.sh` + Gradle。

### 3.1 环境

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"
export PATH="$ANDROID_HOME/platform-tools:$PATH"
```

### 3.2 只编译 Rust 库（自动设好 NDK linker）

```bash
cd src-tauri
./build-android.sh aarch64-linux-android arm64-v8a
# -> gen/android/app/src/main/jniLibs/arm64-v8a/libsendsent_lib.so

# 等价的手动方式（需自己导出 linker 环境）：
# cargo build --lib --target aarch64-linux-android --no-default-features
```

### 3.3 构建 APK

Gradle 的 `cargoBuild` 任务会自动调用 `build-android.sh`（先编 Rust 再打包）：

```bash
cd src-tauri/gen/android
./gradlew assembleDebug
# -> app/build/outputs/apk/debug/app-debug.apk
```

### 3.4 安装 + 启动

```bash
adb devices
adb shell input keyevent KEYCODE_WAKEUP        # 先唤醒！锁屏休眠时安装会 Failure [-99]
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.mankong.sendsent/.MainActivity
```

- 若报签名不一致（之前装过 release 签名包）：先 `adb uninstall com.mankong.sendsent`。
- release 包签名配置在 `gen/android/app/keystore.properties` + `sendsent.keystore`；用 `./gradlew assembleRelease`。

### 3.5 日志 / 数据

```bash
adb logcat -s sendsent NsdBridge AndroidRuntime   # Rust 日志 tag=sendsent；发现桥 tag=NsdBridge
adb exec-out screencap -p > /tmp/android.png

# app 私有数据（debug 包可 run-as）
adb shell run-as com.mankong.sendsent ls -la files/sendsent
adb shell run-as com.mankong.sendsent cat files/sendsent/history.json
```

---

## 4. 真机互传注意事项

- 两台设备连**同一 Wi‑Fi**，路由器关闭客户端/AP 隔离，否则发现不到。
- iPhone 首次启动会弹**本地网络权限**，必须允许；macOS 正式包也已声明该权限。
- **iOS 前台限制**：iOS 在后台会挂起 app，Bonjour 广播停止 → 别的设备一会儿发现、一会儿丢失。传输期间 iPhone 必须保持前台。
- macOS 正式包**不能开 App Sandbox**（会挡 mDNS 多播并把 `$HOME` 重定向）；`src-tauri/Entitlements.plist` 已去掉沙盒。
- Android 发现走系统 `NsdManager`（Kotlin 驱动）；文件走 SAF fd（`/proc/self/fd` 路径被 SELinux 拒），Rust 用 `dup(fd)` 直读。
- 同机同时跑 Mac 桌面 app 与 iOS 模拟器会争 52225 / 1420；模拟器自动用 52226。

---

## 5. 常见问题

| 现象 | 处理 |
| --- | --- |
| iOS `tauri ios build` 报 `error:` 但无细节 | 先 `cargo build --lib --target aarch64-apple-ios --no-default-features` 看 Rust 错误 |
| iOS `process launch` 报 device not unlocked | 解锁 iPhone 再执行 |
| iOS `install` 报 4016 / unavailable | 解锁并保持亮屏，或 `uninstall` 后重装 |
| iOS cargo 拉 swift-rs/crates 卡住 | 设代理（§2.5） |
| 真机部署静默回退到 “My Mac” | 真机 iOS 版本高于 Xcode 支持范围（iOS 27 需 Xcode 27） |
| Android `adb install` 报 `Failure [-99]` | 设备锁屏休眠：`adb shell input keyevent KEYCODE_WAKEUP` 后重试 |
| Android 报签名不一致 | `adb uninstall com.mankong.sendsent` 后重装 |
| Android 发送报 `Permission denied` | SAF 文件用 fd 直读（已实现）；确认走 `build-android.sh` 重新编了 `.so` |
| Android 发现不到设备 | 看 `adb logcat -s NsdBridge`；确认对方在广播（iOS 需前台） |
| `cargo test` 的 `v3_secure_transfer` 失败 | **既有环境性失败**（本机超时），与业务逻辑无关，可忽略 |
| 桌面正式包发现不到设备 | 确认 `Entitlements.plist` 无 `com.apple.security.app-sandbox`，并重新 `pnpm tauri build` |
