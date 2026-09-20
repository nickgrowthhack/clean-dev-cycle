use std::{env, fs, path::Path};

// The published version lives in the release manifest, not in Cargo.toml.
fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let manifest = Path::new(&root).join(".clean-dev-cycle-release.json");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let version = fs::read_to_string(&manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value["version"].as_str().map(str::to_owned))
        .unwrap_or_else(|| env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION"));
    println!("cargo:rustc-env=CLEAN_DEV_CYCLE_VERSION={version}");
}
