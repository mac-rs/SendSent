import SwiftUI

/// 接收请求:底部弹出的玻璃卡。
struct ReceivingSheet: View {
    let request: TransferRequest
    let onAccept: () -> Void
    let onReject: () -> Void

    var body: some View {
        VStack(spacing: DS.Spacing.md) {
            Capsule().fill(Color.secondary.opacity(0.45)).frame(width: 38, height: 5).padding(.top, 8)

            HStack(spacing: DS.Spacing.md) {
                PlatformAvatar(peer: request.sender, size: 56)
                VStack(alignment: .leading, spacing: 3) {
                    Text(request.sender.name).font(Type.title()).lineLimit(1)
                    Text("想发送文件给你").font(Type.callout()).foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
                Image(systemName: "arrow.down.circle.fill")
                    .font(.system(size: 30))
                    .foregroundStyle(LinearGradient(colors: [.green, .mint], startPoint: .top, endPoint: .bottom))
            }

            VStack(spacing: DS.Spacing.xs) {
                HStack {
                    Label("\(request.manifest.total_count) 个文件", systemImage: "doc.on.doc")
                        .font(Type.calloutEm())
                    Spacer()
                    Text(prettySize(request.manifest.total_size)).font(Type.calloutEm())
                }
                ForEach(Array(request.manifest.files.prefix(3)), id: \.name) { f in
                    HStack {
                        Image(systemName: fileSymbol(f.name)).font(Type.caption()).foregroundStyle(.tint)
                        Text(f.name).font(Type.caption()).lineLimit(1)
                        Spacer()
                        Text(prettySize(f.size)).font(Type.mono(11)).foregroundStyle(.secondary)
                    }
                }
                if request.manifest.total_count > 3 {
                    Text("等 \(request.manifest.total_count) 个文件")
                        .font(Type.caption()).foregroundStyle(.tertiary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding(DS.Spacing.md)
            .ssCard(DS.Radius.md)

            HStack(spacing: DS.Spacing.sm) {
                Button(role: .destructive, action: onReject) {
                    Label("拒绝", systemImage: "xmark")
                        .frame(maxWidth: .infinity, minHeight: DS.minTap)
                }
                .buttonStyle(.bordered)
                .buttonBorderShape(.roundedRectangle(radius: DS.Radius.md))
                .controlSize(.large)

                Button(action: onAccept) {
                    Label("接受", systemImage: "checkmark")
                        .frame(maxWidth: .infinity, minHeight: DS.minTap)
                }
                .buttonStyle(.borderedProminent)
                .buttonBorderShape(.roundedRectangle(radius: DS.Radius.md))
                .controlSize(.large)
                .tint(.indigo)
            }
        }
        .padding(.horizontal, DS.Spacing.lg)
        .padding(.bottom, DS.Spacing.lg)
    }
}
