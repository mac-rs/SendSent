# 设计文档:文件传输 v3 安全层

- 日期:2026-06-26
- 项目:SendSent(Tauri 2 + React 19 + TS)
- 范围:**v3 安全层**。v1 打通管线、v2 兑现速度;v3 加可选加密传输 + PIN 身份验证。
- 关系:本设计是 v2 的增量,不改变默认行为(明文+sendfile)。v3 移动端/续传与体验完善为后续独立子项目。

---

## 1. 目标与约束

### 1.1 目标
给现有明文传输加一层**可选**的 TLS 加密 + 6 位 PIN 配对,防止 LAN 偷听和陌生设备接入。默认模式(明文+零拷贝)不受影响;只有发送端主动开"加密"才走 TLS-PIN 流程。

### 1.2 已确认约束(brainstorming 结论)

| # | 约束 | 取值 |
|---|------|------|
| 1 | 模式 | **双模式**:默认明文+sendfile(速度优先),可选 TLS(安全,回退 `pread`+write) |
| 2 | PIN 交互 | 发送端生成 6 位随机 PIN 并显示;接收端输入后回传验证 |
| 3 | TLS 技术 | `rustls`(纯 Rust,移动兼容) + 自签名 ECDSA 证书;不作 CA 验证(PIN 即信任) |
| 4 | 加密范围 | 安全模式下**所有连接**(1 条控制 + N 条数据)均升级为 TLS |

---

## 2. 架构总览

### 2.1 安全模式握手

```
发送端(安全模式)                              接收端
  控制连接 TCP — 明文 Hello{ secure:true }  →   accept → 读首帧
    (发送端 UI 显示 PIN placeholder)             ← 若本端也允许 → HelloAck{ secure_ok:true }
  ← 双方升级此控制连接为 TLS(rustls) →
    [TLS 内] PinCode{ pin: "123456" }  →      接收端 UI 弹出:输入 6 位 PIN
                                               ← PinCode{ pin: "<user input>" }
  发送端比较 PIN:
    匹配 → 继续:发 Manifest、等 Accept
    不匹配 → Error{IncompatibleVersion? → 替换 ErrorCode}、断开、清理
  Accept 后,N 条数据连接:每条
    明文 DataOpen → TLS 升级 → 数据帧(加密)
```

- 安全模式下 sender 不能用 `sendfile`(sendfile 只支持裸 socket fd),全部 payload 走 `pread`+write 回退(zerocopy.rs 的 `fallback_pread_write`)。
- 接收端安全模式下句柄复用 + `write_at` 不受影响(v2 receiver 已具备)。

### 2.2 新增/改动模块

| 文件 | 变化 | 职责 |
|------|------|------|
| `transfer/tls.rs` | **新增** | 证书生成/持久化/加载 + TLS acceptor/connector 工厂 |
| `proto/messages.rs` | 改 | `Hello` 加 `secure: bool`;新增 `PinCode { pin: String }` 消息+MsgType |
| `transfer/sender.rs` | 改 | 安全模式:TLS connect + PIN exchange + 数据连接 TLS;回退 sendfile |
| `transfer/receiver.rs` | 改 | 安全模式:TLS accept + PIN verify + 数据连接 TLS |
| `transfer/manager.rs` | 改 | 根据 session 安全标志,TLS wrap 数据连接流 |
| `lib.rs` | 改 | 启动生成/加载 TLS 证书,存 AppState |
| `state.rs` | 改 | AppState 加 `tls_config: Arc<rustls::ServerConfig>`(或 shared) |
| 前端 | 改 | 加密勾选框(发端)、PIN 输入框(收端) |

---

## 3. TLS 证书管理

### 3.1 生成与持久化
- App 启动时:`transfer/tls.rs::load_or_generate_cert(data_dir)`。
- 首次运行:生成 ECDSA P256 密钥对 + 自签名 X.509 证书(有效期 ~10 年)。
- 持久化为 `key.pem` / `cert.pem`,存入 `<app_data_dir>/tls/`。
- 用 `rcgen` crate 生成证书(轻量、纯 Rust),或直接调用 `rustls` 的 cert 生成 API(`rustls-pki-types` 生态)。推荐 `rcgen`(简洁)。
- 新依赖:`rustls` + `tokio-rustls` + `rcgen`(证书生成)。

### 3.2 TLS 端点
- **接收端是 TLS server**(连接收方,listen→accept → TLS acceptor)。
- **发送端是 TLS client**(发起方,connect → TLS connector)。
- 数据连接同理:每条数据连接,接收端 accept 流后 TLS accept;发送端 connect 流后 TLS connect。
- **不验证对方证书**(不设 CA、不做 hostname check):我们不用 TLS 做身份认证,只做信道加密。身份信任由 PIN 提供。

---

## 4. 协议改动

### 4.1 Hello 加 secure 字段
```rust
pub struct Hello {
    pub device_id: String, pub name: String, pub platform: Platform,
    pub session_id: Uuid, pub proto_ver: u8,
    pub secure: bool,  // NEW
}
```
- `secure=false`(默认):正常明文流程,与 v2 完全一致。
- `secure=true`:发送端请求安全模式。

### 4.2 HelloAck 加 secure_ok
```rust
pub struct HelloAck {
    pub device_id: String, pub name: String,
    pub secure_ok: bool,  // NEW;接收端也支持时才 true
}
```

### 4.3 新增 PinCode 消息
```rust
MsgType::PinCode = 0x0B
pub struct PinCode { pub pin: String }
```
- 发送端 PIN 生成后发此帧。
- 接收端发回用户输入的 PIN。

### 4.4 bincode 兼容性
- `Hello`/`HelloAck` 加字段 → bincode 反序列化旧版会报错(字段数不匹配)。**v3 不与旧版互通**(协议已升级),全员同升。proto_ver 可不动(字段在已有 Hello 后追加,bincode 按序反序列化,旧版读新 Hello 会因字段多而失败——未发布的内部工具,可接受)。

---

## 5. 安全模式发送端 `run_sender_inner`

安全模式分支(Hello.secure=true 时):
1. **控制连接**:connect → 明文 Hello → 读 HelloAck.secure_ok → **升级为 TLS**:
   ```rust
   let tls_stream = client_tls_config.connect(domain, stream).await?;
   ```
   (domain="sendsent.local" 或空,DNS 名不验证)
2. **PIN exchange**(TLS 内):生成 6 位随机 PIN → `write_control(PinCode{pin})` → UI 显示(全流程 event? 前端通过 `events.send(Progress{state=AwaitingPin})` + 带 PIN 的专用事件 或前端另行生成)。发送端生成 PIN → event → 前端显示。
   等接收端回 PinCode → 比对;不匹配则 `Error` + 断开。
3. **Manifest / Accept 不变**(TLS 内)。
4. **数据连接**:每条 connect → 明文 DataOpen → 升级 TLS → 数据帧(加密)。

安全模式下 `send_segment` 不用 `send_payload`(sendfile),改用 `fallback_pread_write`:
- 在 `zerocopy.rs` 暴露 `pub async fn fallback_send_payload(socket, file, offset, len)`(非 cfg-gated)。

---

## 6. 安全模式接收端 `run_receiver`

安全模式分支(hello.secure=true 时):
1. HelloAck.secure_ok=true → **升级控制连接为 TLS**(server accept):
   ```rust
   let tls_stream = server_tls_config.accept(stream).await?;
   ```
2. **PIN verify**(TLS 内):读 `PinCode`(发送端 PIN) → 前端弹窗让用户输 PIN → 发回 `PinCode`。发送端验证结果不传回接收端(发送端直接 pass/fail)。若 fail,发送端断连(接收端 detect EOF → Failed)。
3. Manifest/Accept/数据 drain 不变(TLS 内)。

---

## 7. 数据连接 TLS 的协调

- 管理器 `handle_incoming`:读首帧后根据 session 查找安全标志(已存储在 SessionChannels 或 session 元数据)。若是安全模式 DataOpen,先 TLS accept 再投入 data channel(或先投入再升级——在 drain 函数里做)。
- 建议:在 `SessionChannels` 加 `secure: bool` 字段(Hello 时写入)。DataOpen 时查询:若是安全 → receiver 在 drain_stream 里 TLS accept 后读取;或在 drain 任务入口先升级。
- 发送端数据连接:connect → 明文 DataOpen → TLS connect → 发数据帧。

---

## 8. 前端改动

- **发送端**:在 FilePicker 旁加一个☑"加密"勾选框。勾上时 `sendFiles` 传 `secure: true` 到命令(命令再传给 Hello)。
  或:发 `send_files` 时命令生成 Hello，`secure` 参数需从命令传到 run_sender。选择:在 `start_send`/`run_sender` 参数加 `secure: bool`。命令 `send_files` 的 JS arg 加 `secure?: boolean`。
- **发送端 PIN 显示**:安全模式发送端生成 PIN 后 emit 一个 `transfer://pin` 事件(含 `pin: String`),前端显示"对方需输入 PIN:123456"。
- **接收端 PIN 输入**:安全模式收到 `Request` 后,front end UI 额外给一个 PIN 输入框;用户点接受时一并将 PIN 传给 `respond` 命令。或 PinCode 由接收端自动发(前端输入后经独立命令 `send_pin` 发 PinCode)。

更简洁:接收端的 PIN 输入和"接受"按钮绑定——弹框里同时有"接受"按钮和 PIN 输入框,点接受时一起发(respond 命令加 `pin: String`)。如 PIN 非空,run_receiver 先发 PinCode 再后续流程。

---

## 9. 测试策略

- **证书生成/加载**:单元测试(`tls.rs` 的生成→持久化→加载 round-trip)。
- **TLS 通道 round-trip**:新增 loopback 集成测试——安全模式下完整传输 10MB 文件,验证加密路径 + 完整性。
- **PIN 错误测试**:发送端生成 PIN→ 接收端送错 PIN→ 发送端 Error + 断开,文件不落地。
- **模式切换**:默认模式(secure=false)端到端仍通(回归 v2)。
- **既有 23 测试保持绿**,`cargo clippy --all-targets -- -D warnings` 干净。

---

## 10. 范围边界(v3 不做)

- 不做 CA 证书验证或双向 mTLS。
- 不做 QR 码交换(后续体验完善子项目)。
- 不做 E2E 文件级加密(传输层加密已覆盖)。
- 不强制 TLS(默认仍是明文+sendfile,速度优先)。

---

## 11. v3 完成定义(acceptance)
1. 模式切换:默认明文正常;安全模式下 TLS 通道 + 文件完整传输;
2. PIN 配对:发送端显示 PIN、接收端输入正确→通过;错误→拒绝;
3. 安全模式下不使用 sendfile(回退到 pread+write);
4. 所有连接(控制+数据)在安全模式下均为 TLS;
5. 既有 23 测试全绿、clippy 干净;
6. 证书启动生成+持久化。
