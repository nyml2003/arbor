package com.arbor.wififinder.ui

import android.Manifest
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.net.wifi.ScanResult
import android.net.wifi.WifiManager
import android.os.Bundle
import android.os.Looper
import androidx.core.content.ContextCompat
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import kotlin.coroutines.resume
import org.json.JSONArray
import org.json.JSONObject

// region Data

data class WifiNetwork(
    val ssid: String,
    val bssid: String,
    val rssi: Int,
    val frequency: Int,
    val capabilities: String
)

data class TrajectoryPoint(
    val lat: Double,
    val lng: Double,
    val rssi: Int,
    val accuracy: Float,
    val timestamp: Long
)

data class RadarUiState(
    val isScanning: Boolean = false,
    val networks: List<WifiNetwork> = emptyList(),
    val currentRssi: Int? = null,
    val strongestBssid: String? = null,
    val frequency: Int? = null,
    val band: String = "",
    val signalHistory: List<Int> = emptyList(),
    val lastScanTime: Long = 0,
    val error: String? = null,
    val wifiDisabled: Boolean = false,
    val permissionDenied: Boolean = false,
    // GPS 轨迹
    val currentLat: Double = 0.0,
    val currentLng: Double = 0.0,
    val gpsAccuracy: Float = 0f,
    val trajectory: List<TrajectoryPoint> = emptyList(),
    // HTTP 回传
    val serverUrl: String = ""
)

// endregion

// region Scanner

class WifiScanner(private val context: Context) {

    private val wifiManager: WifiManager =
        context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager

    private val locationManager: LocationManager? =
        context.applicationContext.getSystemService(Context.LOCATION_SERVICE) as? LocationManager

    val isWifiEnabled: Boolean get() = wifiManager.isWifiEnabled

    /** 最新的 GPS 位置，由 requestLocationUpdates 持续更新 */
    @Volatile
    var latestLocation: Location? = null

    private var gpsListener: LocationListener? = null

    fun hasLocationPermission(): Boolean {
        val fine = ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION)
        return fine == PackageManager.PERMISSION_GRANTED
    }

    /** 开始监听 GPS 位置更新 */
    fun startGps() {
        if (!hasLocationPermission()) return
        val lm = locationManager ?: return
        gpsListener = object : LocationListener {
            override fun onLocationChanged(loc: Location) { latestLocation = loc }
            override fun onProviderDisabled(provider: String) {}
            override fun onProviderEnabled(provider: String) {}
            override fun onStatusChanged(provider: String, status: Int, extras: Bundle?) {}
        }
        try {
            // GPS
            lm.requestLocationUpdates(LocationManager.GPS_PROVIDER, 1000L, 0.5f, gpsListener!!, Looper.getMainLooper())
        } catch (_: Exception) {}
        try {
            // 网络定位作为 fallback
            lm.requestLocationUpdates(LocationManager.NETWORK_PROVIDER, 3000L, 1f, gpsListener!!, Looper.getMainLooper())
        } catch (_: Exception) {}
    }

    /** 停止 GPS 监听 */
    fun stopGps() {
        gpsListener?.let { listener ->
            try { locationManager?.removeUpdates(listener) } catch (_: Exception) {}
        }
        gpsListener = null
        latestLocation = null
    }

    /** 执行一次 Wi-Fi 扫描 */
    suspend fun scanForSsid(targetSsid: String): List<WifiNetwork> {
        if (!hasLocationPermission()) return emptyList()
        return withContext(Dispatchers.Default) {
            try {
                scanViaReceiver()
            } catch (_: Exception) {
                wifiManager.scanResults.toNetworks(targetSsid)
            }
        }
    }

    private suspend fun scanViaReceiver(): List<WifiNetwork> {
        return suspendCancellableCoroutine { cont ->
            val receiver = object : BroadcastReceiver() {
                override fun onReceive(ctx: Context?, intent: Intent?) {
                    val updated = intent?.getBooleanExtra(WifiManager.EXTRA_RESULTS_UPDATED, false) ?: false
                    if (updated) {
                        try { ctx?.unregisterReceiver(this) } catch (_: Exception) {}
                        if (cont.isActive) {
                            cont.resume(wifiManager.scanResults)
                        }
                    }
                }
            }
            cont.invokeOnCancellation {
                try { context.unregisterReceiver(receiver) } catch (_: Exception) {}
            }
            context.registerReceiver(receiver, IntentFilter(WifiManager.SCAN_RESULTS_AVAILABLE_ACTION))
            if (!wifiManager.startScan()) {
                try { context.unregisterReceiver(receiver) } catch (_: Exception) {}
                if (cont.isActive) {
                    cont.resume(wifiManager.scanResults)
                }
            }
        }.toNetworks("")
    }

    private fun List<ScanResult>.toNetworks(targetSsid: String): List<WifiNetwork> {
        return this
            .filter {
                val match = it.SSID.equals(targetSsid, ignoreCase = true) ||
                        it.BSSID.equals(targetSsid, ignoreCase = true)
                if (targetSsid.isBlank()) true else match
            }
            .map {
                WifiNetwork(
                    ssid = it.SSID,
                    bssid = it.BSSID,
                    rssi = it.level,
                    frequency = it.frequency,
                    capabilities = it.capabilities
                )
            }
            .sortedByDescending { it.rssi }
    }

    companion object {
        fun getWifiIp(context: Context): String = HttpDataServer.getWifiIp(context)
    }
}

// endregion

// region ViewModel

class WifiScannerViewModel(application: android.app.Application) : AndroidViewModel(application) {

    private val scanner = WifiScanner(application)
    private val httpServer = HttpDataServer(port = 8765)

    private val _uiState = MutableStateFlow(RadarUiState())
    val uiState: StateFlow<RadarUiState> = _uiState.asStateFlow()

    private val _targetSsid = MutableStateFlow("9277")
    val targetSsid: StateFlow<String> = _targetSsid.asStateFlow()

    fun setTargetSsid(ssid: String) {
        _targetSsid.value = ssid
    }

    private var scanJob: Job? = null

    fun startScanning() {
        if (!scanner.hasLocationPermission()) {
            _uiState.update { it.copy(permissionDenied = true, error = "需要位置权限才能扫描 Wi-Fi") }
            return
        }
        if (!scanner.isWifiEnabled) {
            _uiState.update { it.copy(wifiDisabled = true, error = "请先打开 Wi-Fi") }
            return
        }

        scanJob?.cancel()
        scanner.startGps()

        // 启动 HTTP 数据服务器
        val ip = WifiScanner.getWifiIp(getApplication())
        val serverUrl = if (ip != "0.0.0.0") "http://$ip:8765/data" else ""

        // 在后台启动 HTTP Server
        Thread { httpServer.runBlocking() }.start()

        scanJob = viewModelScope.launch {
            _uiState.update { it.copy(wifiDisabled = false, permissionDenied = false, serverUrl = serverUrl) }

            while (isActive) {
                _uiState.update { it.copy(isScanning = true) }

                val networks = scanner.scanForSsid(_targetSsid.value)
                val strongest = networks.firstOrNull()
                val loc = scanner.latestLocation
                val currentLat = loc?.latitude ?: 0.0
                val currentLng = loc?.longitude ?: 0.0
                val gpsAccuracy = loc?.accuracy ?: 0f

                _uiState.update { state ->
                    val newPoint = if (currentLat != 0.0 || currentLng != 0.0) {
                        TrajectoryPoint(
                            lat = currentLat,
                            lng = currentLng,
                            rssi = strongest?.rssi ?: -100,
                            accuracy = gpsAccuracy,
                            timestamp = System.currentTimeMillis()
                        )
                    } else null

                    state.copy(
                        isScanning = false,
                        networks = networks,
                        currentRssi = strongest?.rssi,
                        strongestBssid = strongest?.bssid,
                        frequency = strongest?.frequency,
                        band = frequencyToBand(strongest?.frequency ?: 0),
                        signalHistory = if (strongest != null) {
                            (state.signalHistory + strongest.rssi).takeLast(120)
                        } else state.signalHistory,
                        currentLat = currentLat,
                        currentLng = currentLng,
                        gpsAccuracy = gpsAccuracy,
                        trajectory = if (newPoint != null) {
                            (state.trajectory + newPoint).takeLast(500)
                        } else state.trajectory,
                        lastScanTime = System.currentTimeMillis(),
                        error = when {
                            !scanner.isWifiEnabled -> "Wi-Fi 已关闭"
                            networks.isEmpty() -> "未找到 \"${_targetSsid.value}\""
                            else -> null
                        }
                    )
                }

                // 更新 HTTP Server 的数据
                updateHttpJson()

                val delayMs = if (networks.isEmpty()) 5000L else 3000L
                delay(delayMs)
            }
        }
    }

    private fun updateHttpJson() {
        val state = _uiState.value
        val json = JSONObject().apply {
            put("ssid", _targetSsid.value)
            put("bssid", state.strongestBssid ?: "")
            put("rssi", state.currentRssi ?: -100)
            put("frequency", state.frequency ?: 0)
            put("band", state.band)
            put("channel", channelFromFrequency(state.frequency ?: 0))
            put("distance", estimateDistanceText(state.currentRssi ?: -100))
            put("lat", state.currentLat)
            put("lng", state.currentLng)
            put("gpsAccuracy", state.gpsAccuracy.toDouble())
            put("timestamp", state.lastScanTime)

            // 信号历史
            put("signalHistory", JSONArray(state.signalHistory))

            // 轨迹
            val trajArray = JSONArray()
            for (p in state.trajectory.takeLast(100)) {
                trajArray.put(JSONObject().apply {
                    put("lat", p.lat)
                    put("lng", p.lng)
                    put("rssi", p.rssi)
                    put("accuracy", p.accuracy.toDouble())
                    put("timestamp", p.timestamp)
                })
            }
            put("trajectory", trajArray)
        }
        httpServer.latestJson = json.toString()
    }

    fun stopScanning() {
        scanJob?.cancel()
        scanner.stopGps()
        httpServer.stop()
        _uiState.update { it.copy(isScanning = false, serverUrl = "") }
    }

    override fun onCleared() {
        super.onCleared()
        stopScanning()
    }

    private fun frequencyToBand(freq: Int): String = when {
        freq >= 5900 -> "6 GHz (Wi-Fi 6E)"
        freq >= 5000 -> "5 GHz"
        freq >= 2400 -> "2.4 GHz"
        freq > 0 -> "${freq} MHz"
        else -> ""
    }

    private fun channelFromFrequency(freq: Int): String = when {
        freq in 2412..2484 -> "CH ${(freq - 2407) / 5}"
        freq in 5035..5865 -> "CH ${(freq - 5000) / 5}"
        freq in 5945..7105 -> "CH ${(freq - 5950) / 5}"
        else -> ""
    }

    private fun estimateDistanceText(rssi: Int): String = when {
        rssi >= -40 -> "非常近 (<2m)"
        rssi >= -50 -> "很近 (~3m)"
        rssi >= -60 -> "较近 (~6-10m)"
        rssi >= -70 -> "中等 (~15m)"
        rssi >= -80 -> "较远 (~25m)"
        else -> "很远 (>30m)"
    }
}

// endregion
