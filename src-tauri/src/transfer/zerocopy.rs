use std::fs::File;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// 是否使用零拷贝。环境变量 `SENDSENT_ZEROCOPY` 优先(便于压测),否则用配置里的值。
pub fn enabled(configured: bool) -> bool {
    if let Ok(v) = std::env::var("SENDSENT_ZEROCOPY") {
        return v == "1" || v.eq_ignore_ascii_case("true");
    }
    configured
}

/// 把 file 中 [offset, offset+len) 的字节推到 socket。
/// macOS/BSD:sendfile(2); Linux:sendfile(2); 其它:pread 到缓冲 + write 回退。
pub async fn send_payload(socket: &mut TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()> {
    if len == 0 { return Ok(()); }

    #[cfg(target_os = "macos")]
    { return sendfile_macos(socket, file, offset, len).await; }

    #[cfg(target_os = "linux")]
    { return sendfile_linux(socket, file, offset, len).await; }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    { return fallback_send_payload(socket, file, offset, len).await; }
}

#[cfg(target_os = "macos")]
async fn sendfile_macos(socket: &TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let out_fd = socket.as_raw_fd();
    let in_fd = file.as_raw_fd();
    let mut off = offset as i64;
    let mut remaining = len;
    while remaining > 0 {
        let mut to_send: libc::off_t = remaining as libc::off_t;
        let rc = unsafe {
            libc::sendfile(in_fd, out_fd, off, &mut to_send, std::ptr::null_mut(), 0)
        };
        // macOS 的 len 是 value-result:返回时它 = "已发送字节数",**出错(EAGAIN/EINTR)时也一样**。
        // 之前只在成功路径累加进度;EAGAIN(非阻塞 socket 缓冲写满,大文件/慢接收端必现)时
        // 直接 continue,于是从同一 offset 重发 → 线上出现重复字节且没有新帧头 → 对端帧解析
        // 错位 → 断连 → 发送端 EPIPE(broken-pipe)。这里无论成功失败都先记账。
        let progressed = to_send.clamp(0, remaining as libc::off_t) as usize;
        off += progressed as i64;
        remaining -= progressed;
        if rc < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::WouldBlock {
                socket.writable().await?;
                continue;
            }
            return Err(e);
        }
        if progressed == 0 {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "sendfile made no progress"));
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn sendfile_linux(socket: &mut TcpStream, file: &File, offset: u64, len: usize) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let out_fd = socket.as_raw_fd();
    let in_fd = file.as_raw_fd();
    let mut off: libc::off_t = offset as libc::off_t;
    let mut remaining = len;
    while remaining > 0 {
        let n = unsafe { libc::sendfile(out_fd, in_fd, &mut off, remaining) };
        if n < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::WouldBlock {
                socket.writable().await?;
                continue;
            }
            return Err(e);
        }
        if n == 0 { return Err(io::Error::new(io::ErrorKind::WriteZero, "sendfile made no progress")); }
        remaining -= n as usize;
    }
    Ok(())
}

/// 通用回退:pread + write (TLS 模式下 sendfile 不可用时走此路径)。
pub async fn fallback_send_payload<W: AsyncWriteExt + Unpin>(socket: &mut W, file: &File, offset: u64, len: usize) -> io::Result<()> {
    let mut buf = vec![0u8; 64 * 1024];
    let mut off = offset;
    let mut remaining = len;
    while remaining > 0 {
        let n = buf.len().min(remaining);
        let read = crate::misc::read_at(file, &mut buf[..n], off)?;
        if read == 0 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "file short")); }
        socket.write_all(&buf[..read]).await?;
        off += read as u64;
        remaining -= read;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn send_payload_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ss-zc-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.bin");
        let payload: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = conn.await.unwrap();

        send_payload(&mut client, &file, 0, payload.len()).await.unwrap();
        client.shutdown().await.unwrap();

        use tokio::io::AsyncReadExt;
        let mut got = Vec::new();
        server.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, payload);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn send_payload_offset_range() {
        let dir = std::env::temp_dir().join(format!("ss-zc2-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f2.bin");
        let payload: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = conn.await.unwrap();

        send_payload(&mut client, &file, 1000, 2000).await.unwrap();
        client.shutdown().await.unwrap();

        use tokio::io::AsyncReadExt;
        let mut got = Vec::new();
        server.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, &payload[1000..3000]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sendfile_eagain_no_duplication() {
        use socket2::SockRef;
        let dir = std::env::temp_dir().join(format!("ss-zc-eagain-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.bin");
        let payload: Vec<u8> = (0..512 * 1024u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).await.unwrap();
        // 故意压小发送缓冲,逼出 sendfile 的 EAGAIN(部分发送),这是老代码会重复发送的场景。
        let _ = SockRef::from(&client).set_send_buffer_size(8 * 1024);
        let (mut server, _) = listener.accept().await.unwrap();

        let n = payload.len();
        let sender = tokio::spawn(async move {
            let mut c = client;
            send_payload(&mut c, &file, 0, n).await.unwrap();
            let _ = c.shutdown().await;
        });

        // 读端稍后开始读(独立线程),让发送端先撞上 EAGAIN;整体加超时防挂。
        let receive = async {
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            use tokio::io::AsyncReadExt;
            let mut got = Vec::new();
            server.read_to_end(&mut got).await.unwrap();
            got
        };
        let got = tokio::time::timeout(std::time::Duration::from_secs(20), receive)
            .await
            .expect("sendfile stalled");
        sender.await.unwrap();

        assert_eq!(got.len(), payload.len(), "length mismatch (duplication/omission)");
        assert_eq!(got, payload, "content mismatch");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn fallback_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ss-zc3-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f3.bin");
        let payload: Vec<u8> = (0..5000u32).map(|i| (i % 253) as u8).collect();
        std::fs::write(&path, &payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = conn.await.unwrap();

        fallback_send_payload(&mut client, &file, 0, payload.len()).await.unwrap();
        client.shutdown().await.unwrap();

        use tokio::io::AsyncReadExt;
        let mut got = Vec::new();
        server.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, payload);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
