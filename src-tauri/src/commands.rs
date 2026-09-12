use crate::discovery::Peer;
use crate::state::AppState;
use std::net::SocketAddr;
use std::path::PathBuf;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

#[tauri::command]
pub fn get_identity(state: State<'_, AppState>) -> crate::store::Identity { state.identity.clone() }

#[tauri::command]
pub async fn set_display_name(_state: State<'_, AppState>, _name: String) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub async fn list_peers(state: State<'_, AppState>) -> Result<Vec<Peer>, String> {
    Ok(state.discovery.peers().await)
}

#[tauri::command]
pub async fn add_peer(state: State<'_, AppState>, address: String) -> Result<(), String> {
    let addr: SocketAddr = address.parse().map_err(|e| format!("invalid address: {e}"))?;
    state.discovery.add_manual_peer(addr).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn send_files(app: tauri::AppHandle, state: State<'_, AppState>, peer_device_id: String, files: Vec<String>, secure: bool, verify: bool) -> Result<Uuid, String> {
    // iOS' file picker hands back `file://` URLs; normalize to plain paths.
    let files: Vec<String> = files.into_iter().map(normalize_input_path).collect();
    // Android: content:// URIs can't be opened with std::fs. Open the underlying
    // file descriptor via the ContentPlugin and expose it as `/proc/self/fd/<fd>`
    // through a per-file symlink that preserves the original file name. This
    // avoids copying (large videos stay in place).
    #[cfg(target_os = "android")]
    let files = {
        use tauri::Manager;
        if files.iter().any(|f| f.starts_with("content://")) {
            let base = std::env::temp_dir().join("sendsent-fds");
            let _ = std::fs::create_dir_all(&base);
            let mut out = Vec::with_capacity(files.len());
            for f in files {
                if f.starts_with("content://") {
                    let content = app.state::<crate::content_plugin::Content<tauri::Wry>>();
                    let display_name = content.display_name(&f).unwrap_or_else(|_| "file".to_string());
                    let fd = content.open_fd(&f)?;
                    let safe: String = display_name
                        .chars()
                        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
                        .collect();
                    let safe = if safe.is_empty() { "file".to_string() } else { safe };
                    let dir = base.join(Uuid::new_v4().to_string());
                    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
                    let link = dir.join(&safe);
                    std::os::unix::fs::symlink(format!("/proc/self/fd/{fd}"), &link)
                        .map_err(|e| format!("symlink: {e}"))?;
                    out.push(link.to_string_lossy().into_owned());
                } else {
                    out.push(f);
                }
            }
            out
        } else {
            files
        }
    };
    let peers = state.discovery.peers().await;
    let n = peers.len();
    let Some(p) = peers.into_iter().find(|x| x.device_id == peer_device_id) else {
        tracing::warn!("send_files: peer {peer_device_id} not found ({n} peers known)");
        return Err("peer not found".into());
    };
    tracing::info!("send_files → '{}' addrs={:?} port={} files={}", p.name, p.addrs, p.port, files.len());
    tracing::debug!("send_files paths: {:?}", files);
    state.sessions.start_send(p, files, state.transfer_config.clone(), secure, verify).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_text(state: State<'_, AppState>, peer_device_id: String, text: String, secure: bool, verify: bool) -> Result<Uuid, String> {
    let dir = std::env::temp_dir().join("sendsent-text");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.txt", Uuid::new_v4()));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    let file = path.to_string_lossy().into_owned();
    let peers = state.discovery.peers().await;
    let Some(p) = peers.into_iter().find(|x| x.device_id == peer_device_id) else {
        return Err("peer not found".into());
    };
    state.sessions.start_send(p, vec![file], state.transfer_config.clone(), secure, verify).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn respond(state: State<'_, AppState>, session_id: Uuid, accept: bool, save_dir: Option<String>, pin: Option<String>) -> Result<(), String> {
    let dir = match save_dir {
        Some(d) => PathBuf::from(d),
        None => state.save_dir.clone(),
    };
    state.sessions.respond(session_id, accept, dir, pin).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel(_state: State<'_, AppState>, _session_id: Uuid) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub fn get_transfer_config(state: State<'_, AppState>) -> Result<crate::store::TransferConfig, String> {
    Ok(state.transfer_config.clone())
}

#[tauri::command]
pub fn set_transfer_config(state: State<'_, AppState>, conns: u32, chunk_kb: u64, split_mb: u64) -> Result<(), String> {
    let mut cfg = state.transfer_config.clone();
    cfg.conns = conns;
    cfg.chunk_size = chunk_kb * 1024;
    cfg.split_threshold = split_mb * 1024 * 1024;
    let sanitized = cfg.sanitized();
    let data_dir = state.identity_dir.clone();
    let p = data_dir.join("transfer.json");
    let s = serde_json::to_string_pretty(&sanitized).map_err(|e| e.to_string())?;
    std::fs::write(&p, s).map_err(|e| format!("write transfer.json: {e}"))?;
    tracing::info!("transfer config updated: conns={} chunk={} split={}", sanitized.conns, sanitized.chunk_size, sanitized.split_threshold);
    Ok(())
}

#[tauri::command]
pub fn get_default_save_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.save_dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn list_transfer_history(state: State<'_, AppState>) -> Result<Vec<crate::history::HistoryRecord>, String> {
    Ok(state.history.lock().await.list())
}

#[tauri::command]
pub async fn clear_transfer_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history.lock().await.clear();
    Ok(())
}

// Android 上 dialog.open() 首次不 resolve 的已知问题(tauri plugins-workspace
// #3366):打开系统选择器期间需要周期性调用一个命令,保持前后端 IPC 通道活跃。
#[tauri::command]
pub fn noop() {}

// ── 我的设备信息 · QR / IP 列表 ─────────────────────

/// 返回本机的内网 IP 列表(自动枚举所有非 loopback、非 link-local 的 IPv4 接口)
#[tauri::command]
pub fn get_my_addresses() -> Result<Vec<MyAddress>, String> {
    let mut addrs: Vec<MyAddress> = Vec::new();
    let ifaces = get_if_addrs::get_if_addrs().map_err(|e| format!("enum ifaces: {e}"))?;
    for iface in ifaces {
        if iface.is_loopback() {
            continue;
        }
        if let get_if_addrs::IfAddr::V4(v4) = iface.addr {
            let ip = v4.ip;
            // 跳过 link-local (169.254.x.x) 和 0.0.0.0
            if ip.is_link_local() || ip.is_unspecified() {
                continue;
            }
            addrs.push(MyAddress {
                interface: iface.name.clone(),
                ip: ip.to_string(),
            });
        }
    }
    addrs.sort_by(|a, b| a.ip.cmp(&b.ip));
    Ok(addrs)
}

#[derive(serde::Serialize)]
pub struct MyAddress {
    pub interface: String,
    pub ip: String,
}

/// 返回本机 QR PNG(base64 字符串),payload 为 sendsent:// 协议
/// - `size`: PNG 边长像素 (128..=1024)
/// - `ip`: 可选 — 显式指定要写入 payload 的 IP;默认取枚举出来的第一个
#[tauri::command]
pub fn get_my_qr(state: State<'_, AppState>, size: Option<u32>, ip: Option<String>) -> Result<String, String> {
    use base64::Engine;
    use qrcode::QrCode;
    let sz = size.unwrap_or(300).clamp(128, 1024);
    // 选 IP:显式 > 第一个枚举到的 > 空
    let addrs = get_my_addresses().unwrap_or_default();
    let chosen_ip = ip
        .filter(|s| addrs.iter().any(|a| a.ip == *s))
        .or_else(|| addrs.first().map(|a| a.ip.clone()))
        .unwrap_or_default();
    let payload = format!(
        "sendsent://{}?addr={}:{}&dir={}",
        urlencoding(&state.identity.name),
        chosen_ip,
        state.port,
        urlencoding(&state.save_dir.to_string_lossy()),
    );
    let code = QrCode::new(payload.as_bytes()).map_err(|e| format!("qr: {e}"))?;
    let png_bytes = render_qr_png(&code, sz).map_err(|e| format!("render: {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

fn render_qr_png(code: &qrcode::QrCode, size: u32) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Luma};
    let modules = code.width() as u32;
    let quiet = 4u32; // 4 模块的静默区
    let total = (modules + quiet * 2) * 10; // 10 px per module
    let _ = size; // 我们用固定比例 10px/module,size 只用于限制最大边
    let scale = (size as f32 / total as f32).clamp(0.5, 4.0) as u32;
    let scale = scale.max(1);
    let img_size = (modules + quiet * 2) * scale;
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::from_pixel(img_size, img_size, Luma([255u8]));
    let colors = code.to_colors();
    for y in 0..modules {
        for x in 0..modules {
            let dark = colors[(y * modules + x) as usize] == qrcode::Color::Dark;
            if dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = (x + quiet) * scale + dx;
                        let py = (y + quiet) * scale + dy;
                        if px < img_size && py < img_size {
                            img.put_pixel(px, py, Luma([0u8]));
                        }
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let dyn_img = image::DynamicImage::ImageLuma8(img);
    dyn_img
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(out)
}

// ── iOS 原生文档选择器 ──

#[cfg(target_os = "ios")]
pub(crate) mod ios_picker {
    use std::os::raw::c_char;
    use std::sync::Mutex;
    use tokio::sync::oneshot;

    static PICKER_STATE: Mutex<Option<oneshot::Sender<Vec<String>>>> = Mutex::new(None);

    unsafe extern "C" {
        fn sendsent_pick_files(cb: extern "C" fn(*const c_char));
    }

    extern "C" fn picker_result(ptr: *const c_char) {
        let mut guard = PICKER_STATE.lock().unwrap();
        if let Some(tx) = guard.take() {
            let files = if ptr.is_null() {
                vec![]
            } else {
                let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
                let s = c_str.to_string_lossy();
                serde_json::from_str(&s).unwrap_or_default()
            };
            let _ = tx.send(files);
        }
    }

    #[tauri::command]
    pub async fn pick_files_ios() -> Result<Vec<String>, String> {
        let (tx, rx) = oneshot::channel::<Vec<String>>();
        *PICKER_STATE.lock().unwrap() = Some(tx);
        unsafe { sendsent_pick_files(picker_result); }
        rx.await.map_err(|e| e.to_string())
    }
}

#[cfg(not(target_os = "ios"))]
pub(crate) mod ios_picker {
    #[tauri::command]
    pub async fn pick_files_ios() -> Result<Vec<String>, String> {
        Err("iOS only".into())
    }
}

/// Convert a `file://` URL (as returned by iOS' document picker) into a plain
/// filesystem path, percent-decoding as needed. Non-URL inputs pass through.
fn normalize_input_path(p: String) -> String {
    let rest = match p.strip_prefix("file://") {
        Some(r) => r,
        None => return p,
    };
    // file:///path -> /path ; file://localhost/path -> /path
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((h << 4) | l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ── 启动 splash screen ────────────────────────────
// 在 setup 阶段由 lib.rs 创建独立 splash 窗口,主窗口 visible=false
// 隐藏。前端 ready 后调用 splash_ready():淡出 splash → 关闭 → 显示主窗口。

/// 前端报告"我已 ready"。关闭 splash 窗口并显示主窗口。
/// 在主线程上同步执行,保证 close + show 顺序。
#[tauri::command]
pub fn splash_ready(app: tauri::AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

/// 在 lib.rs setup 中调用,创建 splash 窗口(显示在 main 之上)。
/// 桌面端使用一个小的居中窗口加载 splash.html;移动端通常
/// 不需要 splash(系统启动画面已覆盖),但仍创建以保持一致行为。
pub fn create_splash_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("splash").is_some() {
        return Ok(());
    }
    let _win = WebviewWindowBuilder::new(
        app,
        "splash",
        WebviewUrl::App("splash.html".into()),
    )
    .title("SendSent")
    .inner_size(420.0, 320.0)
    .min_inner_size(420.0, 320.0)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .focused(true)
    .skip_taskbar(true)
    .decorations(false)
    .always_on_top(true)
    .visible(true)
    .center()
    .build()?;
    Ok(())
}
