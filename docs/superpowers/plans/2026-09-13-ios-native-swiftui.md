# iOS 原生客户端 — Plan 2: SwiftUI 界面

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用原生 SwiftUI 实现 iOS 界面（设备 / 传输 / 我的 / 设置 + 发送 / 接收 / 扫码 / 二维码），通过 Plan 1 的 C FFI 调用 Rust 核心，彻底替代 WKWebView。

**Architecture:** 单例 `Core: ObservableObject`（@MainActor）包装 FFI：`sendsent_ios_init` 传入一个全局 `@convention(c)` 回调；回调把 JSON 交给主线程的 `handle(json)`，按 `kind` 更新 `@Published`（peers/progress/history/request/…）。视图全用系统组件（`NavigationStack`/`List`/`Form`/`.alert`/`.sheet`/`.fileImporter`/`AVFoundation`）。

**Tech Stack:** SwiftUI(iOS 17)、Foundation、AVFoundation、UniformTypeIdentifiers；Rust FFI（Plan 1）。

**前置：** Plan 1 已合并（`src-tauri/src/ffi.rs` 导出全部 `sendsent_ios_*`；`gen/apple` 已解耦，`Sources/sendsent/` 为空，`main.mm`/`Picker.swift` 已删除）。**Plan 2 完成前 iOS app target 无入口**，属预期。

---

## 文件结构（`src-tauri/gen/apple/Sources/sendsent/`）

- `SendSentApp.swift` — `@main App` + `RootView`（`TabView`）。
- `FFI.swift` — `@_silgen_name` 声明 + `decodeResult`/`throwIfError` + 便捷 API。
- `Models.swift` — Codable 模型（对齐 Rust serde）。
- `Core.swift` — `ObservableObject` 桥 + 事件分发 + 业务方法。
- `Views/DevicesView.swift`、`TransfersView.swift`、`ProfileView.swift`、`SettingsView.swift`、`AddDeviceView.swift`、`ScanView.swift`、`IncomingRequestView.swift`。
- 修改 `gen/apple/project.yml`（可选：无需，`sources: Sources` 自动纳入新文件；若加 `NSMicrophoneUsageDescription` 等再改）。

---

### Task 1: FFI 桥 + 模型 + 最小 App（跑通链接）

**Files:**
- Create: `Sources/sendsent/FFI.swift`
- Create: `Sources/sendsent/Models.swift`
- Create: `Sources/sendsent/SendSentApp.swift`
- Modify: `gen/apple/project.yml`（`sources` 已含 `Sources`，通常无需改）

- [ ] **Step 1: `Models.swift`**

```swift
import Foundation

struct Identity: Codable, Identifiable {
    let device_id: String
    let name: String
    let platform: String
    var id: String { device_id }
}

struct Peer: Codable, Identifiable, Hashable {
    let device_id: String
    let name: String
    let platform: String
    let proto_version: UInt16
    let addrs: [String]
    let port: UInt16
    var id: String { device_id }
}

struct MyAddress: Codable, Identifiable {
    let interface: String
    let ip: String
    var id: String { "\(interface)-\(ip)" }
}

struct TransferConfig: Codable {
    let conns: UInt32
    let chunk_size: UInt64
    let split_threshold: UInt64
}

// 事件载荷（对齐 Rust serde；内部 tag = kind）
struct KindOnly: Decodable { let kind: String }
struct PeerFoundEvent: Decodable { let peer: Peer }
struct PeerLostEvent: Decodable { let device_id: String }

struct FileMeta: Decodable {
    let name: String
    let rel_path: String
    let size: UInt64
}
struct Manifest: Decodable {
    let session_id: String
    let files: [FileMeta]
    let total_size: UInt64
    let total_count: UInt64
}
struct TransferRequest: Decodable, Identifiable {
    let session_id: String
    let sender: Peer
    let manifest: Manifest
    var id: String { session_id }
}
struct TransferProgress: Decodable, Identifiable {
    let session_id: String
    let state: String
    let bytes_done: UInt64
    let bytes_total: UInt64
    let files_done: UInt64
    let files_total: UInt64
    let speed_bps: UInt64
    var id: String { session_id }
    var fraction: Double { bytes_total == 0 ? 0 : Double(bytes_done) / Double(bytes_total) }
}
struct ErrorPayload: Decodable { let code: Int; let message: String }
struct TransferFinished: Decodable {
    let session_id: String
    let state: String
    let error: ErrorPayload?
}

struct HistoryFile: Decodable { let name: String; let size: UInt64; let rel_path: String }
struct HistoryRecord: Decodable, Identifiable {
    let session_id: String
    let direction: String
    let peer_name: String
    let files: [HistoryFile]
    let total_size: UInt64
    let bytes_done: UInt64
    let status: String
    let started_at_ms: Int64
    let ended_at_ms: Int64
    let save_dir: String?
    let error: String?
    var id: String { session_id }
}
```

- [ ] **Step 2: `FFI.swift`**

```swift
import Foundation

typealias EventCallback = @convention(c) (UnsafePointer<CChar>?) -> Void

@_silgen_name("sendsent_ios_init")
func sendsent_ios_init(_ dataDir: UnsafePointer<CChar>?, _ saveDir: UnsafePointer<CChar>?, _ port: UInt16, _ cb: EventCallback?) -> Int32
@_silgen_name("sendsent_ios_free_string")
func sendsent_ios_free_string(_ ptr: UnsafeMutablePointer<CChar>?)
@_silgen_name("sendsent_ios_identity")
func sendsent_ios_identity() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_peers")
func sendsent_ios_peers() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_addresses")
func sendsent_ios_addresses() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_qr")
func sendsent_ios_qr(_ size: UInt32, _ ip: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_add_peer")
func sendsent_ios_add_peer(_ addr: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_send")
func sendsent_ios_send(_ peerId: UnsafePointer<CChar>?, _ filesJson: UnsafePointer<CChar>?, _ secure: Bool, _ verify: Bool) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_respond")
func sendsent_ios_respond(_ sessionId: UnsafePointer<CChar>?, _ accept: Bool) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_history")
func sendsent_ios_history() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_clear_history")
func sendsent_ios_clear_history() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_get_transfer_config")
func sendsent_ios_get_transfer_config() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_set_transfer_config")
func sendsent_ios_set_transfer_config(_ conns: UInt32, _ chunkKb: UInt64, _ splitMb: UInt64) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_set_display_name")
func sendsent_ios_set_display_name(_ name: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

enum FFIError: Error, LocalizedError {
    case message(String)
    var errorDescription: String? { if case let .message(m) = self { return m }; return nil }
}

/// 解出 err-json 抛错；否则返回原始字符串（并释放指针）。
private func take(_ raw: UnsafeMutablePointer<CChar>?) throws -> String? {
    guard let p = raw else { return nil }
    defer { sendsent_ios_free_string(p) }
    let s = String(cString: p)
    if let d = s.data(using: .utf8),
       let obj = try? JSONSerialization.jsonObject(with: d) as? [String: Any],
       let err = obj["error"] as? String {
        throw FFIError.message(err)
    }
    return s
}

func decodeResult<T: Decodable>(_ type: T.Type, _ raw: UnsafeMutablePointer<CChar>?) throws -> T? {
    guard let s = try take(raw) else { return nil }
    return try JSONDecoder().decode(T.self, from: Data(s.utf8))
}

func throwIfError(_ raw: UnsafeMutablePointer<CChar>?) throws {
    _ = try take(raw)
}

func ffiIdentity() throws -> Identity? { try decodeResult(Identity.self, sendsent_ios_identity()) }
func ffiPeers() throws -> [Peer] { try decodeResult([Peer].self, sendsent_ios_peers()) ?? [] }
func ffiAddresses() throws -> [MyAddress] { try decodeResult([MyAddress].self, sendsent_ios_addresses()) ?? [] }
func ffiHistory() throws -> [HistoryRecord] { try decodeResult([HistoryRecord].self, sendsent_ios_history()) ?? [] }
func ffiConfig() throws -> TransferConfig? { try decodeResult(TransferConfig.self, sendsent_ios_get_transfer_config()) }

func ffiQr(size: UInt32 = 512, ip: String? = nil) throws -> String {
    let raw: UnsafeMutablePointer<CChar>?
    if let ip { raw = ip.withCString { sendsent_ios_qr(size, $0) } }
    else { raw = sendsent_ios_qr(size, nil) }
    return try decodeResult(String.self, raw) ?? ""
}

func ffiAddPeer(_ addr: String) throws {
    try addr.withCString { try throwIfError(sendsent_ios_add_peer($0)) }
}

func ffiSend(peerId: String, files: [String], secure: Bool, verify: Bool) throws -> String {
    let filesJson = String(data: try JSONEncoder().encode(files), encoding: .utf8) ?? "[]"
    let raw = peerId.withCString { pid in filesJson.withCString { fj in sendsent_ios_send(pid, fj, secure, verify) } }
    let r: [String: String]? = try decodeResult([String: String].self, raw)
    guard let sid = r?["session_id"] else { throw FFIError.message("send failed") }
    return sid
}

func ffiRespond(sessionId: String, accept: Bool) throws {
    try sessionId.withCString { try throwIfError(sendsent_ios_respond($0, accept)) }
}
func ffiClearHistory() throws { try throwIfError(sendsent_ios_clear_history()) }
func ffiSetConfig(conns: UInt32, chunkKb: UInt64, splitMb: UInt64) throws {
    try throwIfError(sendsent_ios_set_transfer_config(conns, chunkKb, splitMb))
}
func ffiSetDisplayName(_ name: String) throws {
    try name.withCString { try throwIfError(sendsent_ios_set_display_name($0)) }
}
```

- [ ] **Step 3: `SendSentApp.swift`（最小可跑）**

```swift
import SwiftUI

@main
struct SendSentApp: App {
    @StateObject private var core = Core.shared
    var body: some Scene {
        WindowGroup {
            RootView().environmentObject(core)
        }
    }
}

struct RootView: View {
    @EnvironmentObject var core: Core
    var body: some View {
        Text(core.identity?.name ?? "SendSent")
            .padding()
    }
}
```

- [ ] **Step 4: 构建并运行（验证 FFI 链接 + 启动）**

Run:
```bash
cd src-tauri/gen/apple && xcodegen generate
cd /Users/mankong/volumes/code/mac-rs/SendSent && pnpm tauri ios build -t aarch64 -c '{"build":{"beforeBuildCommand":""}}'
```
Expected: `BUILD SUCCEEDED`（Rust 静态库由 preBuild 阶段用 cargo 构建；Swift 链接到 `sendsent_ios_*`）。

- [ ] **Step 5: 安装启动**

```bash
DEV=00008140-000849C60CD2801C
rm -rf /tmp/ss-app && mkdir -p /tmp/ss-app
unzip -q src-tauri/gen/apple/build/arm64/sendsent.ipa -d /tmp/ss-app
xcrun devicectl device install app --device "$DEV" /tmp/ss-app/Payload/sendsent.app
xcrun devicectl device process launch --device "$DEV" com.mankong.sendsent
```
Expected: app 启动，显示本机名（或 SendSent）。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/gen/apple/Sources/sendsent
git commit -m "feat(ios): swiftui ffi bridge + models + minimal app"
```

---

### Task 2: `Core`（ObservableObject + 事件分发）

**Files:**
- Create: `Sources/sendsent/Core.swift`
- Modify: `Sources/sendsent/SendSentApp.swift`（RootView 用 Core）

- [ ] **Step 1: `Core.swift`**

全局回调 trampoline（`@convention(c)` 不能捕获上下文）：

```swift
import Foundation
import SwiftUI

private var eventSink: ((String) -> Void)?

private func sendsentEventTrampoline(_ ptr: UnsafePointer<CChar>?) {
    guard let ptr else { return }
    let json = String(cString: ptr)
    eventSink?(json)
}

@MainActor
final class Core: ObservableObject {
    static let shared = Core()

    @Published var peers: [Peer] = []
    @Published var progress: [String: TransferProgress] = [:]
    @Published var history: [HistoryRecord] = []
    @Published var request: TransferRequest?
    @Published var identity: Identity?
    @Published var addresses: [MyAddress] = []
    @Published var toast: String?
    @Published var ready = false

    let port: UInt16 = 52225
    let saveDir: URL

    private init() {
        let fm = FileManager.default
        let docs = fm.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let support = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        saveDir = docs.appendingPathComponent("sendsent", isDirectory: true)
        let dataDir = support.appendingPathComponent("sendsent", isDirectory: true)
        try? fm.createDirectory(at: saveDir, withIntermediateDirectories: true)
        try? fm.createDirectory(at: dataDir, withIntermediateDirectories: true)

        eventSink = { [weak self] json in
            Task { @MainActor in self?.handle(json) }
        }
        let rc = dataDir.path.withCString { d in
            saveDir.path.withCString { s in
                sendsent_ios_init(d, s, port, sendsentEventTrampoline)
            }
        }
        ready = rc == 0
        if rc != 0 { toast = "初始化失败 (\(rc))" }
        if ready { reloadAll() }
    }

    func reloadAll() {
        reloadIdentity(); reloadPeers(); reloadAddresses(); reloadHistory()
    }
    func reloadIdentity() { identity = try? ffiIdentity() }
    func reloadPeers() { peers = (try? ffiPeers()) ?? [] }
    func reloadAddresses() { addresses = (try? ffiAddresses()) ?? [] }
    func reloadHistory() { history = (try? ffiHistory()) ?? [] }

    func handle(_ json: String) {
        guard let data = json.data(using: .utf8),
              let k = try? JSONDecoder().decode(KindOnly.self, from: data) else { return }
        switch k.kind {
        case "peer_found":
            if let e = try? JSONDecoder().decode(PeerFoundEvent.self, from: data),
               !peers.contains(where: { $0.device_id == e.peer.device_id }) {
                peers.append(e.peer)
            }
        case "peer_lost":
            if let e = try? JSONDecoder().decode(PeerLostEvent.self, from: data) {
                peers.removeAll { $0.device_id == e.device_id }
            }
        case "request":
            request = try? JSONDecoder().decode(TransferRequest.self, from: data)
        case "progress":
            if let e = try? JSONDecoder().decode(TransferProgress.self, from: data) {
                progress[e.session_id] = e
            }
        case "finished":
            if let e = try? JSONDecoder().decode(TransferFinished.self, from: data) {
                progress.removeValue(forKey: e.session_id)
                reloadHistory()
            }
        case "recorded":
            if let e = try? JSONDecoder().decode(HistoryRecord.self, from: data) {
                history.removeAll { $0.session_id == e.session_id }
                history.insert(e, at: 0)
            }
        default: break
        }
    }

    func flash(_ text: String) {
        toast = text
        Task { try? await Task.sleep(nanoseconds: 2_400_000_000); if toast == text { toast = nil } }
    }

    func send(peer: Peer, files: [URL], secure: Bool, verify: Bool) {
        let paths = files.map(\.path)
        do {
            _ = try ffiSend(peerId: peer.device_id, files: paths, secure: secure, verify: verify)
            flash("已发送到 \(peer.name)")
        } catch { flash("发送失败: \(error.localizedDescription)") }
    }
    func respond(accept: Bool) {
        guard let sid = request?.session_id else { return }
        do { try ffiRespond(sessionId: sid, accept: accept) }
        catch { flash("响应失败: \(error.localizedDescription)") }
        request = nil
    }
    func addPeer(_ addr: String) {
        do { try ffiAddPeer(addr); flash("已添加 \(addr)") }
        catch { flash("添加失败: \(error.localizedDescription)") }
    }
    func clearHistory() {
        do { try ffiClearHistory(); history = [] } catch { flash("清空失败") }
    }
    func rename(_ name: String) { try? ffiSetDisplayName(name); reloadIdentity() }
    func setConfig(conns: UInt32, chunkKb: UInt64, splitMb: UInt64) {
        do { try ffiSetConfig(conns: conns, chunkKb: chunkKb, splitMb: splitMb); flash("已保存") }
        catch { flash("保存失败: \(error.localizedDescription)") }
    }
}
```

- [ ] **Step 2: 更新 `RootView` 为 TabView（占位页）**

```swift
struct RootView: View {
    @EnvironmentObject var core: Core
    var body: some View {
        TabView {
            NavigationStack { Text("设备") .navigationTitle("设备") }
                .tabItem { Label("设备", systemImage: "dot.radiowaves.left.and.right") }
            NavigationStack { Text("传输") .navigationTitle("传输") }
                .tabItem { Label("传输", systemImage: "arrow.left.arrow.right") }
            NavigationStack { Text("我的") .navigationTitle("我的") }
                .tabItem { Label("我的", systemImage: "person.crop.circle") }
            NavigationStack { Text("设置") .navigationTitle("设置") }
                .tabItem { Label("设置", systemImage: "gearshape") }
        }
        .tint(.accentColor)
        .alert("提示", isPresented: Binding(get: { core.toast != nil }, set: { if !$0 { core.toast = nil } })) {
            Button("好", role: .cancel) {}
        } message: { Text(core.toast ?? "") }
    }
}
```

- [ ] **Step 3: 构建 + 安装（脚本同 Task 1 Step 4/5）**

Expected: 4 个 tab 可切换；无崩溃。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/gen/apple/Sources/sendsent
git commit -m "feat(ios): core observable object with ffi event routing"
```

---

### Task 3: 设备页 + 发送（`.fileImporter`）

**Files:**
- Create: `Sources/sendsent/Views/DevicesView.swift`
- Modify: `SendSentApp.swift`（用 `DevicesView`）

- [ ] **Step 1: `DevicesView.swift`**

```swift
import SwiftUI

struct DevicesView: View {
    @EnvironmentObject var core: Core
    @State private var selected = Set<String>()
    @State private var search = ""
    @State private var showImporter = false
    @State private var secure = false
    @State private var verify = false
    @State private var showAdd = false

    private var filtered: [Peer] {
        let k = search.trimmingCharacters(in: .whitespaces).lowercased()
        return k.isEmpty ? core.peers : core.peers.filter { $0.name.lowercased().contains(k) }
    }
    private var selectedPeers: [Peer] { core.peers.filter { selected.contains($0.device_id) } }

    var body: some View {
        List {
            ForEach(filtered) { peer in
                Button { toggle(peer) } label: { row(peer) }
                    .buttonStyle(.plain)
            }
        }
        .searchable(text: $search, prompt: "搜索设备")
        .overlay {
            if core.peers.isEmpty {
                ContentUnavailableView("正在发现附近设备…", systemImage: "dot.radiowaves.left.and.right",
                                       description: Text("确保设备在同一局域网,或点右上角 + 手动添加"))
            }
        }
        .navigationTitle(peerTitle)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button { showAdd = true } label: { Image(systemName: "plus") }
            }
        }
        .safeAreaInset(edge: .bottom) {
            if !selected.isEmpty { sendBar }
        }
        .fileImporter(isPresented: $showImporter,
                      allowedContentTypes: [.data, .image, .movie, .audio, .pdf, .text, .archive],
                      allowsMultipleSelection: true) { result in
            if case let .success(urls) = result { send(urls) }
        }
        .sheet(isPresented: $showAdd) { NavigationStack { AddDeviceView() } }
    }

    private var peerTitle: String {
        if selected.isEmpty { return "设备" }
        return selected.count == 1 ? (selectedPeers.first?.name ?? "设备") : "已选 \(selected.count) 台"
    }

    private func row(_ peer: Peer) -> some View {
        HStack(spacing: 12) {
            PlatformAvatar(peer: peer)
            VStack(alignment: .leading, spacing: 2) {
                Text(peer.name).font(.body)
                Text("\(peer.platform) · \(peer.addrs.first ?? ":\(peer.port)")")
                    .font(.footnote).foregroundStyle(.secondary)
            }
            Spacer()
            Image(systemName: selected.contains(peer.device_id) ? "checkmark.circle.fill" : "circle")
                .foregroundStyle(selected.contains(peer.device_id) ? Color.accentColor : Color.secondary.opacity(0.5))
                .imageScale(.large)
        }
    }

    private var sendBar: some View {
        VStack(spacing: 8) {
            HStack {
                Toggle("加密传输", isOn: $secure)
                Toggle("SHA-256", isOn: $verify)
            }
            .font(.footnote)
            .toggleStyle(.switch)
            Button {
                showImporter = true
            } label: {
                Label("选择文件发送", systemImage: "paperplane.fill")
                    .frame(maxWidth: .infinity).padding(.vertical, 6)
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
        .background(.bar)
    }

    private func toggle(_ peer: Peer) {
        if selected.contains(peer.device_id) { selected.remove(peer.device_id) }
        else { selected.insert(peer.device_id) }
    }

    private func send(_ urls: [URL]) {
        var accessed: [URL] = []
        for u in urls where u.startAccessingSecurityScopedResource() { accessed.append(u) }
        for p in selectedPeers { core.send(peer: p, files: urls, secure: secure, verify: verify) }
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) { accessed.forEach { $0.stopAccessingSecurityScopedResource() } }
        selected.removeAll()
    }
}

struct PlatformAvatar: View {
    let peer: Peer
    private var letter: String { String(peer.name.prefix(1)).uppercased() }
    private var color: Color {
        switch peer.platform {
        case "ios": return .blue
        case "android": return .green
        case "windows": return Color(red: 0, green: 0.47, blue: 0.83)
        case "linux": return .orange
        default: return .gray
        }
    }
    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            RoundedRectangle(cornerRadius: 10)
                .fill(color.gradient).frame(width: 40, height: 40)
                .overlay(Text(letter).font(.headline).foregroundStyle(.white))
            Circle().fill(.green).frame(width: 11, height: 11)
                .overlay(Circle().stroke(Color(.systemGroupedBackground), lineWidth: 2))
                .offset(x: 2, y: 2)
        }
    }
}
```

- [ ] **Step 2: 接入 `SendSentApp`**

将第一个 tab 内容改为 `DevicesView()`（`NavigationStack { DevicesView() }`）。

- [ ] **Step 3: 构建安装**（同 Task 1），并在 Mac/Android 开 app 验证能在设备页看到对方。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/gen/apple/Sources/sendsent
git commit -m "feat(ios): devices list + native file importer sending"
```

---

### Task 4: 传输页（进行中 + 历史）

**Files:**
- Create: `Sources/sendsent/Views/TransfersView.swift`
- Modify: `SendSentApp.swift`

- [ ] **Step 1: `TransfersView.swift`**

```swift
import SwiftUI

struct TransfersView: View {
    @EnvironmentObject var core: Core
    @State private var tab = 0

    private var active: [TransferProgress] {
        core.progress.values.sorted { $0.session_id < $1.session_id }
    }

    var body: some View {
        VStack(spacing: 0) {
            Picker("", selection: $tab) {
                Text("进行中").tag(0)
                Text("历史").tag(1)
            }
            .pickerStyle(.segmented)
            .padding(.horizontal).padding(.top, 8)

            if tab == 0 { activeList } else { historyList }
        }
        .navigationTitle("传输")
    }

    private var activeList: some View {
        List(active) { p in
            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text(p.files_total > 1 ? "\(p.files_done)/\(p.files_total) 个文件" : "1 个文件")
                        .font(.subheadline)
                    Spacer()
                    Text("\(Int(p.fraction * 100))%").font(.subheadline.monospacedDigit())
                }
                ProgressView(value: p.fraction)
                HStack {
                    Text(pretty(p.speed_bps) + "/s").font(.caption).foregroundStyle(.secondary)
                    Spacer()
                    Text(pretty(p.bytes_done) + " / " + pretty(p.bytes_total)).font(.caption).foregroundStyle(.secondary)
                }
            }.padding(.vertical, 4)
        }
        .overlay { if active.isEmpty { ContentUnavailableView("暂无进行中的传输", systemImage: "arrow.left.arrow.right") } }
    }

    private var historyList: some View {
        List {
            ForEach(core.history) { r in
                HStack(spacing: 12) {
                    Image(systemName: r.status == "completed" ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .foregroundStyle(r.status == "completed" ? .green : .red)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(r.files.count > 1 ? "\(r.files.first?.name ?? "") 等 \(r.files.count) 个文件" : (r.files.first?.name ?? "—"))
                            .lineLimit(1)
                        Text("\(r.direction == "send" ? "发送" : "接收") · \(r.peer_name) · \(pretty(r.bytes_done))")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }
            }
        }
        .overlay { if core.history.isEmpty { ContentUnavailableView("暂无历史记录", systemImage: "clock") } }
        .toolbar {
            if !core.history.isEmpty {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("清空", role: .destructive) { core.clearHistory() }
                }
            }
        }
    }

    private func pretty(_ b: UInt64) -> String {
        let f = Double(b)
        if f >= 1_073_741_824 { return String(format: "%.2f GB", f / 1_073_741_824) }
        if f >= 1_048_576 { return String(format: "%.1f MB", f / 1_048_576) }
        if f >= 1024 { return String(format: "%.0f KB", f / 1024) }
        return "\(b) B"
    }
}
```

- [ ] **Step 2: 接入第二个 tab。**
- [ ] **Step 3: 构建安装**；发送大文件观察进度与历史。
- [ ] **Step 4: Commit**

```bash
git add src-tauri/gen/apple/Sources/sendsent
git commit -m "feat(ios): transfers screen with progress and history"
```

---

### Task 5: 我的页（身份 + 二维码 + IP）

**Files:**
- Create: `Sources/sendsent/Views/ProfileView.swift`
- Modify: `SendSentApp.swift`

- [ ] **Step 1: `ProfileView.swift`**

```swift
import SwiftUI

struct ProfileView: View {
    @EnvironmentObject var core: Core
    @State private var qr: UIImage?
    @State private var selectedIP: String?

    var body: some View {
        List {
            Section {
                VStack(spacing: 8) {
                    Text(String((core.identity?.name ?? "S").prefix(1)).uppercased())
                        .font(.system(size: 32, weight: .semibold)).foregroundStyle(.secondary)
                        .frame(width: 76, height: 76)
                        .background(Color(.secondarySystemBackground), in: Circle())
                    Text(core.identity?.name ?? "未命名").font(.title2.bold())
                    Text("端口 \(core.port)").font(.footnote).foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity).listRowBackground(Color.clear)
            }
            Section {
                VStack(spacing: 12) {
                    if let qr {
                        Image(uiImage: qr).interpolation(.none).resizable()
                            .frame(width: 220, height: 220)
                            .padding(10).background(.white, in: RoundedRectangle(cornerRadius: 10))
                    } else {
                        ProgressView().frame(height: 220)
                    }
                    Text("让对方在 SendSent 里扫码即可连接").font(.footnote).foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity).padding(.vertical, 8)
            }
            if !core.addresses.isEmpty {
                Section("本机 IP") {
                    ForEach(core.addresses) { a in
                        HStack {
                            Text(a.ip + ":\(core.port)").font(.body.monospaced())
                            Spacer()
                            Text(a.interface).foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }
        .navigationTitle("我的")
        .task { loadQr() }
        .refreshable { loadQr() }
    }

    private func loadQr() {
        if let b64 = try? ffiQr(), let data = Data(base64Encoded: b64) {
            qr = UIImage(data: data)
        }
    }
}
```

- [ ] **Step 2: 接入第三个 tab。**
- [ ] **Step 3: 构建安装**；用另一台设备扫码验证能加入。
- [ ] **Step 4: Commit**

```bash
git add src-tauri/gen/apple/Sources/sendsent
git commit -m "feat(ios): profile screen with qr and local ips"
```

---

### Task 6: 设置页 + 添加设备（手输 / 扫码）

**Files:**
- Create: `Sources/sendsent/Views/SettingsView.swift`
- Create: `Sources/sendsent/Views/AddDeviceView.swift`
- Create: `Sources/sendsent/Views/ScanView.swift`
- Modify: `SendSentApp.swift`；`gen/apple/project.yml`（加 `NSCameraUsageDescription`）

- [ ] **Step 1: `AddDeviceView.swift`（手输 IP + 打开扫码）**

```swift
import SwiftUI

struct AddDeviceView: View {
    @EnvironmentObject var core: Core
    @Environment(\.dismiss) private var dismiss
    @State private var addr = ""
    @State private var showScan = false

    var body: some View {
        Form {
            Section("手动输入 IP:port") {
                TextField("192.168.1.5:52225", text: $addr)
                    .keyboardType(.URL).autocorrectionDisabled().textInputAutocapitalization(.never)
                Button("添加") { core.addPeer(addr.trimmingCharacters(in: .whitespaces)); dismiss() }
                    .disabled(addr.isEmpty)
            }
            Section {
                Button { showScan = true } label: { Label("扫码添加", systemImage: "qrcode.viewfinder") }
            }
        }
        .navigationTitle("添加设备")
        .toolbar { ToolbarItem(placement: .cancellationAction) { Button("关闭") { dismiss() } } }
        .sheet(isPresented: $showScan) {
            NavigationStack {
                ScanView { payload in
                    if let a = parseAddr(payload) { core.addPeer(a) }
                    showScan = false; dismiss()
                }
            }
        }
    }

    /// 解析 `sendsent://name?addr=ip:port&dir=...` 或裸 `ip:port`
    private func parseAddr(_ s: String) -> String? {
        let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.range(of: #"^[0-9.]+:[0-9]+$"#, options: .regularExpression) != nil { return t }
        guard t.hasPrefix("sendsent://"),
              let u = URLComponents(string: t.replacingOccurrences(of: "sendsent://", with: "http://")) else { return nil }
        return u.queryItems?.first(where: { $0.name == "addr" })?.value
    }
}
```

- [ ] **Step 2: `ScanView.swift`（AVFoundation）**

```swift
import SwiftUI
import AVFoundation

struct ScanView: UIViewControllerRepresentable {
    let onFound: (String) -> Void
    func makeUIViewController(context: Context) -> ScannerVC { let v = ScannerVC(); v.onFound = onFound; return v }
    func updateUIViewController(_ uiViewController: ScannerVC, context: Context) {}
}

final class ScannerVC: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    var onFound: ((String) -> Void)?
    private let session = AVCaptureSession()
    private var done = false

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        guard let device = AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device),
              session.canAddInput(input) else { return }
        session.addInput(input)
        let output = AVCaptureMetadataOutput()
        if session.canAddOutput(output) {
            session.addOutput(output)
            output.setMetadataObjectsDelegate(self, queue: .main)
            output.metadataObjectTypes = [.qr]
        }
        let preview = AVCaptureVideoPreviewLayer(session: session)
        preview.frame = view.bounds
        preview.videoGravity = .resizeAspectFill
        view.layer.addSublayer(preview)
        DispatchQueue.global(qos: .userInitiated).async { self.session.startRunning() }
    }
    override func viewDidDisappear(_ animated: Bool) { session.stopRunning() }

    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput objects: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard !done, let obj = objects.first as? AVMetadataMachineReadableCodeObject, let s = obj.stringValue else { return }
        done = true
        session.stopRunning()
        onFound?(s)
    }
}
```

- [ ] **Step 3: `SettingsView.swift`**

```swift
import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var core: Core
    @State private var name = ""
    @State private var conns = 16
    @State private var chunkKb = 1024
    @State private var splitMb = 8

    var body: some View {
        Form {
            Section("本机") {
                TextField("显示名称", text: $name)
                    .onSubmit { core.rename(name) }
                LabeledContent("端口", value: "\(core.port)")
            }
            Section("传输参数") {
                Stepper("并发连接数: \(conns)", value: $conns, in: 1...32)
                Stepper("数据块: \(chunkKb) KB", value: $chunkKb, in: 64...1024, step: 64)
                Stepper("分片阈值: \(splitMb) MB", value: $splitMb, in: 1...1024)
                Button("保存") { core.setConfig(conns: UInt32(conns), chunkKb: UInt64(chunkKb), splitMb: UInt64(splitMb)) }
            }
            Section("关于") {
                LabeledContent("版本", value: "0.1.0")
                LabeledContent("技术", value: "Tauri 2 + React 19 / Rust")
            }
        }
        .navigationTitle("设置")
        .onAppear {
            name = core.identity?.name ?? ""
            if let c = try? ffiConfig() {
                conns = Int(c.conns); chunkKb = Int(c.chunk_size / 1024); splitMb = Int(c.split_threshold / 1_048_576)
            }
        }
    }
}
```

- [ ] **Step 4: `project.yml` 加相机权限**

在 `info.properties` 增加：

```yaml
        NSCameraUsageDescription: SendSent uses the camera to scan a device QR code.
```

然后 `cd gen/apple && xcodegen generate`。

- [ ] **Step 5: 接入第四个 tab。**
- [ ] **Step 6: 构建安装**；验证手输 IP、扫码、改名、参数保存。
- [ ] **Step 7: Commit**

```bash
git add src-tauri/gen/apple/Sources/sendsent src-tauri/gen/apple/project.yml src-tauri/gen/apple/sendsent_iOS/Info.plist
git commit -m "feat(ios): settings + add-device + qr scanner"
```

---

### Task 7: 收到请求（系统 Alert）+ 收尾

**Files:**
- Modify: `SendSentApp.swift`
- Create: `Sources/sendsent/Views/IncomingRequestView.swift`

- [ ] **Step 1: 在 `RootView` 挂 `.alert`**

```swift
        .alert("收到文件", isPresented: Binding(get: { core.request != nil }, set: { if !$0 { core.request = nil } })) {
            Button("拒绝", role: .destructive) { core.respond(accept: false) }
            Button("接受") { core.respond(accept: true) }
        } message: {
            if let r = core.request {
                Text("\(r.sender.name) 想发送 \(r.manifest.total_count) 个文件 (\(prettySize(r.manifest.total_size)))")
            }
        }
```
（`prettySize` 作为文件内全局函数。）

- [ ] **Step 2: `IncomingRequestView.swift` 可省略**（`.alert` 已覆盖）；如保留则仅作预览组件。也可直接删除该文件计划项。

- [ ] **Step 3: 最终构建 + 真机全量验收**

```bash
cd src-tauri/gen/apple && xcodegen generate
cd /Users/mankong/volumes/code/mac-rs/SendSent && pnpm tauri ios build -t aarch64 -c '{"build":{"beforeBuildCommand":""}}'
```
安装启动后逐项验收（发现/发送/接收/进度/历史/我的/设置/扫码）。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/gen/apple/Sources/sendsent
git commit -m "feat(ios): incoming request alert + polish"
```

---

## Self-Review 记录

- **Spec 覆盖**：spec §6.1 文件结构 → 各任务；§6.2 Core → Task 2；§6.3 各页/交互 → Task 3–7；§6.4 视觉原则 → 全用系统组件；§9 错误处理 → `FFIError` + `toast`；§10 验收 → Task 7 Step 3。
- **无占位**：关键文件给出完整代码；`IncomingRequestView` 明确说明可省略。
- **类型一致**：`Core` 的属性/方法与视图调用一致；FFI 函数签名与 Plan 1 `ffi.rs` 导出严格对应（`bool`↔`Bool`、`u32`↔`UInt32`、`u64`↔`UInt64`、`u16`↔`UInt16`、`Option<fn>`↔可选闭包）。
- **已知风险/注意**：
  - `@convention(c)` 回调不能捕获上下文 → 用全局 `eventSink` + 顶层 trampoline。
  - security-scoped URL 在发送后延迟释放（30s）。
  - `ContentUnavailableView` 需 iOS 17（与本项目 deploymentTarget 一致）。
  - 相机权限键必须写进 `project.yml`（否则 regenerate 会丢）。
  - 端口：真机 52225；simulator 会与 Mac 抢端口，联调时注意。
