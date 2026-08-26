package app.nether.vpn

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.util.Log

/**
 * The VpnService behind Nether's VPN mode.
 *
 * It does exactly one interesting thing: build a TUN interface that routes
 * everything, then hand its file descriptor to the Rust side. The packets
 * themselves are never touched here.
 */
class NetherVpnService : VpnService() {

    companion object {
        const val ACTION_START = "app.nether.vpn.START"
        const val ACTION_STOP = "app.nether.vpn.STOP"

        private const val TAG = "NetherVpn"
        private const val CHANNEL_ID = "nether-vpn"
        private const val NOTIFICATION_ID = 0x4e56 // 'NV'

        /** Address of the TUN itself. Link-local to the device, never routed. */
        private const val TUN_V4 = "10.60.0.2"
        private const val TUN_V6 = "fd00:6e65:7468::2"

        /**
         * Set by [VpnPlugin] immediately before the service is started, and
         * consumed exactly once by [onStartCommand]. Called with (fd, null) on
         * success or (-1, message) on failure.
         */
        @Volatile
        @JvmStatic
        var onReady: ((Int, String?) -> Unit)? = null
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            shutdown()
            return START_NOT_STICKY
        }

        startForeground(NOTIFICATION_ID, buildNotification())

        // Take the callback before doing anything that can throw, so a failure
        // still reports back instead of leaving the caller hanging forever.
        val callback = onReady
        onReady = null

        try {
            val mtu = intent?.getIntExtra("mtu", 1500) ?: 1500
            val dns = intent?.getStringExtra("dns") ?: "1.1.1.1"
            val ipv6 = intent?.getBooleanExtra("ipv6", false) ?: false

            val builder = Builder()
                .setSession("Nether")
                .setMtu(mtu)
                .addAddress(TUN_V4, 32)
                .addRoute("0.0.0.0", 0)

            for (server in dns.split(",")) {
                val trimmed = server.trim()
                if (trimmed.isNotEmpty()) {
                    builder.addDnsServer(trimmed)
                }
            }

            // Only claim v6 when the tunnel can actually carry it. Routing v6
            // into a v4-only tunnel black-holes it instead of letting Android
            // fall back to v4.
            if (ipv6) {
                builder.addAddress(TUN_V6, 128)
                builder.addRoute("::", 0)
            }

            // This is what keeps the tunnel from eating its own traffic:
            // Aether dials Cloudflare from inside this process, so our package
            // has to stay outside the VPN or every packet loops back in.
            builder.addDisallowedApplication(packageName)

            val pfd = builder.establish()
                ?: throw IllegalStateException("establish() returned null (consent revoked?)")

            // detachFd hands sole ownership to Rust, which closes it to bring
            // the tunnel down. Keeping the ParcelFileDescriptor alive here too
            // would double-close it.
            val fd = pfd.detachFd()
            Log.i(TAG, "tun established (fd=$fd mtu=$mtu ipv6=$ipv6)")
            callback?.invoke(fd, null)
        } catch (e: Exception) {
            Log.e(TAG, "failed to establish tun", e)
            callback?.invoke(-1, e.message ?: e.toString())
            shutdown()
            return START_NOT_STICKY
        }

        return START_STICKY
    }

    /** The user revoked the VPN from system settings, or another app took over. */
    override fun onRevoke() {
        Log.i(TAG, "vpn revoked by system")
        shutdown()
    }

    private fun shutdown() {
        onReady = null
        stopForeground(true)
        stopSelf()
    }

    private fun buildNotification(): Notification {
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "VPN status",
                NotificationManager.IMPORTANCE_LOW
            )
            channel.setShowBadge(false)
            manager.createNotificationChannel(channel)
        }

        // Tapping the notification returns to the app rather than doing nothing.
        val launch = packageManager.getLaunchIntentForPackage(packageName)
        val pending = launch?.let {
            val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            } else {
                PendingIntent.FLAG_UPDATE_CURRENT
            }
            PendingIntent.getActivity(this, 0, it, flags)
        }

        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }

        return builder
            .setContentTitle("Nether")
            .setContentText("Tunnel active")
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setOngoing(true)
            .also { b -> pending?.let { b.setContentIntent(it) } }
            .build()
    }
}
