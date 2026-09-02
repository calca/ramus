mod commands;

use std::sync::Mutex;

use commands::AppState;
use ramus_core::{watcher, Config, Vault};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = Config::load_or_init()?;
            let vault_root = config.vault_path.clone();
            Vault::new(vault_root.clone()).ensure_exists()?;

            let app_handle = app.handle().clone();
            let watcher = watcher::watch_vault(vault_root, move |change| {
                let relative_path = change.relative_path.to_string_lossy().to_string();
                let _ = app_handle.emit("vault://file-changed", relative_path);
            })?;

            app.manage(AppState {
                config: Mutex::new(config),
                watcher: Mutex::new(Some(watcher)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_vault_path,
            commands::open_today,
            commands::read_page,
            commands::write_page,
            commands::list_journals,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
