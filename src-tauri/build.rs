use std::fs;
use std::path::Path;

fn main() {
    // tauri-build traccia solo tauri.conf.json per il rebuild: le icone in
    // icons/ non fanno scattare una ricompilazione da sole, quindi
    // cambiarle senza toccare anche la config lascia i byte vecchi
    // incorporati nel binario. Tracciata esplicitamente qui.
    println!("cargo:rerun-if-changed=icons");

    // TARGET è la variabile che Cargo passa a ogni build script col target
    // triple corrente — la incorporiamo come env!() a tempo di compilazione
    // per trovare il sidecar ramus-mcp a runtime, che Tauri nomina
    // "ramus-mcp-<target-triple>" nel bundle finale (externalBin, vedi
    // specs/release/2026-09-03-packaging-mcp.DONE.md). Non disponibile
    // altrimenti: Rust non espone il target triple completo a runtime.
    let target_triple = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=TARGET_TRIPLE={target_triple}");

    // tauri_build::build() valida che ogni bundle.externalBin esista già
    // su disco — anche per un semplice `cargo check`/`cargo test`, non
    // solo per `tauri build`. Senza questo, dichiarare l'externalBin in
    // tauri.conf.json romperebbe ogni build ordinaria finché qualcuno non
    // esegue prima `npm run prepare:mcp-sidecar` a mano. Un segnaposto
    // vuoto (mai un binario reale: build.rs non deve compilare un altro
    // crate del workspace al proprio interno) soddisfa solo il controllo
    // di esistenza — scripts/prepare-mcp-sidecar.mjs lo sovrascrive con
    // il binario vero prima di un `tauri build` reale.
    ensure_mcp_sidecar_placeholder(&target_triple);

    tauri_build::build()
}

fn ensure_mcp_sidecar_placeholder(target_triple: &str) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let path = Path::new(&manifest_dir)
        .join("binaries")
        .join(format!("ramus-mcp-{target_triple}{suffix}"));

    if path.exists() {
        return;
    }

    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if fs::write(&path, []).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
        }
    }
}
