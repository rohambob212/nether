fn main() {
    // Fail early with a helpful message instead of a cryptic cargo error when
    // the Aether dependency hasn't been cloned yet.
    #[allow(unused_mut)]
    let mut wants_aether = std::env::var("CARGO_FEATURE_ENGINE_AETHER")
        .map(|v| v == "1")
        .unwrap_or(false);

    // The mock engine implies no aether linkage even if both flags are set.
    if std::env::var("CARGO_FEATURE_ENGINE_MOCK").map(|v| v == "1").unwrap_or(false) {
        wants_aether = false;
    }

    if wants_aether && !std::path::Path::new("../third_party/Aether/aether/Cargo.toml").exists() {
        panic!(
            "\n\n  Nether links Aether as a library, but third_party/Aether is missing.\n  \
             Run `scripts/setup-deps.sh` (or clone https://github.com/CluvexStudio/Aether\n  \
             into ./third_party/Aether), or build the UI-only shell with:\n\n      \
             cargo build --no-default-features --features mock-engine,custom-protocol\n\n  \
             For npm-driven builds: npm run tauri dev -- --no-default-features ...\n  \
             See README.md for details.\n\n"
        );
    }

    tauri_build::build()
}
