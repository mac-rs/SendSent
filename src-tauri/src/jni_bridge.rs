//! Android JNI 桥：Kotlin `object Native` 通过 JNI 调用共享 engine。
#![cfg(target_os = "android")]

use jni::errors::LogErrorAndDefault;
use jni::objects::{JObject, JString};
use jni::refs::Reference as _;
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::{Env, EnvUnowned};

use crate::engine;

fn js(env: &mut Env<'_>, s: String) -> jstring {
    env.new_string(s).map(|j| j.as_raw()).unwrap_or(std::ptr::null_mut())
}

fn s_arg(env: &Env<'_>, s: &JString<'_>) -> Option<String> {
    if s.is_null() { None } else { s.try_to_string(env).ok() }
}

fn err(env: &mut Env<'_>, msg: impl Into<String>) -> jstring {
    js(env, serde_json::json!({ "error": msg.into() }).to_string())
}

// jni 0.22: native 方法收 EnvUnowned,经 with_env 升级为 &mut Env;
// panic/错误由 LogErrorAndDefault 记日志并返回默认值(null/0),不抛 Java 异常,
// 与 Kotlin 侧"解析 JSON、null 即失败"的约定一致。

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeInit(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    data_dir: JString<'_>,
    save_dir: JString<'_>,
    port: jint,
) -> jint {
    env.with_env(|env| -> jni::errors::Result<jint> {
        let (Some(d), Some(s)) = (s_arg(env, &data_dir), s_arg(env, &save_dir)) else {
            return Ok(1);
        };
        Ok(engine::init(
            std::path::PathBuf::from(d),
            std::path::PathBuf::from(s),
            port as u16,
            None,
        ))
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativePollEvents(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(js(env, engine::poll_events()))
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeIdentity(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(js(env, engine::identity_json()))
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativePeers(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(js(env, engine::peers_json()))
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeHistory(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(js(env, engine::history_json()))
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeAddresses(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(match engine::addresses_json() {
            Ok(s) => js(env, s),
            Err(e) => err(env, e),
        })
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeQr(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    size: jint,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(match engine::qr_json(size as u32, None) {
            Ok(s) => js(env, s),
            Err(e) => err(env, e),
        })
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeGetConfig(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(js(env, engine::get_config_json()))
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeAddPeer(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    addr: JString<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        let Some(a) = s_arg(env, &addr) else { return Ok(err(env, "bad addr")) };
        Ok(match engine::add_peer(&a) {
            Ok(()) => std::ptr::null_mut(),
            Err(e) => err(env, e),
        })
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSend(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    peer: JString<'_>,
    files: JString<'_>,
    secure: jboolean,
    verify: jboolean,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        let (Some(p), Some(f)) = (s_arg(env, &peer), s_arg(env, &files)) else {
            return Ok(err(env, "bad args"));
        };
        Ok(match engine::send(&p, &f, secure, verify) {
            Ok(s) => js(env, s),
            Err(e) => err(env, e),
        })
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeRespond(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    sid: JString<'_>,
    accept: jboolean,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        let Some(s) = s_arg(env, &sid) else { return Ok(err(env, "bad sid")) };
        Ok(match engine::respond(&s, accept) {
            Ok(()) => std::ptr::null_mut(),
            Err(e) => err(env, e),
        })
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeDeleteHistory(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    sid: JString<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        if let Some(s) = s_arg(env, &sid) {
            engine::delete_history(&s);
        }
        Ok(std::ptr::null_mut())
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeClearHistory(
    _env: EnvUnowned<'_>,
    _this: JObject<'_>,
) {
    engine::clear_history();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSetConfig(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    conns: jint,
    chunk_kb: jlong,
    split_mb: jlong,
    zerocopy: jboolean,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        Ok(match engine::set_config(conns as u32, chunk_kb as u64, split_mb as u64, zerocopy) {
            Ok(()) => std::ptr::null_mut(),
            Err(e) => err(env, e),
        })
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeSetDisplayName(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    name: JString<'_>,
) -> jstring {
    env.with_env(|env| -> jni::errors::Result<jstring> {
        let Some(n) = s_arg(env, &name) else { return Ok(err(env, "bad name")) };
        Ok(match engine::set_display_name(&n) {
            Ok(()) => std::ptr::null_mut(),
            Err(e) => err(env, e),
        })
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeOnService(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    name: JString<'_>,
    host: JString<'_>,
    port: jint,
    txt: JString<'_>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let (Some(n), Some(h), Some(t)) =
            (s_arg(env, &name), s_arg(env, &host), s_arg(env, &txt))
        else {
            return Ok(());
        };
        engine::on_service(&n, &h, port as u16, &t);
        Ok(())
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mankong_sendsent_Native_nativeOnServiceLost(
    mut env: EnvUnowned<'_>,
    _this: JObject<'_>,
    name: JString<'_>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        if let Some(n) = s_arg(env, &name) {
            engine::on_service_lost(&n);
        }
        Ok(())
    })
    .resolve::<LogErrorAndDefault>()
}
