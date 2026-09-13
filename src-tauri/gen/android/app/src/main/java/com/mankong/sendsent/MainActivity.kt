package com.mankong.sendsent

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.lifecycle.lifecycleScope
import com.mankong.sendsent.ui.App
import org.json.JSONObject
import java.io.File

class MainActivity : ComponentActivity() {
    private lateinit var saf: SafPicker
    private var pendingPeerId: String? = null
    private var pendingSecure = false
    private var pendingVerify = false

    private val openDocuments =
        registerForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            val peer = pendingPeerId
            if (peer != null && uris.isNotEmpty()) {
                Core.shared.send(peer, saf.filesJson(uris), pendingSecure, pendingVerify)
            }
            pendingPeerId = null
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        saf = SafPicker(this)

        val dataDir = File(filesDir, "sendsent").apply { mkdirs() }
        val saveDir = File(getExternalFilesDir(null) ?: filesDir, "sendsent").apply { mkdirs() }
        val rc = Native.nativeInit(dataDir.absolutePath, saveDir.absolutePath, 52225)

        val core = Core.shared
        if (rc == 0) {
            val identity = runCatching { JSONObject(Native.nativeIdentity()) }.getOrNull()
            val name = identity?.optString("name")?.takeIf { it.isNotEmpty() } ?: "Android"
            val nsd = NsdBridge(this)
            nsd.register(name, identity?.optString("device_id") ?: "", "android", 52225, "")
            nsd.browse()
            core.refresh()
            core.start(lifecycleScope)
        } else {
            core.flash("初始化失败 ($rc)")
        }

        setContent {
            App(core, onPickFiles = { peerId, secure, verify ->
                pendingPeerId = peerId
                pendingSecure = secure
                pendingVerify = verify
                openDocuments.launch(arrayOf("*/*"))
            })
        }
    }
}
