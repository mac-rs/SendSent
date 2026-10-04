<div align="center">

# SendSent

**同一张网,文件直达。**

局域网点对点文件 / 文本互传:16 条连接并行打满网卡,可选 TLS+PIN 加密,无服务器、无账号、无遥测。

[![CI](https://github.com/mac-rs/SendSent/actions/workflows/ci.yml/badge.svg)](https://github.com/mac-rs/SendSent/actions/workflows/ci.yml)
[![Release](https://github.com/mac-rs/SendSent/actions/workflows/release.yml/badge.svg)](https://github.com/mac-rs/SendSent/actions/workflows/release.yml)

`macOS` · `Windows` · `Linux` · `iOS` · `Android`

</div>

---

## 为什么是 SendSent

- **快**:大文件按字节区间分片到 16 条 TCP 连接同时收发,配合零拷贝 `sendfile` 与 16 MiB Socket 缓冲,千兆网卡轻松打满。
- **私密**:点对点直连,文件不经过任何服务器——这是架构保证,不是隐私政策承诺。需要更多确定性时,打开 TLS+PIN 加密,全链路加密且陌生设备无法接入。
- **简单**:同一局域网自动发现,零配置。装好即用,没有"添加设备"这一步。
- **可靠**:逐文件 SHA-256 校验、断线保留分片续传、完整传输历史。
- **原生**:一套 Rust 传输核心,三端原生 UI——桌面(Tauri 2)、iOS(SwiftUI)、Android(Kotlin + Compose),没有 WebView 的性能税。

## 下载

到 [Releases](https://github.com/mac-rs/SendSent/releases) 获取对应平台安装包:

| 平台 | 资产 | 说明 |
|---|---|---|
| macOS (Apple Silicon) | `*_aarch64.dmg` | CI 包为 ad-hoc 签名,首次打开右键 →「打开」 |
| macOS (Intel) | `*_x86_64.dmg` | 同上 |
| Windows | `*-setup.exe` / `*.msi` | |
| Linux | `*.deb` / `*.rpm` / `*.AppImage` | |
| Android (arm64) | `SendSent_*_android_arm64-debug.apk` | 早期版本为 debug 签名 |
| iOS | — | 需自行用 Xcode 签名构建,见[下方](#iosswiftui--rust-ffi) |

## 快速上手

1. 两台设备连入**同一局域网**,各自安装并启动 SendSent;
2. 设备自动出现在列表中(不在同一网段时,扫码或输入 `ip:52225` 直连);
3. 拖入文件 / 粘贴文本,选择目标设备发送;
4. 对不可信网络勾选「加密」:发送端显示 6 位 PIN,在接收端输入即完成配对。

> iOS 后台会暂停 Bonjour 广播,让 iPhone 保持前台即可被发现。

## 工作原理

```
┌────────┐   mDNS 自动发现 (_sendsent._tcp.local. : 52225)   ┌────────┐
│ 设备 A │ ──────────────────────────────────────────────── │ 设备 B │
└────────┘   1 条控制连接 + N 条数据连接(TLS 可选)          └────────┘
             大文件区间分片并行 · 小文件轮询 · SHA-256 校验
```

传输引擎是一个共享 Rust core,桌面 / iOS / Android 三端链接同一份代码,行为完全一致。设计文档与实现计划见 [`docs/superpowers/`](docs/superpowers/),产品介绍页见 [`website/`](website/index.html)。

## 从源码构建

依赖:Node 22+ / pnpm 11 / Rust stable / 各平台原生工具链。

### 桌面(Tauri 2)

```bash
pnpm install
pnpm tauri dev      # 开发调试(注意:不是 pnpm dev)
pnpm tauri build    # 生产构建,产物在 src-tauri/target/release/bundle/
```

### Android(Kotlin + Compose + Rust JNI)

```bash
# 需要 ANDROID_HOME 与 NDK 27
cd src-tauri/gen/android
./gradlew assembleDebug     # cargoBuild 任务会先编译 Rust .so
# 或只构建 Rust 核心:
# ../build-android.sh aarch64-linux-android arm64-v8a
```

### iOS(SwiftUI + Rust FFI)

```bash
# 1. 编译 Rust 静态库
cd src-tauri
cargo build --lib --target aarch64-apple-ios --no-default-features

# 2. 生成 Xcode 工程并打开(需要 brew install xcodegen)
cd gen/apple && xcodegen generate
open sendsent.xcodeproj
# 3. 在 Xcode 中选择自己的开发团队(DEVELOPMENT_TEAM)后构建运行
```

### 质量检查(与 CI 一致)

```bash
cd src-tauri
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked          # 35 单测 + 5 个 loopback 端到端(真实 TCP 路径)
```

## 项目结构

```
src/                 React 19 前端(桌面壳)
src-tauri/src/       Rust 核心
├── proto/           二进制协议(帧编解码 + 消息)
├── transfer/        传输引擎(sender/receiver/zerocopy/tls/meter)
├── discovery/       设备发现(mDNS / NSD / Bonjour 多后端)
├── engine.rs        移动端共享入口(FFI/JNI 复用)
├── ffi.rs           iOS C ABI 桥
└── jni_bridge.rs    Android JNI 桥
src-tauri/gen/apple/     iOS 原生工程(xcodegen)
src-tauri/gen/android/   Android 原生工程(Gradle)
website/             产品介绍页(单文件静态站)
docs/superpowers/    设计文档与实现计划
```

## 路线图

- [ ] macOS / Windows 正式签名公证,Android release 签名
- [ ] 应用内自动更新
- [ ] 断点续传体验完善(会话级恢复)
- [ ] 与 LocalSend 协议互通(评估中)

## 许可

[MIT](LICENSE) © 2026 WenQiang Zhao
