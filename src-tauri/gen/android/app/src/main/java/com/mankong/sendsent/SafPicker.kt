package com.mankong.sendsent

import android.content.Context
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap

/** SAF:打开 URI 的 fd,保持存活,组 `[{"fd","name"}]` 交给 Rust 零拷贝读取。 */
class SafPicker(private val ctx: Context) {
    private val keep = ConcurrentHashMap<Int, ParcelFileDescriptor>()

    fun filesJson(uris: List<Uri>): String {
        val arr = JSONArray()
        for (u in uris) {
            val pfd = runCatching { ctx.contentResolver.openFileDescriptor(u, "r") }.getOrNull() ?: continue
            keep[pfd.fd] = pfd
            arr.put(JSONObject().put("fd", pfd.fd).put("name", displayName(u) ?: "file"))
        }
        return arr.toString()
    }

    private fun displayName(u: Uri): String? {
        ctx.contentResolver.query(u, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
            if (c.moveToFirst()) return c.getString(0)
        }
        return null
    }
}
