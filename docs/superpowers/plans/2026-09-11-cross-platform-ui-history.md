# Cross-Platform UI + Transfer History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the frontend adapt natively across macOS / Windows / Linux / iOS / Android, persist transfer history across restarts, and expose the file save location.

**Architecture:** Backend gains a `HistoryStore` (JSON in `app_data_dir`) fed by a new `TransferEvent::Recorded` emitted by the sender/receiver. The frontend loads history via a command, appends live via a `transfer://history` event, and renders it alongside in-flight progress. CSS/HTML get safe-area, touch, hover, reduced-motion, and platform branches; macOS uses an overlay title bar.

**Tech Stack:** Tauri 2, React 19 + TypeScript (strict), Rust, `@tauri-apps/plugin-opener`.

**Spec:** `docs/superpowers/specs/2026-09-11-cross-platform-ui-history-design.md`

---

## File Structure

- `index.html` — viewport/meta/title (modify)
- `src/theme.css` — platform foundation + visual polish (modify)
- `src/App.tsx` — root platform class, macOS drag region, settings/transfers wiring (modify)
- `src/lib/types.ts` — history types (modify)
- `src/lib/invoke.ts` — history commands (modify)
- `src/lib/events.ts` — `transfer://history` listener (modify)
- `src/hooks/useTransfer.ts` — history state + merge (modify)
- `src/components/TransferProgress.tsx` — direction, reveal, persisted rows (modify)
- `src/components/SaveLocation.tsx` — save dir display + open (create)
- `src/components/Settings.tsx` — mount SaveLocation (modify)
- `src-tauri/src/history.rs` — `HistoryStore` (create)
- `src-tauri/src/events.rs` — `Recorded` variant + event name (modify)
- `src-tauri/src/state.rs` — `history` field (modify)
- `src-tauri/src/lib.rs` — store init, forwarder, capability registration (modify)
- `src-tauri/src/commands.rs` — history commands, pass `Peer` to `start_send` (modify)
- `src-tauri/src/transfer/manager.rs` — `start_send(peer)` (modify)
- `src-tauri/src/transfer/sender.rs` — peer args + record (modify)
- `src-tauri/src/transfer/receiver.rs` — record (modify)
- `src-tauri/tauri.conf.json` — macOS overlay, min size, background (modify)

---

### Task 1: HTML shell meta

**Files:**
- Modify: `index.html`

- [ ] **Step 1: Replace `index.html`**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta
      name="viewport"
      content="width=device-width, initial-scale=1, viewport-fit=cover"
    />
    <meta name="color-scheme" content="light dark" />
    <meta name="theme-color" media="(prefers-color-scheme: light)" content="#f6f6f8" />
    <meta name="theme-color" media="(prefers-color-scheme: dark)" content="#0c0c0e" />
    <meta name="format-detection" content="telephone=no" />
    <meta name="apple-mobile-web-app-capable" content="yes" />
    <title>SendSent</title>
  </head>

  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 2: Verify the build still resolves the entry**

Run: `pnpm build`
Expected: PASS (tsc + vite build succeed).

- [ ] **Step 3: Commit**

```bash
git add index.html
git commit -m "feat(ui): app-store-ready HTML shell (viewport-fit, theme-color, title)"
```

---

### Task 2: Cross-platform CSS foundation

**Files:**
- Modify: `src/theme.css` (append at end, and add `color-scheme` to the token blocks)

- [ ] **Step 1: Add `color-scheme` to both theme roots**

In `src/theme.css`, inside `:root { ... }` add as the first line:
```css
  color-scheme: light;
```
Inside `:root.dark { ... }` add as the first line:
```css
  color-scheme: dark;
```
Inside the `@media (prefers-color-scheme: dark) { :root:not(.light):not(.dark) { ... } }` block add:
```css
    color-scheme: dark;
```

- [ ] **Step 2: Append the platform foundation block to `src/theme.css`**

```css
/* ============================================================
 * Cross-platform foundation
 * ============================================================ */

html,
body {
  overscroll-behavior: none;
  -webkit-tap-highlight-color: transparent;
  -webkit-text-size-adjust: 100%;
}

/* Text stays selectable where it matters; chrome does not. */
.app {
  -webkit-user-select: none;
  user-select: none;
}
input,
textarea,
.transfer-name,
.peer-name,
.page-subtitle {
  -webkit-user-select: text;
  user-select: text;
}

/* iOS momentum scrolling in scrollable panes. */
.main,
.peer-list,
.transfers {
  -webkit-overflow-scrolling: touch;
}

/* Mobile: keep inputs at 16px so iOS does not zoom on focus. */
.app.mobile input,
.app.mobile textarea,
.app.mobile select {
  font-size: 16px;
}

/* Touch targets on mobile. */
.app.mobile .nav-item,
.app.mobile .tabbar-item,
.app.mobile .peer-item,
.app.mobile button,
.app.mobile .btn {
  min-height: 44px;
}

/* Safe areas. Top bar + mobile tab bar + main content island. */
.topbar {
  padding-top: env(safe-area-inset-top, 0);
  height: calc(var(--topbar-h) + env(safe-area-inset-top, 0));
}
.app.mobile .main {
  padding-left: max(var(--s-4), env(safe-area-inset-left, 0));
  padding-right: max(var(--s-4), env(safe-area-inset-right, 0));
  padding-bottom: calc(var(--s-6) + var(--tabbar-h) + env(safe-area-inset-bottom, 0));
}
.app.mobile .tabbar {
  padding-bottom: env(safe-area-inset-bottom, 0);
  height: calc(var(--tabbar-h) + env(safe-area-inset-bottom, 0));
}

/* Only apply hover affordances on real pointers. */
@media (hover: none) {
  .peer-item:hover,
  .nav-item:hover,
  .icon-btn:hover,
  .btn:hover {
    background: inherit;
  }
}

/* Respect reduced motion. */
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
  }
}
```

- [ ] **Step 3: Verify**

Run: `pnpm build`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/theme.css
git commit -m "feat(ui): safe-area, touch, hover and reduced-motion foundations"
```

---

### Task 3: Platform root class + macOS overlay title bar

**Files:**
- Modify: `src/App.tsx:25` and `src/App.tsx:75`
- Modify: `src-tauri/tauri.conf.json:12-23`

- [ ] **Step 1: Expose `platform` from the hook in `App.tsx`**

Replace:
```tsx
  const { layout } = usePlatform();
```
with:
```tsx
  const { layout, platform } = usePlatform();
```

- [ ] **Step 2: Add the platform class and drag region**

Replace:
```tsx
    <div className={`app${isMobile ? " mobile" : ""}`}>
      {/* ── Top bar ──────────────────────────────────── */}
      <header className="topbar">
```
with:
```tsx
    <div className={`app ${platform}${isMobile ? " mobile" : ""}`}>
      {/* ── Top bar ──────────────────────────────────── */}
      <header className="topbar" data-tauri-drag-region>
```

- [ ] **Step 3: Add macOS traffic-light inset CSS**

Append to `src/theme.css`:
```css
/* macOS overlay title bar: leave room for the traffic lights. */
.app.macos .topbar {
  padding-left: 78px;
}
.app.macos .topbar .icon-btn,
.app.macos .topbar input,
.app.macos .topbar button {
  -webkit-app-region: no-drag;
}
```

- [ ] **Step 4: Configure macOS overlay in `tauri.conf.json`**

Replace the `app.windows` array:
```json
  "app": {
    "windows": [
      {
        "title": "SendSent",
        "width": 800,
        "height": 600,
        "minWidth": 480,
        "minHeight": 560,
        "backgroundColor": "#f6f6f8",
        "titleBarStyle": "Overlay",
        "hiddenTitle": true
      }
    ],
    "security": {
      "csp": null
    }
  },
```

- [ ] **Step 5: Verify desktop build launches**

Run: `pnpm tauri build` (or `pnpm tauri dev`, then quit)
Expected: window has no title text; top bar clears the traffic lights; app builds.

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src-tauri/tauri.conf.json
git commit -m "feat(ui): platform root class + macOS overlay title bar"
```

---

### Task 4: Rust `HistoryStore`

**Files:**
- Create: `src-tauri/src/history.rs`
- Modify: `src-tauri/src/lib.rs:1-9` (module declaration)

- [ ] **Step 1: Write `src-tauri/src/history.rs`**

```rust
//! Persistent transfer history, stored as JSON in the app data dir.

use crate::discovery::Platform;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_RECORDS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Send,
    Recv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStatus {
    Completed,
    Failed,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryFile {
    pub name: String,
    pub size: u64,
    pub rel_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub session_id: String,
    pub direction: Direction,
    pub peer_name: String,
    pub peer_platform: Platform,
    pub files: Vec<HistoryFile>,
    pub total_size: u64,
    pub bytes_done: u64,
    pub status: HistoryStatus,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub save_dir: Option<String>,
    pub error: Option<String>,
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
pub fn make_record(
    direction: Direction,
    session_id: &str,
    peer_name: &str,
    peer_platform: Platform,
    files: Vec<HistoryFile>,
    total_size: u64,
    bytes_done: u64,
    status: HistoryStatus,
    started_at_ms: i64,
    save_dir: Option<String>,
    error: Option<String>,
) -> HistoryRecord {
    HistoryRecord {
        session_id: session_id.to_string(),
        direction,
        peer_name: peer_name.to_string(),
        peer_platform,
        files,
        total_size,
        bytes_done,
        status,
        started_at_ms,
        ended_at_ms: now_ms(),
        save_dir,
        error,
    }
}

pub struct HistoryStore {
    path: PathBuf,
    records: Vec<HistoryRecord>,
}

impl HistoryStore {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("history.json");
        let records = match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<Vec<HistoryRecord>>(&s) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("history.json parse failed ({e}); starting empty");
                    let _ = std::fs::rename(&path, dir.join("history.json.bad"));
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        };
        let mut records = records;
        records.truncate(MAX_RECORDS);
        Self { path, records }
    }

    pub fn append(&mut self, record: HistoryRecord) {
        self.records.insert(0, record);
        self.records.truncate(MAX_RECORDS);
        self.persist();
    }

    pub fn list(&self) -> Vec<HistoryRecord> {
        self.records.clone()
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.persist();
    }

    fn persist(&self) {
        match serde_json::to_string_pretty(&self.records) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&self.path, s) {
                    tracing::warn!("history persist failed: {e}");
                }
            }
            Err(e) => tracing::warn!("history serialize failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn rec(id: &str) -> HistoryRecord {
        make_record(
            Direction::Recv, id, "peer", Platform::Ios,
            vec![HistoryFile { name: "a.bin".into(), size: 10, rel_path: "a.bin".into() }],
            10, 10, HistoryStatus::Completed, 1000, Some("/tmp/save".into()), None,
        )
    }

    fn tmp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("ss-hist-{}", Uuid::new_v4()))
    }

    #[test]
    fn append_is_newest_first_and_persists() {
        let dir = tmp_dir();
        let mut store = HistoryStore::load(&dir);
        store.append(rec("one"));
        store.append(rec("two"));
        assert_eq!(store.list()[0].session_id, "two");
        let reloaded = HistoryStore::load(&dir);
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[1].session_id, "one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn caps_at_max_records() {
        let dir = tmp_dir();
        let mut store = HistoryStore::load(&dir);
        for i in 0..(MAX_RECORDS + 5) {
            store.append(rec(&format!("id-{i}")));
        }
        assert_eq!(store.list().len(), MAX_RECORDS);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_empties_and_persists() {
        let dir = tmp_dir();
        let mut store = HistoryStore::load(&dir);
        store.append(rec("one"));
        store.clear();
        assert!(store.list().is_empty());
        assert!(HistoryStore::load(&dir).list().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_falls_back_to_empty() {
        let dir = tmp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("history.json"), b"{not json").unwrap();
        assert!(HistoryStore::load(&dir).list().is_empty());
        assert!(dir.join("history.json.bad").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: Declare the module in `lib.rs`**

In `src-tauri/src/lib.rs`, add after `pub mod events;`:
```rust
pub mod history;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib history 2>&1 | tail -15` (from `src-tauri/`)
Expected: 4 history tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/history.rs src-tauri/src/lib.rs
git commit -m "feat(history): persistent HistoryStore with JSON backing"
```

---

### Task 5: `Recorded` transfer event + store wiring

**Files:**
- Modify: `src-tauri/src/events.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs:110-159`

- [ ] **Step 1: Add the event name and variant**

In `src-tauri/src/events.rs`, inside `pub mod name { ... }` add:
```rust
    pub const TRANSFER_HISTORY: &str = "transfer://history";
```
Add a variant to `enum TransferEvent`:
```rust
    Recorded(crate::history::HistoryRecord),
```

- [ ] **Step 2: Add the store to `AppState`**

Replace `src-tauri/src/state.rs` with:
```rust
use crate::discovery::Discovery;
use crate::history::HistoryStore;
use crate::store::{Identity, TransferConfig};
use crate::transfer::manager::SessionManager;
use crate::transfer::tls::TlsConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub identity: Identity,
    pub identity_dir: PathBuf,
    pub discovery: Arc<dyn Discovery>,
    pub sessions: Arc<SessionManager>,
    pub save_dir: PathBuf,
    pub transfer_config: TransferConfig,
    pub tls_config: TlsConfig,
    pub history: Arc<Mutex<HistoryStore>>,
}
```

- [ ] **Step 3: Initialize the store and handle `Recorded` in `lib.rs`**

In `lib.rs`, before `let (ttx, mut trx) = ...` add:
```rust
            let history = Arc::new(tokio::sync::Mutex::new(crate::history::HistoryStore::load(&data_dir)));
```
Replace the forwarder `while let Some(ev) = trx.recv().await { ... }` body with:
```rust
                let history = history.clone();
                tauri::async_runtime::spawn(async move {
                    while let Some(ev) = trx.recv().await {
                        if let TransferEvent::Recorded(record) = &ev {
                            history.lock().await.append(record.clone());
                            let _ = h.emit(name::TRANSFER_HISTORY, serde_json::to_value(record).unwrap());
                            continue;
                        }
                        let (n, val) = match &ev {
                            TransferEvent::Request { .. } => {
                                (name::TRANSFER_REQUEST, serde_json::to_value(&ev).unwrap())
                            }
                            TransferEvent::Progress { .. } => {
                                (name::TRANSFER_PROGRESS, serde_json::to_value(&ev).unwrap())
                            }
                            TransferEvent::Finished { .. } => {
                                (name::TRANSFER_FINISHED, serde_json::to_value(&ev).unwrap())
                            }
                            TransferEvent::Recorded(_) => unreachable!(),
                        };
                        let _ = h.emit(n, val);
                        if let TransferEvent::Finished { session_id, .. } = &ev {
                            sessions.cleanup_session(*session_id).await;
                        }
                    }
                });
```
(Remove the original `let h = handle.clone();` duplication if present; keep one `h` and `sessions` clone in the block.)

Add `history` to `AppState`:
```rust
            app.manage(AppState { identity, identity_dir: data_dir, discovery, sessions, save_dir, transfer_config, tls_config, history });
```

- [ ] **Step 4: Verify compile**

Run: `cargo check` (from `src-tauri/`)
Expected: compiles. (The new commands are registered in Task 6.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/events.rs src-tauri/src/state.rs src-tauri/src/lib.rs
git commit -m "feat(history): Recorded event, AppState store and forwarder wiring"
```

---

### Task 6: History commands + peer-aware send

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/transfer/manager.rs`
- Modify: `src-tauri/src/transfer/sender.rs` (signature only; record in Task 8)
- Modify: `src-tauri/src/lib.rs:146-159`

- [ ] **Step 1: Add commands in `commands.rs`**

Append:
```rust
#[tauri::command]
pub async fn list_transfer_history(state: State<'_, AppState>) -> Result<Vec<crate::history::HistoryRecord>, String> {
    Ok(state.history.lock().await.list())
}

#[tauri::command]
pub async fn clear_transfer_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history.lock().await.clear();
    Ok(())
}
```
In `send_files`, replace `state.sessions.start_send(p.addrs, files, state.transfer_config.clone(), secure, verify)` with:
```rust
    state.sessions.start_send(p, files, state.transfer_config.clone(), secure, verify).map_err(|e| e.to_string())
```
In `send_text`, replace `state.sessions.start_send(p.addrs, vec![file], state.transfer_config.clone(), secure, verify)` with:
```rust
    state.sessions.start_send(p, vec![file], state.transfer_config.clone(), secure, verify).map_err(|e| e.to_string())
```

- [ ] **Step 2: Update `SessionManager::start_send` in `manager.rs`**

Replace `start_send` with:
```rust
    pub fn start_send(
        &self,
        peer: crate::discovery::Peer,
        files: Vec<String>,
        config: crate::store::TransferConfig,
        secure: bool,
        verify: bool,
    ) -> anyhow::Result<Uuid> {
        let session_id = Uuid::new_v4();
        let our = self.our.clone();
        let events = self.events_tx.clone();
        let id = session_id;
        let addrs = peer.addrs.clone();
        let peer_name = peer.name.clone();
        let peer_platform = peer.platform;
        tokio::spawn(async move {
            let _ = run_sender(id, addrs, files, our, peer_name, peer_platform, events, config, secure, verify).await;
        });
        Ok(session_id)
    }
```

- [ ] **Step 3: Update `run_sender`/`run_sender_inner` signatures**

In `src-tauri/src/transfer/sender.rs`, change the public signature to accept peer name/platform:
```rust
pub async fn run_sender(
    session_id: Uuid, peer_addrs: Vec<SocketAddr>, files: Vec<String>,
    our: Identity, peer_name: String, peer_platform: crate::discovery::Platform,
    events: mpsc::UnboundedSender<TransferEvent>,
    config: TransferConfig, secure: bool, verify: bool,
) -> Result<()> {
    let result = run_sender_inner(session_id, peer_addrs, files, our.clone(), peer_name, peer_platform, events.clone(), config, secure, verify).await;
    if let Err(e) = &result {
        tracing::error!("sender failed: {e}");
        let _ = events.send(TransferEvent::Finished { session_id, state: FinishedState::Failed,
            error: Some(ErrorPayload { code: ErrorCode::Internal, message: e.to_string() }) });
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn run_sender_inner(
    session_id: Uuid, peer_addrs: Vec<SocketAddr>, files: Vec<String>,
    our: Identity, peer_name: String, peer_platform: crate::discovery::Platform,
    events: mpsc::UnboundedSender<TransferEvent>,
    config: TransferConfig, secure: bool, verify: bool,
) -> Result<()> {
```
(Record emission is added in Task 8.)

- [ ] **Step 4: Register the new commands in `lib.rs`**

Add to `generate_handler!`:
```rust
            commands::list_transfer_history,
            commands::clear_transfer_history,
```

- [ ] **Step 5: Verify compile**

Run: `cargo check` (from `src-tauri/`)
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/transfer/manager.rs src-tauri/src/transfer/sender.rs src-tauri/src/lib.rs
git commit -m "feat(history): list/clear commands + send carries peer identity"
```

---

### Task 7: Receiver emits history records

**Files:**
- Modify: `src-tauri/src/transfer/receiver.rs`

- [ ] **Step 1: Capture context at the top of `run_receiver`**

After `let session_id = hello.session_id;` add:
```rust
    let started_at_ms = crate::history::now_ms();
    let peer_platform = peer_platform(hello.platform);
```
After the `manifest` is parsed (right after `let manifest: Manifest = ...;`) add:
```rust
    let hist_files: Vec<crate::history::HistoryFile> = manifest.files.iter()
        .filter(|f| f.kind == FileKind::File)
        .map(|f| crate::history::HistoryFile { name: f.name.clone(), size: f.size, rel_path: f.rel_path.clone() })
        .collect();
    let total_size = manifest.total_size;
```

- [ ] **Step 2: Emit a record on every terminal path**

Build the record inline with `crate::history::make_record(...)` (direction `Recv`, `session_id.to_string()`, `hello.name`, `peer_platform`, `hist_files.clone()`, `manifest.total_size`, bytes done, status, `started_at_ms`, save dir, error) and send `TransferEvent::Recorded(record)`. Insert at:

- after the `!decision.accept` `Finished` (status `Rejected`, bytes_done `0`, save_dir `Some(decision.save_dir.to_string_lossy().into_owned())`, error `None`).
- in the `build_drain_state` `Err(e)` branch (status `Failed`, bytes_done `0`, same save_dir, error `Some(e.to_string())`).
- after `state.writer.cleanup();` (status `Completed` when `all_ok` else `Failed`, bytes_done `done`, same save_dir, error `None`/`Some("finalize failed".into())`).
- in the `incomplete` else branch (status `Failed`, bytes_done `done`, same save_dir, error `Some("transfer incomplete".into())`).

For the two early protocol-error sites before the manifest is parsed (pin mismatch, bad first frame) build with `hist_files: vec![]`, `total_size: 0`, `save_dir: None`, bytes_done `0`, status `Failed`, error message. At the `decision_rx` `Err(_)` return there is no save dir, so use `save_dir: None`.

Example (rejected path):
```rust
        let _ = events.send(TransferEvent::Recorded(crate::history::make_record(
            crate::history::Direction::Recv, &session_id.to_string(), &hello.name, peer_platform,
            hist_files.clone(), manifest.total_size, 0, crate::history::HistoryStatus::Rejected,
            started_at_ms, Some(decision.save_dir.to_string_lossy().into_owned()), None,
        )));
```

- [ ] **Step 3: Verify compile + tests**

Run: `cargo check` then `cargo test --lib` (from `src-tauri/`)
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/transfer/receiver.rs
git commit -m "feat(history): receiver emits history records on all outcomes"
```

---

### Task 8: Sender emits history records

**Files:**
- Modify: `src-tauri/src/transfer/sender.rs`

- [ ] **Step 1: Capture context after the manifest is built**

After `let files_total = manifest.files.iter()...;` (sender.rs) add:
```rust
    let started_at_ms = crate::history::now_ms();
    let hist_files: Vec<crate::history::HistoryFile> = manifest.files.iter()
        .filter(|f| f.kind == FileKind::File)
        .map(|f| crate::history::HistoryFile { name: f.name.clone(), size: f.size, rel_path: f.name.clone() })
        .collect();
```

- [ ] **Step 2: Emit records on terminal paths**

At the two `Rejected` returns (secure and plain branches), immediately before each `return Ok(())`, add:
```rust
            let _ = events.send(TransferEvent::Recorded(crate::history::make_record(
                crate::history::Direction::Send, &session_id.to_string(), &peer_name, peer_platform,
                hist_files.clone(), manifest.total_size, 0, crate::history::HistoryStatus::Rejected,
                started_at_ms, None, None,
            )));
```
At the final `Finished` (after `final_state` is computed), before/after it add:
```rust
    let _ = events.send(TransferEvent::Recorded(crate::history::make_record(
        crate::history::Direction::Send, &session_id.to_string(), &peer_name, peer_platform,
        hist_files.clone(), manifest.total_size, done,
        if final_state == FinishedState::Completed { crate::history::HistoryStatus::Completed } else { crate::history::HistoryStatus::Failed },
        started_at_ms, None,
        if final_state == FinishedState::Failed { Some("no complete".into()) } else { None },
    )));
```

- [ ] **Step 3: Verify compile + tests**

Run: `cargo clippy --all-targets -- -D warnings` then `cargo test --lib` (from `src-tauri/`)
Expected: PASS, clippy clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/transfer/sender.rs
git commit -m "feat(history): sender emits history records on completion/rejection"
```

---

### Task 9: Frontend types, invoke and events

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/invoke.ts`
- Modify: `src/lib/events.ts`

- [ ] **Step 1: Add history types to `types.ts`**

Append:
```ts
export type TransferDirection = "send" | "recv";
export type HistoryStatus = "completed" | "failed" | "rejected" | "cancelled";
export interface HistoryFile { name: string; size: number; rel_path: string }
export interface HistoryRecord {
  session_id: string;
  direction: TransferDirection;
  peer_name: string;
  peer_platform: Platform;
  files: HistoryFile[];
  total_size: number;
  bytes_done: number;
  status: HistoryStatus;
  started_at_ms: number;
  ended_at_ms: number;
  save_dir: string | null;
  error: string | null;
}
```

- [ ] **Step 2: Add commands to `invoke.ts`**

Change the existing import line to:
```ts
import type { HistoryRecord, Identity, Peer } from "./types";
```
Append:
```ts
export const listTransferHistory = () => invoke<HistoryRecord[]>("list_transfer_history");
export const clearTransferHistory = () => invoke<void>("clear_transfer_history");
```

- [ ] **Step 3: Add the history listener to `events.ts`**

Change the existing import line to:
```ts
import type { HistoryRecord, Peer, TransferEvent } from "./types";
```
Append:
```ts
export function onHistoryRecord(cb: (r: HistoryRecord) => void): Promise<UnlistenFn> {
  return listen<HistoryRecord>("transfer://history", (e) => cb(e.payload));
}
```

- [ ] **Step 4: Verify**

Run: `pnpm build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/invoke.ts src/lib/events.ts
git commit -m "feat(history): frontend types, commands and event listener"
```

---

### Task 10: `useTransfer` history integration

**Files:**
- Modify: `src/hooks/useTransfer.ts`

- [ ] **Step 1: Track history and merge**

At the top of `useTransfer`, add state:
```ts
  const [history, setHistory] = useState<HistoryRecord[]>([]);
```
Add an effect that loads history and subscribes:
```ts
  useEffect(() => {
    let un: (() => void) | undefined;
    listTransferHistory().then(setHistory).catch(() => {});
    onHistoryRecord((r) => setHistory((cur) => [r, ...cur.filter((x) => x.session_id !== r.session_id)]));
    return () => { un?.(); };
  }, []);
```
Expose a merged list and clear function:
```ts
  const clearHistory = async () => {
    await clearTransferHistory();
    setHistory([]);
  };
  return { request, progress, history, clearHistory, clearRequest: () => setRequest(null) };
```
Update imports:
```ts
import { listTransferHistory, clearTransferHistory } from "../lib/invoke";
import { onHistoryRecord } from "../lib/events";
import type { HistoryRecord, Manifest } from "../lib/types";
```
(Replace the existing `import type { Manifest } from "../lib/types";` and note the unused `un` may be removed — keep only what is used. If `onHistoryRecord` returns an unlisten, wire it: `const un = await onHistoryRecord(...)` inside an async IIFE, mirroring the existing effect.)

Use this exact effect to avoid an unused variable:
```ts
  useEffect(() => {
    let un: (() => void) | undefined;
    (async () => {
      listTransferHistory().then(setHistory).catch(() => {});
      un = await onHistoryRecord((r) =>
        setHistory((cur) => [r, ...cur.filter((x) => x.session_id !== r.session_id)])
      );
    })();
    return () => { un?.(); };
  }, []);
```

- [ ] **Step 2: Verify**

Run: `pnpm build`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/hooks/useTransfer.ts
git commit -m "feat(history): load and subscribe to transfer history"
```

---

### Task 11: Transfers tab renders persisted history

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/TransferProgress.tsx`

- [ ] **Step 1: Add a history renderer to `TransferProgress.tsx`**

Append:
```tsx
import type { HistoryRecord } from "../lib/types";

function HistoryRow({ r }: { r: HistoryRecord }) {
  const names = r.files.map((f) => f.name);
  const statusText =
    r.status === "completed" ? "完成"
      : r.status === "rejected" ? "已拒绝"
      : r.status === "cancelled" ? "已取消"
      : "失败";
  const dirText = r.direction === "send" ? "发送" : "接收";
  return (
    <div className={`transfer ${r.status === "completed" ? "done" : "failed"} fade-in`}>
      <div className="transfer-head">
        <div className="transfer-icon">
          {r.status === "completed" ? <CheckIcon size={16} /> : <XIcon size={16} />}
        </div>
        <div className="transfer-info">
          <span className="transfer-name">
            {names.length <= 1 ? names[0] : `${names[0]} 等 ${names.length} 个文件`}
          </span>
          <div className="transfer-sub">
            {dirText} · {r.peer_name} · {fmtSize(r.bytes_done)} / {fmtSize(r.total_size)}
          </div>
        </div>
        <div className="transfer-pct">{statusText}</div>
      </div>
    </div>
  );
}

export function TransferHistory({ items, onClear }: { items: HistoryRecord[]; onClear: () => void }) {
  if (items.length === 0) {
    return <div className="card"><div className="transfer-empty">暂无历史记录</div></div>;
  }
  return (
    <div className="transfers">
      <div className="history-actions">
        <button className="btn btn-ghost" onClick={onClear}>清空记录</button>
      </div>
      {items.map((r) => <HistoryRow key={r.session_id} r={r} />)}
    </div>
  );
}
```

- [ ] **Step 2: Wire it in the transfers tab of `App.tsx`**

Update the hook destructure:
```tsx
  const { request, progress, history, clearHistory, clearRequest } = useTransfer();
```
In the `tab === "transfers"` section, after the `<section className="section"><TransferProgress items={transfers} /></section>`, add:
```tsx
                <section className="section">
                  <div className="section-title"><span>历史记录</span></div>
                  <TransferHistory items={history} onClear={() => { if (confirm("清空全部传输记录？")) clearHistory(); }} />
                </section>
```
Add the import:
```tsx
import { TransferProgress, TransferHistory } from "./components/TransferProgress";
```

- [ ] **Step 3: Verify**

Run: `pnpm build`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx src/components/TransferProgress.tsx
git commit -m "feat(ui): render persisted transfer history with clear action"
```

---

### Task 12: Save-location display + reveal

**Files:**
- Create: `src/components/SaveLocation.tsx`
- Modify: `src/components/Settings.tsx`
- Modify: `src/components/TransferProgress.tsx`

- [ ] **Step 1: Create `SaveLocation.tsx`**

```tsx
import { useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { getDefaultSaveDir } from "../lib/invoke";
import { usePlatform } from "../lib/platform";

export function SaveLocation() {
  const { platform } = usePlatform();
  const [dir, setDir] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    getDefaultSaveDir().then(setDir).catch((e) => setMsg(String(e)));
  }, []);

  const isMobile = platform === "ios" || platform === "android";

  return (
    <div className="field">
      <label className="field-label">保存位置</label>
      <div className="field-row">
        <input readOnly value={dir} />
        {!isMobile && (
          <button
            className="btn btn-primary"
            disabled={!dir}
            onClick={() => openPath(dir).catch((e) => setMsg(String(e)))}
          >
            打开文件夹
          </button>
        )}
      </div>
      <div className="field-hint">{isMobile ? "接收的文件保存在此目录" : "接收的文件默认保存于此"}</div>
      {msg && <div className="toast-msg error">{msg}</div>}
    </div>
  );
}
```

- [ ] **Step 2: Mount it in `Settings.tsx`**

Add the import and render it inside the returned `.col`, before the 关于 field:
```tsx
import { SaveLocation } from "./SaveLocation";
```
```tsx
      <SaveLocation />
```

- [ ] **Step 3: Add reveal to history rows**

In `TransferProgress.tsx`, add imports:
```tsx
import { revealItemInDir, openPath } from "@tauri-apps/plugin-opener";
import { usePlatform } from "../lib/platform";
```
Inside `HistoryRow`, add:
```tsx
  const { platform } = usePlatform();
  const isMobile = platform === "ios" || platform === "android";
  const canReveal = r.direction === "recv" && r.status === "completed" && !!r.save_dir && !isMobile;
  const reveal = () => {
    if (!r.save_dir) return;
    const first = r.files[0]?.rel_path;
    const target = first ? `${r.save_dir}/${first}` : r.save_dir;
    revealItemInDir(target).catch(() => openPath(r.save_dir!));
  };
```
Render below the head:
```tsx
      {canReveal && (
        <div className="transfer-meta">
          <button className="btn btn-ghost" onClick={reveal}>在文件夹中显示</button>
        </div>
      )}
```

- [ ] **Step 4: Verify**

Run: `pnpm build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/SaveLocation.tsx src/components/Settings.tsx src/components/TransferProgress.tsx
git commit -m "feat(ui): show save location and reveal received files"
```

---

### Task 13: Restrained visual polish + theme segmented control

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/theme.css`

- [ ] **Step 1: Replace the theme cycle button with a 3-state segmented control**

In `App.tsx`, replace the `topbar-actions` block with:
```tsx
        <div className="topbar-actions">
          <div className="segmented" role="group" aria-label="主题">
            {(["auto", "light", "dark"] as ThemePref[]).map((t) => (
              <button
                key={t}
                className={`segmented-item${theme === t ? " active" : ""}`}
                onClick={() => setTheme(t)}
                aria-pressed={theme === t}
              >
                {t === "auto" ? "系统" : t === "light" ? "浅色" : "深色"}
              </button>
            ))}
          </div>
        </div>
```

- [ ] **Step 2: Add segmented styles**

Append to `src/theme.css`:
```css
.segmented {
  display: inline-flex;
  padding: 2px;
  border-radius: var(--radius-full);
  background: var(--surface-hover);
  border: 1px solid var(--border);
}
.segmented-item {
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--fs-caption);
  padding: 4px 10px;
  border-radius: var(--radius-full);
  cursor: pointer;
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
}
.segmented-item.active {
  background: var(--surface);
  color: var(--text);
  box-shadow: var(--shadow-xs);
}
```

- [ ] **Step 3: Verify**

Run: `pnpm build` and `pnpm tauri dev` (visual check)
Expected: theme segments switch; auto follows system.

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx src/theme.css
git commit -m "feat(ui): segmented theme control and polish"
```

---

### Task 14: Full verification

- [ ] **Step 1: Rust**

Run (from `src-tauri/`):
```bash
cargo test --lib
cargo clippy --all-targets -- -D warnings
```
Expected: all lib tests pass; clippy clean. (The known-broken `v3_secure_transfer` integration test is unrelated.)

- [ ] **Step 2: Frontend**

Run: `pnpm build`
Expected: PASS.

- [ ] **Step 3: Desktop smoke test**

Run: `pnpm tauri dev`
Check: macOS overlay title bar; theme segmented control; receive a file → appears in 传输 历史; restart app → history persists; 设置 shows save dir; 在文件夹中显示 works; clear empties history.

- [ ] **Step 4: Mobile smoke test (optional, if devices available)**

Build + deploy per `docs/BUILD.md`; verify safe areas, tab bar, and history persistence on iOS/Android.

- [ ] **Step 5: Final commit (if any fixes)**

```bash
git add -A
git commit -m "chore: verification fixes for cross-platform UI + history"
```
