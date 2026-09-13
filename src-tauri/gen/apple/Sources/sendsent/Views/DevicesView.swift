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
    @State private var detail: Peer?

    private var filtered: [Peer] {
        let k = search.trimmingCharacters(in: .whitespaces).lowercased()
        return k.isEmpty ? core.visiblePeers : core.visiblePeers.filter { $0.name.lowercased().contains(k) }
    }
    private var selectedPeers: [Peer] { core.peers.filter { selected.contains($0.device_id) } }

    var body: some View {
        List {
            if let active = core.progress.values.first {
                Section { ActiveTransferBanner(p: active) }
            }

            if !filtered.isEmpty {
                Section {
                    ForEach(filtered) { peer in
                        row(peer)
                            .contentShape(Rectangle())
                            .onTapGesture { toggle(peer) }
                            .contextMenu {
                                Button { detail = peer } label: { Label("查看详情", systemImage: "info.circle") }
                                Button(role: .destructive) { core.hide(peer) } label: { Label("删除", systemImage: "trash") }
                            }
                            .swipeActions(edge: .leading, allowsFullSwipe: false) {
                                Button { detail = peer } label: {
                                    Label("详情", systemImage: "info.circle.fill")
                                }
                                .tint(.indigo)
                            }
                            .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                                Button(role: .destructive) { core.hide(peer) } label: {
                                    Label("删除", systemImage: "trash.fill")
                                }
                            }
                    }
                } header: {
                    HStack {
                        Text("附近设备")
                        Spacer()
                        Text("\(filtered.count) 台").textCase(nil).foregroundStyle(.secondary)
                    }
                } footer: {
                    Text("点按选择设备;左滑查看详情,右滑删除")
                }
            }
        }
        .listStyle(.insetGrouped)
        .refreshable { core.reloadPeers() }
        .animation(.snappy, value: filtered.count)
        .sensoryFeedback(.selection, trigger: selected)
        .searchable(text: $search, prompt: "搜索设备")
        .overlay {
            if core.visiblePeers.isEmpty {
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
        .safeAreaInset(edge: .bottom, spacing: 0) {
            if !selected.isEmpty { sendBar }
        }
        .fileImporter(
            isPresented: $showImporter,
            allowedContentTypes: [.data, .image, .movie, .audio, .pdf, .text, .archive],
            allowsMultipleSelection: true
        ) { result in
            if case let .success(urls) = result { send(urls) }
        }
        .sheet(isPresented: $showAdd) { NavigationStack { AddDeviceView() } }
        .sheet(item: $detail) { PeerDetailView(peer: $0) }
    }

    private var title: String {
        if selected.isEmpty { return "设备" }
        return selected.count == 1 ? (selectedPeers.first?.name ?? "设备") : "已选 \(selected.count) 台"
    }

    private func row(_ peer: Peer) -> some View {
        HStack(spacing: 12) {
            PlatformAvatar(peer: peer)
            VStack(alignment: .leading, spacing: 3) {
                Text(peer.name).font(.headline)
                Text("\(platformLabel(peer.platform)) · \(peer.addrs.first ?? ":\(peer.port)")")
                    .font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
            }
            Spacer(minLength: 8)
            Image(systemName: selected.contains(peer.device_id) ? "checkmark.circle.fill" : "circle")
                .font(.title3)
                .foregroundStyle(selected.contains(peer.device_id) ? Color.accentColor : Color.secondary.opacity(0.35))
                .contentTransition(.symbolEffect(.replace))
        }
        .padding(.vertical, 2)
    }

    private var sendBar: some View {
        VStack(spacing: 12) {
            HStack(spacing: 18) {
                Toggle("加密传输", isOn: $secure)
                Toggle("SHA-256", isOn: $verify)
            }
            .font(.subheadline)
            Button {
                showImporter = true
            } label: {
                Label("选择文件发送", systemImage: "paperplane.fill")
                    .font(.headline)
                    .frame(maxWidth: .infinity).padding(.vertical, 6)
            }
            .buttonStyle(.borderedProminent)
            .buttonBorderShape(.roundedRectangle(radius: 14))
            .controlSize(.large)
        }
        .padding(.horizontal).padding(.top, 12).padding(.bottom, 8)
        .background(.ultraThinMaterial)
        .overlay(alignment: .top) { Divider() }
        .transition(.move(edge: .bottom).combined(with: .opacity))
        .animation(.snappy, value: selected)
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

/// 顶部进行中传输的小横幅
private struct ActiveTransferBanner: View {
    let p: TransferProgress
    var body: some View {
        HStack(spacing: 12) {
            ZStack {
                Circle().stroke(Color.accentColor.opacity(0.2), lineWidth: 3)
                Circle()
                    .trim(from: 0, to: max(0.02, p.fraction))
                    .stroke(Color.accentColor, style: StrokeStyle(lineWidth: 3, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                    .animation(.linear(duration: 0.3), value: p.fraction)
                Image(systemName: "arrow.up").font(.caption.bold()).foregroundStyle(Color.accentColor)
            }
            .frame(width: 34, height: 34)
            VStack(alignment: .leading, spacing: 2) {
                Text(p.files_total > 1 ? "发送 \(p.files_done)/\(p.files_total) 个文件" : "正在发送")
                    .font(.subheadline.weight(.medium))
                Text("\(prettySize(p.speed_bps))/s · \(prettySize(p.bytes_done)) / \(prettySize(p.bytes_total))")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            Text("\(Int(p.fraction * 100))%").font(.subheadline.monospacedDigit()).foregroundStyle(.secondary)
        }
        .listRowBackground(Color.accentColor.opacity(0.08))
    }
}

struct PeerDetailView: View {
    let peer: Peer
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    VStack(spacing: 10) {
                        PlatformAvatar(peer: peer, size: 72)
                        Text(peer.name).font(.title2.bold())
                        Text(platformLabel(peer.platform))
                            .font(.subheadline).foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                    .listRowBackground(Color.clear)
                }

                Section("连接") {
                    ForEach(peer.addrs, id: \.self) { addr in
                        LabeledContent("地址", value: addr)
                            .font(.body.monospaced())
                    }
                    LabeledContent("端口", value: "\(peer.port)")
                }

                Section("标识") {
                    LabeledContent("设备 ID") {
                        Text(peer.device_id).font(.footnote.monospaced()).foregroundStyle(.secondary)
                    }
                    LabeledContent("协议版本", value: "v\(peer.proto_version)")
                }
            }
            .navigationTitle("设备详情")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("完成") { dismiss() } }
            }
        }
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
    var size: CGFloat = 44
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
            RoundedRectangle(cornerRadius: size * 0.24, style: .continuous)
                .fill(color.gradient)
                .frame(width: size, height: size)
                .overlay(Text(letter).font(.system(size: size * 0.42, weight: .semibold)).foregroundStyle(.white))
                .shadow(color: color.opacity(0.25), radius: 3, y: 1)
            Circle()
                .fill(.green)
                .frame(width: size * 0.26, height: size * 0.26)
                .overlay(Circle().stroke(Color(.systemGroupedBackground), lineWidth: 2))
                .offset(x: size * 0.05, y: size * 0.05)
        }
    }
}
