package com.arbor.wififinder.ui

import android.content.Context
import android.net.wifi.WifiManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.ServerSocket
import java.net.Socket

/**
 * 极简 HTTP 服务器，用于把当前 Wi-Fi 信号数据暴露给同局域网内的 PC。
 * 单线程 accept，每次请求返回当前缓存的 JSON。
 */
class HttpDataServer(private val port: Int = 8765) {

    @Volatile
    private var running = false

    @Volatile
    var latestJson: String = "{}"

    private var serverSocket: ServerSocket? = null

    /** 启动服务器（阻塞当前线程），在协程 IO 调度器里调用 */
    fun runBlocking() {
        running = true
        try {
            serverSocket = ServerSocket(port)
            serverSocket?.reuseAddress = true

            while (running) {
                val client: Socket
                try {
                    client = serverSocket?.accept() ?: break
                } catch (_: Exception) {
                    continue
                }
                // 每个连接在独立线程里处理
                Thread { handleClient(client) }.start()
            }
        } catch (_: Exception) {
            // 端口被占用等情况
        } finally {
            try { serverSocket?.close() } catch (_: Exception) {}
        }
    }

    fun stop() {
        running = false
        try {
            serverSocket?.close()
        } catch (_: Exception) {}
    }

    private fun handleClient(socket: Socket) {
        try {
            val input = BufferedReader(InputStreamReader(socket.getInputStream()))
            // 读第一行（GET /data HTTP/1.1）
            val requestLine = input.readLine() ?: return

            // 快速跳过剩余 header
            while (input.readLine()?.isNotEmpty() == true) { /* skip */ }

            val json = latestJson
            val body = json.toByteArray(Charsets.UTF_8)

            val response = buildString {
                append("HTTP/1.1 200 OK\r\n")
                append("Content-Type: application/json; charset=utf-8\r\n")
                append("Access-Control-Allow-Origin: *\r\n")
                append("Content-Length: ${body.size}\r\n")
                append("Connection: close\r\n")
                append("\r\n")
            }

            socket.getOutputStream().use { out ->
                out.write(response.toByteArray(Charsets.UTF_8))
                out.write(body)
                out.flush()
            }
        } catch (_: Exception) {
            // 客户端断开，忽略
        } finally {
            try { socket.close() } catch (_: Exception) {}
        }
    }

    companion object {
        /** 获取当前 Wi-Fi 接口的 IPv4 地址（如 192.168.1.100） */
        fun getWifiIp(context: Context): String {
            return try {
                val wm = context.applicationContext
                    .getSystemService(Context.WIFI_SERVICE) as? WifiManager ?: return "0.0.0.0"
                val ip = wm.connectionInfo.ipAddress
                if (ip == 0) return "0.0.0.0"
                "${ip and 0xFF}.${(ip shr 8) and 0xFF}.${(ip shr 16) and 0xFF}.${(ip shr 24) and 0xFF}"
            } catch (_: Exception) {
                "0.0.0.0"
            }
        }
    }
}
