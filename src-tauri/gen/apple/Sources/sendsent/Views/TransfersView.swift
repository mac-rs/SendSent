import SwiftUI

private let relFormatter: RelativeDateTimeFormatter = {
    let f = RelativeDateTimeFormatter()
    f.unitsStyle = .abbreviated
    return f
}()

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
            .padding(.bottom, 4)

            if tab == 0 { activeList } else { historyList }
        }
        .navigationTitle("传输")
    }

    // MARK: 进行中
    private var activeList: some View {
        List(active) { p in
            HStack(spacing: 14) {
                ZStack {
                    Circle().stroke(Color.accentColor.opacity(0.15), lineWidth: 3.5)
                    Circle()
                        .trim(from: 0, to: max(0.02, p.fraction))
                        .stroke(Color.accentColor, style: StrokeStyle(lineWidth: 3.5, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                        .animation(.linear(duration: 0.3), value: p.fraction)
                    Image(systemName: "arrow.up")
                        .font(.caption.bold()).foregroundStyle(Color.accentColor)
                }
                .frame(width: 40, height: 40)

                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Text(p.files_total > 1 ? "发送 \(p.files_done)/\(p.files_total) 个文件" : "正在发送")
                            .font(.subheadline.weight(.semibold))
                        Spacer()
                        Text("\(Int(p.fraction * 100))%")
                            .font(.subheadline.monospacedDigit()).foregroundStyle(.secondary)
                    }
                    ProgressView(value: p.fraction).tint(.accentColor)
                    HStack {
                        Text(prettySize(p.speed_bps) + "/s")
                            .font(.caption).foregroundStyle(.secondary)
                        Spacer()
                        Text("\(prettySize(p.bytes_done)) / \(prettySize(p.bytes_total))")
                            .font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                    }
                }
            }
            .padding(.vertical, 6)
        }
        .listStyle(.insetGrouped)
        .overlay {
            if active.isEmpty {
                ContentUnavailableView("暂无进行中的传输", systemImage: "arrow.left.arrow.right")
            }
        }
    }

    // MARK: 历史
    private var historyList: some View {
        List {
            ForEach(core.history) { r in
                historyRow(r)
                    .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                        Button(role: .destructive) { core.deleteHistory(r.session_id) } label: {
                            Label("删除", systemImage: "trash.fill")
                        }
                    }
            }
        }
        .listStyle(.insetGrouped)
        .overlay {
            if core.history.isEmpty {
                ContentUnavailableView("暂无历史记录", systemImage: "clock")
            }
        }
        .toolbar {
            if !core.history.isEmpty {
                ToolbarItem(placement: .topBarTrailing) {
                    Menu {
                        Button("清空全部记录", systemImage: "trash", role: .destructive) {
                            core.clearHistory()
                        }
                    } label: {
                        Image(systemName: "ellipsis.circle")
                    }
                }
            }
        }
    }

    private func historyRow(_ r: HistoryRecord) -> some View {
        HStack(spacing: 12) {
            Image(systemName: statusIcon(r.status))
                .font(.title2)
                .foregroundStyle(statusColor(r.status))
                .symbolRenderingMode(.hierarchical)

            VStack(alignment: .leading, spacing: 3) {
                Text(fileTitle(r))
                    .font(.subheadline.weight(.medium))
                    .lineLimit(1)
                HStack(spacing: 4) {
                    Image(systemName: r.direction == "send" ? "arrow.up.right" : "arrow.down.left")
                        .font(.caption2.bold())
                        .foregroundStyle(r.direction == "send" ? Color.blue : Color.green)
                    Text(r.direction == "send" ? "发送" : "接收")
                    Text("·"); Text(r.peer_name).lineLimit(1)
                    Text("·"); Text(prettySize(r.bytes_done))
                }
                .font(.caption).foregroundStyle(.secondary)
            }

            Spacer(minLength: 6)

            Text(relativeTime(r.ended_at_ms))
                .font(.caption2).foregroundStyle(.tertiary)
        }
        .padding(.vertical, 4)
    }

    private func fileTitle(_ r: HistoryRecord) -> String {
        guard let first = r.files.first?.name else { return "—" }
        return r.files.count > 1 ? "\(first) 等 \(r.files.count) 个文件" : first
    }

    private func statusIcon(_ s: String) -> String {
        switch s {
        case "completed": return "checkmark.circle.fill"
        case "rejected": return "hand.raised.circle.fill"
        case "cancelled": return "slash.circle.fill"
        default: return "exclamationmark.circle.fill"
        }
    }
    private func statusColor(_ s: String) -> Color {
        switch s {
        case "completed": return .green
        case "rejected": return .orange
        case "cancelled": return .gray
        default: return .red
        }
    }
    private func relativeTime(_ ms: Int64) -> String {
        guard ms > 0 else { return "" }
        return relFormatter.localizedString(for: Date(timeIntervalSince1970: Double(ms) / 1000), relativeTo: Date())
    }
}
