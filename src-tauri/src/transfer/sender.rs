use crate::events::{TransferEvent, SessionState, FinishedState, ErrorPayload};
use crate::proto::frame::{read_control, write_control, write_data_header};
use crate::proto::messages::*;
use crate::store::{Identity, TransferConfig};
use crate::transfer::sock::tune_socket;
use crate::transfer::zerocopy::send_payload;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
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
    config: TransferConfig,
) -> Result<()> {
    let result = run_sender_inner(session_id, peer_addrs, files, our.clone(), events.clone(), config).await;
    if let Err(e) = &result {
        tracing::error!("sender failed: {e}");
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::Internal, message: e.to_string() }) });
    }
    result
}

async fn run_sender_inner(
    session_id: Uuid,
    peer_addrs: Vec<SocketAddr>,
    files: Vec<String>,
    our: Identity,
    events: mpsc::UnboundedSender<TransferEvent>,
    config: TransferConfig,
) -> Result<()> {
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Connecting,
        bytes_done: 0, bytes_total: 0, files_done: 0, files_total: 0, speed_bps: 0 });

    let mut control = match connect_any(&peer_addrs).await {
        Ok(s) => { tracing::info!("sender connected (tried {peer_addrs:?})"); s }
        Err(e) => { tracing::warn!("sender connect failed for {peer_addrs:?}: {e}"); return Err(e); }
    };
    let hello = Hello { device_id: our.device_id.clone(), name: our.name.clone(),
        platform: Platform::Macos, session_id, proto_ver: PROTO_VER };
    write_control(&mut control, MsgType::Hello, &bincode::serialize(&hello)?).await?;
    let (ty, _) = read_control(&mut control).await?;
    if ty != MsgType::HelloAck { return Err(anyhow!("expected helloack, got {ty:?}")); }

    let (manifest, file_map) = build_manifest(session_id, &files)?;
    let files_total = manifest.files.iter().filter(|f| f.kind == FileKind::File).count() as u64;
    write_control(&mut control, MsgType::Manifest, &bincode::serialize(&manifest)?).await?;
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::AwaitingAccept,
        bytes_done: 0, bytes_total: manifest.total_size, files_done: 0, files_total, speed_bps: 0 });

    let (ty, _buf) = read_control(&mut control).await?;
    if ty == MsgType::Reject {
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Rejected, error: None });
        return Ok(());
    }
    if ty != MsgType::Accept { return Err(anyhow!("expected accept, got {ty:?}")); }

    // 规划分发:每个文件切成 segment,round-robin 到 N 个 bucket
    let buckets = plan_buckets(&manifest, &file_map, config.conns as usize, config.split_threshold);
    let chunk = config.chunk_size as usize;

    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Transferring,
        bytes_done: 0, bytes_total: manifest.total_size, files_done: 0, files_total, speed_bps: 0 });

    let total_done = Arc::new(AtomicU64::new(0));
    let total_size = manifest.total_size;

    // 并发开数据连接(每个非空 bucket 一条),各自跑自己的 segment
    let mut join = tokio::task::JoinSet::new();
    for bucket in buckets.into_iter().filter(|b| !b.is_empty()) {
        let addrs = peer_addrs.clone();
        let done = total_done.clone();
        let sid = session_id;
        join.spawn(async move {
            let mut data = connect_any(&addrs).await?;
            write_control(&mut data, MsgType::DataOpen, &bincode::serialize(&DataOpen { session_id: sid })?).await?;
            for seg in &bucket {
                send_segment(&mut data, seg, chunk, &done).await?;
            }
            let _ = data.shutdown().await;
            Ok::<(), anyhow::Error>(())
        });
    }

    while let Some(res) = join.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow!("join: {e}")),
        }
    }

    let done = total_done.load(Ordering::Relaxed);
    let _ = events.send(TransferEvent::Progress { session_id, state: SessionState::Finalizing,
        bytes_done: done, bytes_total: total_size, files_done: files_total, files_total, speed_bps: 0 });

    let (ty, _) = read_control(&mut control).await?;
    let final_state = if ty == MsgType::Complete { FinishedState::Completed } else { FinishedState::Failed };
    let _ = events.send(TransferEvent::Finished { session_id, state: final_state,
        error: if final_state == FinishedState::Failed { Some(ErrorPayload { code: ErrorCode::Internal, message: "no complete".into() }) } else { None } });
    Ok(())
}

struct Segment { file_id: Uuid, path: PathBuf, offset: u64, len: u64 }

/// 返回 conns 个 bucket(每个是一串 Segment)。大文件(>=split)按 conns 等分;小文件整块;再 round-robin。
fn plan_buckets(
    manifest: &Manifest,
    file_map: &HashMap<Uuid, PathBuf>,
    conns: usize,
    split: u64,
) -> Vec<Vec<Segment>> {
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
        } else if f.size > 0 {
            segments.push(Segment { file_id: f.id, path, offset: 0, len: f.size });
        }
    }
    let mut buckets: Vec<Vec<Segment>> = (0..conns).map(|_| Vec::new()).collect();
    for (i, seg) in segments.into_iter().enumerate() { buckets[i % conns].push(seg); }
    buckets
}

async fn send_segment(data: &mut TcpStream, seg: &Segment, chunk: usize, done: &AtomicU64) -> Result<()> {
    let file = std::fs::File::open(&seg.path)?;
    let mut off = seg.offset;
    let end = seg.offset + seg.len;
    while off < end {
        let n = chunk.min((end - off) as usize) as u32;
        write_data_header(data, seg.file_id, off, n).await?;
        send_payload(data, &file, off, n as usize).await?;
        off += n as u64;
        done.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(())
}

async fn connect_any(addrs: &[SocketAddr]) -> Result<TcpStream> {
    let mut last = None;
    for a in addrs {
        match TcpStream::connect(a).await { Ok(s) => { tune_socket(&s); return Ok(s); } Err(e) => last = Some(e) }
    }
    Err(anyhow!("connect failed: {:?}", last))
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
