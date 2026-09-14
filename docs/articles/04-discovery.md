# 04 · 设备发现:三端三套机制,一套注册表

## 零配置发现 = mDNS/DNS-SD

局域网里"看见彼此"最简单的方式是 **mDNS + DNS-SD**:设备把自己注册成一个服务,
其它设备浏览这个服务类型即可。SendSent 用的是:

- 服务类型:`_sendsent._tcp.local.`
- 实例名:设备显示名
- 主机名:`<显示名,空格替换为连字符>.local.`
- TXT 记录:`v`(协议版本)、`id`(设备唯一 id)、`name`、`plat`(平台)、`port`、`ip`

TXT 里同时带 `ip` 很重要——见下面 iOS 的坑。

## 三个平台,三种实现

| 平台 | 实现 | 原因 |
|------|------|------|
| 桌面 | `mdns-sd`(纯 Rust,自己开多播 socket) | 桌面无沙盒限制,纯 Rust 依赖最省事 |
| iOS | 系统 Bonjour(`zeroconf` crate 包 DNS-SD C API) | iOS 不允许 App 自己开原始多播,除非拿到受限的 multicast 权限 |
| Android | 系统 `NsdManager`(Kotlin 驱动) | Android 的 SELinux 拒绝 netlink/多播路径,Rust 侧 `mdns-sd` 不工作 |

三者都注册**同一个** `_sendsent._tcp.local.`,因此可以互相发现。

## 统一注册表:`PeerRegistry`

无论发现来自哪套机制,最终都进入同一个注册表(`discovery/mod.rs`):

```rust
pub struct PeerRegistry { peers: HashMap<String, Peer> }   // key = device_id
```

- `upsert` 只在**首次**出现时发 `Found` 事件(去重,避免重复刷新 UI);
- `remove` 发 `Lost` 事件;
- `sweep` 清掉长时间没被刷新(默认 90s)的条目,作为兜底;
- **自我过滤**:忽略 `device_id == 自己` 的结果,否则会"看见自己"。

平台适配只需要实现一个 `Discovery` trait(`start/peers/set_display_name/reannounce/…`),
注册表与事件投递由内核统一处理。

## iOS:用系统 Bonjour,并"回前台重注册"

iOS 有两个专门的问题:

1. **不能自己开多播**:`mdns-sd` 这类自绘 mDNS 在 iOS 上会被拦(需要受限的
   `com.apple.developer.networking.multicast` 权限,很难申请)。所以 iOS 改用
   **系统 Bonjour**——多播由系统 responder 负责,只需在 `Info.plist` 声明
   `NSBonjourServices` + 触发本地网络权限弹窗。
2. **后台被挂起**:App 进后台后系统会停掉 Bonjour 广播,回到前台**不会自动恢复**。
   做法是让广播线程监听一个"代号"(generation counter):回前台时递增代数,
   线程检测到变化就**注销旧注册、以新名字重新注册**。改显示名走的也是同一条路径。

还有一个隐蔽的数据坑:系统 Bonjour 解析出来的地址在**双栈**环境下可能不可靠
(Bonjour 的地址解析在某些桥接下会把 IPv6 当 IPv4 解析)。所以解析对端地址时
**优先用 TXT 里的 `ip` 字段**,没有再回退到主机名解析,并且**优先 IPv4**,
避免把 TCP 发送端指向一个 IPv6-only 的目标。

## Android:发现交给 Kotlin

Android 的 `NsdManager` 是 Java API,Rust 侧拿不到。做法是:

- Kotlin 用 `NsdManager` 注册/浏览(`NsdBridge.kt`);
- 解析到服务后,把 `(name, host, port, txt)` 通过 `Native.nativeOnService(...)` 推给 Rust;
- Rust 的 `android_native` 适配器把它转成 `Peer` 塞进注册表;
- 服务丢失时 `nativeOnServiceLost`,直接移除。

因为广播由 Kotlin 持有,**改显示名之后也要由 Kotlin 重新 `register`**,Rust 侧无法代劳。

## 下线感知:三条路径

发现"某设备下线"有三条路径,优先级不同:

1. **服务注销(goodbye)**:对端优雅退出时发 TTL=0,浏览方收到 `ServiceRemoved`。
2. **主动探活(桌面)**:mDNS 缓存会"骗人"(见下),桌面每 10s 主动 TCP 连接对端
   接收端口,连续 2 次失败即判定下线并移除。
3. **过期扫描(兜底)**:90s 没被刷新就清掉。

## 一个反直觉的点:`mdns-sd` 会从缓存里"复活"已下线设备

桌面的 `mdns-sd` 会**从自己的缓存里反复重发 `ServiceResolved`**,于是
"最后一次看见该设备的时间"一直被刷新,单靠时间戳判断过期的扫描**永远不会触发**。
iOS/Android 用系统 mDNS,系统缓存会正确过期,所以它们下线感知很快;桌面却感知不到。

这就是为什么桌面额外加了**主动 TCP 探活**:与其相信 mDNS 缓存,不如直接连一次
对端的接收端口——连不上就是下线。代价是每 10s 每个对端一次连接尝试,在局域网上
可以忽略不计。

## 广播与"被限制"的两个系统权限(macOS)

- 桌面用 `mdns-sd` 自绘多播,**不能开 App Sandbox**(沙盒会挡原始多播);
- 正式包必须**正确签名**:macOS 的"本地网络"权限只授予有有效签名的 App,
  ad-hoc 包既不弹权限、也被直接拒绝。开发时用终端启动会借用终端的权限,
  所以"dev 正常、release 发现不到"——这一点在踩坑篇里详述。

## 小结

跨平台发现的本质是:**协议统一(mDNS/DNS-SD)、实现分平台、状态收敛到一张注册表**。
再叠加两件工程现实:**移动端只能用系统发现框架**、**桌面要主动探活来对抗缓存**。
