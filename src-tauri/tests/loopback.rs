use sendsent_lib::events::{FinishedState, TransferEvent};
use sendsent_lib::proto::frame::{read_control, read_data, write_control, write_data};
use sendsent_lib::proto::messages::*;
use sendsent_lib::store::Identity;
use sendsent_lib::transfer::atomic::AtomicWriter;
use sendsent_lib::transfer::receiver::{run_receiver, Decision};
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
// Hello -> run_receiver (auto-accept), DataOpen -> route stream into that session's mpsc data channel.
async fn run_test_server(listener: TcpListener, our: Identity, events: mpsc::UnboundedSender<TransferEvent>, save_dir: PathBuf) {
    let mut channels: HashMap<Uuid, mpsc::Sender<TcpStream>> = HashMap::new();
    loop {
        let (mut stream, _) = match listener.accept().await { Ok(s) => s, Err(_) => break };
        let (ty, buf) = match read_control(&mut stream).await { Ok(x) => x, Err(_) => continue };
        match ty {
            MsgType::Hello => {
                let hello: Hello = match bincode::deserialize(&buf) { Ok(h) => h, Err(_) => continue };
                let sid = hello.session_id;
                let (dtx, drx) = oneshot::channel::<Decision>();
                let (xtx, xrx) = mpsc::channel::<TcpStream>(16);
                channels.insert(sid, xtx);
                let _ = dtx.send(Decision { accept: true, save_dir: save_dir.clone() });
                let ev = events.clone();
                let our = our.clone();
                tokio::spawn(async move { let _ = run_receiver(stream, hello, ev, drx, xrx, our).await; });
            }
            MsgType::DataOpen => {
                let d: DataOpen = match bincode::deserialize(&buf) { Ok(d) => d, Err(_) => continue };
                if let Some(xtx) = channels.get(&d.session_id).cloned() {
                    let _ = xtx.send(stream).await;
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
    let sender = tokio::spawn(run_sender(session_id, vec![addr], files, our_send, ev_tx.clone(), sendsent_lib::store::TransferConfig::defaults()));

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
        session_id: Uuid::new_v4(), proto_ver: PROTO_VER, secure: false };
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn v2_big_file_throughput() {
    let dir = std::env::temp_dir().join(format!("ss-bench-{}", Uuid::new_v4()));
    let save = dir.join("save");
    let src = dir.join("src.bin");
    let size: usize = 10 * 1024 * 1024; // 10 MiB, > split_threshold → exercises multi-conn split
    let big = vec![0u8; size];
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&src, &big).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<TransferEvent>();
    let our_recv = identity("recv");
    let save_clone = save.clone();
    tokio::spawn(run_test_server(listener, our_recv, ev_tx.clone(), save_clone));

    let cfg = sendsent_lib::store::TransferConfig::defaults();
    let our_send = identity("send");
    let sid = Uuid::new_v4();
    let files = vec![src.to_string_lossy().into_owned()];
    let sender = tokio::spawn(async move {
        run_sender(sid, vec![addr], files, our_send, ev_tx.clone(), cfg).await
    });

    let mut completed = false;
    let drain = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while let Some(ev) = ev_rx.recv().await {
            if let TransferEvent::Finished { state: FinishedState::Completed, .. } = ev { completed = true; break; }
        }
    }).await;
    assert!(drain.is_ok(), "timed out");
    assert!(completed, "expected Completed");
    let _ = sender.await;

    let got = std::fs::read(save.join("src.bin")).unwrap();
    assert_eq!(got.len(), size, "size mismatch");
    assert_eq!(got, big, "content mismatch");

    let _ = std::fs::remove_dir_all(&dir);
}

