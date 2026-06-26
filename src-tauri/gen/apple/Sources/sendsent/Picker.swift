import UIKit
import UniformTypeIdentifiers

// Picker delegate — captures file URLs and invokes the Rust callback.
private var activeCallback: (@convention(c) (UnsafePointer<CChar>?) -> Void)? = nil
private var activeController: UIDocumentPickerViewController? = nil

private class PickerDelegate: NSObject, UIDocumentPickerDelegate {
    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        let paths = urls.compactMap { url -> String? in
            guard url.startAccessingSecurityScopedResource() else { return nil }
            defer { url.stopAccessingSecurityScopedResource() }
            // Copy the picked file to a temp directory the Rust side can read
            let tmpDir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            try? FileManager.default.createDirectory(at: tmpDir, withIntermediateDirectories: true)
            let dest = tmpDir.appendingPathComponent(url.lastPathComponent)
            try? FileManager.default.copyItem(at: url, to: dest)
            return dest.path
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

@_cdecl("sendsent_pick_files")
func sendsentPickFiles(callback: @escaping @convention(c) (UnsafePointer<CChar>?) -> Void) {
    activeCallback = callback

    var types: [UTType] = []
    if #available(iOS 14.0, *) {
        types = [.data, .image, .movie, .audio, .pdf, .text, .archive]
    } else {
        types = [.data]
    }

    let picker = UIDocumentPickerViewController(forOpeningContentTypes: types, asCopy: true)
    picker.delegate = delegate
    picker.allowsMultipleSelection = true
    activeController = picker

    // Present on the key window's root
    DispatchQueue.main.async {
        guard let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
              let root = windowScene.keyWindow?.rootViewController else {
            callback(nil)
            return
        }
        var presenter: UIViewController = root
        while let presented = presenter.presentedViewController { presenter = presented }
        presenter.present(picker, animated: true)
    }
}
