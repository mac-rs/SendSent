use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, write_control, write_data};
use crate::proto::messages::*;
use crate::store::Identity;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use uuid::Uuid;

pub async fn run_sender(
    session_id: Uuid,
    peer_addrs: Vec<SocketAddr>,
    files: Vec<String>,
    our: Identity,
    events: mpsc::UnboundedSender<TransferEvent>,
) -> Result<()> {
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Connecting,
        bytes_done: 0, bytes_total: 0, files_done: 0, files_total: 0, speed_bps: 0 });

    let mut control = connect_any(&peer_addrs).await?;
    let hello = Hello { device_id: our.device_id.clone(), name: our.name.clone(),
        platform: Platform::Macos, session_id, proto_ver: PROTO_VER };
    write_control(&mut control, MsgType::Hello, &bincode::serialize(&hello)?).await?;
    let (ty, _) = read_control(&mut control).await?;
    if ty != MsgType::HelloAck { return Err(anyhow!("expected helloack, got {ty:?}")); }

    let (manifest, file_map) = build_manifest(session_id, &files)?;
    write_control(&mut control, MsgType::Manifest, &bincode::serialize(&manifest)?).await?;
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::AwaitingAccept,
        bytes_done: 0, bytes_total: manifest.total_size, files_done: 0,
        files_total: manifest.total_count, speed_bps: 0 });

    let (ty, _buf) = read_control(&mut control).await?;
    if ty == MsgType::Reject {
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
        return Ok(());
    }
    if ty != MsgType::Accept { return Err(anyhow!("expected accept, got {ty:?}")); }

    let mut data = connect_any(&peer_addrs).await?;
    write_control(&mut data, MsgType::DataOpen, &bincode::serialize(&DataOpen { session_id })?).await?;
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
        bytes_done: 0, bytes_total: manifest.total_size, files_done: 0,
        files_total: manifest.total_count, speed_bps: 0 });

    let mut done: u64 = 0;
    let mut files_done: u64 = 0;
    for f in &manifest.files {
        if f.kind == FileKind::File {
            let path = file_map.get(&f.id).cloned().ok_or_else(|| anyhow!("missing path for file id"))?;
            send_file(&mut data, f.id, &path, &mut done).await?;
            files_done += 1;
            let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
                bytes_done: done, bytes_total: manifest.total_size, files_done,
                files_total: manifest.total_count, speed_bps: 0 });
        }
    }
    let _ = data.shutdown().await;

    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Finalizing,
        bytes_done: done, bytes_total: manifest.total_size, files_done,
        files_total: manifest.total_count, speed_bps: 0 });
    let (ty, _) = read_control(&mut control).await?;
    let final_state = if ty == MsgType::Complete { FinishedState::Completed } else { FinishedState::Failed };
    let _ = events.send(TransferEvent::Finished { session_id, state: final_state,
        error: if final_state == FinishedState::Failed { Some(ErrorPayload { code: ErrorCode::Internal, message: "no complete".into() }) } else { None } });
    Ok(())
}

async fn connect_any(addrs: &[SocketAddr]) -> Result<TcpStream> {
    let mut last = None;
    for a in addrs {
        match TcpStream::connect(a).await { Ok(s) => return Ok(s), Err(e) => last = Some(e) }
    }
    Err(anyhow!("connect failed: {:?}", last))
}

async fn send_file(data: &mut TcpStream, id: Uuid, path: &Path, done: &mut u64) -> Result<()> {
    let mut f = tokio::fs::File::open(path).await?;
    let mut offset: u64 = 0;
    let mut buf = vec![0u8; DEFAULT_CHUNK_SIZE];
    loop {
        let n = tokio::io::AsyncReadExt::read(&mut f, &mut buf).await?;
        if n == 0 { break; }
        write_data(data, id, offset, &buf[..n]).await?;
        offset += n as u64;
        *done += n as u64;
    }
    Ok(())
}

pub fn build_manifest(session_id: Uuid, files: &[String]) -> Result<(Manifest, HashMap<Uuid, PathBuf>)> {
    let mut metas = Vec::new();
    let mut map: HashMap<Uuid, PathBuf> = HashMap::new();
    let mut total: u64 = 0;
    for f in files {
        walk(f, "", &mut metas, &mut map, &mut total)?;
    }
    let count = metas.len() as u64;
    Ok((Manifest { session_id, files: metas, total_size: total, total_count: count }, map))
}

fn walk(abs: &str, rel_prefix: &str, metas: &mut Vec<FileMeta>,
        map: &mut HashMap<Uuid, PathBuf>, total: &mut u64) -> Result<()> {
    let p = Path::new(abs);
    let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let rel = if rel_prefix.is_empty() { name.clone() } else { format!("{rel_prefix}/{name}") };
    if p.is_dir() {
        metas.push(FileMeta { id: Uuid::new_v4(), name: name.clone(), rel_path: rel.clone(), size: 0, kind: FileKind::Dir, hash: None });
        for entry in std::fs::read_dir(p)? {
            let entry = entry?;
            walk(&entry.path().to_string_lossy(), &rel, metas, map, total)?;
        }
    } else {
        let size = std::fs::metadata(p)?.len();
        let id = Uuid::new_v4();
        map.insert(id, p.to_path_buf());
        metas.push(FileMeta { id, name, rel_path: rel, size, kind: FileKind::File, hash: None });
        *total += size;
    }
    Ok(())
}
