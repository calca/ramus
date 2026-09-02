//! Command Tauri: wrapper sottili su `ramus-core`. Nessuna decisione di
//! business logic qui, solo lock dello stato e delega al core.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use ramus_core::{Block, Config, CoreError, Page, Vault};
use tauri::State;

pub struct AppState {
    pub config: Mutex<Config>,
    // Mai letto: il campo esiste solo per tenere in vita il watcher (viene
    // fermato quando droppato).
    #[allow(dead_code)]
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

fn lock_config<'a>(state: &'a State<AppState>) -> Result<MutexGuard<'a, Config>, CoreError> {
    state
        .config
        .lock()
        .map_err(|_| CoreError::Config("stato di configurazione corrotto".to_string()))
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, CoreError> {
    let config = lock_config(&state)?;
    Vault::new(config.vault_path.clone()).ensure_exists()?;
    Ok(config.clone())
}

#[tauri::command]
pub fn set_vault_path(path: String, state: State<AppState>) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_vault_path(PathBuf::from(path))?;
    Vault::new(config.vault_path.clone()).ensure_exists()?;
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
