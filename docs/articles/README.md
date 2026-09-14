# SendSent 技术文集

一套面向局域网的点对点文件传输工具:同一个 Rust 内核,一套自定义 TCP 协议,
在桌面(Tauri)、iOS(SwiftUI)、Android(Compose)三种原生外壳下共用。

## 技术篇

| # | 文章 | 主题 |
|---|------|------|
| 01 | [总览](01-overview.md) | 是什么、为什么、整体架构、仓库结构 |
| 02 | [跨平台架构](02-cross-platform-architecture.md) | 一套内核三种外壳:feature 门控、FFI/JNI、生命周期 |
| 03 | [传输协议](03-protocol.md) | 自定义 TCP 二进制协议:帧、握手、多连接、原子落盘 |
| 04 | [设备发现](04-discovery.md) | mDNS / Bonjour / NsdManager 的跨平台原理与权限 |
| 05 | [安全与校验](05-security.md) | 可选 TLS 加密、PIN、SHA-256 校验与安全边界 |
| 06 | [性能设计](06-performance.md) | 多连接并发、缓冲调优、零拷贝现状与配置 |

## 踩坑篇

| # | 文章 | 主题 |
|---|------|------|
| 07 | [踩坑记(一):发现、权限与生命周期](07-pitfalls-discovery.md) | ad-hoc 签名、iOS 后台广播、SELinux、mdns 缓存、**死锁**、时间戳 |
| 08 | [踩坑记(二):协议、TLS 与传输](08-pitfalls-transfer-tls.md) | **TLS RST → 0 字节**、serde tag 大小写、`runtime.enter()`、版本兼容 |
| 09 | [踩坑记(三):UI、构建与发布](09-pitfalls-ui-build.md) | 相册、二维码模糊、空实现、控件度量、xcodegen、装包 |

> 代码引用相对仓库根目录。核心在 `src-tauri/src/`,桌面前端在 `src/`,
> iOS 原生在 `src-tauri/gen/apple/`,Android 原生在 `src-tauri/gen/android/`。
