// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "tauri-shell")]
fn main() {
    sendsent_lib::run()
}

#[cfg(not(feature = "tauri-shell"))]
fn main() {}
