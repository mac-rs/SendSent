import SwiftUI
import UniformTypeIdentifiers

/// AirDrop 式雷达:我居中,附近设备环绕;点选后在底部发送并设置加密/校验。
struct RadarView: View {
    @EnvironmentObject var core: Core
    @Environment(\.colorScheme) private var scheme
    @State private var selected: Peer?
    @State private var showImporter = false
    @State private var showAdd = false
    @State private var secure = true
    @State private var verify = false
    @State private var pulse = false
    @State private var spin = false

    private var isDark: Bool { scheme == .dark }
    private var peers: [Peer] { core.visiblePeers }
    private var active: TransferProgress? {
        core.progress.values.sorted { $0.session_id < $1.session_id }.first
    }
    private var initial: String { String((core.identity?.name ?? "我").prefix(1)).uppercased() }
    private var subtitle: String {
        peers.isEmpty ? "正在同一局域网里寻找…" : "同一局域网 · 找到 \(peers.count) 台设备"
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            radar
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(AmbientBackground())
        .toolbar(.hidden, for: .navigationBar)
        .safeAreaInset(edge: .bottom, spacing: 0) { actionBar }
        .fileImporter(
            isPresented: $showImporter,
            allowedContentTypes: [.data, .image, .movie, .audio, .pdf, .text, .archive],
            allowsMultipleSelection: true
        ) { result in
            if case let .success(urls) = result, !urls.isEmpty, let p = selected {
                core.send(peer: p, files: urls, secure: secure, verify: verify)
            }
        }
        .sheet(isPresented: $showAdd) { NavigationStack { AddDeviceView() } }
        .onAppear { pulse = true; spin = true }
        .onChange(of: peers) { _, new in
            if let s = selected, !new.contains(where: { $0.device_id == s.device_id }) { selected = nil }
        }
    }

    // MARK: 顶部

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text("附近").font(Type.display())
                Text(subtitle).font(Type.footnote()).foregroundStyle(.secondary)
            }
            Spacer()
            Button { showAdd = true } label: {
                Image(systemName: "plus")
                    .font(.system(size: 17, weight: .medium))
                    .foregroundStyle(.primary)
                    .frame(width: 40, height: 40)
                    .background(Circle().fill(Color.ssField))
                    .overlay(Circle().strokeBorder(Color.ssCardBorder, lineWidth: 0.5))
            }
        }
        .padding(.horizontal, DS.Spacing.lg)
        .padding(.top, DS.Spacing.sm)
    }

    // MARK: 雷达

    private var radar: some View {
        GeometryReader { geo in
            let w = geo.size.width
            let h = geo.size.height
            let center = CGPoint(x: w / 2, y: h * 0.44)
            let radius = min(w, h) * 0.33

            ZStack {
                Color.clear
                    .contentShape(Rectangle())
                    .onTapGesture { withAnimation(DS.Anim.quick) { selected = nil } }

                Circle()
                    .fill(RadialGradient(colors: [Color.indigo.opacity(isDark ? 0.28 : 0.13), .clear],
                                         center: .center, startRadius: 0, endRadius: radius * 1.5))
                    .frame(width: radius * 3, height: radius * 3)
                    .position(center)
                    .allowsHitTesting(false)

                ring(radius * 1.34, dashed: false, center)
                ring(radius * 1.02, dashed: true, center)
                ring(radius * 0.70, dashed: false, center)

                Circle()
                    .fill(AngularGradient(
                        gradient: Gradient(colors: [Color.indigo.opacity(isDark ? 0.14 : 0.10), .clear]),
                        center: .center, startAngle: .degrees(0), endAngle: .degrees(90)))
                    .frame(width: radius * 1.34 * 2, height: radius * 1.34 * 2)
                    .rotationEffect(.degrees(spin ? 360 : 0))
                    .animation(.linear(duration: 6).repeatForever(autoreverses: false), value: spin)
                    .position(center)
                    .allowsHitTesting(false)

                pulseRings(center, radius * 0.70)

                meNode.position(center)

                ForEach(Array(peers.enumerated()), id: \.element.id) { idx, peer in
                    let a = angle(index: idx, count: peers.count)
                    peerNode(peer)
                        .position(x: center.x + CGFloat(cos(a)) * radius,
                                  y: center.y + CGFloat(sin(a)) * radius)
                }
            }
        }
    }

    private func ring(_ r: CGFloat, dashed: Bool, _ c: CGPoint) -> some View {
        Circle()
            .strokeBorder(Color.primary.opacity(isDark ? 0.09 : 0.12),
                          style: StrokeStyle(lineWidth: 1, dash: dashed ? [4, 6] : []))
            .frame(width: r * 2, height: r * 2)
            .position(c)
    }

    private func pulseRings(_ c: CGPoint, _ base: CGFloat) -> some View {
        ForEach(0..<3, id: \.self) { i in
            Circle()
                .stroke(Color.indigo.opacity(isDark ? 0.22 : 0.18), lineWidth: 1.5)
                .frame(width: base * 2, height: base * 2)
                .scaleEffect(pulse ? 2.0 : 0.62)
                .opacity(pulse ? 0 : 0.75)
                .animation(.easeOut(duration: 3.4).repeatForever(autoreverses: false).delay(Double(i) * 1.05),
                           value: pulse)
                .position(c)
        }
        .allowsHitTesting(false)
    }

    private var meNode: some View {
        VStack(spacing: 8) {
            ZStack {
                Circle().fill(Color.indigo.opacity(isDark ? 0.35 : 0.22)).frame(width: 78, height: 78).blur(radius: 20)
                Circle()
                    .fill(LinearGradient(colors: [Color(red: 0.65, green: 0.71, blue: 0.99), .indigo],
                                         startPoint: .topLeading, endPoint: .bottomTrailing))
                    .frame(width: 80, height: 80)
                    .overlay(Text(initial).font(.system(size: 30, weight: .semibold)).foregroundStyle(.white))
            }
            Text("我 · \(platformLabel(core.identity?.platform ?? "ios"))")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 10).padding(.vertical, 3)
                .background(Capsule().fill(Color.ssField))
        }
    }

    private func peerNode(_ peer: Peer) -> some View {
        let isSel = selected?.device_id == peer.device_id
        return Button {
            withAnimation(DS.Anim.bouncy) { selected = peer }
        } label: {
            VStack(spacing: 8) {
                ZStack {
                    Circle().fill(Color.indigo.opacity(isSel ? 0.16 : 0)).frame(width: 84, height: 84)
                    PlatformAvatar(peer: peer, size: isSel ? 66 : 58)
                        .shadow(color: isSel ? Color.indigo.opacity(0.35) : .black.opacity(isDark ? 0.12 : 0.10),
                                radius: isSel ? 14 : 5, y: 3)
                }
                .frame(height: 84)
                Text(peer.name)
                    .font(.system(size: 12, weight: isSel ? .semibold : .regular))
                    .foregroundStyle(isSel ? .primary : .secondary)
                    .lineLimit(1)
                    .frame(maxWidth: 100)
            }
        }
        .buttonStyle(.plain)
    }

    private func angle(index: Int, count: Int) -> Double {
        guard count > 0 else { return -.pi / 2 }
        if count == 1 { return -.pi / 2 }
        let start = -140.0 * .pi / 180, end = -40.0 * .pi / 180
        if count <= 4 {
            return start + (end - start) * Double(index) / Double(count - 1)
        }
        return -.pi / 2 + (2 * .pi) * Double(index) / Double(count)
    }

    // MARK: 底部操作

    private var actionBar: some View {
        VStack(spacing: DS.Spacing.sm) {
            if let p = active { progressRow(p) }

            if let p = selected {
                HStack(spacing: DS.Spacing.md) {
                    PlatformAvatar(peer: p, size: 46)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(p.name).font(Type.titleSmall()).lineLimit(1)
                        Text("\(platformLabel(p.platform)) · 已选择")
                            .font(Type.caption()).foregroundStyle(.secondary)
                    }
                    Spacer(minLength: DS.Spacing.xs)
                    Button { showImporter = true } label: {
                        Label("发送文件", systemImage: "paperplane.fill")
                            .font(Type.calloutEm())
                            .foregroundStyle(.white)
                            .padding(.horizontal, DS.Spacing.md)
                            .frame(height: 40)
                            .background(Capsule().fill(Color.indigo))
                    }
                    .buttonStyle(.plain)
                }

                Divider().overlay(Color.ssHairline)

                HStack(spacing: DS.Spacing.sm) {
                    toggleChip("加密传输", "lock.fill", $secure)
                    toggleChip("SHA-256", "checkmark.shield.fill", $verify)
                }
            } else {
                Text(peers.isEmpty ? "正在搜索附近设备…" : "点按附近的设备,再选择文件发送")
                    .font(Type.callout())
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, DS.Spacing.xs)
            }
        }
        .padding(.horizontal, DS.Spacing.md)
        .padding(.top, DS.Spacing.sm)
        .padding(.bottom, DS.Spacing.xs)
        .background(.bar)
        .overlay(alignment: .top) { Divider().overlay(Color.ssHairline) }
        .animation(DS.Anim.snappy, value: selected)
    }

    private func progressRow(_ p: TransferProgress) -> some View {
        HStack(spacing: DS.Spacing.sm) {
            ProgressView(value: p.fraction).tint(.indigo)
            Text("\(Int(p.fraction * 100))% · \(prettySize(p.speed_bps))/s")
                .font(Type.mono(12))
                .foregroundStyle(.secondary)
        }
    }

    private func toggleChip(_ title: String, _ symbol: String, _ isOn: Binding<Bool>) -> some View {
        HStack(spacing: 6) {
            Image(systemName: symbol).font(.system(size: 12))
            Text(title).font(.system(size: 13, weight: .medium))
            Spacer(minLength: 4)
            MiniSwitch(isOn: isOn)
        }
        .foregroundStyle(isOn.wrappedValue ? Color.indigo : Color.secondary)
        .padding(.horizontal, 12)
        .frame(height: 38)
        .frame(maxWidth: .infinity)
        .background(RoundedRectangle(cornerRadius: 12, style: .continuous)
            .fill(isOn.wrappedValue ? Color.indigo.opacity(0.14) : Color.ssCard))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous)
            .strokeBorder(isOn.wrappedValue ? Color.indigo.opacity(0.30) : Color.ssCardBorder, lineWidth: 0.5))
        .contentShape(Rectangle())
        .onTapGesture { withAnimation(DS.Anim.quick) { isOn.wrappedValue.toggle() } }
    }
}

// MARK: - 迷你开关

struct MiniSwitch: View {
    @Binding var isOn: Bool
    var body: some View {
        Capsule()
            .fill(isOn ? Color.indigo : Color.ssField)
            .frame(width: 38, height: 22)
            .overlay(alignment: isOn ? .trailing : .leading) {
                Circle().fill(.white)
                    .frame(width: 18, height: 18)
                    .padding(2)
                    .shadow(color: .black.opacity(0.3), radius: 1, y: 1)
            }
            .animation(DS.Anim.quick, value: isOn)
    }
}

// MARK: - 环境光背景

struct AmbientBackground: View {
    @Environment(\.colorScheme) private var scheme
    var body: some View {
        ZStack {
            Color.ssBg
            RadialGradient(colors: [Color.indigo.opacity(scheme == .dark ? 0.16 : 0.08), .clear],
                           center: .top, startRadius: 0, endRadius: 520)
        }
        .ignoresSafeArea()
    }
}
