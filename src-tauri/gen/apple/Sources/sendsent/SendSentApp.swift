import SwiftUI

@main
struct SendSentApp: App {
    @StateObject private var core = Core.shared
    var body: some Scene {
        WindowGroup {
            RootView().environmentObject(core)
        }
    }
}

func prettySize(_ b: UInt64) -> String {
    let f = Double(b)
    if f >= 1_073_741_824 { return String(format: "%.2f GB", f / 1_073_741_824) }
    if f >= 1_048_576 { return String(format: "%.1f MB", f / 1_048_576) }
    if f >= 1024 { return String(format: "%.0f KB", f / 1024) }
    return "\(b) B"
}

struct RootView: View {
    @EnvironmentObject var core: Core
    @State private var sel = 0

    var body: some View {
        TabView(selection: $sel) {
            NavigationStack { DevicesView() }
                .tabItem { Label("设备", systemImage: "dot.radiowaves.left.and.right") }.tag(0)
            NavigationStack { TransfersView() }
                .tabItem { Label("传输", systemImage: "arrow.left.arrow.right") }.tag(1)
            NavigationStack { ProfileView() }
                .tabItem { Label("我的", systemImage: "person.crop.circle") }.tag(2)
            NavigationStack { SettingsView() }
                .tabItem { Label("设置", systemImage: "gearshape") }.tag(3)
        }
        .alert("提示", isPresented: Binding(get: { core.toast != nil }, set: { if !$0 { core.toast = nil } })) {
            Button("好", role: .cancel) {}
        } message: {
            Text(core.toast ?? "")
        }
        .alert("收到文件", isPresented: Binding(get: { core.request != nil }, set: { if !$0 { core.request = nil } })) {
            Button("拒绝", role: .destructive) { core.respond(accept: false) }
            Button("接受") { core.respond(accept: true) }
        } message: {
            if let r = core.request {
                Text("\(r.sender.name) 想发送 \(r.manifest.total_count) 个文件 (\(prettySize(r.manifest.total_size)))")
            }
        }
    }
}
