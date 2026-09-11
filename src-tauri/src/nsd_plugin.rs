//! Android NSD plugin bridge: talks to the Kotlin `NsdPlugin` (system
//! `NsdManager`). Used because raw-socket mDNS cannot enumerate interfaces for
//! ordinary apps on modern Android (SELinux denies NETLINK_ROUTE bind, so
//! `getifaddrs()` returns EACCES).

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{
    plugin::{Builder, PluginApi, PluginHandle, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[derive(Serialize)]
struct RegisterPayload<'a> {
    name: &'a str,
    id: &'a str,
    plat: &'a str,
    port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NsdService {
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub txt: HashMap<String, String>,
}

#[derive(Deserialize)]
struct PollResponse {
    services: Vec<NsdService>,
}

pub struct Nsd<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Clone for Nsd<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: Runtime> Nsd<R> {
    pub async fn register(&self, name: &str, id: &str, plat: &str, port: u16) -> Result<(), String> {
        self.0
            .run_mobile_plugin_async::<serde_json::Value>(
                "register",
                RegisterPayload { name, id, plat, port },
            )
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn browse(&self) -> Result<(), String> {
        self.0
            .run_mobile_plugin_async::<serde_json::Value>("browse", ())
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn poll(&self) -> Result<Vec<NsdService>, String> {
        let resp: PollResponse = self
            .0
            .run_mobile_plugin_async("poll", ())
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.services)
    }
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> tauri::Result<Nsd<R>> {
    let handle = api.register_android_plugin("com.mankong.sendsent", "NsdPlugin")?;
    Ok(Nsd(handle))
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("nsd")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let nsd = init(app, api)?;
                app.manage(nsd);
            }
            Ok(())
        })
        .build()
}
