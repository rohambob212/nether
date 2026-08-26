// The plugin exposes no JS-invocable commands — `src-tauri` calls it from Rust
// through `run_mobile_plugin`, which bypasses the capability layer — so there
// is nothing to generate permissions for. The build still runs so the Android
// project gets registered with the Tauri CLI.
const COMMANDS: &[&str] = &[];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
