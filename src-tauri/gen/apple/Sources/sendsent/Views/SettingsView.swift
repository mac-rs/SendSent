import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var core: Core
    @State private var name = ""
    @State private var conns = 16
    @State private var chunkKb = 1024
    @State private var splitMb = 8

    var body: some View {
        Form {
            Section("本机") {
                TextField("显示名称", text: $name)
                    .onSubmit { core.rename(name) }
                LabeledContent("端口", value: "\(core.port)")
            }
            Section("传输参数") {
                Stepper("并发连接数：\(conns)", value: $conns, in: 1...32)
                Stepper("数据块：\(chunkKb) KB", value: $chunkKb, in: 64...1024, step: 64)
                Stepper("分片阈值：\(splitMb) MB", value: $splitMb, in: 1...1024)
                Button("保存") {
                    core.setConfig(conns: UInt32(conns), chunkKb: UInt64(chunkKb), splitMb: UInt64(splitMb))
                }
            }
            Section("关于") {
                LabeledContent("版本", value: "0.1.0")
                LabeledContent("技术栈", value: "Rust + SwiftUI")
            }
        }
        .navigationTitle("设置")
        .onAppear {
            name = core.identity?.name ?? ""
            if let c = try? ffiConfig() {
                conns = Int(c.conns)
                chunkKb = Int(c.chunk_size / 1024)
                splitMb = Int(c.split_threshold / 1_048_576)
            }
        }
    }
}
