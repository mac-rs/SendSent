@file:OptIn(
    androidx.compose.material3.ExperimentalMaterial3Api::class,
    androidx.compose.foundation.layout.ExperimentalLayoutApi::class,
)

package com.mankong.sendsent.ui

import android.graphics.BitmapFactory
import android.text.format.DateUtils
import android.util.Base64
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.BorderStroke
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
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.HorizontalDivider
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
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
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
            containerColor = MaterialTheme.colorScheme.surface,
            bottomBar = {
                Surface(color = MaterialTheme.colorScheme.surfaceContainer) {
                    NavigationBar(containerColor = Color.Transparent) {
                        NavigationBarItem(tab == 0, { tab = 0 }, { Icon(Icons.Default.Wifi, null) }, label = { Text("设备") })
                        NavigationBarItem(tab == 1, { tab = 1 }, { Icon(Icons.AutoMirrored.Filled.Send, null) }, label = { Text("传输") })
                        NavigationBarItem(tab == 2, { tab = 2 }, { Icon(Icons.Default.Person, null) }, label = { Text("我的") })
                        NavigationBarItem(tab == 3, { tab = 3 }, { Icon(Icons.Default.Settings, null) }, label = { Text("设置") })
                    }
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
private fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(
        text,
        modifier = modifier.padding(start = 4.dp, top = 12.dp, bottom = 6.dp),
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
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
    val haptic = LocalHapticFeedback.current

    val peers = remember(allPeers, hidden) { allPeers.filterNot { it.id in hidden } }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = {
                Column {
                    Text("设备", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                    Text(
                        if (peers.isEmpty()) "正在发现…" else "${peers.size} 台设备 · 同一局域网",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            actions = { IconButton({ showAdd = true }) { Icon(Icons.Default.Add, "添加设备") } },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
        )

        progress.values.firstOrNull()?.let { ActiveBanner(it) }

        LazyColumn(
            Modifier.weight(1f),
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 4.dp),
        ) {
                item { SectionLabel("附近设备") }
                if (peers.isEmpty()) {
                    item { EmptyDiscovery() }
                } else {
                    items(peers, key = { it.id }) { p ->
                        val dismiss = rememberSwipeToDismissBoxState(confirmValueChange = { v ->
                            when (v) {
                                SwipeToDismissBoxValue.EndToStart -> {
                                    haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                                    core.hide(p.id); true
                                }
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
                                        Modifier.fillMaxSize().padding(vertical = 5.dp).clip(RoundedCornerShape(18.dp))
                                            .background(if (start) MaterialTheme.colorScheme.tertiaryContainer else MaterialTheme.colorScheme.errorContainer)
                                            .padding(horizontal = 22.dp),
                                        contentAlignment = if (start) Alignment.CenterStart else Alignment.CenterEnd,
                                    ) {
                                        Icon(
                                            if (start) Icons.Default.MoreVert else Icons.Default.Delete,
                                            if (start) "详情" else "删除",
                                            tint = if (start) MaterialTheme.colorScheme.onTertiaryContainer else MaterialTheme.colorScheme.onErrorContainer,
                                        )
                                    }
                                }
                            },
                        ) {
                            DeviceCard(p, selected = p.id in selected) {
                                haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                                selected = if (p.id in selected) selected - p.id else selected + p.id
                            }
                        }
                    }
                }
                item { Spacer(Modifier.height(8.dp)) }
            }

        AnimatedVisibility(
            visible = selected.isNotEmpty(),
            enter = slideInVertically { it } + fadeIn(),
            exit = slideOutVertically { it } + fadeOut(),
        ) {
            SendBar(
                secure = secure, verify = verify,
                onSecure = { secure = it }, onVerify = { verify = it },
                onSend = { onPickFiles(selected.first(), secure, verify) },
            )
        }
    }

    detail?.let { p ->
        val sheet = rememberModalBottomSheetState()
        ModalBottomSheet(onDismissRequest = { detail = null }, sheetState = sheet) { PeerDetail(p) }
    }

    if (showAdd) AddDeviceSheet(core) { showAdd = false }
}

@Composable
private fun DeviceCard(p: Peer, selected: Boolean, onClick: () -> Unit) {
    Card(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth().padding(vertical = 5.dp),
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (selected) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceContainerLow,
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = if (selected) 0.dp else 1.dp),
        border = if (selected) BorderStroke(1.dp, MaterialTheme.colorScheme.primary) else null,
    ) {
        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
            Avatar(p, 46)
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(
                    p.name,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                    color = if (selected) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.height(2.dp))
                Text(
                    "${platform(p.platform)} · ${p.addrs.firstOrNull() ?: ":${p.port}"}",
                    style = MaterialTheme.typography.bodySmall,
                    color = if (selected) MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.8f) else MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
            if (selected) {
                Icon(Icons.Default.Check, null, tint = MaterialTheme.colorScheme.primary)
            }
        }
    }
}

@Composable
private fun SendBar(secure: Boolean, verify: Boolean, onSecure: (Boolean) -> Unit, onVerify: (Boolean) -> Unit, onSend: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        shape = RoundedCornerShape(topStart = 24.dp, topEnd = 24.dp),
        tonalElevation = 3.dp,
        shadowElevation = 8.dp,
    ) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                ToggleChip("加密传输", secure, onSecure, Modifier.weight(1f))
                Spacer(Modifier.width(10.dp))
                ToggleChip("SHA-256", verify, onVerify, Modifier.weight(1f))
            }
            Spacer(Modifier.height(14.dp))
            Button(
                onClick = onSend,
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.fillMaxWidth().height(52.dp),
            ) {
                Icon(Icons.AutoMirrored.Filled.Send, null)
                Spacer(Modifier.width(8.dp))
                Text("选择文件发送", style = MaterialTheme.typography.titleMedium)
            }
        }
    }
}

@Composable
private fun ToggleChip(label: String, checked: Boolean, onChange: (Boolean) -> Unit, modifier: Modifier = Modifier) {
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(14.dp),
        color = if (checked) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surfaceContainerHighest,
    ) {
        Row(Modifier.padding(horizontal = 12.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(label, Modifier.weight(1f), style = MaterialTheme.typography.labelLarge)
            Switch(checked, onChange)
        }
    }
}

@Composable
private fun EmptyDiscovery() {
    Column(
        Modifier.fillMaxWidth().padding(vertical = 64.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            Modifier.size(88.dp).clip(CircleShape).background(MaterialTheme.colorScheme.surfaceContainerHigh),
            contentAlignment = Alignment.Center,
        ) {
            Icon(Icons.Default.Wifi, null, Modifier.size(40.dp), tint = MaterialTheme.colorScheme.primary)
        }
        Spacer(Modifier.height(16.dp))
        Text("正在发现附近设备…", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(6.dp))
        Text(
            "确保设备在同一局域网，或点右上角 + 手动添加",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun PeerDetail(p: Peer) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(bottom = 36.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Avatar(p, 64)
            Spacer(Modifier.width(16.dp))
            Column {
                Text(p.name, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
                Text(platform(p.platform), color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        Spacer(Modifier.height(20.dp))
        HorizontalDivider()
        Spacer(Modifier.height(8.dp))
        p.addrs.forEach { addr -> DetailRow("地址", addr) }
        DetailRow("端口", "${p.port}")
        DetailRow("设备 ID", p.id)
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
        Text(label, Modifier.width(88.dp), color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, modifier = Modifier.weight(1f), fontFamily = FontFamily.Monospace)
    }
}

@Composable
private fun ActiveBanner(p: Progress) {
    val animated by animateFloatAsState(p.fraction, label = "frac")
    ElevatedCard(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.primaryContainer),
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            CircularProgressIndicator(
                progress = { animated },
                modifier = Modifier.size(44.dp),
                strokeWidth = 4.dp,
                color = MaterialTheme.colorScheme.primary,
                trackColor = MaterialTheme.colorScheme.primary.copy(alpha = 0.18f),
            )
            Spacer(Modifier.width(16.dp))
            Column(Modifier.weight(1f)) {
                Text("正在发送", style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.height(2.dp))
                Text(
                    "${size(p.speedBps)}/s · ${size(p.bytesDone)} / ${size(p.bytesTotal)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.8f),
                )
            }
            Text("${(p.fraction * 100).toInt()}%", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        }
    }
}

@Composable
private fun TransfersScreen(core: Core) {
    val active by core.progress.collectAsState()
    val history by core.history.collectAsState()
    var t by remember { mutableIntStateOf(0) }
    val haptic = LocalHapticFeedback.current

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = { Text("传输", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold) },
            actions = {
                if (t == 1 && history.isNotEmpty()) {
                    IconButton({ core.clearHistory() }) { Icon(Icons.Default.Delete, "清空") }
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
        )
        TabRow(t, containerColor = Color.Transparent, divider = {}) {
            Tab(t == 0, { t = 0 }, text = { Text("进行中") })
            Tab(t == 1, { t = 1 }, text = { Text("历史") })
        }
        Spacer(Modifier.height(8.dp))
        if (t == 0) {
            if (active.isEmpty()) {
                Empty("暂无进行中的传输")
            } else {
                LazyColumn(contentPadding = PaddingValues(horizontal = 16.dp, vertical = 4.dp)) {
                    items(active.values.toList(), key = { it.sessionId }) { ActiveRow(it) }
                }
            }
        } else {
            if (history.isEmpty()) {
                Empty("暂无历史记录")
            } else {
                LazyColumn(contentPadding = PaddingValues(horizontal = 16.dp, vertical = 4.dp)) {
                    items(history, key = { it.sessionId }) { h ->
                        val dismiss = rememberSwipeToDismissBoxState(confirmValueChange = { v ->
                            if (v == SwipeToDismissBoxValue.EndToStart) {
                                haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                                core.deleteHistory(h.sessionId); true
                            } else false
                        })
                        SwipeToDismissBox(
                            state = dismiss,
                            enableDismissFromStartToEnd = false,
                            backgroundContent = {
                                if (dismiss.dismissDirection != SwipeToDismissBoxValue.Settled) {
                                    Box(
                                        Modifier.fillMaxSize().padding(vertical = 5.dp).clip(RoundedCornerShape(18.dp))
                                            .background(MaterialTheme.colorScheme.errorContainer).padding(horizontal = 22.dp),
                                        contentAlignment = Alignment.CenterEnd,
                                    ) { Icon(Icons.Default.Delete, "删除", tint = MaterialTheme.colorScheme.onErrorContainer) }
                                }
                            },
                        ) {
                            HistoryCard(h)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ActiveRow(p: Progress) {
    val animated by animateFloatAsState(p.fraction, label = "rowFrac")
    ElevatedCard(
        Modifier.fillMaxWidth().padding(vertical = 5.dp),
        shape = RoundedCornerShape(18.dp),
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            CircularProgressIndicator(
                progress = { animated },
                modifier = Modifier.size(42.dp),
                strokeWidth = 4.dp,
                trackColor = MaterialTheme.colorScheme.primary.copy(alpha = 0.18f),
            )
            Spacer(Modifier.width(16.dp))
            Column(Modifier.weight(1f)) {
                Text(
                    if (p.filesTotal > 1) "发送 ${p.filesDone}/${p.filesTotal} 个文件" else "正在发送",
                    style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.SemiBold,
                )
                Spacer(Modifier.height(2.dp))
                Text(
                    "${size(p.speedBps)}/s · ${size(p.bytesDone)} / ${size(p.bytesTotal)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text("${(p.fraction * 100).toInt()}%", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        }
    }
}

@Composable
private fun HistoryCard(h: HistoryItem) {
    ElevatedCard(
        Modifier.fillMaxWidth().padding(vertical = 5.dp),
        shape = RoundedCornerShape(18.dp),
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(
                Modifier.size(40.dp).clip(CircleShape).background(statusColor(h.status).copy(alpha = 0.14f)),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    if (h.status == "completed") Icons.Default.Check else Icons.Default.Error,
                    null, Modifier.size(22.dp), tint = statusColor(h.status),
                )
            }
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(
                    if (h.fileCount > 1) "${h.firstFile} 等 ${h.fileCount} 个文件" else h.firstFile,
                    style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(3.dp))
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
                        maxLines = 1, overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            Spacer(Modifier.width(8.dp))
            Text(relative(h.endedAtMs), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.outline)
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

    LazyColumn(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars),
        contentPadding = PaddingValues(20.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        item {
            Box(
                Modifier.size(80.dp).clip(CircleShape)
                    .background(Brush.linearGradient(listOf(Color(0xFF0A84FF), Color(0xFF5E5CE6)))),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    (identity?.name ?: "?").take(1).uppercase(),
                    style = MaterialTheme.typography.displaySmall,
                    color = Color.White, fontWeight = FontWeight.SemiBold,
                )
            }
            Spacer(Modifier.height(10.dp))
            Text(identity?.name ?: "未命名", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
            Text("端口 52225", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
        }
        item {
            if (bmp != null) {
                ElevatedCard(
                    Modifier.fillMaxWidth().padding(top = 20.dp),
                    shape = RoundedCornerShape(20.dp),
                ) {
                    Column(Modifier.fillMaxWidth().padding(22.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Image(
                            bmp.asImageBitmap(), null,
                            Modifier.size(220.dp).clip(RoundedCornerShape(12.dp)).background(Color.White),
                        )
                        Spacer(Modifier.height(12.dp))
                        Text(
                            "让对方在 SendSent 里扫码，即可连接",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
        item {
            Text(
                "本机 IP",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.fillMaxWidth().padding(top = 22.dp, bottom = 8.dp),
            )
        }
        items(addresses) { a ->
            ElevatedCard(Modifier.fillMaxWidth().padding(vertical = 4.dp), shape = RoundedCornerShape(16.dp)) {
                Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        Modifier.size(36.dp).clip(CircleShape).background(MaterialTheme.colorScheme.primaryContainer),
                        contentAlignment = Alignment.Center,
                    ) { Icon(Icons.Default.Wifi, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onPrimaryContainer) }
                    Spacer(Modifier.width(14.dp))
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
        contentPadding = PaddingValues(20.dp),
    ) {
        item {
            Text("本机", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold, modifier = Modifier.padding(bottom = 10.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    Modifier.size(64.dp).clip(RoundedCornerShape(18.dp))
                        .background(Brush.linearGradient(listOf(Color(0xFF0A84FF), Color(0xFF5E5CE6)))),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        (identity?.name ?: "?").take(1).uppercase(),
                        style = MaterialTheme.typography.headlineSmall,
                        color = Color.White, fontWeight = FontWeight.SemiBold,
                    )
                }
                Spacer(Modifier.width(16.dp))
                Column(Modifier.weight(1f)) {
                    Text(identity?.name ?: "未命名", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                    Text("端口 52225 · ${platform(identity?.platform ?: "")}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            Spacer(Modifier.height(16.dp))
            OutlinedTextField(name, { name = it }, label = { Text("显示名称") }, singleLine = true, modifier = Modifier.fillMaxWidth(), shape = RoundedCornerShape(14.dp))
            Spacer(Modifier.height(10.dp))
            Button({ core.rename(name) }, Modifier.fillMaxWidth().height(48.dp), shape = RoundedCornerShape(14.dp)) { Text("保存名称") }
        }
        item {
            Text("传输参数", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold, modifier = Modifier.padding(top = 28.dp, bottom = 6.dp))
            ElevatedCard(Modifier.fillMaxWidth(), shape = RoundedCornerShape(18.dp)) {
                Column(Modifier.padding(horizontal = 16.dp, vertical = 6.dp)) {
                    Stepper("并发连接数", conns) { conns = it }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Stepper("数据块 (KB)", chunk) { chunk = it }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Stepper("分片阈值 (MB)", split) { split = it }
                }
            }
            Spacer(Modifier.height(12.dp))
            Button({ core.setConfig(conns.toInt(), chunk, split) }, Modifier.fillMaxWidth().height(48.dp), shape = RoundedCornerShape(14.dp)) { Text("保存参数") }
        }
    }
}

@Composable
private fun Stepper(label: String, value: Long, onChange: (Long) -> Unit) {
    Row(
        Modifier.fillMaxWidth().padding(vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(label)
        Row(verticalAlignment = Alignment.CenterVertically) {
            FilledTonalStep("−") { onChange((value - 1).coerceAtLeast(0)) }
            Text("$value", Modifier.width(64.dp), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold, textAlign = androidx.compose.ui.text.style.TextAlign.Center)
            FilledTonalStep("+") { onChange(value + 1) }
        }
    }
}

@Composable
private fun FilledTonalStep(text: String, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = CircleShape,
        color = MaterialTheme.colorScheme.secondaryContainer,
        modifier = Modifier.size(36.dp),
    ) {
        Box(contentAlignment = Alignment.Center) {
            Text(text, style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSecondaryContainer)
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
        Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(bottom = 36.dp)) {
            Text("添加设备", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
            Spacer(Modifier.height(16.dp))
            OutlinedTextField(
                addr, { addr = it },
                label = { Text("IP:port") },
                placeholder = { Text("192.168.1.5:52225") },
                singleLine = true,
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(12.dp))
            Button(
                { core.addPeer(addr.trim()); onClose() },
                enabled = addr.isNotBlank(),
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier.fillMaxWidth().height(50.dp),
            ) { Text("添加") }
            Spacer(Modifier.height(10.dp))
            OutlinedButton({ scanning = true }, Modifier.fillMaxWidth().height(50.dp), shape = RoundedCornerShape(14.dp)) {
                Icon(Icons.Default.QrCode, null); Spacer(Modifier.width(8.dp)); Text("扫码添加")
            }
        }
    }
}

@Composable
private fun Avatar(p: Peer, size: Int) {
    val c = platformColor(p.platform)
    Box(
        Modifier.size(size.dp).clip(RoundedCornerShape((size / 4).dp))
            .background(Brush.linearGradient(listOf(c, c.copy(alpha = 0.72f)))),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            p.name.take(1).uppercase(),
            color = Color.White,
            fontWeight = FontWeight.SemiBold,
            style = if (size >= 56) MaterialTheme.typography.headlineSmall else MaterialTheme.typography.titleMedium,
        )
    }
}

@Composable
private fun Empty(text: String) {
    Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
        Box(
            Modifier.size(80.dp).clip(CircleShape).background(MaterialTheme.colorScheme.surfaceContainerHigh),
            contentAlignment = Alignment.Center,
        ) { Icon(Icons.AutoMirrored.Filled.Send, null, Modifier.size(34.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant) }
        Spacer(Modifier.height(14.dp))
        Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

private fun platformColor(p: String): Color = when (p) {
    "ios" -> Color(0xFF0A84FF)
    "android" -> Color(0xFF34C759)
    "windows" -> Color(0xFF0078D4)
    "linux" -> Color(0xFFFF9500)
    "macos" -> Color(0xFF636366)
    else -> Color(0xFF8E8E93)
}

private fun statusColor(status: String): Color = when (status) {
    "completed" -> Color(0xFF34C759)
    "rejected" -> Color(0xFFFF9500)
    "cancelled" -> Color(0xFF8E8E93)
    else -> Color(0xFFFF3B30)
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
