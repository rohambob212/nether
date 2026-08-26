package app.nether.vpn

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import androidx.activity.result.ActivityResult
import androidx.core.content.ContextCompat
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class StartArgs {
    var mtu: Int = 1500
    var dns: String = "1.1.1.1"
    var ipv6: Boolean = false
}

@TauriPlugin
class VpnPlugin(private val activity: Activity) : Plugin(activity) {

    /**
     * Ask for VPN consent. Resolves `{ granted: true }` when the tunnel may be
     * established — either consent was already given, or the user just gave it.
     */
    @Command
    fun prepare(invoke: Invoke) {
        val intent = VpnService.prepare(activity)
        if (intent == null) {
            val result = JSObject()
            result.put("granted", true)
            invoke.resolve(result)
        } else {
            startActivityForResult(invoke, intent, "consentResult")
        }
    }

    @ActivityCallback
    fun consentResult(invoke: Invoke, result: ActivityResult) {
        val response = JSObject()
        response.put("granted", result.resultCode == Activity.RESULT_OK)
        invoke.resolve(response)
    }

    /**
     * Establish the tunnel and resolve with its TUN descriptor.
     *
     * The service answers through [NetherVpnService.onReady] because
     * `establish()` can only be called from inside the service itself.
     */
    @Command
    fun start(invoke: Invoke) {
        val args = invoke.parseArgs(StartArgs::class.java)

        NetherVpnService.onReady = { fd, error ->
            if (error != null) {
                invoke.reject(error)
            } else {
                val result = JSObject()
                result.put("fd", fd)
                invoke.resolve(result)
            }
        }

        val intent = Intent(activity, NetherVpnService::class.java).apply {
            action = NetherVpnService.ACTION_START
            putExtra("mtu", args.mtu)
            putExtra("dns", args.dns)
            putExtra("ipv6", args.ipv6)
        }

        try {
            ContextCompat.startForegroundService(activity, intent)
        } catch (e: Exception) {
            NetherVpnService.onReady = null
            invoke.reject(e.message ?: e.toString())
        }
    }

    @Command
    fun stop(invoke: Invoke) {
        val intent = Intent(activity, NetherVpnService::class.java).apply {
            action = NetherVpnService.ACTION_STOP
        }
        // Not startForegroundService: a stop must not promote a dead service
        // into the foreground just to kill it again.
        activity.startService(intent)
        invoke.resolve(JSObject())
    }
}
