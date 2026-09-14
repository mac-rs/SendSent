# AGENTS.md

Tauri 2 desktop app: React 19 + TypeScript frontend in `src/`, Rust backend in `src-tauri/`.

## Commands

Package manager is **pnpm** (lockfile committed). Do not use npm/yarn.

- `pnpm tauri dev` — run the actual desktop app (orchestrates frontend + Rust shell). This is the primary dev command, **not** `pnpm dev`.
- `pnpm dev` — Vite web-only dev server on **port 1420** (`strictPort`, required by Tauri — do not change). Useful only for iterating on frontend in isolation.
- `pnpm build` — runs `tsc && vite build`. This is the only typecheck; there is no separate `lint`/`typecheck` script.
- `pnpm tauri build` — full production bundle (calls `pnpm build` via `beforeBuildCommand`, then builds Rust + bundles the app).
- Rust checks: `cargo check` / `cargo build` from `src-tauri/`.
- Rust tests: `cargo test` from `src-tauri/` (unit tests are inline `#[cfg(test)]`; end-to-end loopback test at `src-tauri/tests/loopback.rs`). `cargo clippy --all-targets -- -D warnings` must stay clean.

## Working in the codebase

- TypeScript is **strict** with `noUnusedLocals` and `noUnusedParameters`. Unused imports/vars fail the build.
- Adding a Rust command: define it in `src-tauri/src/lib.rs`, register it in the `tauri::generate_handler![...]` list, then call it from the frontend with `invoke("name", { args })` from `@tauri-apps/api/core`. An unregistered command will silently fail at runtime.
- Tauri 2 permissions are capability-based. New plugins/commands needing privileged access must be granted in `src-tauri/capabilities/default.json` (currently `core:default`, `opener:default`, and `dialog:default`).
- App entrypoints: frontend `src/main.tsx` → `src/App.tsx`; Rust `src-tauri/src/main.rs` → `sendsent_lib::run()` in `src-tauri/src/lib.rs`.
- App identifier: `com.mankong.sendsent` (set in `src-tauri/tauri.conf.json`).

## File transfer (v1)

LAN auto-discovery + peer-to-peer transfer over a custom raw-TCP protocol. Design: `docs/superpowers/specs/2026-06-25-high-speed-file-transfer-design.md`; plan: `docs/superpowers/plans/`.

- Custom binary protocol on **plain TCP** (control frame + data frame; see `proto/`). v1 is plaintext; encryption/zero-copy/mobile are deferred to v2/v3.
- **mDNS** discovery via `mdns-sd` (pure Rust, chosen for mobile compatibility), service `_sendsent._tcp.local.`. Network listener default port **52225** (set in `lib.rs`).
- New deps: `tauri-plugin-dialog`, `mdns-sd`, `tokio`, `bincode`, `uuid`, `anyhow`, `async-trait`.
- Session logic is decoupled from Tauri: it emits `TransferEvent` (see `events.rs`) over an `mpsc` channel; `SessionManager` forwards them as Tauri events (`peer://found|lost`, `transfer://request|progress|finished`). Frontend treats events as the source of truth (the `send_files` command also returns the session id, but the id arrives via the first `transfer://progress` event too).
- Modules are `pub mod` so `tests/loopback.rs` can exercise the real TCP path end-to-end (two in-process nodes, mDNS skipped via direct connection).
- Adding a new Tauri command: define it in `commands.rs`, register in `lib.rs`'s `generate_handler!`. A new wire message: add the struct in `proto/messages.rs` + a `MsgType` variant + handle it in `sender.rs`/`receiver.rs`.

### v2 speed layer

- **Multi-connection** sender (default 16, configurable via `TransferConfig` / `SENDSENT_CONNS`): large files split by offset-range across connections, small files round-robin.
- **Zero-copy (NOT enabled)**: `transfer/zerocopy.rs` has a tested `sendfile(2)` path (macOS/Linux) + `write_data_header`, but it was **reverted** from the send hot path (commit `299fb13`: macOS `sendfile` caused broken-pipe on macOS→Android even with a header flush). The sender uses buffered `pread` + `write_data`; TLS mode also can't use `sendfile` (uses `fallback_send_payload`).
- **Concurrent multi-stream receiver** (`transfer/receiver.rs`): `mpsc` data channel, `DrainState` with `Arc<File>` + `write_at` (positional, concurrency-safe), `AtomicU64` + `Notify` for completion signaling.
- **Socket tuning** (`transfer/sock.rs`): `SO_SNDBUF`/`SO_RCVBUF` 16 MiB, `TCP_NODELAY`.
- **Configurable** via `TransferConfig { conns, chunk_size, split_threshold }`, persisted at `<app_data_dir>/transfer.json`, env overrides `SENDSENT_CONNS` / `SENDSENT_CHUNK_KB` / `SENDSENT_SPLIT_MB`.
- **No wire-protocol change**: same `Hello`/`DataOpen`/data frames; v2 sender → v1 receiver is incompatible (v1 only accepts one data connection per session), but everyone upgrades together.
- New deps: `libc`, `socket2`.
- See `docs/superpowers/specs/2026-06-26-file-transfer-v2-speed-design.md` and `docs/superpowers/plans/2026-06-26-file-transfer-v2-speed.md`.

## iOS development (native SwiftUI + Rust FFI)

iOS is **not** a Tauri/WebView app. The Xcode target in `src-tauri/gen/apple/` links the Rust static lib directly; the UI is native SwiftUI. Desktop/Android keep the Tauri shell. (UI land is Plan 2; `docs/superpowers/plans/2026-09-13-ios-native-swiftui.md`.)

- **Rust is decoupled from Tauri on iOS.** Tauri sits behind the Cargo feature `tauri-shell` (default; enabled for desktop + Android). iOS builds with `--no-default-features`, so `tauri`/`tao` are not compiled.
  - Build the iOS lib: `cd src-tauri && cargo build --lib --target aarch64-apple-ios --no-default-features` → `target/aarch64-apple-ios/<profile>/libsendsent_lib.a`.
  - The Xcode "Build Rust Code" phase (`gen/apple/project.yml`) runs that cargo command and copies the result to `Externals/arm64/<config>/libapp.a`.
- **FFI surface:** `src-tauri/src/ffi.rs` (`#[cfg(target_os = "ios")]`) exposes `#[unsafe(no_mangle)] extern "C"` functions returning JSON `*mut c_char` (free with `sendsent_ios_free_string`) plus one event callback.
  - API: `sendsent_ios_init` / `identity` / `peers` / `add_peer` / `send` / `respond` / `history` / `clear_history` / `get_transfer_config` / `set_transfer_config` / `set_display_name` / `addresses` / `qr`.
  - Events reuse `TransferEvent` serde (internal tag `kind`: `request`/`progress`/`finished`/`recorded`) plus `{"kind":"peer_found"|"peer_lost",…}`. `progress` is throttled to 100 ms per session; terminal states always emit.
- **Project generation:** `gen/apple/sendsent.xcodeproj` is generated from `gen/apple/project.yml` via `xcodegen generate` (run inside `gen/apple/`). Edit `project.yml`, not the pbxproj. `deploymentTarget.iOS = 17.0`; version strings and `Info.plist` keys live in `project.yml` `info.properties` (editing `Info.plist` alone is overwritten on regenerate).
- **Device vs simulator / signing:** a physical iPhone on iOS N needs a matching Xcode; `DEVELOPMENT_TEAM` is baked into `project.yml`.
- **Removed (Tauri/tao-only, no longer needed):** the `[patch.crates-io] tao` pin, the `TaoSceneDelegate` scene-manifest hack, `globalize_symbols.sh`, `Picker.swift`, `commands::ios_picker`, and the Tauri iOS entry (`main.mm` + `bindings/`). Sending uses SwiftUI `.fileImporter`.
- **Status:** Plan 1 (Rust FFI + build decoupling) done. Until Plan 2 adds the SwiftUI `@main` app, the iOS app target has no entry point (only the Rust lib builds).

## Android development (native Kotlin/Compose + Rust JNI)

Android is **not** a Tauri/WebView app. The UI is native Kotlin + Jetpack Compose (Material 3); the Rust core is loaded as `libsendsent_lib.so` and called via JNI. Desktop keeps the Tauri shell.

- **Rust is decoupled from Tauri on Android too.** Tauri sits behind the `tauri-shell` feature (default; desktop only now). Android native builds with `--no-default-features`.
- **Build the `.so`:** `src-tauri/build-android.sh [<rust-target> <abi>]` (default `aarch64-linux-android arm64-v8a`). It sets the NDK linker env and copies the result into `gen/android/app/src/main/jniLibs/<abi>/`. Gradle invokes it via the `cargoBuild` task before `preBuild`.
  - `./gradlew assembleDebug` from `src-tauri/gen/android` (needs `ANDROID_HOME`/`NDK_HOME`).
- **JNI surface:** `src-tauri/src/jni_bridge.rs` exports `Java_com_mankong_sendsent_Native_*` (JSON strings) over the shared `src-tauri/src/engine.rs`. Kotlin declarations live in `Native.kt`.
- **Discovery is Kotlin-driven:** `NsdBridge.kt` owns `NsdManager` and pushes resolved services to Rust via `Native.nativeOnService` → `discovery::android_native`. Rust cannot use raw `mdns-sd` on Android (SELinux denies the netlink/multicast path). Events flow back to Kotlin via a queue polled with `Native.nativePollEvents` (100 ms).
- **Files use SAF:** `SafPicker.kt` opens `content://` via `ParcelFileDescriptor` and passes `[{"fd","name"}]` to Rust, which reads `/proc/self/fd/<fd>` (zero-copy).
- **Tauri Android is removed:** `gen/android` no longer uses `tauri.settings.gradle`, `buildSrc` (rust plugin), `NsdPlugin.kt`/`ContentPlugin.kt`, or the `generated/` Wry classes. **Do not run `pnpm tauri android init`** — it would regenerate the Tauri setup and overwrite the native project.
- **Permissions:** `INTERNET`, `ACCESS_NETWORK_STATE`, `ACCESS_WIFI_STATE`, `CAMERA` (QR scan via CameraX + ZXing); `usesCleartextTraffic=true` (LAN plaintext TCP).
- **Known:** iOS suspends the app in the background, so its Bonjour advertisement stops — the phone must be foregrounded to be discovered.
- **Status:** Plans 1+2 done (engine/JNI/bridges/build + Compose UI). Web `AndroidApp.tsx`/`android.css` are no longer packaged.
