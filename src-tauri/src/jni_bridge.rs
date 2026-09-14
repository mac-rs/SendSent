//! Android JNI 桥：Kotlin `object Native` 通过 JNI 调用共享 engine。
#![cfg(target_os = "android")]

use jni::objects::{JObject, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;

use crate::engine;

fn js(env: &mut JNIEnv, s: String) -> jstring {
    env.new_string(s).map(|j| j.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn s_arg(env: &mut JNIEnv, s: &JString) -> Option<String> {
    env.get_string(s).ok().map(|g| g.into())
}

fn err(env: &mut JNIEnv, msg: impl Into<String>) -> jstring {
    js(env, serde_json::json!({ "error": msg.into() }).to_string())
}

/// 捕获 Rust panic，避免跨 FFI abort，并记录原因。
fn safe<R>(fallback: R, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(e) => {
            let msg = e
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic".to_string());
            log::error!("RUST PANIC in jni: {msg}");
            fallback
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeInit(
    mut env: JNIEnv,
    _this: JObject,
    data_dir: JString,
    save_dir: JString,
    port: jint,
) -> jint {
    let (Some(d), Some(s)) = (s_arg(&mut env, &data_dir), s_arg(&mut env, &save_dir)) else {
        return 1;
    };
    engine::init(std::path::PathBuf::from(d), std::path::PathBuf::from(s), port as u16, None)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativePollEvents(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    js(&mut env, engine::poll_events())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeIdentity(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    js(&mut env, engine::identity_json())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativePeers(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    js(&mut env, engine::peers_json())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeHistory(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    js(&mut env, engine::history_json())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeAddresses(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    match engine::addresses_json() {
        Ok(s) => js(&mut env, s),
        Err(e) => err(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeQr(
    mut env: JNIEnv,
    _this: JObject,
    size: jint,
) -> jstring {
    match engine::qr_json(size as u32, None) {
        Ok(s) => js(&mut env, s),
        Err(e) => err(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeGetConfig(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    js(&mut env, engine::get_config_json())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeAddPeer(
    mut env: JNIEnv,
    _this: JObject,
    addr: JString,
) -> jstring {
    let Some(a) = s_arg(&mut env, &addr) else { return err(&mut env, "bad addr") };
    match engine::add_peer(&a) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSend(
    mut env: JNIEnv,
    _this: JObject,
    peer: JString,
    files: JString,
    secure: jboolean,
    verify: jboolean,
) -> jstring {
    safe(std::ptr::null_mut(), || {
        let (Some(p), Some(f)) = (s_arg(&mut env, &peer), s_arg(&mut env, &files)) else {
            return err(&mut env, "bad args");
        };
        match engine::send(&p, &f, secure != 0, verify != 0) {
            Ok(s) => js(&mut env, s),
            Err(e) => err(&mut env, e),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeRespond(
    mut env: JNIEnv,
    _this: JObject,
    sid: JString,
    accept: jboolean,
) -> jstring {
    safe(std::ptr::null_mut(), || {
        let Some(s) = s_arg(&mut env, &sid) else { return err(&mut env, "bad sid") };
        match engine::respond(&s, accept != 0) {
            Ok(()) => std::ptr::null_mut(),
            Err(e) => err(&mut env, e),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeDeleteHistory(
    mut env: JNIEnv,
    _this: JObject,
    sid: JString,
) -> jstring {
    if let Some(s) = s_arg(&mut env, &sid) {
        engine::delete_history(&s);
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeClearHistory(
    _env: JNIEnv,
    _this: JObject,
) {
    engine::clear_history();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSetConfig(
    mut env: JNIEnv,
    _this: JObject,
    conns: jint,
    chunk_kb: jlong,
    split_mb: jlong,
    zerocopy: jboolean,
) -> jstring {
    match engine::set_config(conns as u32, chunk_kb as u64, split_mb as u64, zerocopy != 0) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSetDisplayName(
    mut env: JNIEnv,
    _this: JObject,
    name: JString,
) -> jstring {
    let Some(n) = s_arg(&mut env, &name) else { return err(&mut env, "bad name") };
    match engine::set_display_name(&n) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeOnService(
    mut env: JNIEnv,
    _this: JObject,
    name: JString,
    host: JString,
    port: jint,
    txt: JString,
) {
    safe((), || {
        let (Some(n), Some(h), Some(t)) =
            (s_arg(&mut env, &name), s_arg(&mut env, &host), s_arg(&mut env, &txt))
        else {
            return;
        };
        engine::on_service(&n, &h, port as u16, &t);
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeOnServiceLost(
    mut env: JNIEnv,
    _this: JObject,
    name: JString,
) {
    if let Some(n) = s_arg(&mut env, &name) {
        engine::on_service_lost(&n);
    }
}
