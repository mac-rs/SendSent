use socket2::SockRef;
use tokio::net::TcpStream;

const BUF: usize = 8 * 1024 * 1024; // 8 MiB

/// 调大收发缓冲 + 关闭 Nagle。失败仅记录,不致命。
pub fn tune_socket(stream: &TcpStream) {
    let sref = SockRef::from(stream);
    if let Err(e) = sref.set_recv_buffer_size(BUF) {
        tracing::warn!("set_recv_buffer_size: {e}");
    }
    if let Err(e) = sref.set_send_buffer_size(BUF) {
        tracing::warn!("set_send_buffer_size: {e}");
    }
    if let Err(e) = stream.set_nodelay(true) {
        tracing::warn!("set_nodelay: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn tune_socket_does_not_panic() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
        let (s, _) = listener.accept().await.unwrap();
        let c = conn.await.unwrap();
        tune_socket(&s);
        tune_socket(&c);
        assert!(s.nodelay().unwrap());
    }
}
