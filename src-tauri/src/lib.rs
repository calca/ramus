mod commands;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use commands::AppState;
use ramus_core::{git_sync, Config, Index, SearchIndex, Vault};
use tauri::Manager;

fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = Config::load_or_init()?;
            let vault_root = config.vault_path.clone();
            let vault = Vault::new(vault_root.clone());
            vault.ensure_exists()?;

            let watcher = commands::spawn_watcher(app.handle(), vault_root.clone())?;

            let index = Index::open(&vault_root)?;
            let outcome = index.sync(&vault)?;

            // L'indice di ricerca è "dumb" (vedi
            // specs/M2/2026-09-02-ricerca-full-text.DONE.md): riceve esattamente i
            // path che Index::sync ha già rilevato come nuovi/cambiati/
            // rimossi, nessuna contabilità di mtime propria.
            let search_index = SearchIndex::open(&vault_root)?;
            for path in &outcome.refreshed {
                search_index.refresh_page(&vault, path)?;
            }
            for path in &outcome.removed {
                search_index.remove_page(path)?;
            }

            app.manage(AppState {
                config: Mutex::new(config),
                index: Mutex::new(index),
                search_index: Mutex::new(search_index),
                watcher: Mutex::new(Some(watcher)),
                recent_writes: Mutex::new(HashMap::new()),
            });

            // Sync Git automatica (M3): primo caso di background task
            // autonomo nell'app (il file watcher è basato su callback di
            // eventi, non su un timer). Tick fisso di 60s indipendente
            // dall'intervallo configurato — quello viene solo confrontato
            // a ogni tick, così un cambio in Impostazioni si applica senza
            // dover ricreare il task.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(60));
                loop {
                    ticker.tick().await;
                    let Some(state) = app_handle.try_state::<AppState>() else {
                        continue;
                    };
                    let Ok(config) = state.config.lock() else {
                        continue;
                    };
                    let vault_path = config.vault_path.clone();
                    let interval_minutes = config.git_sync_interval_minutes;
                    drop(config);

                    let Ok(status) = git_sync::status(&vault_path) else {
                        continue;
                    };
                    if git_sync::is_due(&status, interval_minutes, now_epoch_secs()) {
                        let _ = git_sync::commit_if_dirty(&vault_path);
                    }
                }
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
            commands::find_backlinks,
            commands::list_tags,
            commands::search,
            commands::set_search_shortcut,
            commands::init_git_sync,
            commands::get_sync_status,
            commands::set_git_sync_interval,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
