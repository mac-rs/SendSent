// 平台无关的纯函数：本机地址枚举、percent-encoding、二维码渲染、节流、fd 直读。
// 供 Tauri commands、iOS FFI、Android JNI 共用（不依赖 tauri）。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MyAddress {
    pub interface: String,
    pub ip: String,
}

/// 枚举所有非 loopback / 非 link-local 的 IPv4 接口。
pub fn get_my_addresses() -> Result<Vec<MyAddress>, String> {
    let mut addrs: Vec<MyAddress> = Vec::new();
    let ifaces = get_if_addrs::get_if_addrs().map_err(|e| format!("enum ifaces: {e}"))?;
    for iface in ifaces {
        if iface.is_loopback() {
            continue;
        }
        if let get_if_addrs::IfAddr::V4(v4) = iface.addr {
            let ip = v4.ip;
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

/// 最小 percent-encoding（RFC 3986 unreserved 之外全部转义）。
pub fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

/// 把 QrCode 渲染为 PNG 字节。
pub fn render_qr_png(code: &qrcode::QrCode, size: u32) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Luma};
    let modules = code.width() as u32;
    let quiet = 4u32;
    let total = (modules + quiet * 2) * 10;
    let _ = size;
    let scale = (size as f32 / total as f32).clamp(0.5, 4.0) as u32;
    let scale = scale.max(1);
    let img_size = (modules + quiet * 2) * scale;
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::from_pixel(img_size, img_size, Luma([255u8]));
    let colors = code.to_colors();
    for y in 0..modules {
        for x in 0..modules {
            if colors[(y * modules + x) as usize] == qrcode::Color::Dark {
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
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(out)
}

/// 生成 sendsent:// 二维码 PNG 的 base64。
pub fn my_qr_base64(
    name: &str,
    port: u16,
    save_dir: &str,
    size: Option<u32>,
    ip: Option<String>,
) -> Result<String, String> {
    use base64::Engine;
    let sz = size.unwrap_or(300).clamp(128, 1024);
    let addrs = get_my_addresses().unwrap_or_default();
    let chosen = ip
        .filter(|s| addrs.iter().any(|a| a.ip == *s))
        .or_else(|| addrs.first().map(|a| a.ip.clone()))
        .unwrap_or_default();
    let payload = format!(
        "sendsent://{}?addr={}:{}&dir={}",
        urlencoding(name),
        chosen,
        port,
        urlencoding(save_dir),
    );
    let code = qrcode::QrCode::new(payload.as_bytes()).map_err(|e| format!("qr: {e}"))?;
    let png = render_qr_png(&code, sz)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png))
}

/// 高频事件的按 key 节流（显式传入 now 以便单测）。
#[derive(Default)]
pub struct Throttle {
    min: std::time::Duration,
    last: std::collections::HashMap<uuid::Uuid, std::time::Instant>,
}

impl Throttle {
    pub fn new(min: std::time::Duration) -> Self {
        Self { min, last: Default::default() }
    }
    pub fn allow_at(&mut self, id: uuid::Uuid, now: std::time::Instant) -> bool {
        match self.last.get(&id) {
            Some(t) if now.duration_since(*t) < self.min => false,
            _ => {
                self.last.insert(id, now);
                true
            }
        }
    }
}

// ── Android SAF: fd 直读 ────────────────────────────────────
// SELinux 会拒绝按 `/proc/self/fd/<fd>` 路径 open（EACCES），但 fd 本身可读。
// 因此把逻辑路径映射到已打开 fd，发送时 dup 该 fd 直接读。

#[cfg(target_os = "android")]
static FD_MAP: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, i32>>> =
    std::sync::OnceLock::new();

#[cfg(target_os = "android")]
fn fd_map() -> &'static std::sync::Mutex<std::collections::HashMap<String, i32>> {
    FD_MAP.get_or_init(Default::default)
}

/// 记录逻辑路径 -> 已打开 fd（fd 由 Kotlin 通过 PFD 保活）。
#[cfg(target_os = "android")]
pub fn register_fd(path: String, fd: i32) {
    fd_map().lock().unwrap().insert(path, fd);
}

/// 打开发送源：Android 上若该路径已注册 fd，则 dup 后直接读，避免路径 open。
pub fn open_source(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    #[cfg(target_os = "android")]
    {
        let key = path.to_string_lossy().to_string();
        if let Some(fd) = fd_map().lock().unwrap().get(&key).copied() {
            let dup = unsafe { libc::dup(fd) };
            if dup >= 0 {
                use std::os::unix::io::FromRawFd;
                return Ok(unsafe { std::fs::File::from_raw_fd(dup) });
            }
        }
    }
    std::fs::File::open(path)
}

/// 取发送源大小：Android 上优先用 fd 的 fstat。
pub fn source_len(path: &std::path::Path) -> std::io::Result<u64> {
    #[cfg(target_os = "android")]
    {
        let key = path.to_string_lossy().to_string();
        if let Some(fd) = fd_map().lock().unwrap().get(&key).copied() {
            let mut st: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe { libc::fstat(fd, &mut st) } == 0 {
                return Ok(st.st_size as u64);
            }
        }
    }
    Ok(std::fs::metadata(path)?.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn urlencoding_escapes_reserved() {
        assert_eq!(urlencoding("a b/c?d=1"), "a%20b%2Fc%3Fd%3D1");
        assert_eq!(urlencoding("safe-._~"), "safe-._~");
    }

    #[test]
    fn render_qr_png_has_png_magic() {
        let code = qrcode::QrCode::new(b"sendsent://x?addr=1.2.3.4:52225").unwrap();
        let png = render_qr_png(&code, 300).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn throttle_limits_frequency_but_allows_first() {
        let id = uuid::Uuid::new_v4();
        let t0 = Instant::now();
        let mut th = Throttle::new(Duration::from_millis(100));
        assert!(th.allow_at(id, t0), "首次放行");
        assert!(!th.allow_at(id, t0 + Duration::from_millis(50)), "50ms 内拦截");
        assert!(th.allow_at(id, t0 + Duration::from_millis(100)), "到 100ms 放行");
    }
}
