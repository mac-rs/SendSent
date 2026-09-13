@file:OptIn(
    androidx.compose.material3.ExperimentalMaterial3Api::class,
    androidx.compose.foundation.layout.ExperimentalLayoutApi::class,
)

package com.mankong.sendsent.ui

import android.graphics.BitmapFactory
import android.text.format.DateUtils
import android.util.Base64
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DevicesOther
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.FilterList
import androidx.compose.material.icons.filled.FolderZip
import androidx.compose.material.icons.filled.Image
import androidx.compose.material.icons.filled.InsertDriveFile
import androidx.compose.material.icons.filled.Laptop
import androidx.compose.material.icons.filled.Movie
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.PhoneAndroid
import androidx.compose.material.icons.filled.PictureAsPdf
import androidx.compose.material.icons.filled.QrCode
import androidx.compose.material.icons.filled.Radar
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Smartphone
import androidx.compose.material.icons.filled.Terminal
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
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
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.painter.BitmapPainter
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.gestures.detectTapGestures
import com.mankong.sendsent.Core
import com.mankong.sendsent.HistoryItem
import com.mankong.sendsent.Peer
import com.mankong.sendsent.Progress
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.roundToInt
import kotlin.math.sin

private val Indigo = Color(0xFF5B54F0)
private val Green = Color(0xFF34C759)

@Composable
fun App(core: Core, onPickFiles: (String, Boolean, Boolean) -> Unit) {
    val context = LocalContext.current
    var theme by remember { mutableStateOf(ThemePrefs.get(context)) }
    val dark = when (theme) {
        "light" -> false
        "dark" -> true
        else -> isSystemInDarkTheme()
    }
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
                NavigationBar {
                    NavigationBarItem(tab == 0, { tab = 0 }, { Icon(Icons.Default.Radar, null) }, label = { Text("附近") })
                    NavigationBarItem(tab == 1, { tab = 1 }, { Icon(Icons.AutoMirrored.Filled.Send, null) }, label = { Text("传输") })
                    NavigationBarItem(tab == 2, { tab = 2 }, { Icon(Icons.Default.Person, null) }, label = { Text("我的") })
                    NavigationBarItem(tab == 3, { tab = 3 }, { Icon(Icons.Default.Settings, null) }, label = { Text("设置") })
                }
            },
        ) { pad ->
            Box(Modifier.padding(pad).fillMaxSize()) {
                when (tab) {
                    0 -> RadarScreen(core, onPickFiles)
                    1 -> TransfersScreen(core)
                    2 -> ProfileScreen(core)
                    else -> SettingsScreen(core, theme) { theme = it; ThemePrefs.set(context, it) }
                }
            }
        }
    }
}

// ===================== 附近(雷达) =====================

@Composable
private fun RadarScreen(core: Core, onPickFiles: (String, Boolean, Boolean) -> Unit) {
    val allPeers by core.peers.collectAsState()
    val hidden by core.hidden.collectAsState()
    val progress by core.progress.collectAsState()
    val identity by core.identity.collectAsState()
    val peers = remember(allPeers, hidden) { allPeers.filterNot { it.id in hidden } }
    var selected by remember { mutableStateOf<Peer?>(null) }
    var secure by remember { mutableStateOf(false) }
    var verify by remember { mutableStateOf(false) }
    var showAdd by remember { mutableStateOf(false) }

    LaunchedEffect(peers) {
        val s = selected
        if (s != null && peers.none { it.id == s.id }) selected = null
    }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = {
                Column {
                    Text("附近", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold)
                    Text(
                        if (peers.isEmpty()) "正在同一局域网里寻找…" else "同一局域网 · 找到 ${peers.size} 台设备",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            actions = {
                IconButton({ showAdd = true }) { Icon(Icons.Default.Add, "添加设备") }
            },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
        )

        progress.values.firstOrNull()?.let { ActiveBanner(it) }

        Box(Modifier.weight(1f).fillMaxWidth()) {
            RadarField(
                peers = peers,
                initial = (identity?.name ?: "我").take(1).uppercase(),
                selected = selected,
                onSelect = { selected = it },
                onDeselect = { selected = null },
            )
        }

        RadarActionBar(
            selected = selected, peers = peers, secure = secure, verify = verify,
            onSecure = { secure = it }, onVerify = { verify = it },
            onSend = { selected?.let { onPickFiles(it.id, secure, verify) } },
        )
    }

    if (showAdd) AddDeviceSheet(core) { showAdd = false }
}

@Composable
private fun RadarField(
    peers: List<Peer>,
    initial: String,
    selected: Peer?,
    onSelect: (Peer) -> Unit,
    onDeselect: () -> Unit,
) {
    val transition = rememberInfiniteTransition(label = "radar")
    val sweep by transition.animateFloat(
        0f, 360f, infiniteRepeatable(tween(6000, easing = LinearEasing)), label = "sweep",
    )
    val pulse by transition.animateFloat(
        0f, 1f, infiniteRepeatable(tween(3400, easing = LinearEasing)), label = "pulse",
    )
    val primary = MaterialTheme.colorScheme.primary
    val ringColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.10f)

    BoxWithConstraints(
        Modifier.fillMaxSize().pointerInput(Unit) { detectTapGestures { onDeselect() } },
    ) {
        val density = LocalDensity.current
        val wPx = with(density) { maxWidth.toPx() }
        val hPx = with(density) { maxHeight.toPx() }
        val center = Offset(wPx / 2f, hPx * 0.44f)
        val radius = min(wPx, hPx) * 0.33f
        val half = with(density) { 52.dp.toPx() }

        Canvas(Modifier.fillMaxSize()) {
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(primary.copy(alpha = 0.22f), Color.Transparent),
                    center = center, radius = radius * 1.5f,
                ),
                radius = radius * 1.5f, center = center,
            )
            listOf(1.34f to false, 1.02f to true, 0.70f to false).forEach { (f, dashed) ->
                drawCircle(
                    color = ringColor, radius = radius * f, center = center,
                    style = Stroke(
                        width = 1.dp.toPx(),
                        pathEffect = if (dashed) PathEffect.dashPathEffect(floatArrayOf(10f, 12f)) else null,
                    ),
                )
            }
            val r = radius * 1.34f
            drawArc(
                color = primary.copy(alpha = 0.14f),
                startAngle = sweep, sweepAngle = 70f, useCenter = true,
                topLeft = Offset(center.x - r, center.y - r),
                size = Size(r * 2, r * 2),
            )
            val pr = radius * 0.70f * (0.62f + 0.95f * pulse)
            drawCircle(
                color = primary.copy(alpha = (1f - pulse) * 0.22f),
                radius = pr, center = center, style = Stroke(1.5.dp.toPx()),
            )
        }

        Box(
            Modifier.size(104.dp).offset {
                IntOffset((center.x - half).roundToInt(), (center.y - half).roundToInt())
            },
            contentAlignment = Alignment.TopCenter,
        ) { MeNode(initial) }

        peers.forEachIndexed { i, p ->
            val a = angleFor(i, peers.size)
            val x = center.x + cos(a).toFloat() * radius
            val y = center.y + sin(a).toFloat() * radius
            Box(
                Modifier.size(104.dp).offset {
                    IntOffset((x - half).roundToInt(), (y - half).roundToInt())
                },
                contentAlignment = Alignment.TopCenter,
            ) {
                DeviceNode(p, selected?.id == p.id) { onSelect(it) }
            }
        }
    }
}

@Composable
private fun MeNode(initial: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Box(
            Modifier.size(80.dp).clip(CircleShape)
                .background(Brush.linearGradient(listOf(Color(0xFFA5B4FC), Indigo))),
            contentAlignment = Alignment.Center,
        ) {
            Text(initial, color = Color.White, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold)
        }
        Spacer(Modifier.height(6.dp))
        Surface(shape = CircleShape, color = MaterialTheme.colorScheme.surfaceContainerHighest) {
            Text(
                "我 · 本机",
                Modifier.padding(horizontal = 10.dp, vertical = 3.dp),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun DeviceNode(p: Peer, selected: Boolean, onSelect: (Peer) -> Unit) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.clickable { onSelect(p) },
    ) {
        Box(contentAlignment = Alignment.Center) {
            if (selected) {
                Box(Modifier.size(84.dp).clip(CircleShape).background(MaterialTheme.colorScheme.primary.copy(alpha = 0.16f)))
            }
            Avatar(p, if (selected) 66 else 58)
        }
        Spacer(Modifier.height(6.dp))
        Text(
            p.name,
            style = MaterialTheme.typography.labelMedium,
            fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
            color = if (selected) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = 1, overflow = TextOverflow.Ellipsis,
            modifier = Modifier.widthIn(max = 100.dp),
        )
    }
}

@Composable
private fun RadarActionBar(
    selected: Peer?,
    peers: List<Peer>,
    secure: Boolean,
    verify: Boolean,
    onSecure: (Boolean) -> Unit,
    onVerify: (Boolean) -> Unit,
    onSend: () -> Unit,
) {
    Surface(tonalElevation = 3.dp, shadowElevation = 8.dp, color = MaterialTheme.colorScheme.surfaceContainer) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
            if (selected != null) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Avatar(selected, 46)
                    Spacer(Modifier.width(12.dp))
                    Column(Modifier.weight(1f)) {
                        Text(selected.name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text("${platform(selected.platform)} · 已选择", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Button(onClick = onSend, shape = RoundedCornerShape(14.dp)) {
                        Icon(Icons.AutoMirrored.Filled.Send, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(6.dp))
                        Text("发送文件")
                    }
                }
                Spacer(Modifier.height(12.dp))
                Row {
                    ToggleChip("加密传输", secure, onSecure, Modifier.weight(1f))
                    Spacer(Modifier.width(10.dp))
                    ToggleChip("SHA-256", verify, onVerify, Modifier.weight(1f))
                }
            } else {
                Text(
                    if (peers.isEmpty()) "正在搜索附近设备…" else "点按附近的设备,再选择文件发送",
                    Modifier.fillMaxWidth().padding(vertical = 8.dp),
                    textAlign = TextAlign.Center,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

private fun angleFor(index: Int, count: Int): Double {
    if (count <= 1) return -Math.PI / 2
    val start = -140.0 * Math.PI / 180
    val end = -40.0 * Math.PI / 180
    return if (count <= 4) start + (end - start) * index / (count - 1)
    else -Math.PI / 2 + 2 * Math.PI * index / count
}

// ===================== 传输 =====================

@Composable
private fun TransfersScreen(core: Core) {
    val active by core.progress.collectAsState()
    val history by core.history.collectAsState()
    var dir by remember { mutableStateOf("all") }
    var status by remember { mutableStateOf("all") }
    var showFilter by remember { mutableStateOf(false) }
    val filtering = dir != "all" || status != "all"
    val haptic = LocalHapticFeedback.current

    val filtered = history.filter { h ->
        val d = when (dir) { "sent" -> h.direction == "send"; "received" -> h.direction != "send"; else -> true }
        val s = when (status) { "done" -> h.status == "completed"; "failed" -> h.status != "completed"; else -> true }
        d && s
    }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = {
                Column {
                    Text("传输", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold)
                    Text(
                        transfersSubtitle(filtered, filtering, dir, status),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            actions = {
                IconButton({ showFilter = true }) {
                    Icon(
                        Icons.Default.FilterList, "筛选",
                        tint = if (filtering) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
        )

        if (filtering) {
            Row(Modifier.padding(horizontal = 16.dp, vertical = 4.dp)) {
                AssistChip(
                    onClick = { dir = "all"; status = "all" },
                    label = { Text("${dirLabel(dir)} · ${statusLabel(status)}") },
                    trailingIcon = { Icon(Icons.Default.Close, null, Modifier.size(16.dp)) },
                )
            }
        }

        LazyColumn(Modifier.weight(1f), contentPadding = PaddingValues(horizontal = 16.dp, vertical = 4.dp)) {
            if (active.isNotEmpty()) {
                item { SectionLabel("进行中") }
                items(active.values.toList(), key = { it.sessionId }) { ActiveRow(it) }
            }
            if (filtered.isEmpty() && active.isEmpty()) {
                item { Empty(if (filtering) "没有符合条件的记录" else "还没有传输") }
            } else {
                groupByDay(filtered).forEach { g ->
                    item(key = "day-${g.label}") { DayHeader(g.label, g.items.size) }
                    items(g.items, key = { it.sessionId }) { h ->
                        HistoryRow(h) {
                            haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                            core.deleteHistory(h.sessionId)
                        }
                    }
                }
            }
            item { Spacer(Modifier.height(8.dp)) }
        }
    }

    if (showFilter) {
        FilterSheet(
            dir, status,
            onApply = { d, s -> dir = d; status = s },
            onClearHistory = { core.clearHistory() },
            onDismiss = { showFilter = false },
        )
    }
}

@Composable
private fun FilterSheet(
    dir: String,
    status: String,
    onApply: (String, String) -> Unit,
    onClearHistory: () -> Unit,
    onDismiss: () -> Unit,
) {
    var d by remember { mutableStateOf(dir) }
    var s by remember { mutableStateOf(status) }
    val sheet = rememberModalBottomSheetState()
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheet) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(bottom = 32.dp)) {
            Text("筛选", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
            Spacer(Modifier.height(18.dp))
            Text("方向", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(d == "all", { d = "all" }, { Text("全部") })
                FilterChip(d == "sent", { d = "sent" }, { Text("发送") })
                FilterChip(d == "received", { d = "received" }, { Text("接收") })
            }
            Spacer(Modifier.height(20.dp))
            Text("状态", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(s == "all", { s = "all" }, { Text("全部") })
                FilterChip(s == "done", { s = "done" }, { Text("完成") })
                FilterChip(s == "failed", { s = "failed" }, { Text("失败") })
            }
            Spacer(Modifier.height(26.dp))
            Row {
                OutlinedButton(
                    { d = "all"; s = "all"; onApply("all", "all"); onDismiss() },
                    Modifier.weight(1f).height(48.dp), shape = RoundedCornerShape(14.dp),
                ) { Text("重置") }
                Spacer(Modifier.width(12.dp))
                Button(
                    { onApply(d, s); onDismiss() },
                    Modifier.weight(1f).height(48.dp), shape = RoundedCornerShape(14.dp),
                ) { Text("完成") }
            }
            Spacer(Modifier.height(4.dp))
            TextButton({ onClearHistory(); onDismiss() }, Modifier.fillMaxWidth()) {
                Text("清空全部记录", color = MaterialTheme.colorScheme.error)
            }
        }
    }
}

@Composable
private fun DayHeader(label: String, count: Int) {
    Row(Modifier.fillMaxWidth().padding(top = 16.dp, bottom = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.weight(1f))
        Text("$count 项", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.outline)
    }
}

@Composable
private fun HistoryRow(h: HistoryItem, onDelete: () -> Unit) {
    val (bg, fg) = fileTint(ext(h.firstFile))
    Surface(
        shape = RoundedCornerShape(18.dp),
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
    ) {
        Row(Modifier.padding(start = 14.dp, end = 4.dp, top = 12.dp, bottom = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(
                Modifier.size(42.dp).clip(RoundedCornerShape(12.dp)).background(bg),
                contentAlignment = Alignment.Center,
            ) { Icon(fileIcon(h.firstFile), null, Modifier.size(20.dp), tint = fg) }
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(
                    if (h.fileCount > 1) "${h.firstFile} 等 ${h.fileCount} 项" else h.firstFile,
                    style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Medium,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(3.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        if (h.direction == "send") Icons.Default.ArrowUpward else Icons.Default.ArrowDownward,
                        null, Modifier.size(13.dp),
                        tint = if (h.direction == "send") Indigo else Green,
                    )
                    Spacer(Modifier.width(3.dp))
                    Text(
                        "${if (h.direction == "send") "发送给" else "来自"} ${h.peerName} · ${size(h.bytes)}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1, overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            Spacer(Modifier.width(8.dp))
            Column(horizontalAlignment = Alignment.End) {
                Icon(
                    if (h.status == "completed") Icons.Default.Check else Icons.Default.Error,
                    null, Modifier.size(16.dp), tint = statusColor(h.status),
                )
                Spacer(Modifier.height(2.dp))
                Text(relative(h.endedAtMs), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.outline)
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Default.Delete, "删除", modifier = Modifier.size(18.dp), tint = MaterialTheme.colorScheme.outline)
            }
        }
    }
}

// ===================== 我的 =====================

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

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState())
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(horizontal = 24.dp),
    ) {
        Spacer(Modifier.height(8.dp))
        Text("我的", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.height(4.dp))
        Text(
            identity?.name ?: "未命名",
            style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold,
            maxLines = 1, overflow = TextOverflow.Ellipsis,
        )
        Spacer(Modifier.height(6.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(6.dp).clip(CircleShape).background(Green))
            Spacer(Modifier.width(8.dp))
            Text(
                "可被发现 · ${platform(identity?.platform ?: "android")} · 端口 52225",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        Box(Modifier.fillMaxWidth().padding(vertical = 28.dp), contentAlignment = Alignment.Center) {
            Box(
                Modifier.size(300.dp).clip(CircleShape)
                    .background(Brush.radialGradient(listOf(MaterialTheme.colorScheme.primary.copy(alpha = 0.20f), Color.Transparent))),
            )
            if (bmp != null) {
                Surface(
                    shape = RoundedCornerShape(30.dp), color = Color.White,
                    shadowElevation = 16.dp,
                ) {
                    Image(
                        BitmapPainter(bmp.asImageBitmap(), filterQuality = FilterQuality.None),
                        contentDescription = null,
                        modifier = Modifier.padding(14.dp).size(196.dp),
                    )
                }
            }
        }
        Text(
            "让对方在 SendSent 里扫码连接",
            Modifier.fillMaxWidth(), textAlign = TextAlign.Center,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        SectionLabel("本机地址")
        addresses.forEach { a -> InfoValue("${a.ip}:52225", mono = true, trailing = a.iface) }
        SectionLabel("关于")
        InfoPair("设备 ID", (identity?.deviceId ?: "—").take(17))
        InfoPair("版本", "0.1.0")
        Spacer(Modifier.height(28.dp))
    }
}

@Composable
private fun InfoValue(value: String, mono: Boolean = false, trailing: String = "") {
    Column {
        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
        Row(Modifier.fillMaxWidth().height(46.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(
                value,
                Modifier.weight(1f),
                fontFamily = if (mono) FontFamily.Monospace else FontFamily.Default,
                style = MaterialTheme.typography.bodyMedium,
            )
            if (trailing.isNotEmpty()) {
                Text(trailing, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.outline)
            }
        }
    }
}

@Composable
private fun InfoPair(label: String, value: String) {
    Column {
        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
        Row(Modifier.fillMaxWidth().height(46.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(label, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            Text(value, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, fontFamily = FontFamily.Monospace)
        }
    }
}

// ===================== 设置 =====================

@Composable
private fun SettingsScreen(core: Core, theme: String, onTheme: (String) -> Unit) {
    val identity by core.identity.collectAsState()
    var name by remember(identity) { mutableStateOf(identity?.name ?: "") }
    var showRename by remember { mutableStateOf(false) }
    val cfg = remember { core.config() }
    var conns by remember { mutableLongStateOf((cfg?.conns ?: 16).toLong()) }
    var chunk by remember { mutableLongStateOf(cfg?.chunkKb ?: 1024) }
    var split by remember { mutableLongStateOf(cfg?.splitMb ?: 8) }
    fun persist() = core.setConfig(conns.toInt(), chunk, split)

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = { Text("设置", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold) },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
        )
        LazyColumn(
            Modifier.weight(1f).fillMaxWidth().padding(horizontal = 16.dp),
            contentPadding = PaddingValues(top = 2.dp, bottom = 24.dp),
        ) {
        item {
            GroupLabel("本机")
            GroupCard {
                ClickRow("显示名称", name) { showRename = true }
                RowDivider()
                ClickRow("端口", "52225", null)
            }
        }
        item {
            GroupLabel("外观")
            GroupCard {
                Column(Modifier.padding(16.dp)) {
                    Text("主题", style = MaterialTheme.typography.bodyLarge)
                    Spacer(Modifier.height(12.dp))
                    SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                        ThemePrefs.options.forEachIndexed { index, opt ->
                            SegmentedButton(
                                selected = theme == opt.first,
                                onClick = { onTheme(opt.first) },
                                shape = SegmentedButtonDefaults.itemShape(index, ThemePrefs.options.size),
                            ) { Text(opt.second) }
                        }
                    }
                }
            }
        }
        item {
            GroupLabel("传输")
            GroupCard {
                Stepper("并发连接数", conns, 1, "", 1, 32) { conns = it; persist() }
                RowDivider()
                Stepper("数据块", chunk, 64, "KB", 64, 1024) { chunk = it; persist() }
                RowDivider()
                Stepper("分片阈值", split, 1, "MB", 1, 1024) { split = it; persist() }
            }
            Text(
                "连接越多越快、占用越高;超过阈值的文件会切段并行传输。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
            )
        }
        item {
            GroupLabel("关于")
            GroupCard {
                ClickRow("版本", "0.1.0", null)
                RowDivider()
                ClickRow("技术栈", "Rust + Compose", null)
            }
        }
        }
    }

    if (showRename) {
        var tmp by remember { mutableStateOf(name) }
        AlertDialog(
            onDismissRequest = { showRename = false },
            title = { Text("显示名称") },
            text = {
                OutlinedTextField(tmp, { tmp = it }, singleLine = true, shape = RoundedCornerShape(12.dp), modifier = Modifier.fillMaxWidth())
            },
            confirmButton = { Button({ name = tmp; core.rename(tmp); showRename = false }) { Text("保存") } },
            dismissButton = { OutlinedButton({ showRename = false }) { Text("取消") } },
        )
    }
}

@Composable
private fun GroupLabel(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 8.dp, top = 16.dp, bottom = 8.dp),
    )
}

@Composable
private fun GroupCard(content: @Composable ColumnScope.() -> Unit) {
    Surface(
        shape = RoundedCornerShape(20.dp),
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(vertical = 2.dp), content = content)
    }
}

@Composable
private fun RowDivider() {
    HorizontalDivider(Modifier.padding(start = 16.dp), color = MaterialTheme.colorScheme.outlineVariant)
}

@Composable
private fun ClickRow(label: String, value: String, onClick: (() -> Unit)?) {
    val mod = if (onClick != null) Modifier.clickable { onClick() } else Modifier
    Row(
        mod.fillMaxWidth().padding(horizontal = 16.dp, vertical = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, Modifier.weight(1f))
        Text(value, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (onClick != null) {
            Spacer(Modifier.width(4.dp))
            Icon(Icons.Default.ChevronRight, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.outline)
        }
    }
}

@Composable
private fun Stepper(label: String, value: Long, step: Long, unit: String, min: Long, max: Long, onChange: (Long) -> Unit) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, Modifier.weight(1f))
        Row(verticalAlignment = Alignment.CenterVertically) {
            FilledTonalStep("−") { onChange((value - step).coerceIn(min, max)) }
            Text(
                if (unit.isEmpty()) "$value" else "$value $unit",
                Modifier.width(84.dp), textAlign = TextAlign.Center,
                style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold,
            )
            FilledTonalStep("+") { onChange((value + step).coerceIn(min, max)) }
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

// ===================== 共用 =====================

@Composable
private fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(
        text,
        modifier = modifier.fillMaxWidth().padding(top = 16.dp, bottom = 8.dp),
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun ActiveBanner(p: Progress) {
    val animated by animateFloatAsState(p.fraction, label = "frac")
    Surface(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        shape = RoundedCornerShape(20.dp),
        color = MaterialTheme.colorScheme.primaryContainer,
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            CircularProgressIndicator(
                progress = { animated }, modifier = Modifier.size(44.dp), strokeWidth = 4.dp,
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
private fun ActiveRow(p: Progress) {
    val animated by animateFloatAsState(p.fraction, label = "rowFrac")
    Surface(Modifier.fillMaxWidth().padding(vertical = 4.dp), shape = RoundedCornerShape(18.dp), color = MaterialTheme.colorScheme.surfaceContainerLow) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            CircularProgressIndicator(
                progress = { animated }, modifier = Modifier.size(42.dp), strokeWidth = 4.dp,
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
private fun ToggleChip(label: String, checked: Boolean, onChange: (Boolean) -> Unit, modifier: Modifier = Modifier) {
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp),
        color = if (checked) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surfaceContainerHighest,
    ) {
        Row(Modifier.padding(horizontal = 12.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(label, Modifier.weight(1f), style = MaterialTheme.typography.labelLarge)
            MiniSwitch(checked, onChange)
        }
    }
}

@Composable
private fun MiniSwitch(checked: Boolean, onChange: (Boolean) -> Unit) {
    Box(
        Modifier
            .size(width = 40.dp, height = 24.dp)
            .clip(CircleShape)
            .background(if (checked) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHighest)
            .clickable { onChange(!checked) },
        contentAlignment = if (checked) Alignment.CenterEnd else Alignment.CenterStart,
    ) {
        Box(
            Modifier.padding(3.dp).size(18.dp).clip(CircleShape).background(Color.White),
        )
    }
}

@Composable
private fun Avatar(p: Peer, size: Int, showBadge: Boolean = true) {
    val c = platformColor(p.platform)
    Box(Modifier.size(size.dp)) {
        Box(
            Modifier.fillMaxSize().clip(RoundedCornerShape((size / 4).dp))
                .background(Brush.linearGradient(listOf(c, c.copy(alpha = 0.72f)))),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                p.name.take(1).uppercase(),
                color = Color.White, fontWeight = FontWeight.SemiBold,
                style = if (size >= 56) MaterialTheme.typography.headlineSmall else MaterialTheme.typography.titleMedium,
            )
        }
        if (showBadge && size >= 28) {
            Box(
                Modifier.align(Alignment.BottomStart)
                    .offset(x = (-(size * 0.06f)).dp, y = (size * 0.06f).dp)
                    .size((size * 0.42f).dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.surface),
                contentAlignment = Alignment.Center,
            ) {
                Icon(platformIcon(p.platform), null, Modifier.size((size * 0.22f).dp), tint = c)
            }
        }
        Box(
            Modifier.align(Alignment.BottomEnd)
                .offset(x = (size * 0.04f).dp, y = (size * 0.04f).dp)
                .size((size * 0.26f).dp).clip(CircleShape).background(Green),
        )
    }
}

@Composable
private fun Empty(text: String) {
    Column(
        Modifier.fillMaxWidth().padding(vertical = 72.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            Modifier.size(80.dp).clip(CircleShape).background(MaterialTheme.colorScheme.surfaceContainerHigh),
            contentAlignment = Alignment.Center,
        ) { Icon(Icons.AutoMirrored.Filled.Send, null, Modifier.size(34.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant) }
        Spacer(Modifier.height(14.dp))
        Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant)
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

// ===================== 工具 =====================

private data class DayGroup(val label: String, val items: List<HistoryItem>)

private fun groupByDay(items: List<HistoryItem>): List<DayGroup> {
    val fmt = SimpleDateFormat("M月d日", Locale.CHINA)
    val today = Calendar.getInstance()
    val yesterday = Calendar.getInstance().apply { add(Calendar.DAY_OF_YEAR, -1) }
    val map = LinkedHashMap<String, MutableList<HistoryItem>>()
    for (h in items) {
        val cal = Calendar.getInstance().apply { timeInMillis = h.endedAtMs }
        val label = when {
            sameDay(cal, today) -> "今天"
            sameDay(cal, yesterday) -> "昨天"
            else -> fmt.format(Date(h.endedAtMs))
        }
        map.getOrPut(label) { mutableListOf() }.add(h)
    }
    return map.map { DayGroup(it.key, it.value) }
}

private fun sameDay(a: Calendar, b: Calendar): Boolean =
    a.get(Calendar.YEAR) == b.get(Calendar.YEAR) &&
        a.get(Calendar.DAY_OF_YEAR) == b.get(Calendar.DAY_OF_YEAR)

private fun transfersSubtitle(items: List<HistoryItem>, filtering: Boolean, dir: String, status: String): String {
    if (items.isEmpty()) return if (filtering) "没有符合条件的记录" else "还没有传输记录"
    val total = items.sumOf { it.bytes }
    val prefix = if (filtering) "${dirLabel(dir)} · ${statusLabel(status)}" else "全部"
    return "$prefix · ${items.size} 项 · ${size(total)}"
}

private fun dirLabel(v: String) = when (v) { "sent" -> "发送"; "received" -> "接收"; else -> "全部" }
private fun statusLabel(v: String) = when (v) { "done" -> "完成"; "failed" -> "失败"; else -> "全部" }

private fun ext(name: String) = name.substringAfterLast('.', "").lowercase()

private fun fileIcon(name: String) = when (ext(name)) {
    "png", "jpg", "jpeg", "gif", "heic", "webp", "tiff", "bmp" -> Icons.Default.Image
    "mp4", "mov", "m4v", "avi", "mkv", "webm" -> Icons.Default.Movie
    "mp3", "wav", "m4a", "aac", "flac" -> Icons.Default.MusicNote
    "pdf" -> Icons.Default.PictureAsPdf
    "zip", "rar", "7z", "tar", "gz" -> Icons.Default.FolderZip
    else -> Icons.Default.InsertDriveFile
}

private fun fileTint(e: String): Pair<Color, Color> = when (e) {
    "png", "jpg", "jpeg", "gif", "heic", "webp", "tiff", "bmp" -> Color(0xFF0FB8E6).copy(alpha = 0.16f) to Color(0xFF0A84FF)
    "mp4", "mov", "m4v", "avi", "mkv", "webm" -> Color(0xFF9B51E0).copy(alpha = 0.16f) to Color(0xFF9B51E0)
    "mp3", "wav", "m4a", "aac", "flac" -> Color(0xFFE0409A).copy(alpha = 0.16f) to Color(0xFFE0409A)
    "pdf" -> Color(0xFFFF3B30).copy(alpha = 0.16f) to Color(0xFFE0352B)
    "zip", "rar", "7z", "tar", "gz" -> Color(0xFFFF9500).copy(alpha = 0.16f) to Color(0xFFCC7A00)
    else -> Color(0xFF8E8E93).copy(alpha = 0.16f) to Color(0xFF6C6C70)
}

private fun platformColor(p: String): Color = when (p) {
    "ios" -> Color(0xFF0A84FF)
    "android" -> Color(0xFF34C759)
    "windows" -> Color(0xFF0078D4)
    "linux" -> Color(0xFFFF9500)
    "macos" -> Color(0xFF7C3AED)
    else -> Color(0xFF8E8E93)
}

private fun platformIcon(p: String) = when (p) {
    "ios" -> Icons.Default.Smartphone
    "macos" -> Icons.Default.Laptop
    "android" -> Icons.Default.PhoneAndroid
    "windows" -> Icons.Default.Computer
    "linux" -> Icons.Default.Terminal
    else -> Icons.Default.DevicesOther
}

private fun statusColor(status: String): Color = when (status) {
    "completed" -> Green
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
    return DateUtils.getRelativeTimeSpanString(
        ms, System.currentTimeMillis(), DateUtils.MINUTE_IN_MILLIS, DateUtils.FORMAT_ABBREV_RELATIVE,
    ).toString()
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
