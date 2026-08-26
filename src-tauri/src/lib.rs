//! Nether — a multi-platform desktop client for Aether.
//!
//! The app shell wires three things together:
//!   1. `nether_core` (settings + engine + log hub)
//!   2. Tauri IPC commands the frontend invokes
//!   3. Event forwarders that stream logs/status into the webview

use nether_core::logging;
use nether_core::{EngineManager, EngineStatus, IdentityPaths, LogRecord, NetherSettings};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

const LOG_EVENT: &str = "nether://log";
const STATUS_EVENT: &str = "nether://status";
const LOG_HISTORY_CAP: usize = 5000;

struct AppState {
    manager: EngineManager,
    settings: Arc<Mutex<NetherSettings>>,
    data_dir: PathBuf,
    /// The bundled Xray sidecar process when smart routing is active.
    xray_child: Mutex<Option<CommandChild>>,
    /// The tun2socks pump when VPN mode is active (Android only).
    #[cfg(target_os = "android")]
    vpn_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

fn identity_paths(data_dir: &std::path::Path) -> IdentityPaths {
    let dir = data_dir.join("identities");
    let _ = std::fs::create_dir_all(&dir);
    let join = |name: &str| dir.join(name).to_string_lossy().into_owned();
    IdentityPaths {
        base_config: join("identity.toml"),
        wg_config: join("identity-wg.toml"),
        masque_config: join("identity-masque.toml"),
    }
}

fn settings_file(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("nether-settings.json")
}

fn load_settings(data_dir: &std::path::Path) -> NetherSettings {
    let path = settings_file(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str::<NetherSettings>(&raw) {
            Ok(mut s) => {
                s.version = 1;
                log::info!("[nether] loaded settings from {}", path.display());
                s.normalized()
            }
            Err(e) => {
                log::warn!("[nether] could not parse settings file ({e}); using defaults");
                NetherSettings::default()
            }
        },
        Err(_) => NetherSettings::default(),
    }
}

// ---------------------------------------------------------------------------
// Xray sidecar (smart routing)
// ---------------------------------------------------------------------------

/// Geo assets live next to the binary when bundled; fall back to the repo
/// layout during development.
fn resolve_asset_dir(app: &AppHandle) -> PathBuf {
    if let Ok(dir) = app.path().resource_dir() {
        for cand in [dir.join("resources").join("xray"), dir.join("xray")] {
            if cand.join("geoip.dat").exists() {
                return cand;
            }
        }
    }
    std::env::current_dir()
        .unwrap_or_default()
        .join("src-tauri")
        .join("resources")
        .join("xray")
}

fn start_xray(app: &AppHandle, settings: &NetherSettings) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut guard = state.xray_child.lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }

    let asset_dir = resolve_asset_dir(app);
    if !asset_dir.join("geoip.dat").exists() {
        return Err(format!(
            "geo assets missing in {} — run scripts/fetch_xray.sh",
            asset_dir.display()
        ));
    }

    let cfg_path = state.data_dir.join("xray-config.json");
    std::fs::write(&cfg_path, nether_core::xray::gen_config(settings))
        .map_err(|e| format!("write xray config: {e}"))?;
    log::info!(
        "[nether] smart routing on 127.0.0.1:{} -> tunnel {}:{}",
        settings.xray_socks_port,
        settings.socks_host,
        settings.socks_port
    );

    let sidecar = app.shell().sidecar("binaries/xray").map_err(|e| e.to_string())?;
    let (mut rx, child) = sidecar
        .args(["run", "-c"])
        .args([cfg_path.to_str().ok_or("bad config path")?])
        .env("XRAY_LOCATION_ASSET", asset_dir.to_str().ok_or("bad asset path")?)
        .spawn()
        .map_err(|e| format!("spawn xray: {e}"))?;
    *guard = Some(child);
    drop(guard);

    // Pipe the process into the shared log hub so it shows up in the UI.
    tauri::async_runtime::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                CommandEvent::Stdout(l) => log::info!("[xray] {}", String::from_utf8_lossy(&l)),
                CommandEvent::Stderr(l) => log::warn!("[xray] {}", String::from_utf8_lossy(&l)),
                CommandEvent::Terminated(_) => break,
                _ => {}
            }
        }
        log::warn!("[nether] xray process exited");
    });
    Ok(())
}

fn stop_xray(app: &AppHandle) {
    let child = {
        let state = app.state::<AppState>();
        let mut guard = state.xray_child.lock().unwrap();
        guard.take()
    };
    if let Some(child) = child {
        let _ = child.kill();
        log::info!("[nether] xray stopped");
    }
}

// ---------------------------------------------------------------------------
// VPN mode (Android)
// ---------------------------------------------------------------------------

/// MTU advertised on the TUN. Aether's own tunnel carries 1280, and IPv6
/// guarantees that much everywhere, so anything larger just invites a UDP
/// datagram that cannot fit downstream.
#[cfg(target_os = "android")]
const VPN_MTU: u16 = 1280;

/// Bring up the device-wide tunnel and start pumping its packets into the
/// local SOCKS proxy. No-op unless VPN mode is on and nothing is running yet.
#[cfg(target_os = "android")]
fn start_vpn(app: &AppHandle, settings: &NetherSettings) {
    use tauri_plugin_nethervpn::{NetherVpnExt, StartConfig};

    let state = app.state::<AppState>();
    let mut guard = state.vpn_task.lock().unwrap();
    if guard.is_some() {
        return;
    }

    // Smart routing never runs on mobile (the Xray sidecar is dropped from the
    // Android bundle), so the tunnel's own SOCKS port is always the target.
    let proxy = match format!("{}:{}", settings.socks_host, settings.socks_port).parse() {
        Ok(addr) => addr,
        Err(e) => {
            log::error!("[nether] vpn: bad proxy address: {e}");
            return;
        }
    };

    let config = StartConfig {
        mtu: VPN_MTU,
        dns: settings.dns_resolvers.clone(),
        ipv6: !matches!(settings.ip_version, nether_core::IpVersion::V4),
    };

    let fd = match app.nether_vpn().start(config) {
        Ok(fd) => fd,
        Err(e) => {
            log::error!("[nether] vpn: could not establish tunnel: {e}");
            return;
        }
    };

    // SAFETY: the descriptor was detached from its ParcelFileDescriptor on the
    // Kotlin side, so this task is its only owner.
    *guard = Some(tauri::async_runtime::spawn(async move {
        if let Err(e) = unsafe { nether_core::vpn::run(fd, proxy, VPN_MTU) }.await {
            log::error!("[nether] vpn stopped: {e}");
        }
    }));
    log::info!("[nether] vpn mode active");
}

/// Tear the device-wide tunnel down. Aborting the pump closes the descriptor,
/// which is what actually drops the interface; the service stop is cleanup.
#[cfg(target_os = "android")]
fn stop_vpn(app: &AppHandle) {
    use tauri_plugin_nethervpn::NetherVpnExt;

    let task = {
        let state = app.state::<AppState>();
        let mut guard = state.vpn_task.lock().unwrap();
        guard.take()
    };
    if let Some(task) = task {
        task.abort();
        if let Err(e) = app.nether_vpn().stop() {
            log::warn!("[nether] vpn: service stop failed: {e}");
        }
        log::info!("[nether] vpn mode off");
    }
}

#[cfg(not(target_os = "android"))]
fn start_vpn(_app: &AppHandle, _settings: &NetherSettings) {}

#[cfg(not(target_os = "android"))]
fn stop_vpn(_app: &AppHandle) {}

/// Ask Android for VPN consent. Called when the user flips the Settings
/// switch, so the system dialog lands on a deliberate gesture.
#[tauri::command]
async fn vpn_prepare(_app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_nethervpn::NetherVpnExt;
        _app.nether_vpn().prepare()
    }
    #[cfg(not(target_os = "android"))]
    {
        Err("VPN mode is only available on Android".into())
    }
}

/// Start the full stack: Aether tunnel plus the optional smart-routing layer.
fn do_connect(app: &AppHandle) -> Result<EngineStatus, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    state
        .manager
        .start(&settings, &identity_paths(&state.data_dir))?;
    if settings.smart_routing {
        if let Err(e) = start_xray(app, &settings) {
            // Don't leave a half-started stack behind.
            state.manager.stop(&settings);
            return Err(e);
        }
    }
    Ok(state.manager.status())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn connect(app: AppHandle) -> Result<EngineStatus, String> {
    do_connect(&app)
}

#[tauri::command]
async fn disconnect(app: AppHandle) -> Result<EngineStatus, String> {
    stop_vpn(&app);
    stop_xray(&app);
    let settings = app.state::<AppState>().settings.lock().unwrap().clone();
    app.state::<AppState>().manager.stop(&settings);
    Ok(app.state::<AppState>().manager.status())
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, AppState>) -> Result<EngineStatus, String> {
    Ok(state.manager.status())
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<NetherSettings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
async fn save_settings(
    app: AppHandle,
    settings: NetherSettings,
) -> Result<NetherSettings, String> {
    let state = app.state::<AppState>();
    let normalized = settings.normalized();

    let raw = serde_json::to_string_pretty(&normalized)
        .map_err(|e| format!("serialize settings: {e}"))?;
    let path = settings_file(&state.data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Write-then-rename so an interrupted save can't leave a torn file behind.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| format!("write settings: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("write settings: {e}"))?;

    *state.settings.lock().unwrap() = normalized.clone();
    log::info!("[nether] settings saved to {}", path.display());
    Ok(normalized)
}

#[tauri::command]
async fn recent_logs(limit: Option<usize>) -> Result<Vec<LogRecord>, String> {
    Ok(logging::hub().snapshot(limit.unwrap_or(LOG_HISTORY_CAP)))
}

#[tauri::command]
async fn clear_logs() -> Result<(), String> {
    logging::hub().clear();
    Ok(())
}

#[tauri::command]
async fn app_info(app: AppHandle) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": app.package_info().name,
        "version": app.package_info().version.to_string(),
    }))
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

/// Spawn the single pump that moves every log record to the webview and feeds
/// the status watcher. One task keeps ordering consistent between both sinks.
fn spawn_forwarders(app: AppHandle) {
    let mut rx = logging::hub().subscribe();
    let manager = app.state::<AppState>().manager.clone();

    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(record) => {
                    manager.observe_line(&record.message);
                    // Status changes are broadcast by observe_line's channel;
                    // we don't need to re-emit here.
                    if app.emit(LOG_EVENT, &record).is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("[nether] log forwarder lagged, dropped {n} records");
                }
                Err(_) => break,
            }
        }
    });
}

/// Stream engine status transitions into the webview.
fn spawn_status_forwarder(app: AppHandle) {
    let mut rx = app.state::<AppState>().manager.subscribe();
    tauri::async_runtime::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match rx.recv().await {
                Ok(status) => {
                    // VPN mode follows the tunnel: the TUN only makes sense
                    // once SOCKS is actually accepting, and must come down
                    // when it stops (including an engine-side reconnect).
                    let settings = app.state::<AppState>().settings.lock().unwrap().clone();
                    if settings.vpn_mode {
                        if status.state == nether_core::EngineState::Connected {
                            start_vpn(&app, &settings);
                        } else {
                            stop_vpn(&app);
                        }
                    }
                    if app.emit(STATUS_EVENT, &status).is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // The hub logger must be installed before anything else so Aether's own
    // env_logger try_init() becomes a no-op and its output lands in our hub.
    nether_core::logging::install(LOG_HISTORY_CAP);

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init());

    #[cfg(target_os = "android")]
    {
        builder = builder.plugin(tauri_plugin_nethervpn::init());
    }

    builder
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("no app data directory available");
            std::fs::create_dir_all(&data_dir).ok();

            let settings = Arc::new(Mutex::new(load_settings(&data_dir)));

            app.manage(AppState {
                manager: EngineManager::new(),
                settings,
                data_dir,
                xray_child: Mutex::new(None),
                #[cfg(target_os = "android")]
                vpn_task: Mutex::new(None),
            });

            let handle = app.handle().clone();
            spawn_forwarders(handle);
            spawn_status_forwarder(app.handle().clone());

            log::info!(
                "[nether] v{} ready",
                app.package_info().version
            );

            // Optional auto-connect on launch.
            let wants_auto = {
                let s = app.handle().state::<AppState>();
                let auto = s.settings.lock().unwrap().auto_connect;
                auto
            };
            if wants_auto {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    log::info!("[nether] auto-connect enabled, connecting...");
                    if let Err(e) = do_connect(&handle) {
                        log::error!("[nether] auto-connect failed: {e}");
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connect,
            disconnect,
            get_status,
            get_settings,
            save_settings,
            recent_logs,
            clear_logs,
            app_info,
            vpn_prepare
        ])
        .build(tauri::generate_context!())
        .expect("error while building Nether")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                stop_vpn(app);
                stop_xray(app);
            }
        });
}
