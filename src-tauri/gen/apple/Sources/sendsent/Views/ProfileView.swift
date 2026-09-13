import SwiftUI
import UIKit

/// 极简「我的」:大号设备名 + 二维码海报 + 细线信息区。
struct ProfileView: View {
    @EnvironmentObject var core: Core
    @State private var qr: UIImage?

    private var name: String { core.identity?.name ?? "未命名" }
    private var platform: String { platformLabel(core.identity?.platform ?? "ios") }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("我的").font(.system(size: 14, weight: .medium)).foregroundStyle(.secondary)
                    Text(name).font(Type.display()).lineLimit(1).minimumScaleFactor(0.7)
                    HStack(spacing: 8) {
                        Circle().fill(.green).frame(width: 6, height: 6)
                        Text("可被发现")
                        Text("·").foregroundStyle(.tertiary)
                        Text(platform)
                        Text("·").foregroundStyle(.tertiary)
                        Text("端口 \(core.port)")
                    }
                    .font(Type.footnote()).foregroundStyle(.secondary)
                }
                .padding(.top, DS.Spacing.sm)

                qrHero
                    .padding(.top, DS.Spacing.xl)

                VStack(alignment: .leading, spacing: 0) {
                    sectionTitle("本机地址")
                    ForEach(core.addresses) { a in
                        infoRow(a.ip + ":" + String(core.port), mono: true, trailing: a.interface)
                    }
                    if core.addresses.isEmpty {
                        infoRow("未发现可用的局域网地址", mono: false, trailing: "")
                    }

                    sectionTitle("关于").padding(.top, DS.Spacing.xl)
                    infoRow("设备 ID", trailing: String((core.identity?.device_id ?? "—").prefix(17)))
                    infoRow("版本", trailing: "0.1.0")
                }
                .padding(.top, DS.Spacing.xl)
            }
            .padding(.horizontal, DS.Spacing.lg)
            .padding(.bottom, DS.Spacing.xl)
        }
        .background(AmbientBackground())
        .scrollIndicators(.hidden)
        .toolbar(.hidden, for: .navigationBar)
        .task { loadQr() }
    }

    private var qrHero: some View {
        VStack(spacing: DS.Spacing.md) {
            ZStack {
                Circle()
                    .fill(RadialGradient(colors: [Color.indigo.opacity(0.32), .clear],
                                         center: .center, startRadius: 0, endRadius: 180))
                    .frame(width: 320, height: 320)
                Group {
                    if let qr {
                        Image(uiImage: qr)
                            .interpolation(.none)
                            .resizable()
                            .scaledToFit()
                    } else {
                        ProgressView()
                    }
                }
                .frame(width: 184, height: 184)
                .padding(16)
                .background(.white, in: RoundedRectangle(cornerRadius: 30, style: .continuous))
                .shadow(color: .black.opacity(0.45), radius: 30, y: 16)
            }
            .frame(maxWidth: .infinity)
            Text("让对方在 SendSent 里扫码连接")
                .font(Type.caption()).foregroundStyle(.secondary)
        }
    }

    private func sectionTitle(_ t: String) -> some View {
        Text(t)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(.secondary)
            .padding(.bottom, 4)
    }

    private func infoRow(_ value: String, mono: Bool = false, trailing: String) -> some View {
        HStack {
            Text(value)
                .font(mono ? Type.mono(14) : Type.callout())
                .foregroundStyle(mono ? .primary : .secondary)
                .lineLimit(1).truncationMode(.middle)
            Spacer(minLength: DS.Spacing.sm)
            if !trailing.isEmpty {
                Text(trailing).font(Type.caption()).foregroundStyle(.tertiary)
            }
        }
        .frame(height: 46)
        .overlay(alignment: .top) { Color.ssHairline.frame(height: 0.5) }
    }

    private func loadQr() {
        guard let b64 = core.qrBase64(), let data = Data(base64Encoded: b64) else { return }
        qr = UIImage(data: data)
    }
}
