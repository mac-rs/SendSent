import UIKit
import UniformTypeIdentifiers

// Picker delegate — captures file URLs and invokes the Rust callback.
private var activeCallback: (@convention(c) (UnsafePointer<CChar>?) -> Void)? = nil
private var activeController: UIDocumentPickerViewController? = nil

// Security-scoped URLs we currently hold access to. We keep the original files
// in place (no copy) and retain their scope until the app terminates so the
// Rust sender can read them directly. Large videos must not be copied.
private var activeURLs: [URL] = []

private class PickerDelegate: NSObject, UIDocumentPickerDelegate {
    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        var paths: [String] = []
        for url in urls {
            if url.startAccessingSecurityScopedResource() {
                activeURLs.append(url)
            }
            paths.append(url.path)
        }
        if let cb = activeCallback {
            let json = try? JSONSerialization.data(withJSONObject: paths)
            if let json, let str = String(data: json, encoding: .utf8) {
                str.withCString { cb($0) }
            } else {
                cb(nil)
            }
        }
        activeCallback = nil
        activeController = nil
    }

    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        if let cb = activeCallback { cb(nil) }
        activeCallback = nil
        activeController = nil
    }
}

private let delegate = PickerDelegate()

/// Find the currently-active window scene's top view controller.
private func topViewController() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
    let scene = scenes.first(where: { $0.activationState == .foregroundActive }) ?? scenes.first
    guard let window = scene?.windows.first(where: { $0.isKeyWindow }) ?? scene?.windows.first,
          var top = window.rootViewController else {
        return nil
    }
    while let presented = top.presentedViewController { top = presented }
    return top
}

@_cdecl("sendsent_pick_files")
func sendsentPickFiles(callback: @escaping @convention(c) (UnsafePointer<CChar>?) -> Void) {
    activeCallback = callback

    var types: [UTType] = []
    if #available(iOS 14.0, *) {
        types = [.data, .image, .movie, .audio, .pdf, .text, .archive]
    } else {
        types = [.data]
    }

    // `asCopy: false` keeps the picked file in place (no sandbox copy); we hold
    // its security scope instead.
    let picker = UIDocumentPickerViewController(forOpeningContentTypes: types, asCopy: false)
    picker.delegate = delegate
    picker.allowsMultipleSelection = true
    activeController = picker

    DispatchQueue.main.async {
        guard let presenter = topViewController() else {
            callback(nil)
            return
        }
        presenter.present(picker, animated: true)
    }
}
