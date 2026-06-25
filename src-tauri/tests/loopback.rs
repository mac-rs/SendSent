use sendsent_lib::events::{FinishedState, TransferEvent};
use sendsent_lib::proto::frame::{read_control, read_data, write_control, write_data};
use sendsent_lib::proto::messages::*;
use sendsent_lib::store::Identity;
use sendsent_lib::transfer::atomic::AtomicWriter;
use sendsent_lib::transfer::receiver::{drain_data, run_receiver, Decision};
use sendsent_lib::transfer::sender::run_sender;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

fn identity(name: &str) -> Identity {
    Identity { device_id: Uuid::new_v4().to_string(), name: name.into(), platform: "macos".into() }
}

// Mini server mimicking the manager's first-frame dispatch: one listener,
// Hello -> run_receiver (auto-accept), DataOpen -> route stream into that session's data channel.
async fn run_test_server(listener: TcpListener, our: Identity, events: mpsc::UnboundedSender<TransferEvent>, save_dir: PathBuf) {
    let mut channels: HashMap<Uuid, oneshot::Sender<TcpStream>> = HashMap::new();
    loop {
        let (mut stream, _) = match listener.accept().await { Ok(s) => s, Err(_) => break };
        let (ty, buf) = match read_control(&mut stream).await { Ok(x) => x, Err(_) => continue };
        match ty {
            MsgType::Hello => {
                let hello: Hello = match bincode::deserialize(&buf) { Ok(h) => h, Err(_) => continue };
                let sid = hello.session_id;
                let (dtx, drx) = oneshot::channel::<Decision>();
                let (xtx, xrx) = oneshot::channel::<TcpStream>();
                channels.insert(sid, xtx);
                let _ = dtx.send(Decision { accept: true, save_dir: save_dir.clone() });
                let ev = events.clone();
                let our = our.clone();
                tokio::spawn(async move { let _ = run_receiver(stream, hello, ev, drx, xrx, our).await; });
            }
            MsgType::DataOpen => {
                let d: DataOpen = match bincode::deserialize(&buf) { Ok(d) => d, Err(_) => continue };
                if let Some(xtx) = channels.remove(&d.session_id) {
                    let _ = xtx.send(stream);
                }
            }
            _ => {}
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn end_to_end_send_folder() {
    let dir = std::env::temp_dir().join(format!("ss-loop-{}", Uuid::new_v4()));
    let save = dir.join("save");
    let src = dir.join("src");
    let sub = src.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let big: Vec<u8> = (0..600_000u32).map(|i| (i % 251) as u8).collect(); // > 256KiB chunk
    std::fs::write(src.join("a.txt"), &big).unwrap();
    std::fs::write(sub.join("b.txt"), b"hello b").unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<TransferEvent>();
    let our_recv = identity("recv");
    let save_clone = save.clone();
    tokio::spawn(run_test_server(listener, our_recv, ev_tx.clone(), save_clone));

    let session_id = Uuid::new_v4();
    let our_send = identity("send");
    let files = vec![src.to_string_lossy().into_owned()];
    let sender = tokio::spawn(run_sender(session_id, vec![addr], files, our_send, ev_tx.clone()));

    let mut completed = false;
    let drain = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while let Some(ev) = ev_rx.recv().await {
            if let TransferEvent::Finished { state: FinishedState::Completed, .. } = ev {
                completed = true;
                break;
            }
        }
    }).await;
    assert!(drain.is_ok(), "timed out waiting for Completed event");
    assert!(completed, "expected a Completed finished event");

    let _ = sender.await;

    assert_eq!(std::fs::read(save.join("src").join("a.txt")).unwrap(), big);
    assert_eq!(std::fs::read(save.join("src").join("sub").join("b.txt")).unwrap(), b"hello b");
    assert!(!save.join(".sendsent-tmp").exists(), "temp dir should be cleaned");

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn control_handshake_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
    let (mut server, _) = listener.accept().await.unwrap();
    let mut client = conn.await.unwrap();

    let hello = Hello { device_id: "A".into(), name: "a".into(), platform: Platform::Macos,
        session_id: Uuid::new_v4(), proto_ver: PROTO_VER };
    write_control(&mut client, MsgType::Hello, &bincode::serialize(&hello).unwrap()).await.unwrap();
    let (ty, buf) = read_control(&mut server).await.unwrap();
    assert_eq!(ty, MsgType::Hello);
    let back: Hello = bincode::deserialize(&buf).unwrap();
    assert_eq!(back.device_id, "A");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn data_frame_reassembly() {
    use std::io::{Seek, SeekFrom, Write};
    use tokio::io::AsyncWriteExt;
    let dir = std::env::temp_dir().join(format!("ss-loop2-{}", Uuid::new_v4()));
    let save = dir.clone();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
    let (mut server, _) = listener.accept().await.unwrap();
    let mut client = conn.await.unwrap();

    let id = Uuid::new_v4();
    write_data(&mut client, id, 0, b"hello ").await.unwrap();
    write_data(&mut client, id, 6, b"world").await.unwrap();
    client.shutdown().await.unwrap();

    let writer = AtomicWriter::new(&save, "s1").unwrap();
    let pp = writer.part_path("x.txt");
    std::fs::write(&pp, vec![0u8; 11]).unwrap();
    while let Ok(c) = read_data(&mut server).await {
        let mut f = std::fs::OpenOptions::new().write(true).open(&pp).unwrap();
        f.seek(SeekFrom::Start(c.offset)).unwrap();
        f.write_all(&c.data).unwrap();
    }
    let dest = writer.finalize("x.txt").unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"hello world");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drain_data_reports_failed_on_truncation() {
    use tokio::io::AsyncWriteExt;
    let dir = std::env::temp_dir().join(format!("ss-trunc-{}", Uuid::new_v4()));
    let save = dir.clone();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let conn = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });
    let (server, _) = listener.accept().await.unwrap();
    let mut client = conn.await.unwrap();

    let id = Uuid::new_v4();
    // manifest claims a 1000-byte file
    let manifest = Manifest {
        session_id: Uuid::new_v4(),
        files: vec![FileMeta { id, name: "big.bin".into(), rel_path: "big.bin".into(),
            size: 1000, kind: FileKind::File, hash: None }],
        total_size: 1000, total_count: 1,
    };
    // send only 400 bytes then close (truncation)
    write_data(&mut client, id, 0, &vec![7u8; 400]).await.unwrap();
    client.shutdown().await.unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<TransferEvent>();
    let completed = drain_data(server, &manifest, &save, manifest.session_id, tx).await.unwrap();
    assert!(!completed, "truncated transfer must NOT be complete");

    // must have emitted exactly one Finished::Failed
    let mut got = None;
    while let Ok(Some(ev)) = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
        if let TransferEvent::Finished { state, .. } = ev { got = Some(state); }
    }
    assert_eq!(got, Some(FinishedState::Failed), "expected Failed event, got {got:?}");
    // temp cleaned
    assert!(!save.join(".sendsent-tmp").exists(), "temp must be cleaned on truncation");
    // no finalized file
    assert!(!save.join("big.bin").exists(), "partial file must not be finalized");
    let _ = std::fs::remove_dir_all(&dir);
}
