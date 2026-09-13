import SwiftUI

@main
struct SendSentApp: App {
    @StateObject private var core = Core.shared
    @AppStorage("appTheme") private var themeRaw = AppTheme.dark.rawValue

    init() {
        if let t = ProcessInfo.processInfo.environment["SS_THEME"], AppTheme(rawValue: t) != nil {
            UserDefaults.standard.set(t, forKey: "appTheme")
        }
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(core)
                .preferredColorScheme(AppTheme(rawValue: themeRaw)?.colorScheme)
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
    @State private var sel = RootView.initialTab

    private static var initialTab: Int {
        if let s = ProcessInfo.processInfo.environment["SS_TAB"], let v = Int(s), (0...3).contains(v) { return v }
        return 0
    }

    var body: some View {
        TabView(selection: $sel) {
            RadarView()
                .tabItem { Label("附近", systemImage: "dot.radiowaves.left.and.right") }.tag(0)
            TransfersView()
                .tabItem { Label("传输", systemImage: "arrow.left.arrow.right") }.tag(1)
            ProfileView()
                .tabItem { Label("我的", systemImage: "person.crop.circle") }.tag(2)
            SettingsView()
                .tabItem { Label("设置", systemImage: "gearshape") }.tag(3)
        }
        .tint(.indigo)
        .alert("提示", isPresented: Binding(get: { core.toast != nil }, set: { if !$0 { core.toast = nil } })) {
            Button("好", role: .cancel) {}
        } message: {
            Text(core.toast ?? "")
        }
        .sheet(isPresented: Binding(get: { core.request != nil }, set: { if !$0 { core.request = nil } })) {
            if let r = core.request {
                ReceivingSheet(
                    request: r,
                    onAccept: { core.respond(accept: true) },
                    onReject: { core.respond(accept: false) }
                )
                .presentationDetents([.height(400)])
                .presentationDragIndicator(.hidden)
                .presentationBackground(.thinMaterial)
            }
        }
    }
}
