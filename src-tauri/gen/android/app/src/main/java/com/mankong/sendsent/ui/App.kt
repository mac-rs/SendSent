@file:OptIn(
    androidx.compose.material3.ExperimentalMaterial3Api::class,
    androidx.compose.foundation.layout.ExperimentalLayoutApi::class,
)

package com.mankong.sendsent.ui

import android.graphics.BitmapFactory
import android.text.format.DateUtils
import android.util.Base64
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.QrCode
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Divider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Switch
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.mankong.sendsent.Core
import com.mankong.sendsent.HistoryItem
import com.mankong.sendsent.Peer
import com.mankong.sendsent.Progress
import java.util.Locale

@Composable
fun App(core: Core, onPickFiles: (String, Boolean, Boolean) -> Unit) {
    val context = LocalContext.current
    val dark = isSystemInDarkTheme()
    val scheme = if (android.os.Build.VERSION.SDK_INT >= 31) {
        if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
    } else {
        if (dark) darkColorScheme() else lightColorScheme()
    }

    MaterialTheme(colorScheme = scheme) {
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
        request?.let { r ->
            AlertDialog(
                onDismissRequest = { core.respond(false) },
                title = { Text("收到文件") },
                text = { Text("${r.senderName} 想发送 ${r.count} 个文件 (${size(r.size)})") },
                confirmButton = { Button({ core.respond(true) }) { Text("接受") } },
                dismissButton = { OutlinedButton({ core.respond(false) }) { Text("拒绝") } },
            )
        }

        Scaffold(
            snackbarHost = { SnackbarHost(snackbar) },
            contentWindowInsets = WindowInsets(0, 0, 0, 0),
            bottomBar = {
                NavigationBar {
                    NavigationBarItem(tab == 0, { tab = 0 }, { Icon(Icons.Default.Wifi, null) }, label = { Text("设备") })
                    NavigationBarItem(tab == 1, { tab = 1 }, { Icon(Icons.AutoMirrored.Filled.Send, null) }, label = { Text("传输") })
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

@Composable
private fun DevicesScreen(core: Core, onPickFiles: (String, Boolean, Boolean) -> Unit) {
    val allPeers by core.peers.collectAsState()
    val hidden by core.hidden.collectAsState()
    val progress by core.progress.collectAsState()
    var selected by remember { mutableStateOf<Set<String>>(emptySet()) }
    var secure by remember { mutableStateOf(false) }
    var verify by remember { mutableStateOf(false) }
    var showAdd by remember { mutableStateOf(false) }
    var detail by remember { mutableStateOf<Peer?>(null) }

    val peers = remember(allPeers, hidden) { allPeers.filterNot { it.id in hidden } }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(title = { Text("设备") }, actions = {
            IconButton({ showAdd = true }) { Icon(Icons.Default.Add, "添加") }
        })

        progress.values.firstOrNull()?.let { ActiveBanner(it) }

        if (peers.isEmpty()) {
            Box(Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Default.Wifi, null, Modifier.size(48.dp), tint = MaterialTheme.colorScheme.outline)
                    Spacer(Modifier.height(8.dp))
                    Text("正在发现附近设备…", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        } else {
            LazyColumn(Modifier.weight(1f), contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp)) {
                items(peers, key = { it.id }) { p ->
                    val dismiss = rememberSwipeToDismissBoxState(confirmValueChange = { v ->
                        when (v) {
                            SwipeToDismissBoxValue.EndToStart -> { core.hide(p.id); true }
                            SwipeToDismissBoxValue.StartToEnd -> { detail = p; false }
                            else -> false
                        }
                    })
                    SwipeToDismissBox(
                        state = dismiss,
                        enableDismissFromStartToEnd = true,
                        enableDismissFromEndToStart = true,
                        backgroundContent = {
                            val d = dismiss.dismissDirection
                            if (d != SwipeToDismissBoxValue.Settled) {
                                val start = d == SwipeToDismissBoxValue.StartToEnd
                                Box(
                                    Modifier.fillMaxSize().clip(RoundedCornerShape(14.dp))
                                        .background(if (start) MaterialTheme.colorScheme.tertiaryContainer else MaterialTheme.colorScheme.errorContainer)
                                        .padding(horizontal = 20.dp),
                                    contentAlignment = if (start) Alignment.CenterStart else Alignment.CenterEnd,
                                ) {
                                    Icon(
                                        if (start) Icons.Default.MoreVert else Icons.Default.Delete,
                                        if (start) "详情" else "删除",
                                    )
                                }
                            }
                        },
                    ) {
                        DeviceCard(p, selected = p.id in selected) {
                            selected = if (p.id in selected) selected - p.id else selected + p.id
                        }
                    }
                }
            }
        }

        if (selected.isNotEmpty()) {
            Surface(tonalElevation = 3.dp) {
                Column(Modifier.padding(horizontal = 16.dp, vertical = 12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("加密传输", Modifier.weight(1f))
                        Switch(secure, { secure = it })
                        Spacer(Modifier.width(12.dp))
                        Text("SHA-256", Modifier.weight(1f))
                        Switch(verify, { verify = it })
                    }
                    Spacer(Modifier.height(10.dp))
                    Button(
                        onClick = { onPickFiles(selected.first(), secure, verify) },
                        shape = RoundedCornerShape(14.dp),
                        modifier = Modifier.fillMaxWidth().height(50.dp),
                    ) {
                        Icon(Icons.AutoMirrored.Filled.Send, null)
                        Spacer(Modifier.width(8.dp))
                        Text("选择文件发送", style = MaterialTheme.typography.titleMedium)
                    }
                }
            }
        }
    }

    detail?.let { p ->
        val sheet = rememberModalBottomSheetState()
        ModalBottomSheet(onDismissRequest = { detail = null }, sheetState = sheet) {
            PeerDetail(p)
        }
    }

    if (showAdd) AddDeviceSheet(core) { showAdd = false }
}

@Composable
private fun DeviceCard(p: Peer, selected: Boolean, onClick: () -> Unit) {
    Card(
        Modifier.fillMaxWidth().padding(vertical = 4.dp).clickable(onClick = onClick),
        shape = RoundedCornerShape(14.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (selected) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Avatar(p, 44)
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text(p.name, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(
                    "${platform(p.platform)} · ${p.addrs.firstOrNull() ?: ":${p.port}"}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
            if (selected) Icon(Icons.Default.Check, null, tint = MaterialTheme.colorScheme.primary)
        }
    }
}

@Composable
private fun PeerDetail(p: Peer) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 32.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Avatar(p, 56)
            Spacer(Modifier.width(14.dp))
            Column {
                Text(p.name, style = MaterialTheme.typography.titleLarge)
                Text(platform(p.platform), color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        Spacer(Modifier.height(16.dp))
        Divider()
        Spacer(Modifier.height(12.dp))
        p.addrs.forEach { addr ->
            DetailRow("地址", addr)
        }
        DetailRow("端口", "${p.port}")
        DetailRow("设备 ID", p.id)
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp)) {
        Text(label, Modifier.width(84.dp), color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, modifier = Modifier.weight(1f), fontFamily = FontFamily.Monospace)
    }
}

@Composable
private fun ActiveBanner(p: Progress) {
    Card(Modifier.fillMaxWidth().padding(12.dp), shape = RoundedCornerShape(16.dp)) {
        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
            CircularProgressIndicator(
                progress = { p.fraction },
                modifier = Modifier.size(40.dp),
                strokeWidth = 4.dp,
            )
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text("正在发送", style = MaterialTheme.typography.titleSmall)
                Text(
                    "${size(p.speedBps)}/s · ${size(p.bytesDone)} / ${size(p.bytesTotal)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text("${(p.fraction * 100).toInt()}%", fontWeight = FontWeight.SemiBold)
        }
    }
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
            if (active.isEmpty()) EmptyState("暂无进行中的传输")
            else LazyColumn(contentPadding = PaddingValues(12.dp)) {
                items(active.values.toList(), key = { it.sessionId }) { ActiveRow(it) }
            }
        } else {
            if (history.isEmpty()) EmptyState("暂无历史记录")
            else LazyColumn(contentPadding = PaddingValues(vertical = 6.dp)) {
                items(history, key = { it.sessionId }) { h -> HistoryRow(h) { core.deleteHistory(h.sessionId) } }
            }
        }
    }
}

@Composable
private fun ActiveRow(p: Progress) {
    Card(Modifier.fillMaxWidth().padding(vertical = 4.dp), shape = RoundedCornerShape(14.dp)) {
        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
            CircularProgressIndicator(progress = { p.fraction }, modifier = Modifier.size(38.dp), strokeWidth = 4.dp)
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(
                    if (p.filesTotal > 1) "发送 ${p.filesDone}/${p.filesTotal} 个文件" else "正在发送",
                    style = MaterialTheme.typography.titleSmall,
                )
                Text(
                    "${size(p.speedBps)}/s · ${size(p.bytesDone)} / ${size(p.bytesTotal)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text("${(p.fraction * 100).toInt()}%", fontWeight = FontWeight.SemiBold)
        }
    }
}

@Composable
private fun HistoryRow(h: HistoryItem, onDelete: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            if (h.status == "completed") Icons.Default.Check else Icons.Default.Error,
            null,
            tint = when (h.status) {
                "completed" -> Color(0xFF34C759)
                "rejected" -> Color(0xFFFF9500)
                "cancelled" -> Color(0xFF8E8E93)
                else -> Color(0xFFFF3B30)
            },
            modifier = Modifier.size(28.dp),
        )
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(
                if (h.fileCount > 1) "${h.firstFile} 等 ${h.fileCount} 个文件" else h.firstFile,
                style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    if (h.direction == "send") Icons.Default.ArrowUpward else Icons.Default.ArrowDownward,
                    null, Modifier.size(13.dp),
                    tint = if (h.direction == "send") Color(0xFF0A84FF) else Color(0xFF34C759),
                )
                Spacer(Modifier.width(3.dp))
                Text(
                    "${if (h.direction == "send") "发送" else "接收"} · ${h.peerName} · ${size(h.bytes)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Text(relative(h.endedAtMs), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.outline)
        IconButton(onDelete) { Icon(Icons.Default.Delete, "删除", tint = MaterialTheme.colorScheme.outline) }
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

    LazyColumn(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars),
        contentPadding = PaddingValues(16.dp),
    ) {
        item {
            Text("我的", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.padding(bottom = 12.dp))
        }
        item {
            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                Box(
                    Modifier.size(76.dp).clip(CircleShape).background(MaterialTheme.colorScheme.surfaceVariant),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        (identity?.name ?: "?").take(1).uppercase(),
                        style = MaterialTheme.typography.displaySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Spacer(Modifier.height(8.dp))
                Text(identity?.name ?: "未命名", style = MaterialTheme.typography.headlineSmall)
                Text("端口 52225", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
            }
        }
        item {
            if (bmp != null) {
                Card(Modifier.fillMaxWidth().padding(top = 16.dp), shape = RoundedCornerShape(16.dp)) {
                    Column(Modifier.fillMaxWidth().padding(20.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Image(bmp.asImageBitmap(), null, Modifier.size(220.dp).clip(RoundedCornerShape(10.dp)).background(Color.White))
                        Spacer(Modifier.height(10.dp))
                        Text("让对方在 SendSent 里扫码，即可连接", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
        item {
            Text("本机 IP", style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 20.dp, bottom = 6.dp))
        }
        items(addresses) { a ->
            Card(Modifier.fillMaxWidth().padding(vertical = 3.dp), shape = RoundedCornerShape(12.dp)) {
                Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.Wifi, null, tint = MaterialTheme.colorScheme.primary)
                    Spacer(Modifier.width(12.dp))
                    Text("${a.ip}:52225", fontFamily = FontFamily.Monospace, modifier = Modifier.weight(1f))
                    Text(a.iface, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
                }
            }
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

    LazyColumn(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars),
        contentPadding = PaddingValues(16.dp),
    ) {
        item {
            Text("设置", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.padding(bottom = 12.dp))
        }
        item {
            Text("本机", style = MaterialTheme.typography.labelLarge)
            Spacer(Modifier.height(6.dp))
            OutlinedTextField(name, { name = it }, label = { Text("显示名称") }, singleLine = true, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            Button({ core.rename(name) }, Modifier.fillMaxWidth()) { Text("保存名称") }
        }
        item {
            Text("传输参数", style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 20.dp, bottom = 6.dp))
        }
        item { Stepper("并发连接数", conns) { conns = it } }
        item { Stepper("数据块 (KB)", chunk) { chunk = it } }
        item { Stepper("分片阈值 (MB)", split) { split = it } }
        item {
            Spacer(Modifier.height(8.dp))
            Button({ core.setConfig(conns.toInt(), chunk, split) }, Modifier.fillMaxWidth()) { Text("保存参数") }
        }
    }
}

@Composable
private fun Stepper(label: String, value: Long, onChange: (Long) -> Unit) {
    Row(
        Modifier.fillMaxWidth().padding(vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text("$label：$value")
        Row {
            OutlinedButton({ onChange((value - 1).coerceAtLeast(0)) }) { Text("−") }
            Spacer(Modifier.width(8.dp))
            OutlinedButton({ onChange(value + 1) }) { Text("+") }
        }
    }
}

@Composable
private fun AddDeviceSheet(core: Core, onClose: () -> Unit) {
    var addr by remember { mutableStateOf("") }
    var scanning by remember { mutableStateOf(false) }
    if (scanning) {
        AlertDialog(
            onDismissRequest = { scanning = false },
            confirmButton = {},
            title = { Text("扫描二维码") },
            text = {
                Box(Modifier.fillMaxWidth().height(360.dp)) {
                    ScannerScreen { payload ->
                        runCatching { parseAddr(payload) }.getOrNull()?.let { core.addPeer(it) }
                        scanning = false
                        onClose()
                    }
                }
            },
        )
        return
    }

    val sheet = rememberModalBottomSheetState()
    ModalBottomSheet(onDismissRequest = onClose, sheetState = sheet) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 32.dp)) {
            Text("添加设备", style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.height(12.dp))
            OutlinedTextField(
                addr, { addr = it },
                label = { Text("IP:port") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(10.dp))
            Button(
                { core.addPeer(addr.trim()); onClose() },
                enabled = addr.isNotBlank(),
                modifier = Modifier.fillMaxWidth(),
            ) { Text("添加") }
            Spacer(Modifier.height(8.dp))
            OutlinedButton({ scanning = true }, Modifier.fillMaxWidth()) {
                Icon(Icons.Default.QrCode, null); Spacer(Modifier.width(8.dp)); Text("扫码添加")
            }
        }
    }
}

@Composable
private fun Avatar(p: Peer, size: Int) {
    val color = when (p.platform) {
        "ios" -> Color(0xFF0A84FF)
        "android" -> Color(0xFF34C759)
        "windows" -> Color(0xFF0078D4)
        "linux" -> Color(0xFFFF9500)
        else -> Color(0xFF8E8E93)
    }
    Box(
        Modifier.size(size.dp).clip(RoundedCornerShape((size / 4).dp)).background(color),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            p.name.take(1).uppercase(),
            color = Color.White,
            fontWeight = FontWeight.SemiBold,
            style = MaterialTheme.typography.titleMedium,
        )
    }
}

@Composable
private fun EmptyState(text: String) {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

private fun parseAddr(s: String): String? {
    val t = s.trim()
    if (Regex("^[0-9.]+:[0-9]+$").matches(t)) return t
    if (!t.startsWith("sendsent://")) return null
    val u = java.net.URI(t.replace("sendsent://", "http://"))
    val q = u.query ?: return null
    return q.split("&").firstOrNull { it.startsWith("addr=") }?.substringAfter("addr=")
}

private fun relative(ms: Long): String {
    if (ms <= 0) return ""
    return DateUtils.getRelativeTimeSpanString(ms, System.currentTimeMillis(), DateUtils.MINUTE_IN_MILLIS, DateUtils.FORMAT_ABBREV_RELATIVE).toString()
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
