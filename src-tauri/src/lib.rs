mod commands;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use commands::AppState;
use ramus_core::{Config, Index, SearchIndex, SyncState, Vault};
use tauri::Manager;

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
                sync_network_state: Mutex::new(SyncState::Idle),
            });

            // Sync Git automatica (M3): primo caso di background task
            // autonomo nell'app (il file watcher è basato su callback di
            // eventi, non su un timer). Tick fisso di 60s indipendente
            // dall'intervallo configurato — quello viene solo confrontato
            // a ogni tick, così un cambio in Impostazioni si applica senza
            // dover ricreare il task. Un pull immediato all'avvio (non
            // bloccante: gira nel suo task, la finestra si apre comunque
            // subito), poi lo stesso ciclo a ogni tick — `run_sync_cycle`
            // fa entrambe le cose, nessuna logica duplicata fra le due.
            let startup_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                commands::run_sync_cycle(&startup_handle);
            });

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(60));
                loop {
                    ticker.tick().await;
                    commands::run_sync_cycle(&app_handle);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_vault_path,
            commands::open_today,
            commands::roll_over_unfinished_tasks,
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
            commands::set_shortcut,
            commands::init_git_sync,
            commands::get_sync_status,
            commands::set_git_sync_interval,
            commands::set_git_remote,
            commands::set_task_rollover,
            commands::set_mcp_enabled,
            commands::get_mcp_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
