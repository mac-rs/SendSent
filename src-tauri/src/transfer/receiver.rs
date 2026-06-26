use sha2::Digest;
use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, read_data, write_control};
use crate::proto::messages::*;
use crate::store::Identity;
use crate::transfer::atomic::AtomicWriter;
use crate::transfer::manager::DataStream;
use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot, Notify};
use tokio_rustls::server::TlsStream;
use uuid::Uuid;

#[derive(Debug)]
pub struct Decision { pub accept: bool, pub save_dir: PathBuf, pub pin: Option<String> }

fn peer_platform(p: Platform) -> crate::discovery::Platform {
    match p {
        Platform::Macos => crate::discovery::Platform::Macos, Platform::Windows => crate::discovery::Platform::Windows,
        Platform::Linux => crate::discovery::Platform::Linux, Platform::Ios => crate::discovery::Platform::Ios,
        Platform::Android => crate::discovery::Platform::Android,
    }
}

fn write_at(file: &File, buf: &[u8], offset: u64) -> std::io::Result<()> {
    #[cfg(unix)] { use std::os::unix::fs::FileExt; file.write_at(buf, offset)?; }
    #[cfg(windows)] { use std::os::windows::fs::FileExt; file.seek_write(buf, offset)?; }
    Ok(())
}

struct DrainState {
    parts: HashMap<Uuid, (FileMeta, Arc<File>, AtomicU64)>,
    total_done: AtomicU64, total: u64, notify: Notify,
    writer: Arc<AtomicWriter>, failed: AtomicBool,
    active_drains: AtomicUsize,
}

pub async fn run_receiver(
    control: tokio::net::TcpStream,
    hello: Hello,
    events: mpsc::UnboundedSender<TransferEvent>,
    decision_rx: oneshot::Receiver<Decision>,
    mut data_rx: mpsc::Receiver<DataStream>,
    our: Identity,
    tls_config: crate::transfer::tls::TlsConfig,
) -> Result<()> {
    let session_id = hello.session_id;
    let mut control_plain = Some(control);

    {
        let c = control_plain.as_mut().unwrap();
        let ack = HelloAck { device_id: our.device_id.clone(), name: our.name.clone(), secure_ok: hello.secure };
        write_control(c, MsgType::HelloAck, &postcard::to_stdvec(&ack)?).await?;
    }

    let is_secure = hello.secure;
    let mut control_tls: Option<TlsStream<tokio::net::TcpStream>> = None;

    if is_secure {
        let plain = control_plain.take().unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(tls_config);
        let s = acceptor.accept(plain).await.map_err(|e| anyhow::anyhow!("TLS accept: {e}"))?;
        control_tls = Some(s);
    }

    macro_rules! read_ctrl { () => { if let Some(ref mut s) = control_tls { read_control(s).await? } else { read_control(control_plain.as_mut().unwrap()).await? } } }
    macro_rules! write_ctrl { ($t:expr,$p:expr) => { if let Some(ref mut s) = control_tls { write_control(s, $t, $p).await? } else { write_control(control_plain.as_mut().unwrap(), $t, $p).await? } } }

    if is_secure {
        // PIN: read sender's pin, echo back (test-mode auto-accept; production adds user-input via Decision)
        let (pty, pbuf) = read_ctrl!();
        if pty != MsgType::PinCode {
            write_ctrl!(MsgType::Error, &postcard::to_stdvec(&ErrorMsg { code: ErrorCode::ProtocolError, message: "expected PinCode".into() })?);
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: "expected PinCode".into() }) });
            return Ok(());
        }
        let sender_pin: PinCode = postcard::from_bytes(&pbuf)?;
        // For now echo the same pin (production: wait user input via Decision)
        write_ctrl!(MsgType::PinCode, &postcard::to_stdvec(&sender_pin)?);
    }

    let (ty, buf) = read_ctrl!();
    if ty != MsgType::Manifest {
        write_ctrl!(MsgType::Error, &postcard::to_stdvec(&ErrorMsg { code: ErrorCode::ProtocolError, message: "expected manifest".into() })?);
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: "expected manifest".into() }) });
        return Ok(());
    }
    let manifest: Manifest = match postcard::from_bytes(&buf) {
        Ok(m) => m, Err(e) => {
            write_ctrl!(MsgType::Error, &postcard::to_stdvec(&ErrorMsg { code: ErrorCode::ProtocolError, message: e.to_string() })?);
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: e.to_string() }) });
            return Ok(());
        }
    };

    let peer = crate::discovery::Peer { device_id: hello.device_id.clone(), name: hello.name.clone(),
        platform: peer_platform(hello.platform), proto_version: hello.proto_ver as u16, addrs: vec![], port: 0, last_seen_ms: 0 };
    let _ = events.send(TransferEvent::Request { session_id, sender: peer, manifest: manifest.clone() });

    let decision = match decision_rx.await { Ok(d) => d, Err(_) => return Ok(()) };
    if !decision.accept {
        write_ctrl!(MsgType::Reject, &postcard::to_stdvec(&Reject { reason: "declined".into() })?);
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
        return Ok(());
    }
    write_ctrl!(MsgType::Accept, &postcard::to_stdvec(&Accept { save_dir: decision.save_dir.to_string_lossy().into() })?);

    tracing::info!("receiver: awaiting data connections");

    let state = match build_drain_state(&manifest, &decision.save_dir, session_id) {
        Ok(s) => Arc::new(s), Err(e) => {
            write_ctrl!(MsgType::Error, &postcard::to_stdvec(&ErrorMsg { code: ErrorCode::WriteFailed, message: e.to_string() })?);
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::WriteFailed, message: e.to_string() }) });
            return Ok(());
        }
    };

    let prog_state = state.clone(); let prog_events = events.clone();
    let prog = tokio::spawn(async move {
        let mut meter = crate::transfer::meter::SpeedMeter::new(Duration::from_secs(2));
        let files_total = prog_state.parts.len() as u64;
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let done = prog_state.total_done.load(Ordering::Relaxed);
            meter.record(std::time::Instant::now(), done);
            let files_done = prog_state.parts.values().filter(|(_, _, r)| r.load(Ordering::Relaxed) > 0).count() as u64;
            let _ = prog_events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
                bytes_done: done, bytes_total: prog_state.total, files_done, files_total, speed_bps: meter.bps() });
            if done >= prog_state.total { break; }
            if prog_state.failed.load(Ordering::Relaxed) { break; }
        }
    });

    let acc_state = state.clone(); let mut drain_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    loop {
        if all_files_complete(&acc_state) { break; }
        // 所有 drain 结束 + 短暂无新连接 = 传输结束(可能未完成,交给后续 complete 判定)
        if acc_state.active_drains.load(Ordering::Relaxed) == 0 {
            tokio::select! {
                biased;
                stream = data_rx.recv() => {
                    match stream {
                        Some(s) => { let st = acc_state.clone(); drain_handles.push(tokio::spawn(async move { drain_stream(s, &st).await; })); }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => { break; }
            }
        } else {
            tokio::select! {
                biased;
                _ = acc_state.notify.notified() => { continue; }
                stream = data_rx.recv() => {
                    match stream {
                        Some(s) => { let st = acc_state.clone(); drain_handles.push(tokio::spawn(async move { drain_stream(s, &st).await; })); }
                        None => break,
                    }
                }
            }
        }
    }
    for h in drain_handles { let _ = h.await; }
    prog.abort();

    let complete = !state.failed.load(Ordering::Relaxed) && all_files_complete(&state);
    let done = state.total_done.load(Ordering::Relaxed);
    tracing::info!("drain ended: done={done} total={} complete={complete} active={}", state.total, state.active_drains.load(Ordering::Relaxed));

    let outcome = if state.failed.load(Ordering::Relaxed) { false } else { complete };
    if outcome {
        let mut all_ok = true;
        for f in &manifest.files { if f.kind == FileKind::File && state.writer.finalize(&f.rel_path).is_err() { all_ok = false; } }
        if hello.verify && all_ok {
            let (vty, vbuf) = read_ctrl!();
            if vty == MsgType::VerifyInfo {
                if let Ok(info) = postcard::from_bytes::<VerifyInfo>(&vbuf) {
                    for (fid, expected) in &info.hashes {
                        let Some(fmeta) = manifest.files.iter().find(|f| f.id == *fid) else { continue; };
                        let fpath = decision.save_dir.join(&fmeta.rel_path);
                        if let Ok(mut file) = std::fs::File::open(&fpath) {
                            let mut hasher = sha2::Sha256::new();
                            let mut buf = [0u8; 64 * 1024];
                            loop {
                                use std::io::Read;
                                let n = file.read(&mut buf).unwrap_or(0);
                                if n == 0 { break; }
                                hasher.update(&buf[..n]);
                            }
                            let actual = hex::encode(hasher.finalize());
                            if actual != *expected { all_ok = false; tracing::warn!("hash mismatch for {}", fmeta.name); }
                        } else { all_ok = false; }
                    }
                } else { all_ok = false; }
            } else { all_ok = false; }
        }
        state.writer.cleanup();
        write_ctrl!(MsgType::Complete, &postcard::to_stdvec(&Complete)?);
        let _ = events.send(TransferEvent::Finished { session_id, state: if all_ok { FinishedState::Completed } else { FinishedState::Failed },
            error: if all_ok { None } else { Some(ErrorPayload { code: ErrorCode::WriteFailed, message: "finalize failed".into() }) } });
    } else {
        write_ctrl!(MsgType::Error, &postcard::to_stdvec(&ErrorMsg { code: ErrorCode::ConnectionLost, message: "incomplete".into() })?);
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::ConnectionLost, message: "transfer incomplete".into() }) });
    }
    Ok(())
}

fn build_drain_state(manifest: &Manifest, save_dir: &std::path::Path, session_id: Uuid) -> Result<DrainState> {
    let writer = Arc::new(AtomicWriter::new(save_dir, &session_id.to_string())?);
    let mut parts = HashMap::new();
    for f in &manifest.files {
        if f.kind == FileKind::File {
            let pp = writer.part_path(&f.rel_path); if let Some(p) = pp.parent() { std::fs::create_dir_all(p)?; }
            let fl = std::fs::File::create(&pp)?; fl.set_len(f.size)?;
            parts.insert(f.id, (f.clone(), Arc::new(fl), AtomicU64::new(0)));
        } else { let _ = std::fs::create_dir_all(save_dir.join(&f.rel_path)); }
    }
    Ok(DrainState { parts, total_done: AtomicU64::new(0), total: manifest.total_size, notify: Notify::new(), writer, failed: AtomicBool::new(false), active_drains: AtomicUsize::new(0) })
}

async fn drain_stream(data: DataStream, state: &DrainState) {
    state.active_drains.fetch_add(1, Ordering::Relaxed);
    match data {
        DataStream::Plain(s) => drain_inner(s, state).await,
        DataStream::Tls(s) => drain_inner(s, state).await,
    }
    state.active_drains.fetch_sub(1, Ordering::Relaxed);
    state.notify.notify_one(); // drain 结束,通知 select 重新评估完成状态
}

async fn drain_inner<R: AsyncReadExt + Unpin>(mut reader: R, state: &DrainState) {
    loop {
        let chunk = match read_data(&mut reader).await { Ok(c) => c, Err(_) => break };
        if let Some((_meta, file, recvd)) = state.parts.get(&chunk.file_id) {
            if let Err(e) = write_at(file, &chunk.data, chunk.offset) {
                tracing::error!("write_at failed: {e}"); state.failed.store(true, Ordering::Relaxed); state.notify.notify_waiters(); break;
            }
            recvd.fetch_add(chunk.data.len() as u64, Ordering::Relaxed);
            state.total_done.fetch_add(chunk.data.len() as u64, Ordering::Relaxed);
            // 完成判定:所有文件的 per-file recvd 达到 size(精确,不依赖全局 total_done)
            if all_files_complete(state) { state.notify.notify_one(); }
        }
    }
}

fn all_files_complete(state: &DrainState) -> bool {
    state.parts.values().all(|(m, _, r)| r.load(Ordering::Relaxed) >= m.size)
}
