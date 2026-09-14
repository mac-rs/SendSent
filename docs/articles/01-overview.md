# 01 · 总览:把文件在局域网里丢过去

## 它解决什么问题

在同一 Wi-Fi 下,把文件从一台设备发到另一台,常见方案要么依赖云(上传再下载、
慢且经过第三方),要么受平台生态限制(AirDrop 只认 Apple,Nearby Share 只认 Android)。
SendSent 的目标很直接:**同一局域网内,发现、选择、发送、接收,不经过任何服务器。**

- 不需要账号、不需要配对码之外的任何配置;
- 不经过互联网,速度取决于局域网带宽(实测可跑满千兆);
- 桌面、iOS、Android 三端可以互相发现与互传。

## 设计原则

1. **局域网优先**:发现用 mDNS/Bonjour(零配置),传输走裸 TCP(低开销)。
2. **内核与界面解耦**:所有协议、传输、发现逻辑都在 Rust 里,UI 只是外壳。
3. **能用系统能力就用系统能力**:iOS 用系统 Bonjour、Android 用系统 NsdManager、
   macOS 用 `sendfile(2)` 零拷贝。
4. **默认快、可选安全**:默认明文 + 零拷贝追求速度;加密与校验是按次可选的开关。

## 端到端流程

```
        ┌────────────┐   mDNS/_sendsent._tcp    ┌────────────┐
        │  发送端     │ ◀──── 发现/广播 ────────▶ │  接收端     │
        │            │                          │            │
        │ 1. 发现设备 │                          │            │
        │ 2. 选文件   │ ── Hello/HelloAck ─────▶ │  监听 52225 │
        │            │ ── Manifest ───────────▶ │ 3. 弹窗确认 │
        │            │ ◀──────────── Accept ─── │            │
        │ 4. 开 N 条  │ ── DataOpen + 数据帧 ──▶ │ 5. 落盘     │
        │    数据连接 │ ◀──────────── Complete ─ │            │
        └────────────┘                          └────────────┘
```

## 技术栈一览

| 层 | 技术 | 说明 |
|----|------|------|
| 共享内核 | Rust 2024 + Tokio | 协议、传输、发现、历史、TLS、二维码 |
| 桌面 UI | Tauri 2 + React 19 + TypeScript | `src/` + `src-tauri/` |
| iOS UI | SwiftUI(原生,无 WebView) | 通过 C ABI 调 Rust 静态库 |
| Android UI | Kotlin + Jetpack Compose | 通过 JNI 调 Rust `.so` |
| 发现 | mdns-sd / Bonjour / NsdManager | 各平台"最合适"的实现 |
| 加密 | rustls + rcgen | 纯 Rust,移动端友好 |

## 仓库结构

```
src-tauri/                 # Rust 内核 + 桌面外壳
  src/
    engine.rs              # 平台无关核心:全局 Core + 操作 + 事件
    ffi.rs                 # iOS C ABI(sendsent_ios_*),目标 os=ios
    jni_bridge.rs          # Android JNI(Java_com_mankong_sendsent_Native_*)
    commands.rs, lib.rs    # 桌面(Tauri)命令与启动
    proto/                 # 帧格式 + 消息定义(postcard 载荷)
    transfer/              # sender / receiver / manager / tls / zerocopy / sock
    discovery/             # mdns(桌面) / ios_bonjour / android_native / nsd
    store.rs, history.rs, misc.rs
  gen/apple/               # iOS Xcode 工程 + SwiftUI 源码
  gen/android/             # Android Gradle 工程 + Kotlin 源码
src/                       # 桌面前端(React)
docs/                      # 设计文档 + 本技术文集
```

## 一句话总结架构

> **一份 Rust 内核负责"怎么传",三套原生 UI 负责"怎么用",平台差异被收敛到
> 三个地方:事件投递方式、文件来源、设备发现。**

后续文章依次展开:如何做到一套内核三端复用、协议长什么样、发现怎么跨平台、
加密与校验怎么落地、以及性能是怎么榨出来的。
