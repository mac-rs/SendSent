use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, PluginHandle, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[derive(Serialize)]
struct UriPayload<'a> {
    uri: &'a str,
}

#[derive(Deserialize)]
struct NameResponse {
    name: String,
}

pub struct Content<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Content<R> {
    pub fn display_name(&self, uri: &str) -> Result<String, String> {
        let resp: NameResponse = self
            .0
            .run_mobile_plugin("getDisplayName", UriPayload { uri })
            .map_err(|e| e.to_string())?;
        Ok(resp.name)
    }
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> tauri::Result<Content<R>> {
    let handle = api.register_android_plugin("com.mankong.sendsent", "ContentPlugin")?;
    Ok(Content(handle))
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("content")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let content = init(app, api)?;
                app.manage(content);
            }
            Ok(())
        })
        .build()
}
