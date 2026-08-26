//! Thin bridge to the Android `VpnService` that backs Nether's VPN mode.
//!
//! The plugin owns nothing but the handshake: it asks Android for VPN consent,
//! starts the service, and hands the resulting TUN file descriptor back to
//! Rust. Everything the descriptor is *used* for lives in `nether_core::vpn`.
//!
//! `src-tauri` only depends on this crate for `target_os = "android"`, but the
//! crate still compiles everywhere so a plain `cargo check --workspace` covers
//! it. Off Android every call is an error rather than a silent no-op — a VPN
//! that quietly does nothing is worse than one that says it cannot run.

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.nether.vpn";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartConfig {
    /// MTU for the TUN interface.
    pub mtu: u16,
    /// DNS servers advertised to the system inside the tunnel (comma separated).
    pub dns: String,
    /// Route IPv6 into the tunnel too. Off when Aether is running v4-only,
    /// otherwise v6 flows would be black-holed instead of falling back.
    pub ipv6: bool,
}

// Only read on Android; kept unconditional so the wire shape lives in one place.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Debug, Deserialize)]
struct PrepareResponse {
    granted: bool,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Debug, Deserialize)]
struct StartResponse {
    /// Raw TUN descriptor, already detached from its ParcelFileDescriptor —
    /// Rust is its sole owner and closing it tears the tunnel down.
    fd: i32,
}

pub struct NetherVpn<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    // fn() -> R keeps the struct Send + Sync no matter what R is.
    _runtime: std::marker::PhantomData<fn() -> R>,
}

#[cfg(target_os = "android")]
impl<R: Runtime> NetherVpn<R> {
    /// Show the system VPN consent dialog if it has not been granted yet.
    ///
    /// Must be triggered by a user gesture — Android puts a full-screen dialog
    /// in front of whatever the user was doing.
    pub fn prepare(&self) -> Result<bool, String> {
        self.handle
            .run_mobile_plugin::<PrepareResponse>("prepare", ())
            .map(|r| r.granted)
            .map_err(|e| e.to_string())
    }

    /// Establish the tunnel and return its TUN file descriptor.
    pub fn start(&self, config: StartConfig) -> Result<i32, String> {
        self.handle
            .run_mobile_plugin::<StartResponse>("start", config)
            .map(|r| r.fd)
            .map_err(|e| e.to_string())
    }

    /// Tear the tunnel down. Close the descriptor first.
    pub fn stop(&self) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<serde_json::Value>("stop", ())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(target_os = "android"))]
impl<R: Runtime> NetherVpn<R> {
    const UNSUPPORTED: &'static str = "VPN mode needs Android's VpnService";

    pub fn prepare(&self) -> Result<bool, String> {
        Err(Self::UNSUPPORTED.into())
    }

    pub fn start(&self, _config: StartConfig) -> Result<i32, String> {
        Err(Self::UNSUPPORTED.into())
    }

    pub fn stop(&self) -> Result<(), String> {
        Err(Self::UNSUPPORTED.into())
    }
}

pub trait NetherVpnExt<R: Runtime> {
    fn nether_vpn(&self) -> &NetherVpn<R>;
}

impl<R: Runtime, T: Manager<R>> NetherVpnExt<R> for T {
    fn nether_vpn(&self) -> &NetherVpn<R> {
        self.state::<NetherVpn<R>>().inner()
    }
}

fn build<R: Runtime>(
    _api: PluginApi<R, ()>,
) -> Result<NetherVpn<R>, Box<dyn std::error::Error>> {
    #[cfg(target_os = "android")]
    {
        Ok(NetherVpn {
            handle: _api.register_android_plugin(PLUGIN_IDENTIFIER, "VpnPlugin")?,
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        Ok(NetherVpn {
            _runtime: std::marker::PhantomData,
        })
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("nethervpn")
        .setup(|app: &AppHandle<R>, api| {
            app.manage(build(api)?);
            Ok(())
        })
        .build()
}
