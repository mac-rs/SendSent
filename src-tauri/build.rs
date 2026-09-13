fn main() {
    // Tauri shell is only built with the `tauri-shell` feature (desktop + Android).
    // The native iOS client links the staticlib directly and does not use Tauri.
    #[cfg(feature = "tauri-shell")]
    tauri_build::build();
}
