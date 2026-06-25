# AGENTS.md

Tauri 2 desktop app: React 19 + TypeScript frontend in `src/`, Rust backend in `src-tauri/`.

## Commands

Package manager is **pnpm** (lockfile committed). Do not use npm/yarn.

- `pnpm tauri dev` — run the actual desktop app (orchestrates frontend + Rust shell). This is the primary dev command, **not** `pnpm dev`.
- `pnpm dev` — Vite web-only dev server on **port 1420** (`strictPort`, required by Tauri — do not change). Useful only for iterating on frontend in isolation.
- `pnpm build` — runs `tsc && vite build`. This is the only typecheck; there is no separate `lint`/`typecheck` script.
- `pnpm tauri build` — full production bundle (calls `pnpm build` via `beforeBuildCommand`, then builds Rust + bundles the app).
- Rust checks: `cargo check` / `cargo build` from `src-tauri/`.

No test runner is configured.

## Working in the codebase

- TypeScript is **strict** with `noUnusedLocals` and `noUnusedParameters`. Unused imports/vars fail the build.
- Adding a Rust command: define it in `src-tauri/src/lib.rs`, register it in the `tauri::generate_handler![...]` list, then call it from the frontend with `invoke("name", { args })` from `@tauri-apps/api/core`. An unregistered command will silently fail at runtime.
- Tauri 2 permissions are capability-based. New plugins/commands needing privileged access must be granted in `src-tauri/capabilities/default.json` (currently only `core:default` and `opener:default`).
- App entrypoints: frontend `src/main.tsx` → `src/App.tsx`; Rust `src-tauri/src/main.rs` → `sendsent_lib::run()` in `src-tauri/src/lib.rs`.
- App identifier: `com.mankong.sendsent` (set in `src-tauri/tauri.conf.json`).
