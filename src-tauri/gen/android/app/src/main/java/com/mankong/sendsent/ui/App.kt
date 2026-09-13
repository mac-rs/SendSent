@file:OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)

package com.mankong.sendsent.ui

import android.graphics.BitmapFactory
import android.util.Base64
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.mankong.sendsent.Core
import com.mankong.sendsent.HistoryItem
import com.mankong.sendsent.Peer
import com.mankong.sendsent.Progress
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun App(core: Core, onPickFiles: (String, Boolean, Boolean) -> Unit) {
    MaterialTheme {
        var tab by remember { mutableIntStateOf(0) }
        val snackbar = remember { SnackbarHostState() }
        val toast by core.toast.collectAsState()
        LaunchedEffect(toast) {
            toast?.let {
                snackbar.showSnackbar(it)
                core.clearToast()
            }
        }

        val request by core.request.collectAsState()
        if (request != null) {
            val r = request!!
            AlertDialog(
                onDismissRequest = { core.respond(false) },
                title = { Text("收到文件") },
                text = { Text("${r.senderName} 想发送 ${r.count} 个文件 (${size(r.size)})") },
                confirmButton = { TextButton({ core.respond(true) }) { Text("接受") } },
                dismissButton = { TextButton({ core.respond(false) }) { Text("拒绝") } },
            )
        }

        Scaffold(
            snackbarHost = { SnackbarHost(snackbar) },
            bottomBar = {
                NavigationBar {
                    NavigationBarItem(tab == 0, { tab = 0 }, { Icon(Icons.Default.Wifi, null) }, label = { Text("设备") })
                    NavigationBarItem(tab == 1, { tab = 1 }, { Icon(Icons.Default.Send, null) }, label = { Text("传输") })
                    NavigationBarItem(tab == 2, { tab = 2 }, { Icon(Icons.Default.Person, null) }, label = { Text("我的") })
                    NavigationBarItem(tab == 3, { tab = 3 }, { Icon(Icons.Default.Settings, null) }, label = { Text("设置") })
                }
            },
        ) { pad ->
            Box(Modifier.padding(pad).fillMaxSize()) {
                when (tab) {
                    0 -> DevicesScreen(core, onPickFiles)
                    1 -> TransfersScreen(core)
                    2 -> ProfileScreen(core)
                    else -> SettingsScreen(core)
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DevicesScreen(core: Core, onPickFiles: (String, Boolean, Boolean) -> Unit) {
    val peers by core.peers.collectAsState()
    val progress by core.progress.collectAsState()
    var selected by remember { mutableStateOf<Set<String>>(emptySet()) }
    var secure by remember { mutableStateOf(false) }
    var verify by remember { mutableStateOf(false) }
    var showAdd by remember { mutableStateOf(false) }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(title = { Text("设备") }, actions = {
            IconButton({ showAdd = true }) { Icon(Icons.Default.Add, "添加") }
        })

        progress.values.firstOrNull()?.let { ActiveBanner(it) }

        if (peers.isEmpty()) {
            Box(Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                Text("正在发现附近设备…", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        } else {
            LazyColumn(Modifier.weight(1f), contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp)) {
                items(peers, key = { it.id }) { p ->
                    val isSel = p.id in selected
                    Card(
                        Modifier.fillMaxWidth().padding(vertical = 4.dp).clickable {
                            selected = if (isSel) selected - p.id else selected + p.id
                        },
                    ) {
                        Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                            Avatar(p)
                            Spacer(Modifier.width(12.dp))
                            Column(Modifier.weight(1f)) {
                                Text(p.name, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                Text(
                                    "${platform(p.platform)} · ${p.addrs.firstOrNull() ?: ":${p.port}"}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            if (isSel) Icon(Icons.Default.Check, null, tint = MaterialTheme.colorScheme.primary)
                        }
                    }
                }
            }
        }

        if (selected.isNotEmpty()) {
            Surface(tonalElevation = 3.dp) {
                Column(Modifier.padding(12.dp)) {
                    Row {
                        Text("加密传输", Modifier.weight(1f))
                        Switch(secure, { secure = it })
                        Spacer(Modifier.width(8.dp))
                        Text("SHA-256", Modifier.weight(1f))
                        Switch(verify, { verify = it })
                    }
                    Spacer(Modifier.height(8.dp))
                    Button(
                        onClick = { onPickFiles(selected.first(), secure, verify) },
                        modifier = Modifier.fillMaxWidth(),
                    ) { Icon(Icons.Default.Send, null); Spacer(Modifier.width(6.dp)); Text("选择文件发送") }
                }
            }
        }
    }

    if (showAdd) AddDeviceDialog(core) { showAdd = false }
}

@Composable
private fun TransfersScreen(core: Core) {
    val active by core.progress.collectAsState()
    val history by core.history.collectAsState()
    var t by remember { mutableIntStateOf(0) }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(title = { Text("传输") }, actions = {
            if (t == 1 && history.isNotEmpty()) {
                IconButton({ core.clearHistory() }) { Icon(Icons.Default.Delete, "清空") }
            }
        })
        TabRow(t) {
            Tab(selected = t == 0, onClick = { t = 0 }, text = { Text("进行中") })
            Tab(selected = t == 1, onClick = { t = 1 }, text = { Text("历史") })
        }
        if (t == 0) {
            if (active.isEmpty()) Empty("暂无进行中的传输")
            else LazyColumn { items(active.values.toList(), key = { it.sessionId }) { ActiveRow(it) } }
        } else {
            if (history.isEmpty()) Empty("暂无历史记录")
            else LazyColumn {
                items(history, key = { it.sessionId }) { h -> HistoryRow(h) { core.deleteHistory(h.sessionId) } }
            }
        }
    }
}

@Composable
private fun ProfileScreen(core: Core) {
    val identity by core.identity.collectAsState()
    val addresses by core.addresses.collectAsState()
    val b64 = remember { core.qrBase64() }
    val bmp = remember(b64) {
        b64?.let {
            val bytes = Base64.decode(it, Base64.DEFAULT)
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
        }
    }

    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(16.dp)) {
        item {
            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                Text(
                    (identity?.name ?: "?").take(1).uppercase(),
                    style = MaterialTheme.typography.headlineLarge,
                    modifier = Modifier.clip(CircleShape),
                )
                Text(identity?.name ?: "未命名", style = MaterialTheme.typography.titleLarge)
                Text("端口 52225", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        item {
            if (bmp != null) {
                Box(Modifier.fillMaxWidth().padding(16.dp), contentAlignment = Alignment.Center) {
                    Image(bmp.asImageBitmap(), null, Modifier.size(220.dp).clip(RoundedCornerShape(12.dp)))
                }
            }
        }
        item { Text("本机 IP", style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 8.dp, bottom = 4.dp)) }
        items(addresses) { a ->
            ListItem(
                headlineContent = { Text("${a.ip}:52225", fontFamily = FontFamily.Monospace) },
                supportingContent = { Text(a.iface) },
            )
        }
    }
}

@Composable
private fun SettingsScreen(core: Core) {
    val identity by core.identity.collectAsState()
    var name by remember(identity) { mutableStateOf(identity?.name ?: "") }
    val cfg = remember { core.config() }
    var conns by remember { mutableLongStateOf((cfg?.conns ?: 16).toLong()) }
    var chunk by remember { mutableLongStateOf(cfg?.chunkKb ?: 1024) }
    var split by remember { mutableLongStateOf(cfg?.splitMb ?: 8) }

    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(16.dp)) {
        item {
            OutlinedTextField(name, { name = it }, label = { Text("显示名称") }, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            Button({ core.rename(name) }, Modifier.fillMaxWidth()) { Text("保存名称") }
        }
        item { Text("传输参数", style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 16.dp)) }
        item { Stepper("并发连接数", conns) { conns = it } }
        item { Stepper("数据块 (KB)", chunk) { chunk = it } }
        item { Stepper("分片阈值 (MB)", split) { split = it } }
        item {
            Spacer(Modifier.height(8.dp))
            Button({ core.setConfig(conns.toInt(), chunk, split) }, Modifier.fillMaxWidth()) { Text("保存参数") }
        }
    }
}

// ── 小组件 ──

@Composable
private fun Stepper(label: String, value: Long, onChange: (Long) -> Unit) {
    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("$label: $value", Modifier.weight(1f))
        OutlinedButton({ onChange(value - 1) }) { Text("−") }
        Spacer(Modifier.width(8.dp))
        OutlinedButton({ onChange(value + 1) }) { Text("+") }
    }
}

@Composable
private fun Avatar(p: Peer) {
    val color = when (p.platform) {
        "ios" -> Color(0xFF0A84FF)
        "android" -> Color(0xFF34C759)
        "windows" -> Color(0xFF0078D4)
        "linux" -> Color(0xFFFF9500)
        else -> Color(0xFF8E8E93)
    }
    Box(
        Modifier.size(40.dp).clip(RoundedCornerShape(10.dp)).background(color),
        contentAlignment = Alignment.Center,
    ) {
        Text(p.name.take(1).uppercase(), color = Color.White, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun ActiveBanner(p: Progress) {
    Card(Modifier.fillMaxWidth().padding(12.dp)) {
        Column(Modifier.padding(12.dp)) {
            Text("正在发送", style = MaterialTheme.typography.titleSmall)
            Spacer(Modifier.height(6.dp))
            LinearProgressIndicator({ p.fraction }, Modifier.fillMaxWidth())
            Spacer(Modifier.height(4.dp))
            Text(
                "${size(p.speedBps)}/s · ${size(p.bytesDone)} / ${size(p.bytesTotal)} · ${(p.fraction * 100).toInt()}%",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun ActiveRow(p: Progress) {
    ListItem(
        headlineContent = { Text(if (p.filesTotal > 1) "发送 ${p.filesDone}/${p.filesTotal} 个文件" else "正在发送") },
        supportingContent = {
            Column {
                LinearProgressIndicator({ p.fraction }, Modifier.fillMaxWidth().padding(vertical = 4.dp))
                Text("${size(p.speedBps)}/s · ${size(p.bytesDone)} / ${size(p.bytesTotal)}")
            }
        },
    )
}

@Composable
private fun HistoryRow(h: HistoryItem, onDelete: () -> Unit) {
    ListItem(
        headlineContent = { Text(if (h.fileCount > 1) "${h.firstFile} 等 ${h.fileCount} 个文件" else h.firstFile, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    if (h.direction == "send") Icons.Default.ArrowUpward else Icons.Default.ArrowDownward,
                    null, Modifier.size(14.dp),
                    tint = if (h.direction == "send") Color(0xFF0A84FF) else Color(0xFF34C759),
                )
                Spacer(Modifier.width(4.dp))
                Text("${if (h.direction == "send") "发送" else "接收"} · ${h.peerName} · ${size(h.bytes)}")
            }
        },
        leadingContent = {
            Icon(
                if (h.status == "completed") Icons.Default.Check else Icons.Default.MoreVert,
                null,
                tint = if (h.status == "completed") Color(0xFF34C759) else Color(0xFF8E8E93),
            )
        },
        trailingContent = { IconButton(onDelete) { Icon(Icons.Default.Delete, "删除") } },
    )
}

@Composable
private fun Empty(text: String) {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun AddDeviceDialog(core: Core, onClose: () -> Unit) {
    var addr by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onClose,
        title = { Text("添加设备") },
        text = { OutlinedTextField(addr, { addr = it }, label = { Text("IP:port") }, singleLine = true) },
        confirmButton = { TextButton({ core.addPeer(addr.trim()); onClose() }, enabled = addr.isNotBlank()) { Text("添加") } },
        dismissButton = { TextButton(onClose) { Text("取消") } },
    )
}

private fun size(b: Long): String {
    val f = b.toDouble()
    return when {
        f >= 1_073_741_824 -> String.format(Locale.US, "%.2f GB", f / 1_073_741_824)
        f >= 1_048_576 -> String.format(Locale.US, "%.1f MB", f / 1_048_576)
        f >= 1024 -> String.format(Locale.US, "%.0f KB", f / 1024)
        else -> "$b B"
    }
}

private fun platform(p: String) = when (p) {
    "macos" -> "macOS"; "ios" -> "iOS"; "android" -> "Android"; "windows" -> "Windows"; "linux" -> "Linux"; else -> "Unknown"
}
