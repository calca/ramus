//! Command Tauri: wrapper sottili su `ramus-core`. Nessuna decisione di
//! business logic qui, solo lock dello stato e delega al core.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use ramus_core::{watcher, Block, Config, CoreError, JournalDate, Page, Theme, Vault, VaultStats};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub config: Mutex<Config>,
    // Mai letto direttamente: il campo esiste per tenere in vita il watcher
    // (viene fermato quando droppato) e per poterlo sostituire quando il
    // vault path cambia a runtime.
    #[allow(dead_code)]
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

fn lock_config<'a>(state: &'a State<AppState>) -> Result<MutexGuard<'a, Config>, CoreError> {
    state
        .config
        .lock()
        .map_err(|_| CoreError::Config("stato di configurazione corrotto".to_string()))
}

/// Osserva `root` ed emette `vault://file-changed` per ogni file toccato
/// dall'esterno. Condiviso fra il setup iniziale e `set_vault_path`, che
/// deve ricreare il watcher ogni volta che il vault cambia a runtime.
pub(crate) fn spawn_watcher(
    app: &AppHandle,
    root: PathBuf,
) -> Result<notify::RecommendedWatcher, CoreError> {
    let app_handle = app.clone();
    watcher::watch_vault(root, move |change| {
        let relative_path = change.relative_path.to_string_lossy().to_string();
        let _ = app_handle.emit("vault://file-changed", relative_path);
    })
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, CoreError> {
    let config = lock_config(&state)?;
    Vault::new(config.vault_path.clone()).ensure_exists()?;
    Ok(config.clone())
}

#[tauri::command]
pub fn set_vault_path(
    path: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_vault_path(PathBuf::from(path))?;
    Vault::new(config.vault_path.clone()).ensure_exists()?;

    // Il watcher osservava la cartella vecchia: va ricreato sulla nuova,
    // altrimenti resterebbe a guardare un vault che non è più attivo.
    let new_watcher = spawn_watcher(&app, config.vault_path.clone())?;
    let mut watcher_guard = state
        .watcher
        .lock()
        .map_err(|_| CoreError::Config("stato del watcher corrotto".to_string()))?;
    *watcher_guard = Some(new_watcher);

    Ok(config.clone())
}

#[tauri::command]
pub fn open_today(state: State<AppState>) -> Result<Page, CoreError> {
    let config = lock_config(&state)?;
    let vault = Vault::new(config.vault_path.clone());
    vault.ensure_exists()?;
    vault.open_today()
}

#[tauri::command]
pub fn read_page(path: String, state: State<AppState>) -> Result<Page, CoreError> {
    let config = lock_config(&state)?;
    Vault::new(config.vault_path.clone()).read_page(&path)
}

#[tauri::command]
pub fn write_page(
    path: String,
    blocks: Vec<Block>,
    state: State<AppState>,
) -> Result<(), CoreError> {
    let config = lock_config(&state)?;
    Vault::new(config.vault_path.clone()).write_page(&path, &blocks)
}

#[tauri::command]
pub fn list_journals(
    before: Option<String>,
    limit: u32,
    state: State<AppState>,
) -> Result<Vec<Page>, CoreError> {
    let before = before
        .map(|text| JournalDate::parse(&text).ok_or(CoreError::InvalidDate(text)))
        .transpose()?;
    let config = lock_config(&state)?;
    Vault::new(config.vault_path.clone()).list_journals(before, limit as usize)
}

#[tauri::command]
pub fn vault_stats(state: State<AppState>) -> Result<VaultStats, CoreError> {
    let config = lock_config(&state)?;
    Vault::new(config.vault_path.clone()).stats()
}

#[tauri::command]
pub fn set_theme(theme: Theme, state: State<AppState>) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_theme(theme)?;
    Ok(config.clone())
}

/// Apre la dialog nativa "scegli cartella". `None` se l'utente annulla.
/// Puro I/O di sistema delegato al plugin: nessuna decisione qui.
#[tauri::command]
pub fn pick_vault_folder(app: AppHandle) -> Result<Option<String>, CoreError> {
    match app.dialog().file().blocking_pick_folder() {
        None => Ok(None),
        Some(file_path) => {
            let path = file_path
                .into_path()
                .map_err(|e| CoreError::Config(e.to_string()))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
    }
}
