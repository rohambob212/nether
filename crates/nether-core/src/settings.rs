use serde::{Deserialize, Serialize};

/// Which transport protocol Aether should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// MASQUE over HTTP/3 (QUIC) or HTTP/2. Resembles ordinary HTTPS.
    #[default]
    Masque,
    /// Classic WireGuard.
    WireGuard,
    /// WARP-in-WARP ("gool"): a WireGuard tunnel inside another WireGuard tunnel.
    Gool,
}

impl Protocol {
    pub const ALL: [Protocol; 3] = [Protocol::Masque, Protocol::WireGuard, Protocol::Gool];

    pub fn label(&self) -> &'static str {
        match self {
            Protocol::Masque => "MASQUE",
            Protocol::WireGuard => "WireGuard",
            Protocol::Gool => "Gool (WARP-in-WARP)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Protocol::Masque => "HTTP/3 or HTTP/2 tunnel that looks like normal HTTPS traffic. Recommended.",
            Protocol::WireGuard => "Fast and lightweight. Good for networks without aggressive inspection.",
            Protocol::Gool => "WireGuard nested in WireGuard for an extra layer of encryption.",
        }
    }

    fn flag(&self) -> &'static str {
        match self {
            Protocol::Masque => "--masque",
            Protocol::WireGuard => "--wg",
            Protocol::Gool => "--gool",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    Turbo,
    #[default]
    Balanced,
    Thorough,
    Stealth,
    Ironclad,
}

impl ScanMode {
    pub const ALL: [ScanMode; 5] = [
        ScanMode::Turbo,
        ScanMode::Balanced,
        ScanMode::Thorough,
        ScanMode::Stealth,
        ScanMode::Ironclad,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ScanMode::Turbo => "Turbo",
            ScanMode::Balanced => "Balanced",
            ScanMode::Thorough => "Thorough",
            ScanMode::Stealth => "Stealth",
            ScanMode::Ironclad => "Ironclad",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ScanMode::Turbo => "Fastest scan, fewer candidates tested.",
            ScanMode::Balanced => "Default trade-off between speed and reliability.",
            ScanMode::Thorough => "Tests more endpoints before connecting.",
            ScanMode::Stealth => "Low and slow scanning to stay under the radar.",
            ScanMode::Ironclad => "Real tunnel + real HTTP check per candidate. Slowest, most reliable.",
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            ScanMode::Turbo => "turbo",
            ScanMode::Balanced => "balanced",
            ScanMode::Thorough => "thorough",
            ScanMode::Stealth => "stealth",
            ScanMode::Ironclad => "ironclad",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    #[default]
    V4,
    V6,
    Dual,
}

impl IpVersion {
    pub const ALL: [IpVersion; 3] = [IpVersion::V4, IpVersion::V6, IpVersion::Dual];

    pub fn label(&self) -> &'static str {
        match self {
            IpVersion::V4 => "IPv4",
            IpVersion::V6 => "IPv6",
            IpVersion::Dual => "Dual",
        }
    }

    fn flag(&self) -> &'static str {
        match self {
            IpVersion::V4 => "-4",
            IpVersion::V6 => "-6",
            IpVersion::Dual => "--dual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub const ALL: [LogLevel; 5] = [
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
        LogLevel::Trace,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            LogLevel::Error => "Error",
            LogLevel::Warn => "Warnings",
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace (noisy)",
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

/// Well-known obfuscation profile names offered by Aether.
pub const NOIZE_PROFILES: &[&str] = &[
    "off",
    "light",
    "firewall",
    "balanced",
    "aggressive",
    "gfw",
];

/// Paths of the identity files Aether maintains.
#[derive(Debug, Clone)]
pub struct IdentityPaths {
    pub base_config: String,
    pub wg_config: String,
    pub masque_config: String,
}

/// The complete user-facing configuration of the app.
///
/// Every field is optional-friendly and serialized camelCase so it maps 1:1
/// onto the TypeScript side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NetherSettings {
    pub version: u32,

    // -- connection ------------------------------------------------------
    pub protocol: Protocol,
    pub scan_mode: ScanMode,
    pub ip_version: IpVersion,
    /// Reuse the last known-good gateway without rescanning when possible.
    pub quick_reconnect: bool,
    /// Try to connect automatically on app launch.
    pub auto_connect: bool,
    /// Keep the tunnel core running across connect/disconnect cycles; only
    /// the user-facing proxy port is toggled.
    pub always_on: bool,

    // -- network ---------------------------------------------------------
    pub socks_host: String,
    pub socks_port: u16,
    pub http_proxy_enabled: bool,
    pub http_proxy_port: u16,
    /// Optional upstream proxy URL Aether dials out through.
    pub upstream_proxy: Option<String>,

    // -- smart routing -----------------------------------------------------
    /// Route traffic through the bundled Xray core with Iran split rules.
    pub smart_routing: bool,
    /// SOCKS5 port of the Xray inbound apps should point at.
    pub xray_socks_port: u16,

    // -- obfuscation -----------------------------------------------------
    pub noize_profile: String,

    // -- MASQUE specifics --------------------------------------------------
    pub masque_h2: bool,
    /// "auto" fetches ECH configs automatically, otherwise base64 ECHConfigList.
    pub ech: Option<String>,
    pub fragment: bool,
    pub fragment_size: String,
    pub fragment_delay: String,
    pub dns_resolvers: String,
    pub validate_secs: Option<u64>,
    pub startup_secs: Option<u64>,
    pub reconnect_secs: Option<u64>,
    pub disable_data_check: bool,

    // -- WireGuard specifics ----------------------------------------------
    pub keepalive_secs: Option<u16>,
    pub no_profile_retry: bool,
    pub forced_peer: Option<String>,
    pub forced_wg_peer: Option<String>,

    // -- Cloudflare Zero Trust ---------------------------------------------
    pub team_name: Option<String>,
    pub access_client_id: Option<String>,
    pub access_client_secret: Option<String>,
    pub access_token: Option<String>,
    pub use_gateway_proxy: bool,

    // -- routing -----------------------------------------------------------
    pub route_block: Option<String>,
    pub route_direct: Option<String>,

    // -- app -----------------------------------------------------------------
    pub log_level: LogLevel,
    pub perf_profile: Option<String>,
    pub tls_groups: Option<String>,
}

impl Default for NetherSettings {
    fn default() -> Self {
        Self {
            version: 1,
            protocol: Protocol::Masque,
            scan_mode: ScanMode::Balanced,
            ip_version: IpVersion::V4,
            quick_reconnect: true,
            auto_connect: false,
            always_on: false,
            socks_host: "127.0.0.1".into(),
            socks_port: 1819,
            http_proxy_enabled: false,
            http_proxy_port: 1820,
            upstream_proxy: None,
            smart_routing: false,
            xray_socks_port: 1817,
            noize_profile: "firewall".into(),
            masque_h2: false,
            ech: None,
            fragment: false,
            fragment_size: String::new(),
            fragment_delay: String::new(),
            dns_resolvers: "1.1.1.1,1.0.0.1".into(),
            validate_secs: None,
            startup_secs: None,
            reconnect_secs: None,
            disable_data_check: false,
            keepalive_secs: None,
            no_profile_retry: false,
            forced_peer: None,
            forced_wg_peer: None,
            team_name: None,
            access_client_id: None,
            access_client_secret: None,
            access_token: None,
            use_gateway_proxy: false,
            route_block: None,
            route_direct: None,
            log_level: LogLevel::Info,
            perf_profile: None,
            tls_groups: None,
        }
    }
}

impl NetherSettings {
    /// Clamp obviously-wrong values into sane ranges.
    pub fn normalized(mut self) -> Self {
        if self.socks_host.trim().is_empty() {
            self.socks_host = "127.0.0.1".into();
        }
        if self.socks_port == 0 {
            self.socks_port = 1819;
        }
        if self.http_proxy_enabled && self.http_proxy_port == 0 {
            self.http_proxy_port = 1820;
        }
        if self.smart_routing && self.xray_socks_port == 0 {
            self.xray_socks_port = 1817;
        }
        // Ports must not collide or Xray's inbound will fight Aether's bind.
        if self.smart_routing && self.xray_socks_port == self.socks_port {
            self.xray_socks_port = if self.socks_port == 1817 { 1816 } else { 1817 };
        }
        if self.noize_profile.trim().is_empty() {
            self.noize_profile = "firewall".into();
        }
        if self.dns_resolvers.trim().is_empty() {
            self.dns_resolvers = "1.1.1.1,1.0.0.1".into();
        }
        if let Some(ech) = &self.ech {
            if ech.trim().is_empty() {
                self.ech = None;
            }
        }
        let trim_opt = |v: &Option<String>| -> Option<String> {
            v.as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        self.upstream_proxy = trim_opt(&self.upstream_proxy);
        self.forced_peer = trim_opt(&self.forced_peer);
        self.forced_wg_peer = trim_opt(&self.forced_wg_peer);
        self.team_name = trim_opt(&self.team_name);
        self.access_client_id = trim_opt(&self.access_client_id);
        self.access_client_secret = trim_opt(&self.access_client_secret);
        self.access_token = trim_opt(&self.access_token);
        self.route_block = trim_opt(&self.route_block);
        self.route_direct = trim_opt(&self.route_direct);
        self.perf_profile = trim_opt(&self.perf_profile);
        self.tls_groups = trim_opt(&self.tls_groups);
        self.ech = trim_opt(&self.ech);
        self
    }

    fn push_opt(args: &mut Vec<String>, flag: &str, value: &str) {
        args.push(flag.to_string());
        args.push(value.to_string());
    }

    /// Translate settings into the exact CLI argument vector accepted by
    /// `aether::run_with`.
    pub fn to_aether_args(&self, paths: &IdentityPaths) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();

        args.push(self.protocol.flag().to_string());
        args.push("--scan".into());
        args.push(self.scan_mode.as_str().into());
        args.push(self.ip_version.flag().into());

        args.push("--bind".into());
        args.push(format!("{}:{}", self.socks_host, self.socks_port));

        if self.http_proxy_enabled {
            args.push("--http-proxy".into());
            args.push(format!("{}:{}", self.socks_host, self.http_proxy_port));
        }

        if let Some(upstream) = &self.upstream_proxy {
            Self::push_opt(&mut args, "--upstream", upstream);
        }

        // Never let Aether block on interactive stdin prompts from a GUI.
        args.push(if self.quick_reconnect {
            "--quick-reconnect".into()
        } else {
            "--no-quick-reconnect".into()
        });

        Self::push_opt(&mut args, "--noize", &self.noize_profile);

        if self.masque_h2 {
            args.push("--h2".into());
        }
        if let Some(ech) = &self.ech {
            Self::push_opt(&mut args, "--ech", ech);
        }
        if self.fragment {
            args.push("--fragment".into());
            if !self.fragment_size.trim().is_empty() {
                Self::push_opt(&mut args, "--fragment-size", self.fragment_size.trim());
            }
            if !self.fragment_delay.trim().is_empty() {
                Self::push_opt(&mut args, "--fragment-delay", self.fragment_delay.trim());
            }
        }
        Self::push_opt(&mut args, "--dns", &self.dns_resolvers);
        if let Some(v) = self.validate_secs {
            Self::push_opt(&mut args, "--validate-secs", &v.to_string());
        }
        if let Some(v) = self.startup_secs {
            Self::push_opt(&mut args, "--startup-secs", &v.to_string());
        }
        if let Some(v) = self.reconnect_secs {
            Self::push_opt(&mut args, "--reconnect-secs", &v.to_string());
        }
        if self.disable_data_check {
            args.push("--no-data-check".into());
        }

        if let Some(v) = self.keepalive_secs {
            Self::push_opt(&mut args, "--keepalive", &v.to_string());
        }
        if self.no_profile_retry {
            args.push("--no-profile-retry".into());
        }
        if let Some(peer) = &self.forced_peer {
            Self::push_opt(&mut args, "--peer", peer);
        }
        if let Some(peer) = &self.forced_wg_peer {
            Self::push_opt(&mut args, "--wg-peer", peer);
        }

        if let Some(team) = &self.team_name {
            Self::push_opt(&mut args, "--team", team);
        }
        if let Some(id) = &self.access_client_id {
            Self::push_opt(&mut args, "--access-id", id);
        }
        if let Some(secret) = &self.access_client_secret {
            Self::push_opt(&mut args, "--access-secret", secret);
        }
        if let Some(token) = &self.access_token {
            Self::push_opt(&mut args, "--access-token", token);
        }
        if self.use_gateway_proxy {
            args.push("--gateway".into());
        }

        if let Some(routes) = &self.route_block {
            Self::push_opt(&mut args, "--route-block", routes);
        }
        if let Some(routes) = &self.route_direct {
            Self::push_opt(&mut args, "--route-direct", routes);
        }

        if let Some(perf) = &self.perf_profile {
            Self::push_opt(&mut args, "--perf", perf);
        }
        if let Some(groups) = &self.tls_groups {
            Self::push_opt(&mut args, "--tls-groups", groups);
        }

        Self::push_opt(&mut args, "--log-level", self.log_level.as_str());

        // Keep identity files inside the app data dir instead of the CWD.
        args.push("--config".into());
        args.push(paths.base_config.clone());
        args.push("--wg-config".into());
        args.push(paths.wg_config.clone());
        args.push("--masque-config".into());
        args.push(paths.masque_config.clone());

        args
    }
}
