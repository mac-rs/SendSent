import SwiftUI
import UIKit

struct ProfileView: View {
    @EnvironmentObject var core: Core
    @State private var qr: UIImage?
    @State private var showFullQr = false

    var body: some View {
        List {
            Section {
                VStack(spacing: 8) {
                    Text(String((core.identity?.name ?? "S").prefix(1)).uppercased())
                        .font(.system(size: 32, weight: .semibold))
                        .foregroundStyle(.secondary)
                        .frame(width: 76, height: 76)
                        .background(Color(.secondarySystemBackground), in: Circle())
                    Text(core.identity?.name ?? "未命名").font(.title2.bold())
                    Text(verbatim: "端口 \(core.port)").font(.footnote).foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity)
                .listRowBackground(Color.clear)
            }

            Section {
                Button { showFullQr = true } label: {
                    VStack(spacing: 12) {
                        if let qr {
                            Image(uiImage: qr)
                                .interpolation(.none)
                                .resizable()
                                .frame(width: 220, height: 220)
                                .padding(10)
                                .background(.white, in: RoundedRectangle(cornerRadius: 10))
                        } else {
                            ProgressView().frame(height: 220)
                        }
                        Text("让对方在 SendSent 里扫码，即可连接这台设备")
                            .font(.footnote).foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
                .buttonStyle(.plain)
            }

            if !core.addresses.isEmpty {
                Section("本机 IP") {
                    ForEach(core.addresses) { a in
                        HStack {
                            Text(verbatim: "\(a.ip):\(core.port)").font(.body.monospaced())
                            Spacer()
                            Text(a.interface).foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }
        .navigationTitle("我的")
        .task { loadQr() }
        .refreshable { loadQr() }
        .sheet(isPresented: $showFullQr) {
            NavigationStack {
                VStack {
                    if let qr {
                        Image(uiImage: qr).interpolation(.none).resizable()
                            .frame(width: 300, height: 300)
                            .padding()
                            .background(.white, in: RoundedRectangle(cornerRadius: 16))
                    }
                    Text("扫描后自动添加").foregroundStyle(.secondary).padding(.top)
                }
                .navigationTitle("我的二维码")
                .toolbar { ToolbarItem(placement: .confirmationAction) { Button("完成") { showFullQr = false } } }
            }
        }
    }

    private func loadQr() {
        if let b64 = try? ffiQr(), let data = Data(base64Encoded: b64) {
            qr = UIImage(data: data)
        }
    }
}
