package com.mankong.sendsent

import android.app.Activity
import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import app.tauri.annotation.Command
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.concurrent.ConcurrentHashMap

/**
 * Discovery via the platform NSD service (NsdManager).
 *
 * Raw-socket mDNS (mdns-sd) fails on modern Android because SELinux denies
 * `bind()` on NETLINK_ROUTE sockets for ordinary apps, which makes libc
 * `getifaddrs()` return EACCES. NsdManager runs the mDNS/DNS-SD stack in the
 * system process, so it works inside the app sandbox and stays interoperable
 * with the Bonjour/mDNS peers used on iOS/macOS/desktop.
 */
class NsdPlugin(private val activity: Activity) : Plugin(activity) {
    private val nsdManager: NsdManager by lazy {
        activity.getSystemService(Context.NSD_SERVICE) as NsdManager
    }

    private data class Entry(
        val host: String,
        val port: Int,
        val txt: Map<String, String>,
        val lastSeen: Long,
    )

    private val discovered = ConcurrentHashMap<String, Entry>()
    private var registrationListener: NsdManager.RegistrationListener? = null
    private var discoveryListener: NsdManager.DiscoveryListener? = null

    @Command
    fun register(invoke: Invoke) {
        try {
            val args = invoke.getArgs()
            val name = args.optString("name", "android")
            val id = args.optString("id", "")
            val plat = args.optString("plat", "android")
            val port = args.optInt("port", 52225)
            val ip = args.optString("ip", "")

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

            registrationListener?.let { runCatching { nsdManager.unregisterService(it) } }
            val listener = object : NsdManager.RegistrationListener {
                override fun onServiceRegistered(serviceInfo: NsdServiceInfo) {
                    android.util.Log.d("NsdPlugin", "registered as ${serviceInfo.serviceName}")
                }
                override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                    android.util.Log.e("NsdPlugin", "registration failed: $errorCode")
                }
                override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) {}
                override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {}
            }
            registrationListener = listener
            nsdManager.registerService(info, NsdManager.PROTOCOL_DNS_SD, listener)
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject(e.message)
        }
    }

    @Command
    fun browse(invoke: Invoke) {
        try {
            if (discoveryListener == null) {
                val listener = object : NsdManager.DiscoveryListener {
                    override fun onStartDiscoveryFailed(type: String, errorCode: Int) {
                        android.util.Log.e("NsdPlugin", "discovery start failed: $errorCode")
                    }
                    override fun onStopDiscoveryFailed(type: String, errorCode: Int) {}
                    override fun onDiscoveryStarted(type: String) {}
                    override fun onDiscoveryStopped(type: String) {}
                    override fun onServiceFound(service: NsdServiceInfo) {
                        if (service.serviceName == null) return
                        runCatching {
                            nsdManager.resolveService(service, object : NsdManager.ResolveListener {
                                override fun onResolveFailed(info: NsdServiceInfo, errorCode: Int) {}
                                override fun onServiceResolved(info: NsdServiceInfo) {
                                    val txt = HashMap<String, String>()
                                    for ((k, v) in info.attributes) {
                                        txt[k] = String(v, Charsets.UTF_8)
                                    }
                                    val host = info.host?.hostAddress ?: return
                                    discovered[info.serviceName] = Entry(
                                        host, info.port, txt, System.currentTimeMillis(),
                                    )
                                }
                            })
                        }
                    }
                    override fun onServiceLost(service: NsdServiceInfo) {
                        discovered.remove(service.serviceName)
                    }
                }
                discoveryListener = listener
                nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
            }
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject(e.message)
        }
    }

    /** Returns services seen in the last 60s. Rust polls this on a timer. */
    @Command
    fun poll(invoke: Invoke) {
        val now = System.currentTimeMillis()
        val services = JSArray()
        val stale = ArrayList<String>()
        for ((name, e) in discovered) {
            if (now - e.lastSeen > 60_000) {
                stale.add(name)
                continue
            }
            val o = JSObject()
            o.put("name", name)
            o.put("host", e.host)
            o.put("port", e.port)
            val txt = JSObject()
            for ((k, v) in e.txt) txt.put(k, v)
            o.put("txt", txt)
            services.put(o)
        }
        for (s in stale) discovered.remove(s)
        val ret = JSObject()
        ret.put("services", services)
        invoke.resolve(ret)
    }

    companion object {
        private const val SERVICE_TYPE = "_sendsent._tcp"
    }
}
