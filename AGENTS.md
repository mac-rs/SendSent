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
