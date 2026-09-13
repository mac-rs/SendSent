package com.mankong.sendsent.ui

import android.content.Context

/** 主题偏好:system / light / dark。 */
object ThemePrefs {
    private const val PREF = "sendsent_ui"
    private const val KEY = "theme"

    fun get(ctx: Context): String =
        ctx.getSharedPreferences(PREF, Context.MODE_PRIVATE).getString(KEY, "dark") ?: "dark"

    fun set(ctx: Context, value: String) {
        ctx.getSharedPreferences(PREF, Context.MODE_PRIVATE).edit().putString(KEY, value).apply()
    }

    val options = listOf("system" to "跟随系统", "light" to "浅色", "dark" to "深色")
}
