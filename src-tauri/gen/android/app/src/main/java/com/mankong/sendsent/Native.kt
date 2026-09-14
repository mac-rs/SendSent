package com.mankong.sendsent

object Native {
    init { System.loadLibrary("sendsent_lib") }

    external fun nativeInit(dataDir: String, saveDir: String, port: Int): Int
    external fun nativePollEvents(): String
    external fun nativeIdentity(): String
    external fun nativePeers(): String
    external fun nativeHistory(): String
    external fun nativeAddresses(): String
    external fun nativeQr(size: Int): String
    external fun nativeGetConfig(): String
    external fun nativeAddPeer(addr: String): String?
    external fun nativeSend(peerId: String, filesJson: String, secure: Boolean, verify: Boolean): String?
    external fun nativeRespond(sessionId: String, accept: Boolean): String?
    external fun nativeDeleteHistory(sessionId: String): String?
    external fun nativeClearHistory()
    external fun nativeSetConfig(conns: Int, chunkKb: Long, splitMb: Long, zerocopy: Boolean): String?
    external fun nativeSetDisplayName(name: String): String?
    external fun nativeOnService(name: String, host: String, port: Int, txtJson: String)
    external fun nativeOnServiceLost(name: String)
}
