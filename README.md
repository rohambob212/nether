# Nether

A multi-platform GUI client for [Aether](https://github.com/CluvexStudio/Aether), a
censorship-circumvention engine exposing an encrypted tunnel as a local SOCKS5 proxy
(MASQUE / WireGuard / gool).

Built with Tauri 2 + React. Ships a bundled Xray-core sidecar for Hiddify-style
**smart routing** tuned for Iran: Iranian sites and private IPs go direct, ads are
blocked, everything else is tunneled.

```
apps → Nether (Xray, :1817) ──┬─ geosite:category-ir / geoip:ir → DIRECT
                              ├─ ads → blocked
                              └─ foreign traffic → Aether tunnel (:1819)
```

## Features

- One-click connect with live status (phase, gateway, uptime)
- Protocol choice: MASQUE (recommended), WireGuard, Gool
- Scan modes from Turbo to Ironclad
- **Smart routing** via bundled Xray-core: Iran split tunneling + ad blocking,
  or plain direct-proxy mode like other Aether clients
- **Always-on core**: the tunnel stays established across connect/disconnect
  cycles — START/STOP only toggles the proxy port, reconnects are instant
- Advanced settings for every Aether knob (ECH, fragmentation, noize,
  Zero Trust / Access, routing tables, ...)
- Live log viewer with level filters and copy

### Proxy chain

```
Standard mode:   apps ──────────────────────► Aether SOCKS5 (:1819)

Smart routing:   apps → Xray (:1817) ──┬─ geosite:category-ir / geoip:ir → DIRECT
                                       ├─ ads → blocked
                                       └─ foreign → Aether tunnel (:1819)
```

With **always-on core**, the Aether engine keeps running behind a local gate:
STOP closes the gate (and Xray), leaving the handshake warm; START reopens it
instantly. ponytail ceiling: the core lives as long as the app process —
surviving full app restarts is future work (OS service wrapper).

## Building

Prerequisites:

- Rust (stable) via [rustup](https://rustup.rs)
- Node.js 18+
- Linux: `webkit2gtk-4.1` (`libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev` on Debian/Ubuntu)
- Windows: WebView2 (preinstalled on Win11) + MSVC Build Tools
- macOS: Xcode Command Line Tools

### 1. Fetch dependencies

```sh
./scripts/setup-deps.sh        # clones Aether into third_party/ (~large)
```

Linking Aether requires a C/C++ toolchain with **CMake** and **Go**
(BoringSSL build). Without them you can still develop the UI against the
simulated engine: `cargo check --no-default-features --features mock-engine`.

### 2. Fetch the Xray sidecar (smart routing)

```sh
./scripts/fetch_xray.sh        # downloads Xray binary + geo assets per platform
```

Run this once per target platform before bundling; CI should run it with the
matching target triple installed.

### 3. Develop / build

```sh
npm install
npm run tauri dev              # dev app with hot reload
npm run tauri build            # bundles installers into src-tauri/target/release/bundle/
```

Feature flags (on the `nether` crate):

| feature         | effect                                        |
| --------------- | --------------------------------------------- |
| `engine-aether` | default; links Aether in-process              |
| `mock-engine`   | simulated tunnel for UI development           |

## Architecture

```
crates/nether-core     settings model, engine abstraction, log hub, xray config
src-tauri              Tauri shell: IPC commands, event forwarders, sidecar mgmt
src                    React UI (Home / Logs / Settings)
third_party/Aether     upstream library (git-ignored, via setup-deps.sh)
scripts/               dependency fetchers (Aether clone, Xray release)
```

Settings persist to the OS app-data dir as JSON; identity files stay in
`<appdata>/identities/`.

## License

AGPL-3.0-or-later
