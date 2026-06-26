package com.mankong.sendsent

import android.app.Activity
import android.net.Uri
import android.provider.OpenableColumns
import app.tauri.annotation.Command
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

class ContentPlugin(private val activity: Activity) : Plugin(activity) {
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
}
