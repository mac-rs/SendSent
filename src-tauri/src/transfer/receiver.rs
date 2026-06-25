use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, read_data, write_control};
use crate::proto::messages::*;
use crate::store::Identity;
use crate::transfer::atomic::AtomicWriter;
use crate::transfer::meter::{SpeedMeter, Throttle};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
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

pub async fn run_receiver(
    mut control: TcpStream,
    hello: Hello,
    events: mpsc::UnboundedSender<TransferEvent>,
    decision_rx: oneshot::Receiver<Decision>,
    data_rx: oneshot::Receiver<TcpStream>,
    our: Identity,
) -> Result<()> {
    let session_id = hello.session_id;

    let ack = HelloAck { device_id: our.device_id.clone(), name: our.name.clone() };
    write_control(&mut control, MsgType::HelloAck, &bincode::serialize(&ack)?).await?;

    let (ty, buf) = read_control(&mut control).await?;
    if ty != MsgType::Manifest {
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::ProtocolError, message: "expected manifest".into() }) });
        return Ok(());
    }
    let manifest: Manifest = match bincode::deserialize(&buf) {
        Ok(m) => m,
        Err(e) => {
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

    let data = match data_rx.await {
        Ok(s) => s,
        Err(_) => {
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::ConnectionLost, message: "no data connection".into() }) });
            return Ok(());
        }
    };

    match drain_data(data, &manifest, &decision.save_dir, session_id, events.clone()).await {
        Ok(()) => {
            let _ = write_control(&mut control, MsgType::Complete, &bincode::serialize(&Complete)?).await;
        }
        Err(e) => {
            let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
                error: Some(ErrorPayload { code: ErrorCode::WriteFailed, message: e.to_string() }) });
        }
    }
    Ok(())
}

pub async fn drain_data(
    mut data: TcpStream,
    manifest: &Manifest,
    save_dir: &Path,
    session_id: Uuid,
    events: mpsc::UnboundedSender<TransferEvent>,
) -> Result<()> {
    let writer = AtomicWriter::new(save_dir, &session_id.to_string())?;
    let mut parts: std::collections::HashMap<Uuid, (FileMeta, PathBuf, u64)> = std::collections::HashMap::new();
    for f in &manifest.files {
        if f.kind == FileKind::File {
            let pp = writer.part_path(&f.rel_path);
            if let Some(p) = pp.parent() { std::fs::create_dir_all(p)?; }
            let fl = std::fs::File::create(&pp)?;
            fl.set_len(f.size)?; // sparse pre-allocate, no memory blowup
            parts.insert(f.id, (f.clone(), pp, 0));
        } else {
            let _ = std::fs::create_dir_all(save_dir.join(&f.rel_path));
        }
    }
    let total = manifest.total_size;
    let files_total = manifest.files.iter().filter(|f| f.kind == FileKind::File).count() as u64;
    let mut total_done: u64 = 0;
    let mut meter = SpeedMeter::new(std::time::Duration::from_secs(2));
    let mut throttle = Throttle::new(8);

    loop {
        let chunk = match read_data(&mut data).await { Ok(c) => c, Err(_) => break };
        if let Some((meta, pp, recvd)) = parts.get_mut(&chunk.file_id) {
            use std::io::{Seek, SeekFrom, Write};
            let mut fl = std::fs::OpenOptions::new().write(true).open(pp)?;
            fl.seek(SeekFrom::Start(chunk.offset))?;
            fl.write_all(&chunk.data)?;
            *recvd += chunk.data.len() as u64;
            let _ = meta;
            total_done = total_done.saturating_add(chunk.data.len() as u64);
        }
        let now = std::time::Instant::now();
        meter.record(now, total_done);
        if throttle.allow(now, false) {
            let files_done = parts.values().filter(|(m, _, r)| *r >= m.size).count() as u64;
            let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
                bytes_done: total_done, bytes_total: total, files_done, files_total, speed_bps: meter.bps() });
        }
    }

    let mut all_ok = true;
    for f in &manifest.files {
        if f.kind == FileKind::File
            && writer.finalize(&f.rel_path).is_err() { all_ok = false; }
    }
    writer.cleanup();
    let _ = events.send(TransferEvent::Finished {
        session_id,
        state: if all_ok { FinishedState::Completed } else { FinishedState::Failed },
        error: if all_ok { None } else { Some(ErrorPayload { code: ErrorCode::WriteFailed, message: "finalize failed".into() }) },
    });
    Ok(())
}
