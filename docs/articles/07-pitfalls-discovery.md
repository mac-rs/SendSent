# 07 · 踩坑记(一):发现、权限与生命周期

这一篇收集**设备发现/网络/系统权限/后台生命周期**相关的坑。每一个都是真实踩过、
并已修复的。格式:现象 → 根因 → 修复 → 教训。

---

## 坑 1:「dev 能发现,release 发现不到」——ad-hoc 签名没有本地网络权限

**现象**:`pnpm tauri dev` 正常发现设备;`pnpm tauri build` 出来的正式包一个都发现不到。

**根因**:正式 `.app` 是 **ad-hoc / linker-signed**——没有 Team ID,`Entitlements.plist`
实际上从未被应用。macOS 的"本地网络"权限只授予**有有效签名**的 App;
ad-hoc 包既不弹权限、也直接被拒,`mdns-sd` 的原始多播被挡。
dev 之所以正常,是因为裸二进制由终端启动,借用了终端已获得的本地网络权限。

**修复**:在 `tauri.conf.json` 配置真实签名身份,并注意**同名证书可能有多份
(含已吊销),必须用证书哈希**消歧:

```json
"macOS": {
  "entitlements": "Entitlements.plist",
  "signingIdentity": "7FEA3AD4E758C6123AA187F9593DE6A1D8344289"
}
```

签名后校验:`TeamIdentifier` 存在、`entitlements` 里有 network 权限。

**教训**:`dev 正常 ≠ release 正常`。涉及系统权限(本地网络、沙盒、多播)时,
**正式包必须单独验**;"没报错"往往意味着"被静默拒绝"。

---

## 坑 2:iOS 切后台再回来,别的设备再也发现不到

**现象**:iPhone 切到后台,Android 就发现不到它;切回前台仍然发现不到,必须重启 App。

**根因**:iOS 在后台会**挂起** App,系统的 Bonjour 广播随之停止;
回到前台时系统**不会自动恢复**这条注册。广播线程还在(进程没死),
但它注册的那条服务已经失效了。

**修复**:引入"代号"(generation counter)。回前台时递增代数,广播线程发现代号变了,
就**注销旧注册、重新注册**。改名走的也是同一条路径。

```rust
async fn reannounce(&self) -> Result<()> {
    self.generation.fetch_add(1, Ordering::SeqCst);   // 触发重注册
    Ok(())
}
```

Swift 侧监听场景相位:

```swift
.onChange(of: scenePhase) { _, phase in
    if phase == .active { core.reactivate() }   // 回前台重新广播
}
```

**教训**:移动端的"进程还活着"不等于"服务还在广播"。**生命周期事件要显式
驱动系统服务的重注册**。

---

## 坑 3:iOS 用 `mdns-sd` 开不了多播

**现象**:iOS 上自绘 mDNS(纯 Rust 开原始多播 socket)不工作。

**根因**:iOS 不允许 App(非系统框架)自行开原始多播,除非拿到
`com.apple.developer.networking.multicast` 受限权限——对"自己实现 mDNS"的
App 很难申请。

**修复**:iOS 改用**系统 Bonjour**(`zeroconf` crate 包 DNS-SD C API),
多播交给系统 responder;只需在 `Info.plist` 声明 `NSBonjourServices` 并触发权限弹窗。

**教训**:能借系统能力就别自己造。**跨平台不是"一套实现处处跑",而是
"协议统一、实现分平台"**。

---

## 坑 4:Android 读 `/proc/self/fd/N` 被 SELinux 拒绝

**现象**:Android 通过 SAF 选文件后,发送时报权限错误。

**根因**:SAF 返回 `content://`,无法按路径 `open`;先前的做法是建一个指向
`/proc/self/fd/<fd>` 的符号链接按路径打开——但 **SELinux 拒绝按路径访问该 fd**。
另外 Android 后台/多播路径也被限制(见坑 3),所以发现改为 Kotlin 驱动。

**修复**:取出底层 fd 后,一方面建符号链接保留原文件名,**同时把 fd 登记到内核**,
发送时直接 `dup(fd)` 读取:

```rust
crate::misc::register_fd(link_str, fd);   // 发送时 dup 后直读
```

**教训**:移动端的"文件路径"是一种假象。**尽早拿到 fd,而不是路径**。

---

## 坑 5:macOS App Sandbox 挡多播、还重定向 `$HOME`

**现象**:某个阶段的 macOS 正式包发现不到设备,且默认保存目录不对。

**根因**:App Sandbox 会**拦住原始多播**(mDNS),还会把 `$HOME` 重定向到容器,
导致 `~/Downloads/...` 之类的路径不再是用户以为的那个。

**修复**:直接分发的 App **去掉 App Sandbox**(`Entitlements.plist` 移除
`com.apple.security.app-sandbox`),并补上本地网络用途声明与 Bonjour 服务声明。

**教训**:沙盒是"要么全有要么没有"的约束。**不打算上架 Mac App Store 时,
沙盒带来的麻烦往往大于收益**。

---

## 坑 6:`mdns-sd` 会从缓存里"复活"已下线设备

**现象**:iOS/Android 下线很快就能感知;**macOS 却始终不移除**——明明设备已经关了。

**根因**:桌面用的 `mdns-sd` 会**从自己的缓存**反复重发 `ServiceResolved`。
于是"最后看见时间"一直被刷新,靠时间戳判断过期的扫描**永远不触发**。
系统 mDNS(iOS/Android)会正确过期,macOS 这套不会。

**修复**:桌面额外加**主动 TCP 探活**——每 10s 连接对端接收端口,连续 2 次失败即移除。
不再相信 mDNS 缓存。

**教训**:用户态实现的 mDNS 库,"事件"不等于"网络事实"。**需要下线判定时,
用能被证伪的信号(连一次)而不是缓存的时间戳**。

---

## 坑 7:一个死锁,让上一条修复完全失效

**现象**:加了主动探活后,设备**仍然**不移除。

**根因**:探活任务里写了——

```rust
for p in registry.lock().await.list() {   // ❌
    registry.lock().await.touch(&p.device_id);   // 死锁
}
```

`for x in expr {}` 中,**`expr` 里的临时量会存活到整个循环结束**。于是
`MutexGuard` 一直没释放,循环体里再 `lock()` 就永久阻塞(tokio 的 Mutex 不可重入)。
探活任务**第一遍就卡死**,自然什么都不做。

**修复**:先把列表取到变量,释放锁后再进循环:

```rust
let peers = registry.lock().await.list();   // ✅ 快照
for p in peers { registry.lock().await.touch(&p.device_id); }
```

**教训**:`for ... in guard-producing-expression` 是一个经典陷阱;
**在循环体里再取同一把锁之前,先让 guard 落到一个变量上**。
修复后实测:强杀对端 App,约 20s 内桌面端移除该设备。

---

## 坑 8:过期扫描从来没生效过——时间戳算错了

**现象**:注册表里的设备永远不过期。

**根因**:实现把"最后看见时间"存成"**本次调用新建的 `Instant` 的 `elapsed()`**":

```rust
p.last_seen_ms = now.elapsed().as_millis();        // upsert:调用者传 now=Instant::now()
let now_ms = now.elapsed().as_millis();            // sweep:又新建一个 Instant
now_ms.saturating_sub(p.last_seen_ms) > STALE      // ≈ 0,永远为假
```

两边都各自新建 Instant,算出来永远是 ~0,比较永远不成立。

**修复**:改用**进程级单调基准**(自启动以来的毫秒数),并去掉调用点的
`Instant` 参数;补上回归测试锁住行为。

**教训**:时间戳要用**同一个基准**。跨调用比较的绝对时间,不能是"相对某个临时值"。
这类 bug 不会崩溃、只会"静默不生效",**必须靠测试兜住**。

---

## 坑 9:下线事件匹配不上(空格/转义)

**现象**:对端优雅退出会发 goodbye,但桌面端没删掉对应条目。

**根因**:`ServiceRemoved` 事件其实带了**实例名**,但代码忽略它,改从 `fullname`
里截第一段,再和 `name.replace(' ', "-")` 比较——**带空格/转义的实例名对不上**。

**修复**:直接用事件给的实例名与 `peer.name` 比较(保留空格→连字符的兼容分支)。

**教训**:**别自己从完整字符串里"截"出字段**,事件既然给了结构化字段就用它。

---

## 小结:这一类坑的共同模式

| 模式 | 例子 | 对策 |
|------|------|------|
| 系统权限被"静默拒绝" | 本地网络、多播、SELinux | 正式包单独验;优先系统框架;拿 fd |
| 生命周期不同步 | iOS 后台挂起广播 | 用生命周期事件驱动重注册 |
| 缓存 ≠ 事实 | mdns-sd 复活已下线设备 | 用主动探活这种"可证伪"信号 |
| 静默不生效 | 时间戳基准错、for 循环 guard 死锁 | 回归测试 + 锁粒度审查 |
| 字符串硬解析 | fullname 截字段 | 用结构化字段 |
