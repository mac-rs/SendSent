import SwiftUI
import UniformTypeIdentifiers

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
        List(filtered) { peer in
            Button { toggle(peer) } label: { row(peer) }
                .buttonStyle(.plain)
        }
        .searchable(text: $search, prompt: "搜索设备")
        .overlay {
            if core.peers.isEmpty {
                ContentUnavailableView(
                    "正在发现附近设备…",
                    systemImage: "dot.radiowaves.left.and.right",
                    description: Text("确保设备在同一局域网，或点右上角 + 手动添加")
                )
            }
        }
        .navigationTitle(title)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button { showAdd = true } label: { Image(systemName: "plus") }
            }
        }
        .safeAreaInset(edge: .bottom) { if !selected.isEmpty { sendBar } }
        .fileImporter(
            isPresented: $showImporter,
            allowedContentTypes: [.data, .image, .movie, .audio, .pdf, .text, .archive],
            allowsMultipleSelection: true
        ) { result in
            if case let .success(urls) = result { send(urls) }
        }
        .sheet(isPresented: $showAdd) { NavigationStack { AddDeviceView() } }
    }

    private var title: String {
        if selected.isEmpty { return "设备" }
        return selected.count == 1 ? (selectedPeers.first?.name ?? "设备") : "已选 \(selected.count) 台"
    }

    private func row(_ peer: Peer) -> some View {
        HStack(spacing: 12) {
            PlatformAvatar(peer: peer)
            VStack(alignment: .leading, spacing: 2) {
                Text(peer.name).font(.body).foregroundStyle(.primary)
                Text("\(platformLabel(peer.platform)) · \(peer.addrs.first ?? ":\(peer.port)")")
                    .font(.footnote).foregroundStyle(.secondary)
            }
            Spacer()
            Image(systemName: selected.contains(peer.device_id) ? "checkmark.circle.fill" : "circle")
                .foregroundStyle(selected.contains(peer.device_id) ? Color.accentColor : Color.secondary.opacity(0.4))
                .imageScale(.large)
        }
        .contentShape(Rectangle())
    }

    private var sendBar: some View {
        VStack(spacing: 10) {
            HStack(spacing: 16) {
                Toggle("加密传输", isOn: $secure)
                Toggle("SHA-256", isOn: $verify)
            }
            .font(.footnote)
            Button {
                showImporter = true
            } label: {
                Label("选择文件发送", systemImage: "paperplane.fill")
                    .frame(maxWidth: .infinity).padding(.vertical, 6)
            }
            .buttonStyle(.borderedProminent)
        }
        .padding(.horizontal).padding(.top, 10).padding(.bottom, 6)
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
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) {
            accessed.forEach { $0.stopAccessingSecurityScopedResource() }
        }
        selected.removeAll()
    }
}

func platformLabel(_ p: String) -> String {
    switch p {
    case "macos": return "macOS"
    case "ios": return "iOS"
    case "android": return "Android"
    case "windows": return "Windows"
    case "linux": return "Linux"
    default: return "Unknown"
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
                .fill(color.gradient)
                .frame(width: 40, height: 40)
                .overlay(Text(letter).font(.headline).foregroundStyle(.white))
            Circle()
                .fill(.green)
                .frame(width: 11, height: 11)
                .overlay(Circle().stroke(Color(.systemGroupedBackground), lineWidth: 2))
                .offset(x: 2, y: 2)
        }
    }
}
