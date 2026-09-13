import Foundation

typealias EventCallback = @convention(c) (UnsafePointer<CChar>?) -> Void

@_silgen_name("sendsent_ios_init")
func sendsent_ios_init(_ dataDir: UnsafePointer<CChar>?, _ saveDir: UnsafePointer<CChar>?, _ port: UInt16, _ cb: EventCallback?) -> Int32
@_silgen_name("sendsent_ios_free_string")
func sendsent_ios_free_string(_ ptr: UnsafeMutablePointer<CChar>?)
@_silgen_name("sendsent_ios_identity")
func sendsent_ios_identity() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_peers")
func sendsent_ios_peers() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_addresses")
func sendsent_ios_addresses() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_qr")
func sendsent_ios_qr(_ size: UInt32, _ ip: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_add_peer")
func sendsent_ios_add_peer(_ addr: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_send")
func sendsent_ios_send(_ peerId: UnsafePointer<CChar>?, _ filesJson: UnsafePointer<CChar>?, _ secure: Bool, _ verify: Bool) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_respond")
func sendsent_ios_respond(_ sessionId: UnsafePointer<CChar>?, _ accept: Bool) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_history")
func sendsent_ios_history() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_clear_history")
func sendsent_ios_clear_history() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_get_transfer_config")
func sendsent_ios_get_transfer_config() -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_set_transfer_config")
func sendsent_ios_set_transfer_config(_ conns: UInt32, _ chunkKb: UInt64, _ splitMb: UInt64) -> UnsafeMutablePointer<CChar>?
@_silgen_name("sendsent_ios_set_display_name")
func sendsent_ios_set_display_name(_ name: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

enum FFIError: Error, LocalizedError {
    case message(String)
    var errorDescription: String? { if case let .message(m) = self { return m }; return nil }
}

/// 解出 err-json 抛错;否则返回原始字符串(并释放指针)。
private func take(_ raw: UnsafeMutablePointer<CChar>?) throws -> String? {
    guard let p = raw else { return nil }
    defer { sendsent_ios_free_string(p) }
    let s = String(cString: p)
    if let d = s.data(using: .utf8),
       let obj = try? JSONSerialization.jsonObject(with: d) as? [String: Any],
       let err = obj["error"] as? String {
        throw FFIError.message(err)
    }
    return s
}

func decodeResult<T: Decodable>(_ type: T.Type, _ raw: UnsafeMutablePointer<CChar>?) throws -> T? {
    guard let s = try take(raw) else { return nil }
    return try JSONDecoder().decode(T.self, from: Data(s.utf8))
}

func throwIfError(_ raw: UnsafeMutablePointer<CChar>?) throws {
    _ = try take(raw)
}

func ffiIdentity() throws -> Identity? { try decodeResult(Identity.self, sendsent_ios_identity()) }
func ffiPeers() throws -> [Peer] { try decodeResult([Peer].self, sendsent_ios_peers()) ?? [] }
func ffiAddresses() throws -> [MyAddress] { try decodeResult([MyAddress].self, sendsent_ios_addresses()) ?? [] }
func ffiHistory() throws -> [HistoryRecord] { try decodeResult([HistoryRecord].self, sendsent_ios_history()) ?? [] }
func ffiConfig() throws -> TransferConfig? { try decodeResult(TransferConfig.self, sendsent_ios_get_transfer_config()) }

func ffiQr(size: UInt32 = 512, ip: String? = nil) throws -> String {
    let raw: UnsafeMutablePointer<CChar>?
    if let ip { raw = ip.withCString { sendsent_ios_qr(size, $0) } }
    else { raw = sendsent_ios_qr(size, nil) }
    return try decodeResult(String.self, raw) ?? ""
}

func ffiAddPeer(_ addr: String) throws {
    try addr.withCString { try throwIfError(sendsent_ios_add_peer($0)) }
}

func ffiSend(peerId: String, files: [String], secure: Bool, verify: Bool) throws -> String {
    let filesJson = String(data: try JSONEncoder().encode(files), encoding: .utf8) ?? "[]"
    let raw = peerId.withCString { pid in filesJson.withCString { fj in sendsent_ios_send(pid, fj, secure, verify) } }
    let r: [String: String]? = try decodeResult([String: String].self, raw)
    guard let sid = r?["session_id"] else { throw FFIError.message("send failed") }
    return sid
}

func ffiRespond(sessionId: String, accept: Bool) throws {
    try sessionId.withCString { try throwIfError(sendsent_ios_respond($0, accept)) }
}
func ffiClearHistory() throws { try throwIfError(sendsent_ios_clear_history()) }
func ffiSetConfig(conns: UInt32, chunkKb: UInt64, splitMb: UInt64) throws {
    try throwIfError(sendsent_ios_set_transfer_config(conns, chunkKb, splitMb))
}
func ffiSetDisplayName(_ name: String) throws {
    try name.withCString { try throwIfError(sendsent_ios_set_display_name($0)) }
}
