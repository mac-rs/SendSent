//! iOS 原生 FFI：SwiftUI 通过 C ABI 调用共享 engine。
#![cfg(target_os = "ios")]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::engine;

type EventCb = extern "C" fn(*const c_char);

unsafe fn cstr(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("null pointer".into());
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| e.to_string())
}

fn to_c(s: String) -> *mut c_char {
    CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn err(msg: impl Into<String>) -> *mut c_char {
    to_c(serde_json::json!({ "error": msg.into() }).to_string())
}

fn ffi_try(r: Result<(), String>) -> *mut c_char {
    match r {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(e),
    }
}

fn ffi_res(r: Result<String, String>) -> *mut c_char {
    match r {
        Ok(s) => to_c(s),
        Err(e) => err(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_init(
    data_dir: *const c_char,
    save_dir: *const c_char,
    port: u16,
    event_cb: Option<EventCb>,
) -> i32 {
    let d = match unsafe { cstr(data_dir) } {
        Ok(s) => s,
        Err(_) => return 1,
    };
    let s = match unsafe { cstr(save_dir) } {
        Ok(s) => s,
        Err(_) => return 2,
    };
    let sink: Option<engine::EventSink> = event_cb.map(|cb| {
        Box::new(move |json: String| {
            if let Ok(c) = CString::new(json) {
                cb(c.as_ptr());
            }
        }) as engine::EventSink
    });
    engine::init(PathBuf::from(d), PathBuf::from(s), port, sink)
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_identity() -> *mut c_char {
    match engine::core() {
        Some(_) => to_c(engine::identity_json()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_peers() -> *mut c_char {
    match engine::core() {
        Some(_) => to_c(engine::peers_json()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_addresses() -> *mut c_char {
    ffi_res(engine::addresses_json())
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_qr(size: u32, ip: *const c_char) -> *mut c_char {
    let ip = if ip.is_null() { None } else { unsafe { cstr(ip) }.ok() };
    ffi_res(engine::qr_json(size, ip))
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_add_peer(addr: *const c_char) -> *mut c_char {
    let a = match unsafe { cstr(addr) } {
        Ok(s) => s,
        Err(e) => return err(e),
    };
    ffi_try(engine::add_peer(&a))
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_send(
    peer_id: *const c_char,
    files_json: *const c_char,
    secure: bool,
    verify: bool,
) -> *mut c_char {
    let p = match unsafe { cstr(peer_id) } {
        Ok(s) => s,
        Err(e) => return err(e),
    };
    let f = match unsafe { cstr(files_json) } {
        Ok(s) => s,
        Err(e) => return err(e),
    };
    ffi_res(engine::send(&p, &f, secure, verify))
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_respond(session_id: *const c_char, accept: bool) -> *mut c_char {
    let s = match unsafe { cstr(session_id) } {
        Ok(s) => s,
        Err(e) => return err(e),
    };
    ffi_try(engine::respond(&s, accept))
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_history() -> *mut c_char {
    match engine::core() {
        Some(_) => to_c(engine::history_json()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_clear_history() -> *mut c_char {
    engine::clear_history();
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_delete_history(session_id: *const c_char) -> *mut c_char {
    if let Ok(s) = unsafe { cstr(session_id) } {
        engine::delete_history(&s);
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_get_transfer_config() -> *mut c_char {
    match engine::core() {
        Some(_) => to_c(engine::get_config_json()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_set_transfer_config(conns: u32, chunk_kb: u64, split_mb: u64, zerocopy: bool) -> *mut c_char {
    ffi_try(engine::set_config(conns, chunk_kb, split_mb, zerocopy))
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_set_display_name(name: *const c_char) -> *mut c_char {
    let n = match unsafe { cstr(name) } {
        Ok(s) => s,
        Err(e) => return err(e),
    };
    ffi_try(engine::set_display_name(&n))
}

/// 从后台回到前台后调用:重新广播自身,让其它设备重新发现本机。
#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_reactivate() -> *mut c_char {
    ffi_try(engine::reactivate())
}
