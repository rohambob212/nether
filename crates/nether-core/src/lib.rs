//! Nether core: settings model, tunnel engine abstraction and the log
//! pipeline shared by the Aether library integration and the UI.

pub mod engine;
pub mod gate;
pub mod logging;
pub mod settings;
#[cfg(all(unix, feature = "vpn"))]
pub mod vpn;
pub mod xray;

pub use engine::{EngineManager, EngineState, EngineStatus};
pub use logging::{LogHub, LogRecord};
pub use settings::{
    IdentityPaths, IpVersion, LogLevel, NetherSettings, NOIZE_PROFILES, Protocol, ScanMode,
};

/// Every `AETHER_*` environment variable the CLI layer can set. We scrub all
/// of them before launching a fresh run so nothing leaks between sessions.
pub const AETHER_ENV_KEYS: &[&str] = &[
    "AETHER_SOCKS",
    "AETHER_HTTP_PROXY",
    "AETHER_UPSTREAM",
    "AETHER_QUICK_RECONNECT",
    "AETHER_IP",
    "AETHER_PEER",
    "AETHER_WG_PEER",
    "AETHER_PROTOCOL",
    "AETHER_SCAN",
    "AETHER_NOIZE",
    "AETHER_MASQUE_HTTP2",
    "AETHER_MASQUE_H2_PEER",
    "AETHER_ECH",
    "AETHER_MASQUE_NO_DATA_CHECK",
    "AETHER_WG_NO_DATA_CHECK",
    "AETHER_MASQUE_VALIDATE_SECS",
    "AETHER_WG_VALIDATE_SECS",
    "AETHER_MASQUE_STARTUP_SECS",
    "AETHER_MASQUE_RECONNECT_SECS",
    "AETHER_WG_RECONNECT_SECS",
    "AETHER_DNS",
    "AETHER_MASQUE_H2_FRAGMENT",
    "AETHER_MASQUE_H2_FRAGMENT_SIZE",
    "AETHER_MASQUE_H2_FRAGMENT_DELAY",
    "AETHER_WG_KEEPALIVE",
    "AETHER_WG_NO_PROFILE_RETRY",
    "AETHER_CONFIG",
    "AETHER_WG_CONFIG",
    "AETHER_MASQUE_CONFIG",
    "AETHER_TEAM",
    "AETHER_ACCESS_CLIENT_ID",
    "AETHER_ACCESS_CLIENT_SECRET",
    "AETHER_ACCESS_TOKEN",
    "AETHER_ACCESS_EMAIL",
    "AETHER_GATEWAY",
    "AETHER_ROUTE_BLOCK",
    "AETHER_ROUTE_DIRECT",
    "AETHER_ROUTES_FILE",
    "AETHER_TLS_GROUPS",
    "AETHER_PERF_PROFILE",
    "AETHER_LOG_LEVEL",
    "AETHER_TEAM_ENDPOINT",
    "AETHER_WG_ENDPOINT_COOLDOWN_SECS",
];
