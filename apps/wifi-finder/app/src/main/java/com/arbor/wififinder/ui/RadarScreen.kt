package com.arbor.wififinder.ui

import android.Manifest
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.MyLocation
import androidx.compose.material.icons.filled.Navigation
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.SearchOff
import androidx.compose.material.icons.filled.WifiTethering
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material.icons.filled.TrendingDown
import androidx.compose.material.icons.filled.TrendingFlat
import androidx.compose.material.icons.filled.TrendingUp
import androidx.compose.material.icons.filled.Wifi
// import androidx.compose.material.icons.filled.WifiFind // not available in this version
import androidx.compose.material.icons.filled.WifiOff
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.IconButton
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.roundToInt
import kotlin.math.sin

// region Theme Colors

private val DarkBackground = Color(0xFF0D1117)
private val DarkSurface = Color(0xFF161B22)
private val DarkCard = Color(0xFF1C2333)
private val AccentGreen = Color(0xFF4ADE80)
private val AccentYellow = Color(0xFFFACC15)
private val AccentOrange = Color(0xFFFB923C)
private val AccentRed = Color(0xFFF87171)
private val TextPrimary = Color(0xFFE6EDF3)
private val TextSecondary = Color(0xFF8B949E)
private val ScanLineColor = Color(0xFF58A6FF)

// endregion

// region Root Composable

@Composable
fun RadarApp(viewModel: WifiScannerViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.uiState.collectAsState()
    val targetSsid by viewModel.targetSsid.collectAsState()
    var ssidInput by remember { mutableStateOf("9277") }

    // 位置权限请求
    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions()
    ) { grants ->
        if (grants.values.any { it }) {
            viewModel.startScanning()
        } else {
            viewModel.startScanning() // 让 VM 自己检测并显示错误
        }
    }

    // 权限检测 → 自动请求
    LaunchedEffect(Unit) {
        val hasPermission = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            // Android 13+ 可以用 NEARBY_WIFI_DEVICES 替代位置
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) ==
                    android.content.pm.PackageManager.PERMISSION_GRANTED ||
                    context.checkSelfPermission(Manifest.permission.NEARBY_WIFI_DEVICES) ==
                    android.content.pm.PackageManager.PERMISSION_GRANTED
        } else {
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) ==
                    android.content.pm.PackageManager.PERMISSION_GRANTED
        }
        if (!hasPermission) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                permissionLauncher.launch(
                    arrayOf(
                        Manifest.permission.ACCESS_FINE_LOCATION,
                        Manifest.permission.NEARBY_WIFI_DEVICES
                    )
                )
            } else {
                permissionLauncher.launch(arrayOf(Manifest.permission.ACCESS_FINE_LOCATION))
            }
        } else {
            viewModel.startScanning()
        }
    }

    DisposableEffect(Unit) {
        onDispose { viewModel.stopScanning() }
    }

    Scaffold(
        topBar = { RadarTopBar() },
        containerColor = DarkBackground
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp)
                .verticalScroll(rememberScrollState())
        ) {
            // SSID 输入行
            SsidInputRow(
                value = ssidInput,
                onValueChange = { ssidInput = it },
                onSearch = {
                    viewModel.stopScanning()
                    viewModel.setTargetSsid(ssidInput.trim())
                    viewModel.startScanning()
                },
                isScanning = state.isScanning,
                onStop = { viewModel.stopScanning() }
            )

            Spacer(modifier = Modifier.height(12.dp))

            // 主雷达显示区
            RadarGaugeSection(state = state)

            Spacer(modifier = Modifier.height(16.dp))

            // 数据回传 & GPS 信息
            if (state.serverUrl.isNotEmpty() || state.currentLat != 0.0) {
                DataLinkCard(state = state)
                Spacer(modifier = Modifier.height(16.dp))
            }

            // 信号历史
            if (state.signalHistory.size >= 2) {
                SignalTrendCard(state = state)
                Spacer(modifier = Modifier.height(12.dp))
                SignalSparklineCard(history = state.signalHistory)
                Spacer(modifier = Modifier.height(16.dp))
            }

            // BSSID 列表
            if (state.networks.isNotEmpty()) {
                BssidListSection(networks = state.networks)
            }

            // 错误提示
            state.error?.let { error ->
                Spacer(modifier = Modifier.weight(1f))
                ErrorBanner(message = error, state = state)
            }
        }
    }
}

// endregion

// region Top Bar

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun RadarTopBar() {
    TopAppBar(
        title = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    Icons.Filled.WifiTethering,
                    contentDescription = null,
                    tint = AccentGreen,
                    modifier = Modifier.size(28.dp)
                )
                Spacer(modifier = Modifier.width(10.dp))
                Text(
                    text = "WiFi 探测器",
                    fontWeight = FontWeight.Bold,
                    color = TextPrimary
                )
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(containerColor = DarkBackground.copy(alpha = 0.8f))
    )
}

// endregion

// region SSID Input

@Composable
private fun SsidInputRow(
    value: String,
    onValueChange: (String) -> Unit,
    onSearch: () -> Unit,
    isScanning: Boolean,
    onStop: () -> Unit
) {
    val focusManager = LocalFocusManager.current

    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            modifier = Modifier.weight(1f),
            placeholder = { Text("输入 SSID 或 BSSID", color = TextSecondary) },
            leadingIcon = {
                Icon(Icons.Filled.Search, contentDescription = null, tint = AccentGreen)
            },
            trailingIcon = {
                if (value.isNotEmpty()) {
                    IconButton(onClick = { onValueChange("") }) {
                        Icon(
                            Icons.Filled.Close,
                            contentDescription = "清除",
                            tint = TextSecondary
                        )
                    }
                }
            },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { onSearch(); focusManager.clearFocus() }),
            shape = RoundedCornerShape(12.dp),
            colors = OutlinedTextFieldDefaults.colors(
                focusedTextColor = TextPrimary,
                unfocusedTextColor = TextPrimary,
                focusedBorderColor = AccentGreen,
                unfocusedBorderColor = DarkCard,
                cursorColor = AccentGreen,
                focusedContainerColor = DarkCard,
                unfocusedContainerColor = DarkCard
            )
        )

        // 搜索/停止按钮
        if (isScanning) {
            Button(
                onClick = onStop,
                colors = ButtonDefaults.buttonColors(containerColor = AccentRed),
                shape = RoundedCornerShape(12.dp)
            ) {
                Icon(Icons.Filled.Stop, contentDescription = "停止", tint = Color.White)
            }
        } else {
            Button(
                onClick = onSearch,
                colors = ButtonDefaults.buttonColors(containerColor = AccentGreen),
                shape = RoundedCornerShape(12.dp)
            ) {
                Icon(Icons.Filled.PlayArrow, contentDescription = "扫描", tint = DarkBackground)
            }
        }
    }
}

// endregion

// region Radar Gauge

@Composable
private fun RadarGaugeSection(state: RadarUiState) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = DarkCard)
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(320.dp),
            contentAlignment = Alignment.Center
        ) {
            if (state.wifiDisabled) {
                // Wi-Fi 关闭提示
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(
                        Icons.Filled.WifiOff,
                        contentDescription = null,
                        tint = TextSecondary,
                        modifier = Modifier.size(56.dp)
                    )
                    Spacer(modifier = Modifier.height(12.dp))
                    Text("Wi-Fi 未开启", color = TextSecondary, fontSize = 16.sp)
                    Spacer(modifier = Modifier.height(8.dp))
                    val wifiCtx = LocalContext.current
                    Button(
                        onClick = { wifiCtx.startActivity(Intent(Settings.ACTION_WIFI_SETTINGS)) },
                        colors = ButtonDefaults.buttonColors(containerColor = AccentGreen),
                        shape = RoundedCornerShape(10.dp)
                    ) {
                        Icon(Icons.Filled.Wifi, contentDescription = null, tint = DarkBackground)
                        Spacer(modifier = Modifier.width(6.dp))
                        Text("打开 Wi-Fi 设置", color = DarkBackground, fontWeight = FontWeight.Bold)
                    }
                }
            } else if (state.currentRssi == null && !state.isScanning) {
                // 未找到网络
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(
                        Icons.Filled.SearchOff,
                        contentDescription = null,
                        tint = TextSecondary,
                        modifier = Modifier.size(56.dp)
                    )
                    Spacer(modifier = Modifier.height(12.dp))
                    Text("等待扫描…", color = TextSecondary, fontSize = 16.sp)
                }
            } else {
                // 绘制雷达
                RadarCanvas(
                    rssi = state.currentRssi ?: -100,
                    isScanning = state.isScanning,
                    band = state.band,
                    frequency = state.frequency
                )
            }
        }
    }
}

@Composable
private fun RadarCanvas(rssi: Int, isScanning: Boolean, band: String, frequency: Int?) {
    // 平滑动画
    val animatedRssi by animateFloatAsState(
        targetValue = rssi.toFloat().coerceIn(-100f, -20f),
        animationSpec = tween(durationMillis = 800),
        label = "rssi"
    )

    // 扫描线旋转动画
    val infiniteTransition = rememberInfiniteTransition(label = "scan")
    val scanAngle by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(
            animation = tween(2500, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "scanAngle"
    )

    // 脉冲 alpha
    val pulseAlpha by infiniteTransition.animateFloat(
        initialValue = 0.15f,
        targetValue = 0.35f,
        animationSpec = infiniteRepeatable(
            animation = tween(1200, easing = LinearEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse"
    )

    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Canvas(modifier = Modifier.fillMaxSize().padding(24.dp)) {
            val centerX = size.width / 2
            val centerY = size.height / 2
            val radius = min(centerX, centerY) * 0.82f

            // --- 同心圆环（雷达刻度） ---
            for (i in 1..4) {
                val r = radius * i / 4
                drawCircle(
                    color = Color.White.copy(alpha = 0.06f),
                    radius = r,
                    center = Offset(centerX, centerY),
                    style = Stroke(width = 1.5f)
                )
            }

            // --- 十字参考线 ---
            drawLine(
                color = Color.White.copy(alpha = 0.04f),
                start = Offset(centerX - radius, centerY),
                end = Offset(centerX + radius, centerY),
                strokeWidth = 1f
            )
            drawLine(
                color = Color.White.copy(alpha = 0.04f),
                start = Offset(centerX, centerY - radius),
                end = Offset(centerX, centerY + radius),
                strokeWidth = 1f
            )

            // --- 信号强度映射到距离中心的距离 ---
            // -20 dBm (极强) → 靠近中心 (0.1 * radius)
            // -100 dBm (极弱) → 边缘 (0.92 * radius)
            val normalized = ((-20f).coerceAtLeast(rssi.toFloat()).coerceAtMost(-100f) + 100) / 80f
            val distFromCenter = (0.92f - normalized * 0.82f) * radius

            // 信号点颜色
            val signalColor = rssiToColor(animatedRssi.toInt())

            // --- 脉冲波纹 ---
            if (isScanning) {
                drawCircle(
                    color = signalColor.copy(alpha = pulseAlpha),
                    radius = distFromCenter,
                    center = Offset(centerX, centerY),
                    style = Stroke(width = 2f)
                )
                // 第二圈波纹
                drawCircle(
                    color = signalColor.copy(alpha = pulseAlpha * 0.5f),
                    radius = distFromCenter * 1.15f,
                    center = Offset(centerX, centerY),
                    style = Stroke(width = 1f)
                )
            }

            // --- 信号点 ---
            val dotRadius = 10f
            // 外层光晕
            drawCircle(
                color = signalColor.copy(alpha = 0.25f),
                radius = dotRadius * 3f,
                center = Offset(centerX, centerY)
            )
            drawCircle(
                color = signalColor.copy(alpha = 0.15f),
                radius = dotRadius * 5f,
                center = Offset(centerX, centerY)
            )
            // 实心点
            drawCircle(
                color = signalColor,
                radius = dotRadius,
                center = Offset(centerX, centerY)
            )

            // --- 扫描线 ---
            if (isScanning) {
                val scanRad = Math.toRadians(scanAngle.toDouble()).toFloat()
                val sweepPath = Path().apply {
                    moveTo(centerX, centerY)
                    arcTo(
                        rect = androidx.compose.ui.geometry.Rect(
                            centerX - radius, centerY - radius,
                            centerX + radius, centerY + radius
                        ),
                        startAngleDegrees = scanAngle - 15f,
                        sweepAngleDegrees = 30f,
                        forceMoveTo = false
                    )
                    close()
                }
                drawPath(
                    path = sweepPath,
                    brush = Brush.sweepGradient(
                        0f to ScanLineColor.copy(alpha = 0f),
                        0.5f to ScanLineColor.copy(alpha = 0.12f),
                        1f to ScanLineColor.copy(alpha = 0f),
                        center = Offset(centerX, centerY)
                    )
                )
            }

            // --- 刻度标记 ---
            for (dbm in listOf(-90, -80, -70, -60, -50, -40, -30)) {
                val n = (dbm + 100) / 80f
                val r = (0.92f - n * 0.82f) * radius
                val tickColor = rssiToColor(dbm).copy(alpha = 0.5f)
                // 右侧小刻线
                drawLine(
                    color = tickColor,
                    start = Offset(centerX + r - 8, centerY),
                    end = Offset(centerX + r + 4, centerY),
                    strokeWidth = 1.5f
                )
            }
        }

        // --- 叠加文字：dBm 数值 ---
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            val color = rssiToColor(animatedRssi.toInt())
            Text(
                text = "${animatedRssi.roundToInt()}",
                fontSize = 72.sp,
                fontWeight = FontWeight.Bold,
                color = color,
                fontFamily = FontFamily.Default
            )
            Text(
                text = "dBm",
                fontSize = 18.sp,
                color = color.copy(alpha = 0.7f),
                letterSpacing = 4.sp
            )

            // 距离估算
            Spacer(modifier = Modifier.height(6.dp))
            val distance = estimateDistance(animatedRssi.toInt())
            Text(
                text = distance,
                fontSize = 14.sp,
                color = TextSecondary
            )

            // 频段
            if (band.isNotEmpty()) {
                Text(
                    text = band,
                    fontSize = 12.sp,
                    color = TextSecondary.copy(alpha = 0.7f)
                )
            }
        }

        // 右上角：扫描状态指示
        if (isScanning) {
            CircularProgressIndicator(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(12.dp)
                    .size(18.dp),
                color = AccentGreen,
                strokeWidth = 2.dp
            )
        }
    }
}

// endregion

// region Data Link Card

@Composable
private fun DataLinkCard(state: RadarUiState) {
    val context = LocalContext.current
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = DarkCard)
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            // 标题
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    Icons.Filled.WifiTethering,
                    contentDescription = null,
                    tint = AccentGreen,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text("数据回传", color = AccentGreen, fontSize = 14.sp, fontWeight = FontWeight.Medium)
            }

            Spacer(modifier = Modifier.height(10.dp))

            // 服务器 URL
            if (state.serverUrl.isNotEmpty()) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(8.dp))
                        .background(DarkBackground)
                        .padding(10.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = state.serverUrl,
                        color = ScanLineColor,
                        fontSize = 13.sp,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.weight(1f)
                    )
                    IconButton(
                        onClick = {
                            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as? android.content.ClipboardManager
                            clipboard?.setPrimaryClip(android.content.ClipData.newPlainText("url", state.serverUrl))
                        },
                        modifier = Modifier.size(32.dp)
                    ) {
                        Icon(
                            Icons.Filled.ContentCopy,
                            contentDescription = "复制",
                            tint = TextSecondary,
                            modifier = Modifier.size(16.dp)
                        )
                    }
                }
                Text(
                    text = "👆 PC 浏览器打开此地址即可获取实时数据",
                    color = TextSecondary.copy(alpha = 0.6f),
                    fontSize = 11.sp
                )
            }

            // GPS 坐标（如果有）
            if (state.currentLat != 0.0 || state.currentLng != 0.0) {
                Spacer(modifier = Modifier.height(10.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Filled.MyLocation,
                        contentDescription = null,
                        tint = AccentOrange,
                        modifier = Modifier.size(16.dp)
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = "%.6f, %.6f".format(state.currentLat, state.currentLng),
                        color = TextSecondary,
                        fontSize = 12.sp,
                        fontFamily = FontFamily.Monospace
                    )
                    Spacer(modifier = Modifier.weight(1f))
                    Text(
                        text = "轨迹: ${state.trajectory.size}点",
                        color = TextSecondary.copy(alpha = 0.7f),
                        fontSize = 11.sp
                    )
                }
                if (state.gpsAccuracy > 0) {
                    Text(
                        text = "GPS 精度: ±%.1fm".format(state.gpsAccuracy),
                        color = TextSecondary.copy(alpha = 0.5f),
                        fontSize = 10.sp
                    )
                }
            }
        }
    }
}

// endregion

// region Signal Trend

@Composable
private fun SignalTrendCard(state: RadarUiState) {
    val history = state.signalHistory
    val trend = if (history.size >= 3) {
        val recent = history.takeLast(3)
        recent.last() - recent.first()
    } else 0

    val (trendText, trendIcon, trendColor) = when {
        trend > 3 -> Triple("信号增强中", Icons.Filled.TrendingUp, AccentGreen)
        trend < -3 -> Triple("信号减弱中", Icons.Filled.TrendingDown, AccentRed)
        else -> Triple("信号稳定", Icons.Filled.TrendingFlat, AccentYellow)
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = DarkCard)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(trendIcon, contentDescription = null, tint = trendColor, modifier = Modifier.size(24.dp))
            Spacer(modifier = Modifier.width(8.dp))
            Text(trendText, color = trendColor, fontSize = 14.sp, fontWeight = FontWeight.Medium)
            Spacer(modifier = Modifier.weight(1f))
            Text(
                text = "△ ${if (trend > 0) "+" else ""}$trend dBm",
                color = trendColor.copy(alpha = 0.8f),
                fontSize = 13.sp,
                fontFamily = FontFamily.Monospace
            )
        }
    }
}

// endregion

// region Signal Sparkline

@Composable
private fun SignalSparklineCard(history: List<Int>) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = DarkCard)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = "信号历史 (最近 ${history.size} 次)",
                color = TextSecondary,
                fontSize = 12.sp
            )
            Spacer(modifier = Modifier.height(8.dp))
            Canvas(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(60.dp)
            ) {
                if (history.size < 2) return@Canvas

                val minRssi = history.min()
                val maxRssi = history.max()
                val range = if (maxRssi == minRssi) 1f else (maxRssi - minRssi).toFloat()

                val stepX = size.width / (history.size - 1).coerceAtLeast(1)
                val padY = 8f

                val path = Path()
                val fillPath = Path()

                history.forEachIndexed { i, rssi ->
                    val x = i * stepX
                    val y = padY + (maxRssi - rssi) / range * (size.height - 2 * padY)

                    if (i == 0) {
                        path.moveTo(x, y)
                        fillPath.moveTo(x, size.height)
                        fillPath.lineTo(x, y)
                    } else {
                        path.lineTo(x, y)
                        fillPath.lineTo(x, y)
                    }
                }

                // 填充
                fillPath.lineTo((history.size - 1) * stepX, size.height)
                fillPath.close()

                drawPath(
                    path = fillPath,
                    brush = Brush.verticalGradient(
                        0f to AccentGreen.copy(alpha = 0.25f),
                        1f to AccentGreen.copy(alpha = 0.02f)
                    )
                )

                // 线条
                drawPath(
                    path = path,
                    color = AccentGreen,
                    style = Stroke(width = 2f, cap = StrokeCap.Round)
                )

                // 最后一个数据点
                val lastX = (history.size - 1) * stepX
                val lastY = padY + (maxRssi - history.last()) / range * (size.height - 2 * padY)
                drawCircle(color = AccentGreen, radius = 4f, center = Offset(lastX, lastY))
            }

            // X 轴标签
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(text = "-100 dBm", color = TextSecondary.copy(alpha = 0.5f), fontSize = 10.sp)
                Text(text = "-20 dBm", color = TextSecondary.copy(alpha = 0.5f), fontSize = 10.sp)
            }
        }
    }
}

// endregion

// region BSSID List

@Composable
private fun BssidListSection(networks: List<WifiNetwork>) {
    Text(
        text = "发现的接入点",
        color = TextSecondary,
        fontSize = 12.sp,
        modifier = Modifier.padding(bottom = 6.dp)
    )

    LazyColumn(
        verticalArrangement = Arrangement.spacedBy(6.dp),
        modifier = Modifier.height(140.dp)
    ) {
        items(networks, key = { it.bssid }) { net ->
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(10.dp),
                colors = CardDefaults.cardColors(containerColor = DarkCard)
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 14.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // 信号强度圆形指示
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(CircleShape)
                            .background(rssiToColor(net.rssi).copy(alpha = 0.2f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "${net.rssi}",
                            fontSize = 13.sp,
                            fontWeight = FontWeight.Bold,
                            color = rssiToColor(net.rssi)
                        )
                    }

                    Spacer(modifier = Modifier.width(12.dp))

                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = net.bssid.uppercase(),
                            fontSize = 14.sp,
                            fontFamily = FontFamily.Monospace,
                            color = TextPrimary,
                            fontWeight = FontWeight.Medium
                        )
                        Text(
                            text = "${net.frequency} MHz · ${channelFromFrequency(net.frequency)}",
                            fontSize = 11.sp,
                            color = TextSecondary
                        )
                    }

                    // 信号条
                    SignalBars(rssi = net.rssi)
                }
            }
        }
    }
}

@Composable
private fun SignalBars(rssi: Int) {
    val bars = when {
        rssi >= -50 -> 4
        rssi >= -65 -> 3
        rssi >= -75 -> 2
        rssi >= -85 -> 1
        else -> 0
    }
    val color = rssiToColor(rssi)

    Row(
        verticalAlignment = Alignment.Bottom,
        horizontalArrangement = Arrangement.spacedBy(2.dp)
    ) {
        for (i in 1..4) {
            Box(
                modifier = Modifier
                    .width(3.dp)
                    .height((8 + i * 4).dp)
                    .clip(RoundedCornerShape(1.dp))
                    .background(if (i <= bars) color else color.copy(alpha = 0.15f))
            )
        }
    }
}

// endregion

// region Error Banner

@Composable
private fun ErrorBanner(message: String, state: RadarUiState) {
    val context = LocalContext.current
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(bottom = 16.dp),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (state.wifiDisabled || state.permissionDenied)
                AccentOrange.copy(alpha = 0.15f) else DarkCard
        )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = if (state.wifiDisabled) Icons.Filled.WifiOff
                else if (state.permissionDenied) Icons.Filled.MyLocation
                else Icons.Filled.SearchOff,
                contentDescription = null,
                tint = AccentOrange,
                modifier = Modifier.size(20.dp)
            )
            Spacer(modifier = Modifier.width(10.dp))
            Text(
                text = message,
                color = TextSecondary,
                fontSize = 13.sp,
                modifier = Modifier.weight(1f)
            )
            if (state.wifiDisabled) {
                Button(
                    onClick = { context.startActivity(Intent(Settings.ACTION_WIFI_SETTINGS)) },
                    colors = ButtonDefaults.buttonColors(containerColor = AccentGreen),
                    shape = RoundedCornerShape(8.dp),
                    contentPadding = ButtonDefaults.TextButtonContentPadding
                ) {
                    Text("去设置", color = DarkBackground, fontSize = 12.sp)
                }
            }
            if (state.permissionDenied) {
                Button(
                    onClick = {
                        context.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                            data = Uri.parse("package:${context.packageName}")
                        })
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = AccentOrange),
                    shape = RoundedCornerShape(8.dp),
                    contentPadding = ButtonDefaults.TextButtonContentPadding
                ) {
                    Text("授权限", color = DarkBackground, fontSize = 12.sp)
                }
            }
        }
    }
}

// endregion

// region Utilities

private fun rssiToColor(rssi: Int): Color = when {
    rssi >= -50 -> AccentGreen
    rssi >= -60 -> Color(0xFF84CC16) // lime
    rssi >= -70 -> AccentYellow
    rssi >= -80 -> AccentOrange
    else -> AccentRed
}

private fun estimateDistance(rssi: Int): String {
    // 基于 2.4 GHz 自由空间路径损耗的粗略估算（室内多径效应下仅供参考）
    return when {
        rssi >= -40 -> "📡 非常近 (<2m)"
        rssi >= -50 -> "📡 很近 (~3m)"
        rssi >= -60 -> "📶 较近 (~6-10m)"
        rssi >= -70 -> "📶 中等 (~15m)"
        rssi >= -80 -> "📶 较远 (~25m)"
        else -> "📡 很远 (>30m)"
    }
}

private fun channelFromFrequency(freq: Int): String = when {
    freq in 2412..2484 -> "CH ${(freq - 2407) / 5}"
    freq in 5035..5865 -> "CH ${(freq - 5000) / 5}"
    freq in 5945..7105 -> "CH ${(freq - 5950) / 5}"
    else -> ""
}

// endregion
