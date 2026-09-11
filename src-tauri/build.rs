fn main() {
    // On iOS the Swift native picker (`sendsent_pick_files` in Picker.swift) is
    // compiled into the Xcode app target, not the Rust cdylib. The cdylib built
    // by `cargo build --lib` is unused on iOS (Xcode links the staticlib), so let
    // its link step tolerate the not-yet-defined Swift symbol. The symbol is
    // resolved when Xcode links libapp.a together with the Swift objects.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    }
    tauri_build::build()
}
