use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, read_data, write_control};
use crate::proto::messages::*;
use crate::store::Identity;
use crate::transfer::atomic::AtomicWriter;
use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Notify};
use uuid::Uuid;

#[derive(Debug)]
pub struct Decision { pub accept: bool, pub save_dir: PathBuf }

fn peer_platform(p: Platform) -> crate::discovery::Platform {
    match p {
        Platform::Macos => crate::discovery::Platform::Macos,
        Platform::Windows => crate::discovery::Platform::Windows,
        Platform::Linux => crate::discovery::Platform::Linux,
        Platform::Ios => crate::discovery::Platform::Ios,
        Platform::Android => crate::discovery::Platform::Android,
    }
}

/// 位置写:无 seek 状态,多任务并发写同一文件不同 offset 安全。
fn write_at(file: &File, buf: &[u8], offset: u64) -> std::io::Result<()> {
    #[cfg(unix)]
    { use std::os::unix::fs::FileExt; file.write_at(buf, offset)?; }
    #[cfg(windows)]
    { use std::os::windows::fs::FileExt; file.seek_write(buf, offset)?; }
    Ok(())
}

struct DrainState {
    parts: HashMap<Uuid, (FileMeta, Arc<File>, AtomicU64)>, // (meta, handle, received)
    total_done: AtomicU64,
    total: u64,
    notify: Notify,
    writer: Arc<AtomicWriter>,
    failed: AtomicBool,
}

pub async fn run_receiver(
    mut control: TcpStream,
    hello: Hello,
    events: mpsc::UnboundedSender<TransferEvent>,
    decision_rx: oneshot::Receiver<Decision>,
    mut data_rx: mpsc::Receiver<TcpStream>,
    our: Identity,
) -> Result<()> {
    let session_id = hello.session_id;

    let ack = HelloAck { device_id: our.device_id.clone(), name: our.name.clone(), secure_ok: hello.secure };
    write_control(&mut control, MsgType::HelloAck, &bincode::serialize(&ack)?).await?;

    let (ty, buf) = read_control(&mut control).await?;
    if ty != MsgType::Manifest {
        let _ = write_control(&mut control, MsgType::Error,
            &bincode::serialize(&ErrorMsg { code: ErrorCode::ProtocolError, message: "expected manifest".into() })?).await;
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: "expected manifest".into() }) });
        return Ok(());
    }
    let manifest: Manifest = match bincode::deserialize(&buf) {
        Ok(m) => m,
        Err(e) => {
            let _ = write_control(&mut control, MsgType::Error,
                &bincode::serialize(&ErrorMsg { code: ErrorCode::ProtocolError, message: e.to_string() })?).await;
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: e.to_string() }) });
            return Ok(());
        }
    };

    let peer = crate::discovery::Peer {
        device_id: hello.device_id.clone(), name: hello.name.clone(),
        platform: peer_platform(hello.platform),
        proto_version: hello.proto_ver as u16, addrs: vec![], port: 0, last_seen_ms: 0,
    };
    let _ = events.send(TransferEvent::Request { session_id, sender: peer, manifest: manifest.clone() });

    let decision = match decision_rx.await { Ok(d) => d, Err(_) => return Ok(()) };
    if !decision.accept {
        let _ = write_control(&mut control, MsgType::Reject,
            &bincode::serialize(&Reject { reason: "declined".into() })?).await;
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
        return Ok(());
    }
    let _ = write_control(&mut control, MsgType::Accept,
        &bincode::serialize(&Accept { save_dir: decision.save_dir.to_string_lossy().into() })?).await;

    tracing::info!("receiver: awaiting data connections");

    let state = match build_drain_state(&manifest, &decision.save_dir, session_id) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            let _ = write_control(&mut control, MsgType::Error,
                &bincode::serialize(&ErrorMsg { code: ErrorCode::WriteFailed, message: e.to_string() })?).await;
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::WriteFailed, message: e.to_string() }) });
            return Ok(());
        }
    };

    // 进度上报任务(独立,按 total_done 周期发)
    let prog_state = state.clone();
    let prog_events = events.clone();
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

    // accept 循环:收到一条流就 drain;total_done 到了(经 notify)或 data_rx 关闭就退出
    let acc_state = state.clone();
    let mut drain_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    loop {
        tokio::select! {
            biased;
            _ = acc_state.notify.notified() => {
                if acc_state.total_done.load(Ordering::Relaxed) >= acc_state.total { break; }
                // 否则虚假唤醒,继续循环
            }
            stream = data_rx.recv() => {
                match stream {
                    Some(s) => {
                        let st = acc_state.clone();
                        drain_handles.push(tokio::spawn(async move { let _ = drain_stream(s, &st).await; }));
                    }
                    None => break,
                }
            }
        }
    }
    for h in drain_handles { let _ = h.await; }
    prog.abort();

    let done = state.total_done.load(Ordering::Relaxed);
    let complete = done == state.total
        && state.parts.values().all(|(m, _, r)| r.load(Ordering::Relaxed) == m.size);
    tracing::info!("drain ended: done={done} total={} complete={complete}", state.total);

    let outcome: bool = if state.failed.load(Ordering::Relaxed) { false } else { complete };
    if outcome {
        let mut all_ok = true;
        for f in &manifest.files {
            if f.kind == FileKind::File && state.writer.finalize(&f.rel_path).is_err() { all_ok = false; }
        }
        state.writer.cleanup();
        let _ = write_control(&mut control, MsgType::Complete, &bincode::serialize(&Complete)?).await;
        let _ = events.send(TransferEvent::Finished { session_id,
            state: if all_ok { FinishedState::Completed } else { FinishedState::Failed },
            error: if all_ok { None } else { Some(ErrorPayload { code: ErrorCode::WriteFailed, message: "finalize failed".into() }) } });
    } else {
        state.writer.cleanup();
        let _ = write_control(&mut control, MsgType::Error,
            &bincode::serialize(&ErrorMsg { code: ErrorCode::ConnectionLost, message: "incomplete".into() })?).await;
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::ConnectionLost, message: "transfer incomplete".into() }) });
    }
    Ok(())
}

fn build_drain_state(manifest: &Manifest, save_dir: &Path, session_id: Uuid) -> Result<DrainState> {
    let writer = Arc::new(AtomicWriter::new(save_dir, &session_id.to_string())?);
    let mut parts: HashMap<Uuid, (FileMeta, Arc<File>, AtomicU64)> = HashMap::new();
    for f in &manifest.files {
        if f.kind == FileKind::File {
            let pp = writer.part_path(&f.rel_path);
            if let Some(p) = pp.parent() { std::fs::create_dir_all(p)?; }
            let fl = std::fs::File::create(&pp)?;
            fl.set_len(f.size)?;
            parts.insert(f.id, (f.clone(), Arc::new(fl), AtomicU64::new(0)));
        } else {
            let _ = std::fs::create_dir_all(save_dir.join(&f.rel_path));
        }
    }
    Ok(DrainState {
        parts,
        total_done: AtomicU64::new(0),
        total: manifest.total_size,
        notify: Notify::new(),
        writer,
        failed: AtomicBool::new(false),
    })
}

async fn drain_stream(mut data: TcpStream, state: &DrainState) {
    loop {
        let chunk = match read_data(&mut data).await { Ok(c) => c, Err(_) => break };
        if let Some((_meta, file, recvd)) = state.parts.get(&chunk.file_id) {
            if let Err(e) = write_at(file, &chunk.data, chunk.offset) {
                tracing::error!("write_at failed: {e}");
                state.failed.store(true, Ordering::Relaxed);
                state.notify.notify_waiters();
                break;
            }
            recvd.fetch_add(chunk.data.len() as u64, Ordering::Relaxed);
            let prev = state.total_done.fetch_add(chunk.data.len() as u64, Ordering::Relaxed);
            if prev + chunk.data.len() as u64 >= state.total {
                state.notify.notify_one();
            }
        }
    }
}
