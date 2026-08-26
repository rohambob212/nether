You're continuing development on **Nether** — a multi-platform GUI client for Aether (censorship-circumvention tunnel). It's a Tauri 2 app (Rust backend, React + Mantine frontend) that bundles an Xray-core sidecar for smart routing.

## Repo structure

```
crates/nether-core/       Core Rust lib: settings, engine abstraction, log hub, xray config gen
  src/engine.rs           EngineManager + mock/real Aether impl
  src/gate.rs             ProxyGate for always-on core mode
  src/logging.rs          LogHub (broadcast + history + consecutive-duplicate suppression)
  src/settings.rs         NetherSettings model + to_aether_args()
  src/vpn.rs              tun2socks for VPN mode (feature "vpn", unix only)
  src/xray.rs             Xray JSON config generator
crates/tauri-plugin-nethervpn/   Android VpnService bridge (android-only dep)
  src/lib.rs              prepare/start/stop over run_mobile_plugin
  android/                Kotlin VpnService + Tauri plugin + manifest
src-tauri/                Tauri 2 shell: IPC commands, event forwarders, sidecar mgmt
  src/lib.rs              Connect/disconnect, xray start/stop, log/status forwarders
  tauri.conf.json         App config, bundle externalBin [xray], version
  tauri.android.conf.json Overrides bundle (drops xray sidecar for mobile)
src/                      React UI (Mantine v8, React 18, Vite 6)
  views/Home.tsx          Power button + status badge + address cards
  views/Settings.tsx      Auto-save settings (debounced 600ms, no save button), card UI
  views/Logs.tsx          Filtered log viewer (active-tab gated 250ms refresh)
  App.tsx                 Persistent view shell (no remount on tab switch)
  api.ts                  Tauri IPC wrappers + types
  styles.css              App shell, power button, tabs, log list
scripts/
  setup-deps.sh           Clones Aether into third_party/Aether (shallow)
  fetch_xray.sh           Downloads Xray sidecar per host triple
.github/workflows/
  desktop.yml             Win/mac/linux matrix build
  android.yml             Arm64+armv7 universal APK, signed with apksigner
```

## Current state

- **Desktop builds**: green on Windows, macOS (aarch64), Linux
- **Android build**: green, signed universal APK (arm64 + armv7), pixel-portal launcher icon
- **All todos from original plan are done** and verified (cargo check + cargo test + tsc + vite build + xray config validation)
- **UI rebuilt** on Mantine v8 (React 18 peer dep, v9 requires React 19)
- **Log hub**: consecutive duplicate collapse with ×N counter, `jni::` targets filtered out, max level Debug (no TRACE)
- **Auto-save**: Settings save on every change (debounced), no save button
- **Persistent views**: all 3 views stay mounted, tab switch is animated, Logs refresh gated on active tab

## VPN mode (Android)

Settings → VPN mode routes the **whole device** through the tunnel instead of
only apps pointed at the SOCKS port.

```
VpnService.establish() ──fd──► nether_core::vpn ──► SOCKS5 ──► 127.0.0.1:1819
```

- `crates/tauri-plugin-nethervpn` is a real Tauri mobile plugin; its Kotlin
  lives there because `src-tauri/gen/android/` is regenerated on every CI build
  and would lose anything written into it.
- **The loop-breaker is `addDisallowedApplication(packageName)`.** Aether opens
  its Cloudflare sockets from inside this process — without that exclusion every
  tunnel packet gets routed straight back into the tunnel.
- The TUN fd is `detachFd()`-ed in Kotlin, so Rust is its only owner. Closing it
  (aborting the pump task) is what actually drops the interface. Do not also
  hold the ParcelFileDescriptor, or it double-closes.
- MTU is 1280: Aether's tunnel carries 1280 and it is the IPv6 floor.
- IPv6 routes are only claimed when `ipVersion != v4`, otherwise v6 flows get
  black-holed instead of falling back.
- Consent (`VpnService.prepare`) is requested when the user flips the switch, so
  the system dialog lands on a deliberate gesture rather than mid-connect.
- Smart routing (Xray) is desktop-only, so VPN mode always targets `socksPort`.

**Not yet verified on a real device.** The Rust tun2socks half has unit tests
(`cargo test -p nether-core --features engine-mock,vpn`), but the Kotlin, the
gradle wiring and the fd handoff have only ever been compiled by CI.

## Known issues to fix

1. **Navbar still slow on low-end Android** — the Logs interval is gated on `active` now, but Settings' auto-save fires on every draft change. On slow devices, rapid typing in text inputs triggers many saves. Consider: (a) longer debounce for text fields, or (b) save only on blur for text inputs.

1b. **VPN mode UDP** opens one SOCKS5 association per flow, so DNS makes a new
   TCP control connection per query. Pool by source address if it shows up in a
   profile (marked with a `ponytail:` comment in `vpn.rs`).

2. **JNI spam** — filtered at logger level and the global filter is now `Debug`, so TRACE callsites no longer format args just to be dropped. Verify on a real device.

3. ~~Settings UX: the auto-save "saved" badge~~ — now a sticky pill that fades
   out after 1.6s instead of shoving the form down.

4. **Home still needs the polish Settings got** — user referenced WhiteDNS-style
   VPN apps and Flutter feel. Settings is now card-based (`.set-card` in
   styles.css); Home has not been reworked to match. Still open there:
   - Better card shadows/elevation on Home
   - Connection progress animation (dots, ring fill, etc.)
   - Haptic feedback hints (if Tauri supports it)
   - The power button could pulse green when connected

5. **No toast/snackbar** — the app lacks a toast system for feedback (save success, copy success, errors). Consider @mantine/modals or a simple custom toast.

6. **Window handling on Android** — no back button handling, no status bar color matching.

## Build & verify

```sh
# Mock engine (fast, for UI dev):
cargo check --no-default-features --features nether/mock-engine --workspace
npm run build  # tsc + vite

# VPN engine (tun2socks) — builds and tests on plain Linux, no NDK needed:
cargo test -p nether-core --no-default-features --features engine-mock,vpn

# Real engine (requires third_party/Aether + cmake + go):
cargo check --workspace  # ~40min first time (BoringSSL)
cargo test -p nether-core

# Android (CI only — needs JDK + NDK):
npm run tauri android init
npm run tauri android build -- --apk --target aarch64 --target armv7

# CI auto-publishes on v* tags via .github/workflows/
```

## Version

Current: 0.1.3 (tagged, released). When you push to main, CI builds. Tag `v0.1.4` to release.
