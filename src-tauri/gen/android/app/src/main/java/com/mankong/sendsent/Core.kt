package com.mankong.sendsent

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

data class Peer(val id: String, val name: String, val platform: String, val addrs: List<String>, val port: Int)
data class Progress(
    val sessionId: String, val bytesDone: Long, val bytesTotal: Long,
    val filesDone: Long, val filesTotal: Long, val speedBps: Long,
) { val fraction: Float get() = if (bytesTotal <= 0) 0f else (bytesDone.toDouble() / bytesTotal).toFloat() }

data class HistoryItem(
    val sessionId: String, val direction: String, val peerName: String, val status: String,
    val firstFile: String, val fileCount: Int, val bytes: Long, val endedAtMs: Long,
)
data class RequestInfo(val sessionId: String, val senderName: String, val count: Long, val size: Long)
data class Identity(val deviceId: String, val name: String, val platform: String)
data class Config(val conns: Int, val chunkKb: Long, val splitMb: Long)
data class MyAddr(val iface: String, val ip: String)

class Core private constructor() {
    companion object { val shared = Core() }

    private val _peers = MutableStateFlow<List<Peer>>(emptyList())
    val peers: StateFlow<List<Peer>> = _peers

    /** 用户手动删除(隐藏)的设备 id。 */
    private val _hidden = MutableStateFlow<Set<String>>(emptySet())
    val hidden: StateFlow<Set<String>> = _hidden
    fun hide(id: String) { _hidden.value = _hidden.value + id }

    private val _progress = MutableStateFlow<Map<String, Progress>>(emptyMap())
    val progress: StateFlow<Map<String, Progress>> = _progress

    private val _history = MutableStateFlow<List<HistoryItem>>(emptyList())
    val history: StateFlow<List<HistoryItem>> = _history

    private val _request = MutableStateFlow<RequestInfo?>(null)
    val request: StateFlow<RequestInfo?> = _request

    private val _identity = MutableStateFlow<Identity?>(null)
    val identity: StateFlow<Identity?> = _identity

    private val _addresses = MutableStateFlow<List<MyAddr>>(emptyList())
    val addresses: StateFlow<List<MyAddr>> = _addresses

    private val _toast = MutableStateFlow<String?>(null)
    val toast: StateFlow<String?> = _toast

    fun refresh() {
        runCatching { parseIdentity(Native.nativeIdentity()) }.getOrNull()?.let { _identity.value = it }
        runCatching { _peers.value = parsePeers(Native.nativePeers()) }
        runCatching { _history.value = parseHistory(Native.nativeHistory()) }
        runCatching { _addresses.value = parseAddresses(Native.nativeAddresses()) }
    }

    fun start(scope: CoroutineScope) {
        scope.launch {
            while (isActive) {
                pollOnce()
                delay(100)
            }
        }
    }

    private fun pollOnce() {
        val raw = runCatching { Native.nativePollEvents() }.getOrNull() ?: return
        val arr = runCatching { JSONArray(raw) }.getOrNull() ?: return
        for (i in 0 until arr.length()) {
            val o = runCatching { JSONObject(arr.getString(i)) }.getOrNull() ?: continue
            when (o.optString("kind")) {
                "peer_found" -> {
                    val p = parsePeer(o.optJSONObject("peer") ?: continue)
                    if (_peers.value.none { it.id == p.id }) _peers.value = _peers.value + p
                }
                "peer_lost" -> {
                    val id = o.optString("device_id")
                    _peers.value = _peers.value.filterNot { it.id == id }
                }
                "progress" -> {
                    val p = Progress(
                        o.getString("session_id"),
                        o.optLong("bytes_done"), o.optLong("bytes_total"),
                        o.optLong("files_done"), o.optLong("files_total"), o.optLong("speed_bps"),
                    )
                    _progress.value = _progress.value + (p.sessionId to p)
                }
                "finished" -> {
                    _progress.value = _progress.value - o.optString("session_id")
                    _history.value = parseHistory(Native.nativeHistory())
                }
                "recorded" -> {
                    _history.value = listOf(parseRecord(o)) + _history.value.filterNot { it.sessionId == o.optString("session_id") }
                }
                "request" -> {
                    val sender = o.optJSONObject("sender")
                    val manifest = o.optJSONObject("manifest")
                    _request.value = RequestInfo(
                        o.getString("session_id"),
                        sender?.optString("name") ?: "?",
                        manifest?.optLong("total_count") ?: 0,
                        manifest?.optLong("total_size") ?: 0,
                    )
                }
            }
        }
        runCatching { _peers.value = parsePeers(Native.nativePeers()) }
    }

    fun addPeer(addr: String) {
        val err = runCatching { Native.nativeAddPeer(addr) }.getOrNull()
        flash(err ?: "已添加 $addr")
    }
    fun send(peerId: String, filesJson: String, secure: Boolean, verify: Boolean) {
        val err = runCatching { Native.nativeSend(peerId, filesJson, secure, verify) }.getOrNull()
        flash(err ?: "已发送")
    }
    fun respond(accept: Boolean) {
        val sid = _request.value?.sessionId ?: return
        runCatching { Native.nativeRespond(sid, accept) }
        _request.value = null
    }
    fun deleteHistory(id: String) {
        runCatching { Native.nativeDeleteHistory(id) }
        _history.value = _history.value.filterNot { it.sessionId == id }
    }
    fun clearHistory() {
        runCatching { Native.nativeClearHistory() }
        _history.value = emptyList()
    }
    fun rename(name: String) { runCatching { Native.nativeSetDisplayName(name) }; refresh(); flash("已保存") }
    fun setConfig(conns: Int, chunkKb: Long, splitMb: Long) {
        runCatching { Native.nativeSetConfig(conns, chunkKb, splitMb) }
        flash("已保存")
    }
    fun config(): Config? =
        runCatching {
            val o = JSONObject(Native.nativeGetConfig())
            Config(o.optInt("conns"), o.optLong("chunk_size") / 1024, o.optLong("split_threshold") / 1048576)
        }.getOrNull()

    fun qrBase64(): String? =
        runCatching { org.json.JSONTokener(Native.nativeQr(512)).nextValue() as String }.getOrNull()

    fun flash(msg: String) { _toast.value = msg }
    fun clearToast() { _toast.value = null }

    private fun parseIdentity(raw: String?): Identity? {
        val o = JSONObject(raw ?: return null)
        return Identity(o.optString("device_id"), o.optString("name"), o.optString("platform"))
    }
    private fun parsePeer(o: JSONObject): Peer {
        val addrs = o.optJSONArray("addrs")?.let { a -> (0 until a.length()).map { a.optString(it) } } ?: emptyList()
        return Peer(o.optString("device_id"), o.optString("name"), o.optString("platform"), addrs, o.optInt("port"))
    }
    private fun parsePeers(raw: String?): List<Peer> {
        val a = JSONArray(raw ?: return emptyList())
        return (0 until a.length()).map { parsePeer(a.getJSONObject(it)) }
    }
    private fun parseAddresses(raw: String?): List<MyAddr> {
        val a = JSONArray(raw ?: return emptyList())
        return (0 until a.length()).map { MyAddr(a.getJSONObject(it).optString("interface"), a.getJSONObject(it).optString("ip")) }
    }
    private fun parseHistory(raw: String?): List<HistoryItem> {
        val a = JSONArray(raw ?: return emptyList())
        return (0 until a.length()).map { parseRecord(a.getJSONObject(it)) }
    }
    private fun parseRecord(o: JSONObject): HistoryItem {
        val files = o.optJSONArray("files")
        val first = files?.optJSONObject(0)?.optString("name") ?: "—"
        return HistoryItem(
            o.optString("session_id"), o.optString("direction"), o.optString("peer_name"),
            o.optString("status"), first, files?.length() ?: 0, o.optLong("bytes_done"), o.optLong("ended_at_ms"),
        )
    }
}
