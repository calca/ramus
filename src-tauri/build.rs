fn main() {
    // tauri-build traccia solo tauri.conf.json per il rebuild: le icone in
    // icons/ non fanno scattare una ricompilazione da sole, quindi
    // cambiarle senza toccare anche la config lascia i byte vecchi
    // incorporati nel binario. Tracciata esplicitamente qui.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
