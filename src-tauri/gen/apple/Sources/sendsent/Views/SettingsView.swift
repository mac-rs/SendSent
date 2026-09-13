import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var core: Core
    @AppStorage("appTheme") private var themeRaw = AppTheme.dark.rawValue
    @State private var name = ""
    @State private var conns = 16
    @State private var chunkKb = 1024
    @State private var splitMb = 8
    @State private var loaded = false

    var body: some View {
        NavigationStack {
            List {
                Section("本机") {
                    HStack {
                        Text("显示名称")
                        Spacer()
                        TextField("名称", text: $name)
                            .multilineTextAlignment(.trailing)
                            .foregroundStyle(.secondary)
                            .onSubmit { core.rename(name) }
                    }
                    LabeledContent("端口", value: "\(core.port)")
                }

                Section("外观") {
                    Picker("主题", selection: $themeRaw) {
                        ForEach(AppTheme.allCases) { Text($0.label).tag($0.rawValue) }
                    }
                    .pickerStyle(.segmented)
                }

                Section {
                    Stepper(value: $conns, in: 1...32) {
                        numRow("并发连接数", "\(conns)")
                    }
                    Stepper(value: $chunkKb, in: 64...1024, step: 64) {
                        numRow("数据块", "\(chunkKb) KB")
                    }
                    Stepper(value: $splitMb, in: 1...1024) {
                        numRow("分片阈值", "\(splitMb) MB")
                    }
                } header: {
                    Text("传输")
                } footer: {
                    Text("连接越多越快、占用越高;超过阈值的文件会切段并行传输。")
                }

                Section("关于") {
                    LabeledContent("版本", value: "0.1.0")
                    LabeledContent("技术栈", value: "Rust + SwiftUI")
                }
            }
            .listStyle(.insetGrouped)
            .scrollContentBackground(.hidden)
            .background(AmbientBackground())
            .scrollDismissesKeyboard(.interactively)
            .navigationTitle("设置")
            .navigationBarTitleDisplayMode(.large)
            .onAppear(perform: load)
            .onChange(of: conns) { _, _ in save() }
            .onChange(of: chunkKb) { _, _ in save() }
            .onChange(of: splitMb) { _, _ in save() }
        }
    }

    private func numRow(_ title: String, _ value: String) -> some View {
        HStack {
            Text(title)
            Spacer()
            Text(value).foregroundStyle(.secondary).monospacedDigit()
        }
    }

    private func load() {
        name = core.identity?.name ?? ""
        if let c = try? ffiConfig() {
            conns = Int(c.conns); chunkKb = Int(c.chunk_size / 1024); splitMb = Int(c.split_threshold / 1_048_576)
        }
        DispatchQueue.main.async { loaded = true }
    }

    private func save() {
        guard loaded else { return }
        core.setConfig(conns: UInt32(conns), chunkKb: UInt64(chunkKb), splitMb: UInt64(splitMb))
    }
}
