You're continuing development on **Nether** — a multi-platform GUI client for Aether (censorship-circumvention tunnel). It's a Tauri 2 app (Rust backend, React + Mantine frontend) that bundles an Xray-core sidecar for smart routing.

## Repo structure

```
crates/nether-core/       Core Rust lib: settings, engine abstraction, log hub, xray config gen
  src/engine.rs           EngineManager + mock/real Aether impl
  src/gate.rs             ProxyGate for always-on core mode
  src/logging.rs          LogHub (broadcast + history + consecutive-duplicate suppression)
  src/settings.rs         NetherSettings model + to_aether_args()
  src/xray.rs             Xray JSON config generator
src-tauri/                Tauri 2 shell: IPC commands, event forwarders, sidecar mgmt
  src/lib.rs              Connect/disconnect, xray start/stop, log/status forwarders
  tauri.conf.json         App config, bundle externalBin [xray], version
  tauri.android.conf.json Overrides bundle (drops xray sidecar for mobile)
src/                      React UI (Mantine v8, React 18, Vite 6)
  views/Home.tsx          Power button + status badge + address cards
  views/Settings.tsx      Auto-save settings (debounced 600ms, no save button)
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

## Known issues to fix

1. **Navbar still slow on low-end Android** — the Logs interval is gated on `active` now, but Settings' auto-save fires on every draft change. On slow devices, rapid typing in text inputs triggers many saves. Consider: (a) longer debounce for text fields, or (b) save only on blur for text inputs.

2. **The JNI spam is partially fixed** (filtered at logger level) but may still show if `jni::` target leaks through other paths. The consecutive-duplicate suppression handles it, but verify on a real device.

3. **Settings UX**: the auto-save "saved" badge at the top may be jarring — consider a subtle toast or just the badge briefly visible.

4. **The views need visual polish** — user referenced WhiteDNS-style VPN apps and Flutter feel. Current Mantine setup is functional but could use:
   - Better card shadows/elevation on Home
   - Connection progress animation (dots, ring fill, etc.)
   - Settings section transitions
   - Haptic feedback hints (if Tauri supports it)
   - The power button could pulse green when connected

5. **No toast/snackbar** — the app lacks a toast system for feedback (save success, copy success, errors). Consider @mantine/modals or a simple custom toast.

6. **Window handling on Android** — no back button handling, no status bar color matching.

## Build & verify

```sh
# Mock engine (fast, for UI dev):
cargo check --no-default-features --features nether/mock-engine --workspace
npm run build  # tsc + vite

# Real engine (requires third_party/Aether + cmake + go):
cargo check --workspace  # ~40min first time (BoringSSL)
cargo test -p nether-core

# Android (CI only — needs JDK + NDK):
npm run tauri android init
npm run tauri android build -- --apk --target aarch64 --target armv7

# CI auto-publishes on v* tags via .github/workflows/
```

## Version

Current: 0.1.2 (tagged, released). When you push to main, CI builds. Tag `v0.1.3` to release.
