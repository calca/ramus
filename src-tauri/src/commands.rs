//! Command Tauri: wrapper sottili su `ramus-core`. Nessuna decisione di
//! business logic qui, solo lock dello stato e delega al core.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use ramus_core::{
    watcher, Backlink, Block, Config, CoreError, Index, JournalDate, Page, PageSummary, Theme,
    Vault, VaultStats,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub config: Mutex<Config>,
    pub index: Mutex<Index>,
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

fn lock_index<'a>(state: &'a State<AppState>) -> Result<MutexGuard<'a, Index>, CoreError> {
    state
        .index
        .lock()
        .map_err(|_| CoreError::Config("stato dell'indice corrotto".to_string()))
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

        // Tiene l'indice allineato anche per modifiche esterne (non passate
        // da write_page). Un file rimosso resta stale fino al prossimo
        // `sync` completo (apertura vault): coerente con
        // specs/2026-09-02-indice-sqlite.md, non vale la complessità di
        // gestirlo anche qui.
        if let Some(state) = app_handle.try_state::<AppState>() {
            if let (Ok(config), Ok(index)) = (state.config.lock(), state.index.lock()) {
                let vault = Vault::new(config.vault_path.clone());
                if vault
                    .resolve(&relative_path)
                    .map(|abs| abs.exists())
                    .unwrap_or(false)
                {
                    let _ = index.refresh_page(&vault, &relative_path);
                }
            }
        }

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
    let vault = Vault::new(config.vault_path.clone());
    vault.ensure_exists()?;

    // Il watcher osservava la cartella vecchia: va ricreato sulla nuova,
    // altrimenti resterebbe a guardare un vault che non è più attivo.
    let new_watcher = spawn_watcher(&app, config.vault_path.clone())?;
    let mut watcher_guard = state
        .watcher
        .lock()
        .map_err(|_| CoreError::Config("stato del watcher corrotto".to_string()))?;
    *watcher_guard = Some(new_watcher);

    // L'indice era per il vault precedente: se ne apre uno nuovo per la
    // cartella appena scelta e lo si allinea subito al suo contenuto.
    let new_index = Index::open(&vault.root)?;
    new_index.sync(&vault)?;
    let mut index_guard = lock_index(&state)?;
    *index_guard = new_index;

    Ok(config.clone())
}

#[tauri::command]
pub fn open_today(state: State<AppState>) -> Result<Page, CoreError> {
    let config = lock_config(&state)?;
    let vault = Vault::new(config.vault_path.clone());
    vault.ensure_exists()?;
    let page = vault.open_today()?;
    lock_index(&state)?.refresh_page(&vault, &page.path.to_string_lossy())?;
    Ok(page)
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
    let vault = Vault::new(config.vault_path.clone());
    vault.write_page(&path, &blocks)?;
    lock_index(&state)?.refresh_page(&vault, &path)
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

#[tauri::command]
pub fn list_pages(state: State<AppState>) -> Result<Vec<PageSummary>, CoreError> {
    let config = lock_config(&state)?;
    Vault::new(config.vault_path.clone()).list_pages()
}

#[tauri::command]
pub fn open_page(name: String, state: State<AppState>) -> Result<Page, CoreError> {
    let config = lock_config(&state)?;
    let vault = Vault::new(config.vault_path.clone());
    vault.ensure_exists()?;
    let page = vault.open_page(&name)?;
    lock_index(&state)?.refresh_page(&vault, &page.path.to_string_lossy())?;
    Ok(page)
}

#[tauri::command]
pub fn find_backlinks(
    target_title: String,
    state: State<AppState>,
) -> Result<Vec<Backlink>, CoreError> {
    lock_index(&state)?.find_backlinks(&target_title)
}

#[tauri::command]
pub fn list_tags(state: State<AppState>) -> Result<Vec<String>, CoreError> {
    lock_index(&state)?.list_tags()
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
