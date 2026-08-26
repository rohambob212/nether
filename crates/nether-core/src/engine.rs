use crate::gate::ProxyGate;
use crate::settings::{IdentityPaths, NetherSettings};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// Everything the UI needs to render connection state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EngineStatus {
    pub state: EngineState,
    /// Human readable current step, e.g. "scanning for gateways".
    pub phase: Option<String>,
    /// Gateway the tunnel is currently using, e.g. "104.17.x.x:443".
    pub gateway: Option<String>,
    /// Last notable detail line (or error text).
    pub detail: Option<String>,
    /// Epoch ms at which the proxy became usable.
    pub connected_since_ms: Option<u64>,
}

/// A pluggable tunnel engine. The real implementation drives Aether's
/// `run_with` in-process; a mock exists for UI development.
pub trait Engine: Send {
    fn start(&mut self, settings: &NetherSettings, paths: &IdentityPaths) -> Result<(), String>;
    fn stop(&mut self);
    fn is_running(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Real engine: Aether linked as a library
// ---------------------------------------------------------------------------
#[cfg(feature = "engine-aether")]
pub mod aether_impl {
    use super::{Engine, IdentityPaths, NetherSettings};
    use crate::AETHER_ENV_KEYS;

    /// Drives `aether::run_with` on its own tokio task. Stopping means
    /// aborting that task — Aether's run loop never returns on its own.
    pub struct AetherEngine {
        handle: Option<tokio::task::JoinHandle<()>>,
    }

    impl AetherEngine {
        pub fn new() -> Self {
            Self { handle: None }
        }
    }

    impl Engine for AetherEngine {
        fn start(&mut self, settings: &NetherSettings, paths: &IdentityPaths) -> Result<(), String> {
            if self.handle.is_some() {
                return Err("engine already running".into());
            }

            // Make sure stale variables from a previous run can't leak into
            // this one; every knob we expose goes through explicit flags.
            for key in AETHER_ENV_KEYS {
                std::env::remove_var(key);
            }

            let args = settings.to_aether_args(paths);
            log::info!("[nether] launching aether: {}", args.join(" "));

            self.handle = Some(tokio::spawn(async move {
                match aether::run_with(args).await {
                    Ok(()) => log::warn!("[nether] aether run loop returned"),
                    Err(e) => log::error!("[nether] aether exited with an error: {e}"),
                }
            }));
            Ok(())
        }

        fn stop(&mut self) {
            if let Some(handle) = self.handle.take() {
                handle.abort();
                log::info!("[nether] aether engine stopped");
            }
        }

        fn is_running(&self) -> bool {
            self.handle.is_some()
        }
    }
}

// ---------------------------------------------------------------------------
// Mock engine: simulates the same lifecycle through the same log pipeline
// ---------------------------------------------------------------------------
#[cfg(feature = "engine-mock")]
pub mod mock_impl {
    use super::{Engine, IdentityPaths, NetherSettings};
    use std::time::Duration;

    pub struct MockEngine {
        handle: Option<tokio::task::JoinHandle<()>>,
    }

    impl MockEngine {
        pub fn new() -> Self {
            Self { handle: None }
        }
    }

    impl Engine for MockEngine {
        fn start(&mut self, settings: &NetherSettings, _paths: &IdentityPaths) -> Result<(), String> {
            if self.handle.is_some() {
                return Err("engine already running".into());
            }
            let bind = format!("{}:{}", settings.socks_host, settings.socks_port);
            let proto = settings.protocol.label().to_string();
            let scan = format!("{:?}", settings.scan_mode).to_lowercase();

            self.handle = Some(tokio::spawn(async move {
                let step = |msg: &str| log::info!("[mock] {msg}");
                step(&format!("Aether v0.0.0-mock (protocol: {proto})"));
                step("provisioning mock identity...");
                tokio::time::sleep(Duration::from_millis(600)).await;
                step("[+] identity ready: device=mock ipv4=198.18.0.2");
                step(&format!("[*] hunting for a working gateway ({scan})..."));
                tokio::time::sleep(Duration::from_millis(900)).await;
                step("[+] selected gateway 203.0.113.7:443 (rtt 21ms)");
                step("[+] using cloudflare edge 203.0.113.7:443");
                tokio::time::sleep(Duration::from_millis(500)).await;
                step(&format!("[+] socks5 server listening on {bind}"));
                loop {
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    log::debug!("[mock] heartbeat: tunnel healthy, 3 active streams");
                }
            }));
            Ok(())
        }

        fn stop(&mut self) {
            if let Some(handle) = self.handle.take() {
                handle.abort();
                log::info!("[mock] engine stopped");
            }
        }

        fn is_running(&self) -> bool {
            self.handle.is_some()
        }
    }
}

// ---------------------------------------------------------------------------
// Manager: owns the engine, derives UI-facing status from the log stream
// ---------------------------------------------------------------------------
#[derive(Clone)]
pub struct EngineManager {
    inner: std::sync::Arc<std::sync::Mutex<Inner>>,
}

struct Inner {
    engine: Box<dyn Engine>,
    status_tx: tokio::sync::broadcast::Sender<EngineStatus>,
    status: EngineStatus,
    gate: ProxyGate,
    always_on: bool,
    /// Port the engine actually bound while in always-on mode.
    internal_port: Option<u16>,
}

impl EngineManager {
    pub fn new() -> Self {
        let (status_tx, _) = tokio::sync::broadcast::channel(64);

        #[cfg(all(feature = "engine-aether", not(feature = "engine-mock")))]
        let engine = Box::new(aether_impl::AetherEngine::new()) as Box<dyn Engine>;
        #[cfg(feature = "engine-mock")]
        let engine = Box::new(mock_impl::MockEngine::new()) as Box<dyn Engine>;
        #[cfg(not(any(feature = "engine-aether", feature = "engine-mock")))]
        compile_error!("enable one of the \"engine-aether\" or \"engine-mock\" features");

        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(Inner {
                engine,
                status_tx,
                status: EngineStatus::default(),
                gate: ProxyGate::new(),
                always_on: false,
                internal_port: None,
            })),
        }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<EngineStatus> {
        self.inner.lock().unwrap().status_tx.subscribe()
    }

    pub fn status(&self) -> EngineStatus {
        self.inner.lock().unwrap().status.clone()
    }

    /// Start the stack.
    ///
    /// Standard mode: the engine binds the user-facing SOCKS port directly.
    /// Always-on mode: the engine gets a private internal port and stays up
    /// across stop/start cycles; the gate owns the user-facing port, so
    /// connect/disconnect is instant.
    pub fn start(&self, settings: &NetherSettings, paths: &IdentityPaths) -> Result<(), String> {
        let mut g = self.inner.lock().unwrap();
        g.always_on = settings.always_on;
        let core_running = g.engine.is_running();

        if settings.always_on {
            if matches!(
                g.status.state,
                EngineState::Connected | EngineState::Disconnecting
            ) {
                return Err("already running".into());
            }
            // (Re)start the core only when it isn't warm already.
            if !core_running {
                let mut engine_settings = settings.clone();
                let internal = free_port().ok_or("no free internal port")?;
                engine_settings.socks_port = internal;
                g.internal_port = Some(internal);
                log::info!("[nether] always-on core starting on internal port {internal}");
                g.engine.start(&engine_settings, paths)?;
            }
            let upstream_port = g
                .internal_port
                .unwrap_or(settings.socks_port);
            g.status.state = EngineState::Connecting;
            g.status.phase = Some(
                if core_running { "opening gate" } else { "starting core" }.to_string(),
            );
            let _ = g.status_tx.send(g.status.clone());

            if let Err(e) =
                g.gate.open(&settings.socks_host, settings.socks_port, (settings.socks_host.clone(), upstream_port))
            {
                g.status = EngineStatus {
                    detail: Some(e.clone()),
                    ..Default::default()
                };
                let _ = g.status_tx.send(g.status.clone());
                return Err(e);
            }
            g.status.state = EngineState::Connected;
            g.status.phase = Some("connected".into());
            g.status.connected_since_ms = Some(now_ms());
            let _ = g.status_tx.send(g.status.clone());
            return Ok(());
        }

        if matches!(g.status.state, EngineState::Connecting | EngineState::Connected | EngineState::Disconnecting)
            || core_running
        {
            return Err("already running".into());
        }

        g.gate.close();
        g.status = EngineStatus {
            state: EngineState::Connecting,
            phase: Some("starting engine".into()),
            ..Default::default()
        };
        let _ = g.status_tx.send(g.status.clone());

        match g.engine.start(settings, paths) {
            Ok(()) => Ok(()),
            Err(e) => {
                g.status = EngineStatus {
                    detail: Some(e.clone()),
                    ..Default::default()
                };
                let _ = g.status_tx.send(g.status.clone());
                Err(e)
            }
        }
    }

    /// Stop the user-facing proxy. In always-on mode the core keeps running.
    pub fn stop(&self, settings: &NetherSettings) {
        let mut g = self.inner.lock().unwrap();
        g.gate.close();
        if settings.always_on {
            g.status.state = EngineState::Disconnected;
            g.status.phase = Some("core idle".into());
            g.status.connected_since_ms = None;
        } else if g.engine.is_running() {
            g.status.state = EngineState::Disconnecting;
            g.status.phase = Some("closing tunnel".into());
            let _ = g.status_tx.send(g.status.clone());
            g.engine.stop();
            g.status = EngineStatus {
                detail: g.status.detail.clone(),
                ..Default::default()
            };
        }
        let _ = g.status_tx.send(g.status.clone());
    }

    /// Feed one raw log line in; returns true when UI-visible status changed.
    /// Called from the single log-forwarder task in the app shell.
    pub fn observe_line(&self, line: &str) -> bool {
        let mut g = self.inner.lock().unwrap();
        let mut changed = false;

        if let Some(rest) = line.split("using cloudflare edge ").nth(1) {
            let gw = rest.trim().to_string();
            if g.status.gateway.as_deref() != Some(gw.as_str()) {
                g.status.gateway = Some(gw);
                changed = true;
            }
        } else if let Some(rest) = line
            .split("selected MASQUE gateway ")
            .nth(1)
            .or_else(|| line.split("selected WireGuard endpoint ").nth(1))
        {
            if let Some(gw) = rest.split_whitespace().next() {
                if g.status.gateway.as_deref() != Some(gw) {
                    g.status.gateway = Some(gw.to_string());
                    changed = true;
                }
            }
        }

        if line.contains("socks5 server listening") && !g.always_on {
            if g.status.state != EngineState::Connected {
                g.status.state = EngineState::Connected;
                g.status.connected_since_ms = Some(now_ms());
                g.status.phase = Some("connected".into());
                changed = true;
            }
        } else if line.contains("identity ready") {
            if g.status.state == EngineState::Connecting && g.status.phase.as_deref() != Some("identity ready") {
                g.status.phase = Some("identity ready".into());
                changed = true;
            }
        } else if line.contains("hunting for") || line.contains("rescanning") {
            if g.status.state == EngineState::Connecting && g.status.phase.as_deref() != Some("scanning for gateways") {
                g.status.phase = Some("scanning for gateways".into());
                changed = true;
            }
        } else if line.contains("validating") || line.contains("verifying") {
            if g.status.state == EngineState::Connecting && g.status.phase.as_deref() != Some("validating tunnel") {
                g.status.phase = Some("validating tunnel".into());
                changed = true;
            }
        } else if line.contains("reconnecting") {
            if g.status.state == EngineState::Connected {
                g.status.state = EngineState::Connecting;
                g.status.connected_since_ms = None;
                g.status.phase = Some("reconnecting".into());
                changed = true;
            }
        } else if line.contains("no usable")
            || line.contains("exited with an error")
            || line.contains("failed to provision")
        {
            let detail = line.trim_start_matches("[-] ").to_string();
            if g.status.detail.as_deref() != Some(detail.as_str()) {
                g.status.detail = Some(detail);
                changed = true;
            }
        }

        // Rolling detail text keeps the newest notable progress line.
        if line.starts_with("[+]") || line.starts_with("[*]") {
            g.status.detail = Some(line.to_string());
        }

        if changed {
            let _ = g.status_tx.send(g.status.clone());
        }
        changed
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Reserve an ephemeral port for the engine's internal bind. Tiny TOCTOU
/// window between drop and the engine's own bind; fine on localhost.
fn free_port() -> Option<u16> {
    let l = std::net::TcpListener::bind(("127.0.0.1", 0)).ok()?;
    l.local_addr().ok().map(|a| a.port())
}
