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
            .padding(.horizontal)
            .padding(.top, 8)

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
                    Text(prettySize(p.speed_bps) + "/s").font(.caption).foregroundStyle(.secondary)
                    Spacer()
                    Text(prettySize(p.bytes_done) + " / " + prettySize(p.bytes_total))
                        .font(.caption).foregroundStyle(.secondary)
                }
            }
            .padding(.vertical, 4)
        }
        .overlay {
            if active.isEmpty {
                ContentUnavailableView("暂无进行中的传输", systemImage: "arrow.left.arrow.right")
            }
        }
    }

    private var historyList: some View {
        List(core.history) { r in
            HStack(spacing: 12) {
                Image(systemName: r.status == "completed" ? "checkmark.circle.fill" : "xmark.circle.fill")
                    .foregroundStyle(r.status == "completed" ? .green : .red)
                VStack(alignment: .leading, spacing: 2) {
                    Text(r.files.count > 1
                         ? "\(r.files.first?.name ?? "") 等 \(r.files.count) 个文件"
                         : (r.files.first?.name ?? "—"))
                        .lineLimit(1)
                    Text("\(r.direction == "send" ? "发送" : "接收") · \(r.peer_name) · \(prettySize(r.bytes_done))")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }
        }
        .overlay {
            if core.history.isEmpty {
                ContentUnavailableView("暂无历史记录", systemImage: "clock")
            }
        }
        .toolbar {
            if !core.history.isEmpty {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("清空", role: .destructive) { core.clearHistory() }
                }
            }
        }
    }
}
