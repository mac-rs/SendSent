# 06 · 性能设计:把局域网带宽吃满

## 先想清楚瓶颈在哪

局域网传输的瓶颈通常不在 CPU,而在:

1. **单条 TCP 的吞吐上限**(受窗口、往返、丢失重传影响);
2. **用户态拷贝次数**(读文件 → 缓冲区 → 内核 socket);
3. **系统调用的粒度**(块太小,调用太频繁)。

对应三个手段:**多连接并行**、**尽量少的拷贝**、**合适的块大小 + 缓冲调优**。

## 多连接并行:大文件切段

单条 TCP 在 WiFi 上很容易成为瓶颈。v2 的做法是:一个会话开 N 条数据连接,
把大文件**按偏移切段**,每段走一条连接并行发送。

```rust
// 文件 >= split_threshold 就切 conns 段;否则整段,并轮询分配到各连接
if f.size >= split && conns > 1 {
    let each = f.size / conns as u64;
    // 段 i:[i*each, (i+1)*each),最后一段收尾到 size
}
```

- 默认 `conns = 16`,`split_threshold = 8 MiB`;
- 小文件不切分,但会**轮询**到不同连接,多发几个小文件也能并行;
- 段级切分对接收端是透明的:每个数据帧自带 `(file_id, offset)`,按位置写即可。

这让"一个 1 GB 大文件"从"单管慢跑"变成"多管齐下"。参数可按机器/网络调整,
写入 `transfer.json`,也可用环境变量临时覆盖(`SENDSENT_CONNS`、`SENDSENT_CHUNK_KB`、
`SENDSENT_SPLIT_MB`)。

## 块大小与套接字调优

- **块大小**(chunk)默认 **1 MiB**(等于数据帧载荷上限),减少系统调用次数;
- **套接字缓冲**:收发缓冲都调到 **16 MiB**,并关闭 Nagle(`TCP_NODELAY`)——
  避免小包延迟与"攒包"造成的抖动。

```rust
const BUF: usize = 16 * 1024 * 1024;
sref.set_recv_buffer_size(BUF); sref.set_send_buffer_size(BUF);
stream.set_nodelay(true);
```

## 零拷贝:一次被回退的实验

仓库里有一份**零拷贝原语** `transfer/zerocopy.rs`,在 macOS/Linux 上用
`sendfile(2)` 把文件内容直接从文件描述符推到 socket,绕过用户态:

```rust
#[cfg(target_os = "macos")]
async fn sendfile_macos(socket: &TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()> {
    // libc::sendfile(in_fd, out_fd, ...) 循环直到发完
}
```

它配套 `write_data_header`(先写 29 字节帧头,再用 `sendfile` 推载荷)以及测试。
需要如实说明的是:**这条路曾经真的接进了发送热路径,后来被主动回退。**

- `641d9da` 引入 `write_data_header` + `send_payload`(macOS `sendfile`);
- `10a22bd` 把它接进 v2 多连接发送;
- `299fb13` **回退**:commit message 明确写着——
  > Write-data-header + flush + sendfile still causes broken-pipe on
  > macOS→Android. Keep pread + write_data (combined frame), memcpy overhead
  > <1% at Gbps. Zero-copy code preserved in zerocopy.rs for future investigation.

即:**macOS `sendfile` 跨网络(macOS→Android)会 broken-pipe,即便先 `flush`
帧头也没解决**。于是当前发送热路径改回 `pread` 到缓冲 + `write_data`
(帧头与载荷合并写),作者评估在千兆下多一次 memcpy 的开销 <1%,不值得为此冒险。
`zerocopy.rs` 与测试被保留,供将来继续排查。

另外,**加密模式天然用不了它**:`sendfile` 只能把文件字节推进裸 socket,
无法经过 TLS 层,安全模式必须走 `fallback_send_payload`(即 `pread` + `write`)。

因此当前实际生效的提速手段是:**多连接 + 1 MiB 块 + 16 MiB 缓冲 + TCP_NODELAY**。
零拷贝原语已就绪但**未启用**(且不是"没写完",而是**踩过坑后有意回退**),
`fallback_send_payload` 这条回退路径则同时服务于加密模式。

### 后续:根因已定位并修复(可选启用)

回溯那次回退,根因是一个很具体的错误:**macOS `sendfile` 在 `EAGAIN` 时的记账漏了**。

macOS 的 `sendfile(fd, s, offset, *len, ...)` 里 `len` 是 value-result:
返回时它 = "已发送字节数",而且**在 `EAGAIN`(非阻塞 socket 缓冲写满)时也会写回**。
旧代码只在成功路径累加进度,`EAGAIN` 时直接 `continue` → **从同一 offset 重发** →
线上出现重复字节且没有新帧头 → 对端帧解析错位 → 断连 → 发送端 `EPIPE`(broken-pipe)。
小文件不触发 `EAGAIN`,所以"看着没事";macOS→Android 大文件最容易撞上。

修复:无论成功失败,都先把 `*len`(已发送字节)计入 `off/remaining`,再决定重试还是报错。
并补了一个**确定性回归测试**(把发送缓冲压到 8 KiB + 延迟读取,逼出 `EAGAIN`):
旧逻辑下该测试失败(长度不一致),新逻辑下通过。

真机验证:macOS→Android 传 **200 MiB** 且开启 **SHA-256 校验** → 接收端
`complete=true`、无 hash mismatch。

出于稳妥,零拷贝仍**默认关闭**,用环境变量按需开启:

```bash
SENDSENT_ZEROCOPY=1 ./sendsent        # 桌面;未设置或 =0 时走缓冲路径
```

在更广范围(不同文件系统/网络/平台)验证充分后,再考虑默认开启——
毕竟它在千兆下的收益 <1%,不值得为它承担未经充分验证的风险。

## 进度与统计:别让 UI 拖慢传输

进度事件如果每读一个块就 emit 一次,会淹没 UI。内核做了**节流**:

- 原生(iOS/Android)引擎对 `Progress` 事件按 **100ms/会话** 节流;
- 发送端用 `SpeedMeter` 每 200ms 采样一次吞吐,计算瞬时速度;
- 接收端同样每 200ms 汇报一次(按 `file_id` 统计已完成字节)。

统计用的是**原子计数 + 定时采样**,不阻塞数据传输任务。

## 接收端并发:位置写入,天然并行

接收端对每条数据连接起一个 drain 任务,多个任务**并发写同一个文件**:

- `Arc<File>` 共享句柄,按 `(offset)` **定位写入**(`pwrite`/`write_at`),
  不使用文件游标,所以并发安全;
- 完成判定用 `AtomicU64` + `Notify`:所有文件的已收字节达到大小即通知;
- 数据连接通过 `mpsc` 交给会话,会话起 drain;空闲 2s 无新连接即认为收尾。

## 参数速查

| 参数 | 默认 | 位置 | 环境变量 |
|------|------|------|----------|
| 并发连接数 `conns` | 16 | `store.rs` | `SENDSENT_CONNS` |
| 块大小 `chunk_size` | 1 MiB | `store.rs` | `SENDSENT_CHUNK_KB` |
| 分片阈值 `split_threshold` | 8 MiB | `store.rs` | `SENDSENT_SPLIT_MB` |
| 收发缓冲 | 16 MiB | `transfer/sock.rs` | — |
| 进度节流 | 100 ms | `misc.rs::Throttle` | — |

## 小结

- 大文件靠**多连接分片**并行;
- 块大小/缓冲/NODELAY 决定系统调用开销与抖动;
- **零拷贝原语已备好但未上热路径**,加密模式也用不了它——这是速度与安全的取舍点;
- 进度统计必须节流,否则会反过来拖慢传输。
