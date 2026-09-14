# 03 · 传输协议:为什么自己写一个二进制协议

## 设计取舍

文件传输听起来可以套用现成方案:HTTP、gRPC、WebSocket。但局域网点对点传输的需求
很具体,反而让"裸 TCP + 极简自描述帧"成为最省事、最快的选择:

- **控制与数据分离**:小控制消息需要确认/等待,大块数据需要尽可能少的结构开销;
- **多连接并发**:同一会话可以把大文件切成多段,在多条 TCP 上并行;
- **断点语义简单**:每个数据块自带 `(file_id, offset)`,接收端按位置写入,天然支持乱序/并行;
- **零依赖**:不引入 HTTP/gRPC 语义,接收端是一个几十行的读循环。

载荷序列化用 **postcard**(`no_std` 友好的紧凑二进制),帧头则手写定长字段
(big-endian),两边都不需要额外的 schema 代码生成。

## 两种帧

### 控制帧(7 字节头 + 载荷)

```
 0        1        2        3                              6
+--------+--------+--------+------------------------------+
| MAGIC  | VER    | TYPE   |        PAYLOAD LEN (u32BE)   |
| 0x53   | 0x01   | u8     |                              |
+--------+--------+--------+------------------------------+
|                     PAYLOAD (postcard)                    |
+-----------------------------------------------------------+
```

```rust
pub async fn write_control<W: AsyncWriteExt + Unpin>(w: &mut W, ty: MsgType, payload: &[u8]) -> io::Result<()> {
    let mut hdr = [0u8; 7];
    hdr[0] = MAGIC; hdr[1] = PROTO_VER; hdr[2] = ty as u8;
    hdr[3..7].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    w.write_all(&hdr).await?; w.write_all(payload).await?; Ok(())
}
```

### 数据帧(29 字节头 + 载荷)

```
 0        1                                17        25        29
+--------+--------------------------------+---------+---------+
| TAG    |           FILE_ID (uuid)        | OFFSET  | LEN     |
| 0xD5   |              16 bytes           | u64 BE  | u32 BE  |
+--------+--------------------------------+---------+---------+
|                        PAYLOAD (raw bytes)                 |
+------------------------------------------------------------+
```

`(file_id, offset)` 是这套协议的"定位器":接收端据此 `pwrite` 到文件的正确位置,
因此**多连接、乱序到达、重排都不影响正确性**。上限 `MAX_DATA_PAYLOAD = 1 MiB`、
`MAX_CONTROL_PAYLOAD = 4 MiB`。

## 消息类型

```rust
pub enum MsgType {
    Hello = 0x01, HelloAck = 0x02, Manifest = 0x03, Accept = 0x04,
    Reject = 0x05, Progress = 0x06, Complete = 0x07, Error = 0x08,
    Cancel = 0x09, DataOpen = 0x0A, PinCode = 0x0B, VerifyInfo = 0x0C,
}
```

一次会话的生命周期:

```
发送端                                       接收端(单监听端口 52225)
  │  TCP 连接(控制)                            │
  │ ── Hello{ session_id, proto_ver, secure } ▶│  读首帧 → 新建会话
  │ ◀──────────────────────── HelloAck{ secure_ok } │
  │ ── Manifest{ files[], total_size } ───────▶│  emit Request → 用户确认
  │ ◀────────────────────────────── Accept ────│
  │                                            │
  │  N × TCP 连接(数据)                        │
  │ ── DataOpen{ session_id } ────────────────▶│  按 session_id 路由到会话
  │ ── 数据帧 ... 数据帧 ... ─────────────────▶│  pwrite 到 .part 文件
  │                                            │
  │ ── VerifyInfo{ hashes }(可选) ───────────▶│  SHA-256 校验
  │ ◀────────────────────────────── Complete ──│  重命名 .part → 最终文件
```

接收端只有一个监听端口:**首帧是 `Hello` 就是新会话,是 `DataOpen` 就把这条连接
按 `session_id` 交给已存在的会话**(`manager.rs::handle_incoming`)。

## 多连接与分片

`plan_buckets` 决定"哪个文件走哪条连接":

- 文件 `size >= split_threshold`(默认 8 MiB):切成 `conns` 段,**每段一条连接并行**;
- 小文件:作为整段,轮询分配到各连接(多条小文件也能并行)。

```
                 ┌── conn0 ──┐
  big.bin  ──┬──▶│ [0,  1/16) │──┐
             ├──▶│ [1/16,2/16)│  │
             │   └───────────┘  │
             │   ┌── conn1 ──┐  │   接收端按 (file_id, offset)
             └──▶│ [2/16,...) │  ├─▶ pwrite 到同一 .part 文件
                 └───────────┘  │
                    ...          ┘
```

发送端的 `send_bucket` 对每条连接:

1. `connect_any(peer_addrs)` —— 依次尝试对端解析出的地址;
2. 明文写 `DataOpen{ session_id }`;
3. 循环 `pread` 文件 → 写数据帧(加密模式下先把这条连接升级为 TLS)。

> 注意:接收端会并行 `pwrite` 同一文件的不同区段,所以句柄用 `Arc<File>` 共享、
> 定位写入,不依赖文件游标。

## 原子落盘:下载到一半不会留脏文件

接收端不直接写目标文件,而是写进临时目录,全部完成且校验通过后再改名:

```
<save_dir>/.sendsent-tmp/<session_id>/<扁平化文件名>.part
        │  完成 + (可选)校验通过
        ▼
<save_dir>/<原始相对路径>/<文件名>(重名自动 " (1)")
```

- 只有 `all_files_complete && !failed` 才 `rename`;
- 中途失败/取消:临时目录被清理,用户看不到半截文件;
- `rename` 在同一文件系统内是原子的。

## 版本与兼容

- 帧头带 `PROTO_VER`,不匹配直接 `bad magic / incompatible version`;
- `Hello.proto_ver` 用于会话层校验;
- 演进是**破坏性**的:v2 的发送端会对一个大文件开多条数据连接,而 v1 接收端
  只接受一条——两者不兼容。项目选择"大家一起升级",用代码里的显式版本闸门兜底。

## 为什么不用 QUIC / WebRTC

- QUIC 在局域网会引入额外的握手与拥塞控制,且移动端集成成本高;
- WebRTC 依赖 STUN/信令,是"跨 NAT"的解法,而我们的前提就是同一局域网;
- 裸 TCP + 自描述帧让发送端与接收端都能保持极小的复杂度,调试也直观
  (可以用 `nc`/脚本手写几帧就能复现问题)。
