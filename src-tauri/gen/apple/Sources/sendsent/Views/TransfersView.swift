import SwiftUI

struct TransfersView: View {
    @EnvironmentObject var core: Core
    @State private var dir: DirFilter = .all
    @State private var status: StatusFilter = .all

    private var isFiltering: Bool { dir != .all || status != .all }

    private var active: [TransferProgress] {
        core.progress.values.sorted { $0.session_id < $1.session_id }
    }

    private var filtered: [HistoryRecord] {
        core.history.filter { r in
            let dirOK: Bool
            switch dir {
            case .all: dirOK = true
            case .sent: dirOK = r.direction == "send"
            case .received: dirOK = r.direction != "send"
            }
            guard dirOK else { return false }
            switch status {
            case .all: return true
            case .done: return r.status == "completed"
            case .failed: return r.status != "completed"
            }
        }
    }

    private var subtitle: String {
        if filtered.isEmpty {
            return isFiltering ? "没有符合条件的记录" : "还没有传输记录"
        }
        let total = filtered.reduce(0) { $0 + $1.bytes_done }
        let prefix = isFiltering ? "\(dir.label) · \(status.label)" : "全部"
        return "\(prefix) · \(filtered.count) 项 · \(prettySize(total))"
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: DS.Spacing.lg) {
                header

                if isFiltering { filterChip }

                if !active.isEmpty {
                    VStack(spacing: DS.Spacing.sm) {
                        ForEach(active) { ActiveCard(p: $0) }
                    }
                }

                if filtered.isEmpty && active.isEmpty {
                    EmptyStateView(
                        icon: isFiltering ? "line.3.horizontal.decrease.circle" : "arrow.left.arrow.right",
                        title: isFiltering ? "没有符合条件的记录" : "还没有传输",
                        subtitle: isFiltering ? "换个筛选条件试试" : "去「附近」选择设备并发送文件"
                    )
                }

                ForEach(groups, id: \.0) { group in
                    VStack(alignment: .leading, spacing: DS.Spacing.sm) {
                        HStack {
                            Text(group.0).font(.system(size: 12, weight: .semibold)).foregroundStyle(.secondary)
                            Spacer()
                            Text("\(group.1.count) 项").font(Type.caption()).foregroundStyle(.tertiary)
                        }
                        VStack(spacing: 0) {
                            ForEach(Array(group.1.enumerated()), id: \.element.id) { idx, r in
                                HistoryRow(record: r)
                                    .contextMenu {
                                        Button(role: .destructive) { core.deleteHistory(r.session_id) } label: {
                                            Label("删除记录", systemImage: "trash")
                                        }
                                    }
                                if idx < group.1.count - 1 {
                                    Divider().overlay(Color.ssHairline).padding(.leading, 62)
                                }
                            }
                        }
                        .ssCard()
                    }
                }
            }
            .padding(.horizontal, DS.Spacing.md)
            .padding(.top, DS.Spacing.sm)
            .padding(.bottom, DS.Spacing.xl)
        }
        .background(AmbientBackground())
        .scrollIndicators(.hidden)
        .toolbar(.hidden, for: .navigationBar)
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text("传输").font(Type.display())
                Text(subtitle).font(Type.footnote()).foregroundStyle(.secondary)
            }
            Spacer()
            Menu {
                Picker("方向", selection: $dir) {
                    ForEach(DirFilter.allCases) { Text($0.label).tag($0) }
                }
                Picker("状态", selection: $status) {
                    ForEach(StatusFilter.allCases) { Text($0.label).tag($0) }
                }
                Divider()
                Button("清空全部记录", systemImage: "trash", role: .destructive) { core.clearHistory() }
            } label: {
                Image(systemName: "line.3.horizontal.decrease.circle")
                    .font(.system(size: 16, weight: .medium))
                    .foregroundStyle(isFiltering ? Color.indigo : Color.primary)
                    .frame(width: 40, height: 40)
                    .background(Circle().fill(isFiltering ? Color.indigo.opacity(0.14) : Color.ssField))
                    .overlay(Circle().strokeBorder(isFiltering ? Color.indigo.opacity(0.30) : Color.ssCardBorder, lineWidth: 0.5))
            }
        }
        .padding(.top, DS.Spacing.sm)
    }

    private var filterChip: some View {
        HStack(spacing: 6) {
            Image(systemName: "line.3.horizontal.decrease")
            Text("\(dir.label) · \(status.label)")
            Button {
                withAnimation(DS.Anim.snappy) { dir = .all; status = .all }
            } label: {
                Image(systemName: "xmark")
            }
        }
        .font(.system(size: 12, weight: .medium))
        .foregroundStyle(Color.indigo)
        .padding(.horizontal, 10)
        .frame(height: 28)
        .background(Capsule().fill(Color.indigo.opacity(0.14)))
        .overlay(Capsule().strokeBorder(Color.indigo.opacity(0.30), lineWidth: 0.5))
    }

    // MARK: 分天

    private var groups: [(String, [HistoryRecord])] {
        let cal = Calendar.current
        var order: [String] = []
        var map: [String: [HistoryRecord]] = [:]
        for r in filtered {
            let d = Date(timeIntervalSince1970: Double(r.ended_at_ms) / 1000)
            let key: String
            if cal.isDateInToday(d) { key = "今天" }
            else if cal.isDateInYesterday(d) { key = "昨天" }
            else { let f = DateFormatter(); f.dateFormat = "M月d日"; key = f.string(from: d) }
            if map[key] == nil { order.append(key) }
            map[key, default: []].append(r)
        }
        return order.map { ($0, map[$0] ?? []) }
    }
}

// MARK: - 筛选

private enum DirFilter: String, CaseIterable, Identifiable {
    case all, sent, received
    var id: String { rawValue }
    var label: String {
        switch self {
        case .all: return "全部"
        case .sent: return "发送"
        case .received: return "接收"
        }
    }
}

private enum StatusFilter: String, CaseIterable, Identifiable {
    case all, done, failed
    var id: String { rawValue }
    var label: String {
        switch self {
        case .all: return "全部"
        case .done: return "完成"
        case .failed: return "失败"
        }
    }
}

// MARK: - 进行中卡片

private struct ActiveCard: View {
    let p: TransferProgress
    var body: some View {
        VStack(spacing: DS.Spacing.md) {
            HStack(spacing: DS.Spacing.md) {
                ZStack {
                    Circle().stroke(Color.white.opacity(0.15), lineWidth: 3)
                    Circle()
                        .trim(from: 0, to: max(0.02, p.fraction))
                        .stroke(Color.white, style: StrokeStyle(lineWidth: 3, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                        .animation(.linear(duration: 0.3), value: p.fraction)
                    Text("\(Int(p.fraction * 100))%")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(.white)
                }
                .frame(width: 54, height: 54)

                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Circle().fill(.white.opacity(0.85)).frame(width: 6, height: 6)
                        Text(p.files_total > 1 ? "正在发送 \(p.files_done)/\(p.files_total) 个文件" : "正在发送")
                            .font(Type.titleSmall()).foregroundStyle(.white).lineLimit(1)
                    }
                    Text(prettySize(p.bytes_total))
                        .font(Type.caption()).foregroundStyle(.white.opacity(0.65))
                    HStack(spacing: 6) {
                        Image(systemName: "bolt.fill")
                        Text("\(prettySize(p.speed_bps))/s")
                        Text("·").foregroundStyle(.white.opacity(0.3))
                        Image(systemName: "lock.fill")
                        Text("加密")
                    }
                    .font(.system(size: 11)).foregroundStyle(.white.opacity(0.6))
                }
                Spacer(minLength: 0)
            }
            ProgressView(value: p.fraction).tint(.white)
        }
        .padding(DS.Spacing.md)
        .background(
            RoundedRectangle(cornerRadius: DS.Radius.xl, style: .continuous)
                .fill(LinearGradient(colors: [Color.indigo.opacity(0.9), Color.indigo.opacity(0.65)],
                                     startPoint: .top, endPoint: .bottom))
        )
        .overlay(
            RoundedRectangle(cornerRadius: DS.Radius.xl, style: .continuous)
                .strokeBorder(Color.white.opacity(0.12), lineWidth: 0.5)
        )
    }
}

// MARK: - 历史行

private struct HistoryRow: View {
    let record: HistoryRecord
    private var isOut: Bool { record.direction == "send" }
    private var title: String {
        guard let first = record.files.first?.name else { return "—" }
        return record.files.count > 1 ? "\(first) 等 \(record.files.count) 项" : first
    }
    private var ext: String { (record.files.first?.name as NSString? ?? "").pathExtension.lowercased() }

    var body: some View {
        HStack(spacing: DS.Spacing.md) {
            let tint = Color.fileTint(ext)
            ZStack {
                RoundedRectangle(cornerRadius: 12, style: .continuous).fill(tint.bg)
                Image(systemName: fileSymbol(record.files.first?.name ?? ""))
                    .font(.system(size: 18)).foregroundStyle(tint.fg)
            }
            .frame(width: 42, height: 42)

            VStack(alignment: .leading, spacing: 3) {
                Text(title).font(Type.titleSmall()).lineLimit(1).truncationMode(.middle)
                HStack(spacing: 4) {
                    Image(systemName: isOut ? "arrow.up.right" : "arrow.down.left")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(isOut ? Color.indigo : Color.green)
                    Text(isOut ? "发送给 \(record.peer_name)" : "来自 \(record.peer_name)")
                    Text("·")
                    Text(prettySize(record.bytes_done))
                }
                .font(Type.caption()).foregroundStyle(.secondary).lineLimit(1)
            }

            Spacer(minLength: DS.Spacing.xs)

            VStack(alignment: .trailing, spacing: 4) {
                Image(systemName: statusIcon(record.status))
                    .font(.system(size: 15)).foregroundStyle(statusColor(record.status))
                Text(formatRelative(record.ended_at_ms)).font(Type.caption()).foregroundStyle(.tertiary)
            }
        }
        .padding(.horizontal, DS.Spacing.md)
        .padding(.vertical, 12)
    }
}
