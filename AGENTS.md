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

- **Multi-connection** sender (default 4, configurable via `TransferConfig` / `SENDSENT_CONNS`): large files split by offset-range across connections, small files round-robin.
- **Zero-copy** on macOS (`sendfile(2)` in `transfer/zerocopy.rs`); Linux/Windows fall back to `pread`+write. Payload bytes go kernel-direct; each chunk's 29-byte frame header (`write_data_header`) is written normally.
- **Concurrent multi-stream receiver** (`transfer/receiver.rs`): `mpsc` data channel, `DrainState` with `Arc<File>` + `write_at` (positional, concurrency-safe), `AtomicU64` + `Notify` for completion signaling.
- **Socket tuning** (`transfer/sock.rs`): `SO_SNDBUF`/`SO_RCVBUF` 8 MiB, `TCP_NODELAY`.
- **Configurable** via `TransferConfig { conns, chunk_size, split_threshold }`, persisted at `<app_data_dir>/transfer.json`, env overrides `SENDSENT_CONNS` / `SENDSENT_CHUNK_KB` / `SENDSENT_SPLIT_MB`.
- **No wire-protocol change**: same `Hello`/`DataOpen`/data frames; v2 sender → v1 receiver is incompatible (v1 only accepts one data connection per session), but everyone upgrades together.
- New deps: `libc`, `socket2`.
- See `docs/superpowers/specs/2026-06-26-file-transfer-v2-speed-design.md` and `docs/superpowers/plans/2026-06-26-file-transfer-v2-speed.md`.

## iOS development

The iOS target lives in `src-tauri/gen/apple/` (Tauri-generated Xcode project). Hard-won gotchas:

- **Init needs CocoaPods:** `brew install cocoapods`, then `pnpm tauri ios init` (regenerates the Xcode project from `gen/apple/project.yml`).
- **Never build via Xcode GUI directly.** The "Build Rust Code" phase runs `tauri ios xcode-script`, which connects back to an IPC server that only `tauri ios dev`/`tauri ios build` start. Always run via `pnpm tauri ios dev "<sim name>"` / `pnpm tauri ios build`. (The `project.yml` build phase exports `PATH` so the phase finds `pnpm`/`cargo` regardless of how Xcode was launched.)
- **Command args are camelCase from JS (Tauri 2):** `invoke("send_files", { peerDeviceId, files })`, not `peer_device_id`. Mismatch → "missing required key peerDeviceId".
- **Device vs simulator:** a physical iPhone on iOS N needs a matching Xcode (e.g. iOS 27 needs Xcode 27 beta); Xcode 26.5 can't deploy to iOS 27 and silently falls back to "My Mac". The simulator runs the Mac's iOS runtime (no such constraint).
- **Same-host port collisions (Mac app + iOS simulator together):** both bind 52225 and both start a Vite on 1420.
  - Transfer port is configurable via `SENDSENT_PORT`; the simulator auto-uses **52226** (`cfg!(target_abi = "sim")` in `lib.rs`). Mac stays 52225.
  - Share one Vite: run the Mac app normally (`pnpm tauri dev`), then run the simulator with `pnpm tauri ios dev "iPhone 17" -c '{"build":{"beforeDevCommand":""}}'` so it reuses the existing Vite on 1420.
- **iOS sandbox `$HOME` is read-only** (`EROFS`). The save directory is resolved from `app.path().document_dir()` at startup and threaded through `AppState.save_dir` (not from an env var). Desktop still uses `~/Downloads/sendsent`.
- **Signing:** `DEVELOPMENT_TEAM` is baked into `gen/apple/project.yml` (re-init preserves it). Set the team / bundle id there, not just in Xcode UI (which gets wiped on re-init).
- **iPhone→Mac sending is not wired** in v1: `@tauri-apps/plugin-dialog`'s file `open()` is unsupported on iOS. iOS can only receive. Sending needs a native iOS document picker (deferred).

## Android development

- **Init:** `pnpm tauri android init` (generates `gen/android/`). Prerequisites: Android SDK (API 34+), NDK 27, Java 21, Rust Android targets (`rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`).
- **Env:** `ANDROID_HOME` and `NDK_HOME` must be set. The NDK clang must be in `PATH` and `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` set to the NDK's `aarch64-linux-android21-clang` for `cargo build` to work standalone. The real build goes through `pnpm tauri android build` (Gradle), which handles the linker internally.
- **Build lib only:** `cargo build --lib --target aarch64-linux-android` (the binary crate `main.rs` won't compile for Android — that's expected; only the lib is used).
- **Permissions:** `INTERNET` for networking, `ACCESS_NETWORK_STATE`/`ACCESS_WIFI_STATE` for mDNS. The NDK ships with `mdns-sd` (pure Rust) so no special multicast permissions needed.
- **File picker:** `@tauri-apps/plugin-dialog` `open()` works on Android (unlike iOS), so Android can both send and receive files natively. No custom picker needed.
