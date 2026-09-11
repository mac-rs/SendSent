package com.mankong.sendsent

import android.app.Activity
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import app.tauri.annotation.Command
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.concurrent.ConcurrentHashMap

class ContentPlugin(private val activity: Activity) : Plugin(activity) {
    // Keep opened descriptors alive for the lifetime of the process so Rust can
    // read the original file (via `/proc/self/fd/<fd>`) without copying it.
    private val openFds = ConcurrentHashMap<Int, ParcelFileDescriptor>()

    @Command
    fun getDisplayName(invoke: Invoke) {
        try {
            val uri = invoke.getArgs().optString("uri")
            if (uri.isNullOrEmpty()) {
                invoke.reject("missing uri", null, null, null)
                return
            }
            val resolver = activity.contentResolver
            var name: String? = null
            resolver.query(Uri.parse(uri), arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                if (c.moveToFirst()) name = c.getString(0)
            }
            val ret = JSObject()
            ret.put("name", name ?: "file")
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject(e.message, null, null, null)
        }
    }

    /** Open a `content://` URI read-only and return its OS file descriptor. */
    @Command
    fun openFd(invoke: Invoke) {
        try {
            val uri = invoke.getArgs().optString("uri")
            if (uri.isNullOrEmpty()) {
                invoke.reject("missing uri", null, null, null)
                return
            }
            val pfd = activity.contentResolver.openFileDescriptor(Uri.parse(uri), "r")
            if (pfd == null) {
                invoke.reject("openFileDescriptor returned null", null, null, null)
                return
            }
            val fd = pfd.fd
            openFds[fd] = pfd
            val ret = JSObject()
            ret.put("fd", fd)
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject(e.message, null, null, null)
        }
    }
}
