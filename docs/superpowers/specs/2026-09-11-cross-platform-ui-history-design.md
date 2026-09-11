# 跨平台 UI 优化 + 传输历史持久化 · 设计

日期：2026-09-11
状态：已评审（待用户复核）

## 背景

SendSent（Tauri 2 + React 19）已能在 macOS / iOS / Android / Windows / Linux 运行，但前端存在三类问题：

1. **平台适配不足**：`index.html` 缺 `viewport-fit=cover`，iOS 安全区失效；无 macOS 融栏；触控目标偏小；无 `hover`/`reduced-motion` 降级；输入框在 iOS 会聚焦缩放。
2. **传输历史不持久**：`useTransfer` 的进度只存在 React 内存，重启即丢；后端 `TransferEvent::Finished` 不携带方向/对端/文件名/保存路径。
3. **找不到保存位置**：接收的文件落在 `state.save_dir`，但 UI 无任何入口展示或打开；`get_default_save_dir` 命令与 `opener` 能力都已就绪却未被使用。

## 目标

- 让 UI 在五大平台上原生、舒适、一致（克制的视觉提升 + 平台适配）。
- 传输历史跨重启持久化，可查看、可清空。
- 用户能看到并打开文件保存位置。

## 非目标（YAGNI）

- 不做全平台自定义无边框标题栏（Windows/Linux 保留原生装饰）。
- 不改保存目录（不提供自定义/接收时选择）。
- 不做拖拽发送、通知、云同步。
- 不改动传输线协议。

## 决策摘要

| 议题 | 决策 |
| --- | --- |
| 视觉力度 | B：明显提升但克制，保留现有设计令牌与结构 |
| 历史持久化 | 后端 JSON（`<app_data_dir>/history.json`） |
| 保存位置 | 展示 + 打开（不自定义） |
| 窗口外观 | macOS 融栏（Overlay），其余原生标题栏，移动端无标题栏 |

---

## 1. 跨平台适配

### 1.1 `index.html`

- `viewport`：`width=device-width, initial-scale=1, viewport-fit=cover`。
- `<meta name="color-scheme" content="light dark">`、`<meta name="theme-color" media="(prefers-color-scheme: light|dark)">`、`<meta name="format-detection" content="telephone=no">`。
- `<title>SendSent</title>`。

### 1.2 `theme.css`

- `:root { color-scheme: light }` / `:root.dark { color-scheme: dark }`，使原生控件/滚动条随主题。
- 安全区：顶栏 `padding-top: env(safe-area-inset-top)`；移动端底栏 `padding-bottom: env(safe-area-inset-bottom)`；主内容区左右 `env(safe-area-inset-left|right)`。
- 触控与滚动：`-webkit-tap-highlight-color: transparent`；`overscroll-behavior: none`；`-webkit-overflow-scrolling: touch`；移动端 `input/textarea/select { font-size: 16px }` 防聚焦缩放。
- 交互态：hover 规则包进 `@media (hover: hover)`；`@media (prefers-reduced-motion: reduce)` 关闭非必要动画。
- 触控目标：`.nav-item`/`.tabbar-item`/按钮/`.peer-item` 在移动端 ≥44px。
- 字体栈补 `"Noto Sans SC"`, `"Source Han Sans SC"`（Linux）。
- 根节点平台 class：`.app` 追加 `macos|windows|linux|ios|android`，供平台分支。

### 1.3 平台分支

- **macOS**：`tauri.conf.json` → `app.windows[0].titleBarStyle = "Overlay"`、`hiddenTitle = true`。顶栏加 `data-tauri-drag-region`，左侧留出红绿灯内边距（`.app.macos .topbar`）。交互元素标 `data-tauri-drag-region` 排除。
- **Windows/Linux**：保持原生标题栏；仅通过 `color-scheme` 适配深浅色。
- **iOS/Android**：无标题栏；安全区 + 底部 tab 栏。
- 桌面设 `minWidth: 480`、`minHeight: 560`；移动端全屏。
- 启动背景色与主题一致，避免白闪（`backgroundColor`）。

### 1.4 平台探测

`usePlatform()` 已可返回 `platform`；在 `App.tsx` 把 `platform` 加入根 class（当前仅有 `mobile`）。

---

## 2. 传输历史持久化

### 2.1 数据模型（Rust）

```rust
pub enum Direction { Send, Recv }
pub enum HistoryStatus { Completed, Failed, Rejected, Cancelled }

pub struct HistoryFile { pub name: String, pub size: u64, pub rel_path: String }

pub struct HistoryRecord {
    pub session_id: String,
    pub direction: Direction,
    pub peer_name: String,
    pub peer_platform: String,
    pub files: Vec<HistoryFile>,
    pub total_size: u64,
    pub bytes_done: u64,
    pub status: HistoryStatus,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub save_dir: Option<String>,
    pub error: Option<String>,
}
```

> `rel_path` 仅接收方向用于 reveal（相对 `save_dir`）；发送方向置为与 `name` 相同。

### 2.2 `HistoryStore`（新模块 `src-tauri/src/history.rs`）

- 内存 `Vec<HistoryRecord>`（新在前），落盘 `<app_data_dir>/history.json`。
- 上限 `MAX = 200`，超出裁剪最旧。
- API：`load(dir)`、`append(record)`（写盘）、`list()`、`clear()`（写盘）。
- 落盘失败仅记日志，不 panic；读取损坏文件回退为空表并备份为 `history.json.bad`。
- 并发：`Arc<Mutex<HistoryStore>>` 存入 `AppState`。

### 2.3 事件与数据来源

- 给 `TransferEvent::Finished` 增加 `record: Option<HistoryRecord>`（由 sender/receiver 在结束时构造并填充）。
  - **receiver**：已有 `Hello`(对端 name/platform)、`Manifest`(files)、`Decision.save_dir`；`save_dir.join(rel_path)` 得绝对路径用于 reveal。
  - **sender**：`send_files`/`send_text` 命令解析出目标 `Peer p`，把 `name/platform` 传入 `start_send`；结束时用 manifest/files 构造。
- `lib.rs` 的事件转发器：收到带 `record` 的 `Finished` 时 `history.append(...)`，并 emit `transfer://history`（payload 为该 record）。
- 崩溃/强杀导致未完成会话：不落盘（仅在 Finished 落盘）。

### 2.4 命令

- `list_transfer_history() -> Vec<HistoryRecord>`
- `clear_transfer_history() -> ()`

### 2.5 前端

- `src/lib/types.ts`：新增 `HistoryRecord` 等类型。
- `src/lib/invoke.ts`：`listTransferHistory`、`clearTransferHistory`。
- `src/lib/events.ts`：订阅 `transfer://history`。
- `useTransfer`：
  - 挂载时拉取历史，订阅新 record 追加。
  - “传输”页数据 = **持久化历史** ∪ **进行中进度**，按 `session_id` 去重（进行中优先并实时更新；完成后由历史记录接管）。
  - 新会话在 `Progress` 到达时若不在历史中，也即时显示。
- `TransferProgress`：展示方向（发送/接收）、状态、耗时、大小、文件名；已接收成功记录带「在文件夹中显示」。
- “传输”页增加「清空记录」（二次确认）。

---

## 3. 保存位置 UX

- **设置页**：新增「保存位置」行，显示 `getDefaultSaveDir()`；桌面端加「打开文件夹」按钮（`openPath(dir)`）。
- **传输行**（接收、`status == Completed` 且有 `save_dir`）：
  - 桌面：`revealItemInDir(<save_dir>/<第一个文件 rel_path>)`；失败回退 `openPath(save_dir)`。
  - 移动端：仅展示路径文字（系统不支持 reveal）。
- 平台判定用 `usePlatform().platform`。

---

## 4. 克制的视觉提升

- 统一间距/字号标尺的落地（复用现有 token，微调层级：标题、分组、次级文本）。
- 动效：页面/区块进入淡入、列表 stagger（已有基础上统一节奏）、进度条宽度/颜色平滑过渡；`prefers-reduced-motion` 下禁用。
- 组件质感：卡片边框/阴影层级统一；按钮 primary/ghost/danger 明确；输入聚焦环；Toast 规范化。
- 主题切换改为三态分段控件（跟随系统/浅色/深色），显示当前生效项。
- 空态/加载态：发现中、无传输、无设备等文案与视觉统一。
- 无障碍：可见焦点（`:focus-visible`）、对比度 AA、触控目标尺寸。

---

## 事件 / 命令清单（新增或变更）

| 名称 | 类型 | 说明 |
| --- | --- | --- |
| `transfer://history` | 事件（新增） | 新增一条历史记录时推送 |
| `TransferEvent::Finished.record` | 结构变更 | 追加可选历史记录概要 |
| `list_transfer_history` | 命令（新增） | 返回全部历史 |
| `clear_transfer_history` | 命令（新增） | 清空历史 |
| `get_default_save_dir` | 命令（已存在） | 前端首次使用 |

## 平台矩阵

| 能力 | macOS | Windows | Linux | iOS | Android |
| --- | --- | --- | --- | --- | --- |
| 标题栏 | Overlay 融栏 | 原生 | 原生 | 无 | 无 |
| 安全区 | 顶部(菜单/刘海) | — | — | 全 | 全 |
| 布局 | sidebar | sidebar | sidebar | tabbar | tabbar |
| 打开文件夹 | ✅ | ✅ | ✅ | 仅路径 | 仅路径 |
| 历史持久化 | ✅ | ✅ | ✅ | ✅ | ✅ |

## 错误处理

- 历史文件读写失败：日志 + 空表回退，不影响传输主流程。
- `revealItemInDir`/`openPath` 失败：捕获并提示，不崩溃。
- 历史 record 缺失（老事件/异常）：跳过持久化，UI 仍显示进行中。

## 测试与验证

- **Rust 单测**（`history.rs`）：append 后 list 顺序；超过 200 裁剪；写盘后重新 load 一致；clear 清空并落盘；损坏文件回退。
- `cargo test`（注意既有 `v3_secure_transfer` 已知失败，与本改动无关）。
- `cargo clippy --all-targets -- -D warnings` 干净。
- `pnpm build` 通过。
- **手动**：macOS 融栏与红绿灯、Windows/Linux 原生标题栏、iOS 安全区与 tabbar、Android 返回键与安全区；接收完成后 reveal 文件夹；重启历史仍在；清空历史。

## 风险

- `titleBarStyle: Overlay` 在部分 macOS 版本与 `decorations` 组合需实测；交互区域需正确标注 drag region，避免按钮被拖拽拦截。
- 移动端 `openPath` 支持有限，故仅展示路径。
- 历史记录依赖 sender/receiver 正确填充 `record`，需保证所有失败/取消路径都带 record。
