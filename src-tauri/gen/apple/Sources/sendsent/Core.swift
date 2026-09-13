import Foundation
import SwiftUI

// @convention(c) 回调不能捕获上下文 → 全局 sink + 顶层 trampoline
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
    /// 用户手动删除(隐藏)的设备。保留在 peers 里但不出现在列表,
    /// 这样 mDNS 重新广播也不会把它顶回来;重启后清空。
    @Published var hidden: Set<String> = []

    var visiblePeers: [Peer] {
        peers.filter { !hidden.contains($0.device_id) }
    }

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
        case "Request":
            request = try? JSONDecoder().decode(TransferRequest.self, from: data)
        case "Progress":
            if let e = try? JSONDecoder().decode(TransferProgress.self, from: data) {
                progress[e.session_id] = e
            }
        case "Finished":
            if let e = try? JSONDecoder().decode(TransferFinished.self, from: data) {
                progress.removeValue(forKey: e.session_id)
                reloadHistory()
            }
        case "Recorded":
            if let e = try? JSONDecoder().decode(HistoryRecord.self, from: data) {
                history.removeAll { $0.session_id == e.session_id }
                history.insert(e, at: 0)
            }
        default:
            break
        }
    }

    func flash(_ text: String) {
        toast = text
        Task {
            try? await Task.sleep(nanoseconds: 2_400_000_000)
            if toast == text { toast = nil }
        }
    }

    func send(peer: Peer, files: [URL], secure: Bool, verify: Bool) {
        var accessed: [URL] = []
        for u in files where u.startAccessingSecurityScopedResource() { accessed.append(u) }
        let paths = files.map(\.path)
        do {
            _ = try ffiSend(peerId: peer.device_id, files: paths, secure: secure, verify: verify)
            flash("已发送到 \(peer.name)")
        } catch {
            flash("发送失败: \(error.localizedDescription)")
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 60) {
            accessed.forEach { $0.stopAccessingSecurityScopedResource() }
        }
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

    func hide(_ peer: Peer) {
        hidden.insert(peer.device_id)
        flash("已删除 \(peer.name)")
    }

    func clearHistory() {
        do { try ffiClearHistory(); history = [] } catch { flash("清空失败") }
    }

    func deleteHistory(_ id: String) {
        do {
            try ffiDeleteHistory(sessionId: id)
            history.removeAll { $0.session_id == id }
        } catch {
            flash("删除失败")
        }
    }

    func rename(_ name: String) {
        do { try ffiSetDisplayName(name); reloadIdentity() }
        catch { flash("改名失败: \(error.localizedDescription)") }
    }

    func setConfig(conns: UInt32, chunkKb: UInt64, splitMb: UInt64) {
        do { try ffiSetConfig(conns: conns, chunkKb: chunkKb, splitMb: splitMb); flash("已保存") }
        catch { flash("保存失败: \(error.localizedDescription)") }
    }

    // MARK: - 查询

    func qrBase64() -> String? { try? ffiQr() }

    func history(for peer: Peer) -> [HistoryRecord] {
        history.filter { $0.peer_name == peer.name }
    }
}
