import SwiftUI

struct AddDeviceView: View {
    @EnvironmentObject var core: Core
    @Environment(\.dismiss) private var dismiss
    @State private var addr = ""
    @State private var showScan = false

    var body: some View {
        Form {
            Section("手动输入 IP:port") {
                TextField("192.168.1.5:52225", text: $addr)
                    .keyboardType(.URL)
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                Button("添加") {
                    core.addPeer(addr.trimmingCharacters(in: .whitespaces))
                    dismiss()
                }
                .disabled(addr.trimmingCharacters(in: .whitespaces).isEmpty)
            }
            Section {
                Button { showScan = true } label: {
                    Label("扫码添加", systemImage: "qrcode.viewfinder")
                }
            }
        }
        .navigationTitle("添加设备")
        .toolbar { ToolbarItem(placement: .cancellationAction) { Button("关闭") { dismiss() } } }
        .sheet(isPresented: $showScan) {
            NavigationStack {
                ScanView { payload in
                    if let a = parseAddr(payload) { core.addPeer(a) }
                    showScan = false
                    dismiss()
                }
                .ignoresSafeArea()
                .navigationTitle("扫描二维码")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar { ToolbarItem(placement: .cancellationAction) { Button("取消") { showScan = false } } }
            }
        }
    }

    /// 解析 `sendsent://name?addr=ip:port&dir=...` 或裸 `ip:port`
    private func parseAddr(_ s: String) -> String? {
        let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.range(of: #"^[0-9.]+:[0-9]+$"#, options: .regularExpression) != nil { return t }
        guard t.hasPrefix("sendsent://"),
              let u = URLComponents(string: t.replacingOccurrences(of: "sendsent://", with: "http://")) else {
            return nil
        }
        return u.queryItems?.first(where: { $0.name == "addr" })?.value
    }
}
