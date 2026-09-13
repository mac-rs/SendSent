package com.mankong.sendsent

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.util.Log
import org.json.JSONObject

/** Android 系统 NsdManager 驱动发现,解析后推给 Rust。 */
class NsdBridge(private val ctx: Context) {
    private val nsd = ctx.getSystemService(Context.NSD_SERVICE) as NsdManager
    private var registration: NsdManager.RegistrationListener? = null
    private var discovery: NsdManager.DiscoveryListener? = null

    fun register(name: String, id: String, plat: String, port: Int, ip: String) {
        val info = NsdServiceInfo().apply {
            serviceName = name
            serviceType = SERVICE_TYPE
            this.port = port
            setAttribute("id", id)
            setAttribute("name", name)
            setAttribute("plat", plat)
            setAttribute("v", "1")
            setAttribute("port", port.toString())
            if (ip.isNotEmpty()) setAttribute("ip", ip)
        }
        registration?.let { runCatching { nsd.unregisterService(it) } }
        val l = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(i: NsdServiceInfo) { Log.d(TAG, "registered ${i.serviceName}") }
            override fun onRegistrationFailed(i: NsdServiceInfo, e: Int) { Log.e(TAG, "register failed $e") }
            override fun onServiceUnregistered(i: NsdServiceInfo) {}
            override fun onUnregistrationFailed(i: NsdServiceInfo, e: Int) {}
        }
        registration = l
        runCatching { nsd.registerService(info, NsdManager.PROTOCOL_DNS_SD, l) }
            .onFailure { Log.e(TAG, "registerService threw", it) }
    }

    fun browse() {
        if (discovery != null) return
        val l = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(t: String) { Log.d(TAG, "discovery started $t") }
            override fun onDiscoveryStopped(t: String) { Log.d(TAG, "discovery stopped $t") }
            override fun onStartDiscoveryFailed(t: String, e: Int) { Log.e(TAG, "start discovery failed $e") }
            override fun onStopDiscoveryFailed(t: String, e: Int) { Log.e(TAG, "stop discovery failed $e") }
            override fun onServiceFound(s: NsdServiceInfo) {
                Log.d(TAG, "found ${s.serviceName} ${s.serviceType}")
                runCatching {
                    nsd.resolveService(s, object : NsdManager.ResolveListener {
                        override fun onResolveFailed(i: NsdServiceInfo, e: Int) { Log.e(TAG, "resolve failed ${i.serviceName} $e") }
                        override fun onServiceResolved(i: NsdServiceInfo) {
                            val host = i.host?.hostAddress ?: return
                            val txt = JSONObject()
                            for ((k, v) in i.attributes) txt.put(k, String(v, Charsets.UTF_8))
                            Log.d(TAG, "resolved ${i.serviceName} $host:${i.port} $txt")
                            Native.nativeOnService(i.serviceName, host, i.port, txt.toString())
                        }
                    })
                }.onFailure { Log.e(TAG, "resolveService threw", it) }
            }
            override fun onServiceLost(s: NsdServiceInfo) {
                Log.d(TAG, "lost ${s.serviceName}")
                Native.nativeOnServiceLost(s.serviceName)
            }
        }
        discovery = l
        runCatching { nsd.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, l) }
            .onFailure { Log.e(TAG, "discoverServices threw", it) }
    }

    companion object {
        private const val TAG = "NsdBridge"
        private const val SERVICE_TYPE = "_sendsent._tcp"
    }
}
