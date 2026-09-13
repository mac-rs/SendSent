import Foundation

struct Identity: Codable, Identifiable {
    let device_id: String
    let name: String
    let platform: String
    var id: String { device_id }
}

struct Peer: Codable, Identifiable, Hashable {
    let device_id: String
    let name: String
    let platform: String
    let proto_version: UInt16
    let addrs: [String]
    let port: UInt16
    var id: String { device_id }
}

struct MyAddress: Codable, Identifiable {
    let interface: String
    let ip: String
    var id: String { "\(interface)-\(ip)" }
}

struct TransferConfig: Codable {
    let conns: UInt32
    let chunk_size: UInt64
    let split_threshold: UInt64
}

// 事件载荷(对齐 Rust serde;内部 tag = kind)
struct KindOnly: Decodable { let kind: String }
struct PeerFoundEvent: Decodable { let peer: Peer }
struct PeerLostEvent: Decodable { let device_id: String }

struct FileMeta: Decodable {
    let name: String
    let rel_path: String
    let size: UInt64
}
struct Manifest: Decodable {
    let session_id: String
    let files: [FileMeta]
    let total_size: UInt64
    let total_count: UInt64
}
struct TransferRequest: Decodable, Identifiable {
    let session_id: String
    let sender: Peer
    let manifest: Manifest
    var id: String { session_id }
}
struct TransferProgress: Decodable, Identifiable {
    let session_id: String
    let state: String
    let bytes_done: UInt64
    let bytes_total: UInt64
    let files_done: UInt64
    let files_total: UInt64
    let speed_bps: UInt64
    var id: String { session_id }
    var fraction: Double { bytes_total == 0 ? 0 : Double(bytes_done) / Double(bytes_total) }
}
struct ErrorPayload: Decodable { let code: String; let message: String }
struct TransferFinished: Decodable {
    let session_id: String
    let state: String
    let error: ErrorPayload?
}

struct HistoryFile: Decodable { let name: String; let size: UInt64; let rel_path: String }
struct HistoryRecord: Decodable, Identifiable {
    let session_id: String
    let direction: String
    let peer_name: String
    let files: [HistoryFile]
    let total_size: UInt64
    let bytes_done: UInt64
    let status: String
    let started_at_ms: Int64
    let ended_at_ms: Int64
    let save_dir: String?
    let error: String?
    var id: String { session_id }
}
