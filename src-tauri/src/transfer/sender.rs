use sha2::Digest;
use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, write_control, write_data};
use crate::proto::messages::*;
use crate::store::{Identity, TransferConfig};
use crate::transfer::sock::tune_socket;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::client::TlsStream;
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub async fn run_sender(
    session_id: Uuid, peer_addrs: Vec<SocketAddr>, files: Vec<String>,
    our: Identity, peer_name: String, peer_platform: crate::discovery::Platform,
    events: mpsc::UnboundedSender<TransferEvent>,
    config: TransferConfig, secure: bool, verify: bool,
) -> Result<()> {
    let result = run_sender_inner(session_id, peer_addrs, files, our.clone(), peer_name, peer_platform, events.clone(), config, secure, verify).await;
    if let Err(e) = &result {
        tracing::error!("sender failed: {e}");
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::Internal, message: e.to_string() }) });
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn run_sender_inner(
    session_id: Uuid, peer_addrs: Vec<SocketAddr>, files: Vec<String>,
    our: Identity, peer_name: String, peer_platform: crate::discovery::Platform,
    events: mpsc::UnboundedSender<TransferEvent>,
    config: TransferConfig, secure: bool, verify: bool,
) -> Result<()> {
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Connecting,
        bytes_done: 0, bytes_total: 0, files_done: 0, files_total: 0, speed_bps: 0 });

    let mut control = Some(match connect_any(&peer_addrs).await {
        Ok(s) => { tracing::info!("sender connected (tried {peer_addrs:?})"); s }
        Err(e) => { tracing::warn!("sender connect failed for {peer_addrs:?}: {e}"); return Err(e); }
    });
    let c = control.as_mut().unwrap();
    let hello = Hello { device_id: our.device_id.clone(), name: our.name.clone(),
        platform: Platform::from_label(&our.platform), session_id, proto_ver: PROTO_VER, secure, verify };
    write_control(c, MsgType::Hello, &postcard::to_stdvec(&hello)?).await?;
    let (ty, buf) = read_control(c).await?;
    if ty != MsgType::HelloAck { return Err(anyhow!("expected helloack, got {ty:?}")); }
    let ack: HelloAck = postcard::from_bytes(&buf)?;

    let (manifest, file_map) = build_manifest(session_id, &files)?;
    let files_total = manifest.files.iter().filter(|f| f.kind == FileKind::File).count() as u64;
    let started_at_ms = crate::history::now_ms();
    let hist_files: Vec<crate::history::HistoryFile> = manifest.files.iter()
        .filter(|f| f.kind == FileKind::File)
        .map(|f| crate::history::HistoryFile { name: f.name.clone(), size: f.size, rel_path: f.name.clone() })
        .collect();
    let is_secure = secure && ack.secure_ok;
    let mut control_tls: Option<TlsStream<TcpStream>> = None;

    if is_secure {
        let plain = control.take().unwrap();
        let client_cfg = crate::transfer::tls::make_client_config();
        let mut stream = tokio_rustls::TlsConnector::from(client_cfg)
            .connect("sendsent".try_into().unwrap(), plain).await
            .map_err(|e| anyhow!("TLS connect: {e}"))?;
        let pin = format!("{:06}", rand::random::<u32>() % 1_000_000);
        write_control(&mut stream, MsgType::PinCode, &postcard::to_stdvec(&PinCode { pin: pin.clone() })?).await?;
        let (ty_pin, buf_pin) = read_control(&mut stream).await?;
        if ty_pin != MsgType::PinCode { return Err(anyhow!("expected PinCode response, got {ty_pin:?}")); }
        let resp: PinCode = postcard::from_bytes(&buf_pin)?;
        if resp.pin != pin { return Err(anyhow!("PIN mismatch")); }
        write_control(&mut stream, MsgType::Manifest, &postcard::to_stdvec(&manifest)?).await?;
        let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::AwaitingAccept,
            bytes_done: 0, bytes_total: manifest.total_size, files_done: 0, files_total, speed_bps: 0 });
        let (ty2, _) = read_control(&mut stream).await?;
        if ty2 == MsgType::Reject {
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
            let _ = events.send(TransferEvent::Recorded(crate::history::make_record(
                crate::history::Direction::Send, &session_id.to_string(), &peer_name, peer_platform,
                hist_files.clone(), manifest.total_size, 0, crate::history::HistoryStatus::Rejected,
                started_at_ms, None, None,
            )));
            return Ok(());
        }
        if ty2 != MsgType::Accept { return Err(anyhow!("expected accept, got {ty2:?}")); }
        control_tls = Some(stream);
    } else {
        let c = control.as_mut().unwrap();
        write_control(c, MsgType::Manifest, &postcard::to_stdvec(&manifest)?).await?;
        let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::AwaitingAccept,
            bytes_done: 0, bytes_total: manifest.total_size, files_done: 0, files_total, speed_bps: 0 });
        let (ty2, _) = read_control(c).await?;
        if ty2 == MsgType::Reject {
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
            let _ = events.send(TransferEvent::Recorded(crate::history::make_record(
                crate::history::Direction::Send, &session_id.to_string(), &peer_name, peer_platform,
                hist_files.clone(), manifest.total_size, 0, crate::history::HistoryStatus::Rejected,
                started_at_ms, None, None,
            )));
            return Ok(());
        }
        if ty2 != MsgType::Accept { return Err(anyhow!("expected accept, got {ty2:?}")); }
    }

    // 数据连接
    let buckets = plan_buckets(&manifest, &file_map, config.conns as usize, config.split_threshold);
    let chunk = config.chunk_size as usize;
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
        bytes_done: 0, bytes_total: manifest.total_size, files_done: 0, files_total, speed_bps: 0 });
    let total_done = Arc::new(AtomicU64::new(0));

    // 发送端网速统计(周期性 emit Progress 事件,保持前端速度显示)
    let sm_done = total_done.clone();
    let sm_events = events.clone();
    let sm_sid = session_id;
    let sm_total = manifest.total_size;
    let sm_ftotal = files_total;
    let sm_task = tokio::spawn(async move {
        let mut meter = crate::transfer::meter::SpeedMeter::new(std::time::Duration::from_secs(1));
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let done = sm_done.load(Ordering::Relaxed);
            meter.record(std::time::Instant::now(), done);
            let _ = sm_events.send(TransferEvent::Progress {
                session_id: sm_sid, state: SessionState::Transferring,
                bytes_done: done, bytes_total: sm_total,
                files_done: 0, files_total: sm_ftotal, speed_bps: meter.bps(),
            });
            if done >= sm_total { break; }
        }
    });

    let mut handles = Vec::new();
    for bucket in buckets.into_iter().filter(|b| !b.is_empty()) {
        let addrs = peer_addrs.clone();
        let done = total_done.clone();
        let sid = session_id;
        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            // 不自动重试:重试会重发整个 bucket,receiver 会重复累加进度导致完成判定错乱。
            // 失败即止——局域网稳定,单连接失败概率低;失败由用户决定是否重发整个文件。
            if let Err(e) = send_bucket(&addrs, &bucket, chunk, &done, sid, is_secure).await {
                tracing::error!("data bucket failed: {e}");
            }
        });
        handles.push(handle);
    }
    for h in handles { let _ = h.await; }
    sm_task.abort();

    if verify {
        let mut hashes: Vec<(Uuid, String)> = Vec::new();
        for (id, path) in &file_map {
            let mut file = crate::misc::open_source(path)?;
            let mut hasher = sha2::Sha256::new();
            let mut buf = [0u8; 64 * 1024];
            loop {
                use std::io::Read;
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            let hash = hex::encode(hasher.finalize());
            hashes.push((*id, hash));
        }
        let payload = postcard::to_stdvec(&VerifyInfo { hashes })?;
        if let Some(ref mut s) = control_tls {
            write_control(s, MsgType::VerifyInfo, &payload).await?;
        } else {
            write_control(control.as_mut().unwrap(), MsgType::VerifyInfo, &payload).await?;
        }
    }

    let done = total_done.load(Ordering::Relaxed);
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Finalizing,
        bytes_done: done, bytes_total: manifest.total_size, files_done: files_total, files_total, speed_bps: 0 });

    let final_state = if let Some(ref mut s) = control_tls {
        let (ty, _) = read_control(s).await?;
        if ty == MsgType::Complete { FinishedState::Completed } else { FinishedState::Failed }
    } else {
        let (ty, _) = read_control(control.as_mut().unwrap()).await?;
        if ty == MsgType::Complete { FinishedState::Completed } else { FinishedState::Failed }
    };
    let _ = events.send(TransferEvent::Recorded(crate::history::make_record(
        crate::history::Direction::Send, &session_id.to_string(), &peer_name, peer_platform,
        hist_files.clone(), manifest.total_size, done,
        if final_state == FinishedState::Completed { crate::history::HistoryStatus::Completed } else { crate::history::HistoryStatus::Failed },
        started_at_ms, None,
        if final_state == FinishedState::Failed { Some("no complete".into()) } else { None },
    )));
    let _ = events.send(TransferEvent::Finished { session_id, state: final_state,
        error: if final_state == FinishedState::Failed { Some(ErrorPayload { code: ErrorCode::Internal, message: "no complete".into() }) } else { None } });
    Ok(())
}

struct Segment { file_id: Uuid, path: PathBuf, offset: u64, len: u64 }

fn plan_buckets(manifest: &Manifest, file_map: &HashMap<Uuid, PathBuf>, conns: usize, split: u64) -> Vec<Vec<Segment>> {
    let conns = conns.max(1);
    let mut segments: Vec<Segment> = Vec::new();
    for f in &manifest.files {
        if f.kind != FileKind::File { continue; }
        let path = match file_map.get(&f.id) { Some(p) => p.clone(), None => continue };
        if f.size >= split && conns > 1 {
            let each = f.size / conns as u64;
            let mut off = 0u64;
            for i in 0..conns {
                let end = if i == conns - 1 { f.size } else { off + each };
                if end > off { segments.push(Segment { file_id: f.id, path: path.clone(), offset: off, len: end - off }); }
                off = end;
            }
        } else if f.size > 0 { segments.push(Segment { file_id: f.id, path, offset: 0, len: f.size }); }
    }
    let mut buckets: Vec<Vec<Segment>> = (0..conns).map(|_| Vec::new()).collect();
    for (i, seg) in segments.into_iter().enumerate() { buckets[i % conns].push(seg); }
    buckets
}

async fn send_segment(data: &mut TcpStream, seg: &Segment, chunk: usize, done: &AtomicU64) -> Result<()> {
    use std::os::unix::fs::FileExt;
    let file = crate::misc::open_source(&seg.path)?;
    let mut off = seg.offset;
    let end = seg.offset + seg.len;

    // 可选零拷贝路径(sendfile);默认关闭,用 SENDSENT_ZEROCOPY=1 启用并验证。
    if crate::transfer::zerocopy::enabled() {
        use crate::proto::frame::write_data_header;
        while off < end {
            let n = chunk.min((end - off) as usize) as u32;
            write_data_header(data, seg.file_id, off, n).await?;
            data.flush().await?; // 确保 29 字节帧头先于 sendfile 到达对端
            crate::transfer::zerocopy::send_payload(data, &file, off, n as usize).await?;
            off += n as u64;
            done.fetch_add(n as u64, Ordering::Relaxed);
        }
        return Ok(());
    }

    let mut buf = vec![0u8; chunk];
    while off < end {
        let n = chunk.min((end - off) as usize);
        let read = file.read_at(&mut buf[..n], off)?;
        if read == 0 { return Err(anyhow!("file short at {}", off)); }
        write_data(data, seg.file_id, off, &buf[..read]).await?;
        off += read as u64; done.fetch_add(read as u64, Ordering::Relaxed);
    }
    Ok(())
}

async fn send_segment_secure(data: &mut TlsStream<TcpStream>, seg: &Segment, chunk: usize, done: &AtomicU64) -> Result<()> {
    use std::os::unix::fs::FileExt;
    let file = crate::misc::open_source(&seg.path)?;
    let mut buf = vec![0u8; chunk];
    let mut off = seg.offset; let end = seg.offset + seg.len;
    while off < end {
        let n = chunk.min((end - off) as usize);
        let read = file.read_at(&mut buf[..n], off)?;
        if read == 0 { return Err(anyhow!("file short at {}", off)); }
        write_data(data, seg.file_id, off, &buf[..read]).await?;
        off += read as u64; done.fetch_add(read as u64, Ordering::Relaxed);
    }
    Ok(())
}

async fn connect_any(addrs: &[SocketAddr]) -> Result<TcpStream> {
    let mut last = None;
    for a in addrs { match TcpStream::connect(a).await { Ok(s) => { tune_socket(&s); return Ok(s); } Err(e) => last = Some(e) } }
    Err(anyhow!("connect failed: {:?}", last))
}

async fn send_bucket(addrs: &[SocketAddr], bucket: &[Segment], chunk: usize, done: &AtomicU64, sid: Uuid, secure: bool) -> Result<()> {
    let mut data = match connect_any(addrs).await {
        Ok(s) => { tracing::debug!("send_bucket {sid}: connected"); s }
        Err(e) => { tracing::warn!("send_bucket {sid}: connect failed: {e}"); return Err(e); }
    };
    if let Err(e) = write_control(&mut data, MsgType::DataOpen, &postcard::to_stdvec(&DataOpen { session_id: sid })?).await {
        tracing::warn!("send_bucket {sid}: DataOpen write failed: {e}");
        return Err(e.into());
    }
    if secure {
        let client_cfg = crate::transfer::tls::make_client_config();
        let mut tls = tokio_rustls::TlsConnector::from(client_cfg)
            .connect("sendsent".try_into().unwrap(), data).await
            .map_err(|e| anyhow!("data TLS: {e}"))?;
        for seg in bucket { send_segment_secure(&mut tls, seg, chunk, done).await?; }
        // 关键:TLS 1.3 服务端握手后会发 NewSessionTicket 等记录。发送端若从不读取
        // 对端数据就直接关闭 socket,内核会发 RST,接收端会丢弃已缓冲的应用数据
        // (表现为收到 0 字节)。这里先发 close_notify,再把对端记录读到 EOF,
        // 让连接优雅关闭(发 FIN 而非 RST)。
        let _ = tls.shutdown().await;
        let mut sink = [0u8; 4096];
        loop {
            match tls.read(&mut sink).await {
                Ok(0) | Err(_) => break,
                Ok(_) => continue,
            }
        }
    } else {
        for seg in bucket { send_segment(&mut data, seg, chunk, done).await?; }
        let _ = data.shutdown().await;
    }
    Ok(())
}

pub fn build_manifest(session_id: Uuid, files: &[String]) -> Result<(Manifest, HashMap<Uuid, PathBuf>)> {
    let mut metas = Vec::new(); let mut map: HashMap<Uuid, PathBuf> = HashMap::new(); let mut total: u64 = 0;
    for f in files { walk(f, "", &mut metas, &mut map, &mut total)?; }
    let count = metas.len() as u64;
    Ok((Manifest { session_id, files: metas, total_size: total, total_count: count }, map))
}

fn walk(abs: &str, rel_prefix: &str, metas: &mut Vec<FileMeta>, map: &mut HashMap<Uuid, PathBuf>, total: &mut u64) -> Result<()> {
    let p = Path::new(abs);
    let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let rel = if rel_prefix.is_empty() { name.clone() } else { format!("{rel_prefix}/{name}") };
    if p.is_dir() {
        metas.push(FileMeta { id: Uuid::new_v4(), name: name.clone(), rel_path: rel.clone(), size: 0, kind: FileKind::Dir, hash: None });
        for entry in std::fs::read_dir(p)? { let entry = entry?; walk(&entry.path().to_string_lossy(), &rel, metas, map, total)?; }
    } else {
        let size = crate::misc::source_len(p)?; let id = Uuid::new_v4();
        map.insert(id, p.to_path_buf());
        metas.push(FileMeta { id, name, rel_path: rel, size, kind: FileKind::File, hash: None });
        *total += size;
    }
    Ok(())
}
