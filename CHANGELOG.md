# 更新日志

本项目所有显著变更记录在此文件中。格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/),版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

- CI:新增五路检查流水线(前端 / Rust Linux / Rust macOS+iOS / iOS App / Android),见 `.github/workflows/ci.yml`。
- Release:打 `v*` tag 自动构建三平台桌面包与 Android APK,并以 CHANGELOG 对应小节为正文生成草稿 Release,见 `.github/workflows/release.yml`。
- 依赖升级:mdns-sd 0.21、sha2 0.11、base64 0.23、android_logger 0.15、jni 0.22(含 Android JNI 桥的 `EnvUnowned`/`with_env` API 迁移,Kotlin 侧无感知)。
- FFI:iOS 接收裸指针的 C ABI 函数改为 `unsafe extern` 并补齐 `# Safety` 文档(ABI 不变)。
- 产品介绍页:`website/index.html`(单文件、零依赖)。

## [0.1.0] - 2026-10-05

首个公开版本:局域网点对点文件 / 文本互传,无服务器、无账号、无遥测。

### 传输核心(Rust)

- 自定义二进制协议(控制帧 + 数据帧,postcard 序列化),TCP 直连(`_sendsent._tcp.local.`,默认端口 52225)。
- mDNS 自动发现,设备上下线实时感知(主动 TCP 探活);支持手动 IP 直连与二维码配对。
- 多连接并行传输(默认 16 条):大文件按字节区间分片,小文件轮询流水线分发。
- 零拷贝 `sendfile(2)` 发送路径(设置中可开关;TLS 模式或平台不支持时自动回退 `pread` + write)。
- Socket 调优:16 MiB 收发缓冲 + `TCP_NODELAY`。
- 可选 TLS 加密(rustls + 自签 ECDSA 证书):1 条控制连接与全部数据连接均升级加密,6 位 PIN 完成设备配对,陌生设备无法接入。
- 可选 SHA-256 逐文件完整性校验(流式计算,恒定内存)。
- 断线保留已接收分片,重连按数据桶续传。
- 传输历史持久化(JSON 落盘,含成功 / 失败 / 取消结果)。

### 桌面应用(macOS / Windows / Linux)

- Tauri 2 + React 19;macOS 玻璃质感 UI、覆盖式标题栏、暗色模式。
- 雷达式设备视图、多选群发、文本互传、传输速度表、历史记录。

### iOS(SwiftUI 原生)

- Rust 静态库直链(无 WebView、无 Tauri 运行时)。
- 雷达首页 / 传输列表 / 个人资料 / 设置;PhotosPicker 直接发送相册照片。
- 系统 Bonjour 广播;回前台自动重新广播,其他设备可重新发现。

### Android(Kotlin + Jetpack Compose 原生)

- JNI 桥接共享 Rust 引擎;NSD 发现由 Kotlin 驱动(Rust 侧不可用组播)。
- SAF 文件经 `ParcelFileDescriptor` 传入 fd 读取(绕过 SELinux 路径限制)。
- CameraX + ZXing 扫码添加设备;Material 3 交互。

### 已知限制

- iOS 后台会挂起 App,Bonjour 广播随之暂停——保持前台即可被发现。
- CI 产出的 macOS 包为 ad-hoc 签名,首次打开需右键 →「打开」;本机正式签名构建见 README。
- Release 附带的 Android APK 为 debug 签名(正式签名密钥就绪后切换)。
- 协议未做向后兼容:0.1.0 的多连接发送端无法与更早的单连接接收端互传。
