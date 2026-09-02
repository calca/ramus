mod commands;

use std::sync::Mutex;

use commands::AppState;
use ramus_core::{Config, Vault};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = Config::load_or_init()?;
            let vault_root = config.vault_path.clone();
            Vault::new(vault_root.clone()).ensure_exists()?;

            let watcher = commands::spawn_watcher(app.handle(), vault_root)?;

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
            commands::pick_vault_folder,
            commands::vault_stats,
            commands::set_theme,
            commands::list_pages,
            commands::open_page,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
