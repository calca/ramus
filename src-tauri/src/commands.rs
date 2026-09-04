//! Command Tauri: wrapper sottili su `ramus-core`. Nessuna decisione di
//! business logic qui, solo lock dello stato e delega al core.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use ramus_core::{
    git_sync, rollover, watcher, Backlink, Block, Config, CoreError, Index, JournalDate, Locale,
    Page, PageSummary, RolloverOutcome, SearchHit, SearchIndex, SyncState, SyncStatus, TaskHit,
    Theme, Vault, VaultStats,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
#[cfg(desktop)]
use tauri_plugin_dialog::DialogExt;

/// Quanto a lungo un path scritto da un command dell'app viene ignorato dal
/// watcher: copre la latenza fra `fs::write` e la consegna dell'evento del
/// filesystem, molto più ampia del necessario di proposito (millisecondi
/// nella pratica) per non rischiare falsi negativi.
const SELF_WRITE_WINDOW: Duration = Duration::from_secs(2);

pub struct AppState {
    pub config: Mutex<Config>,
    pub index: Mutex<Index>,
    pub search_index: Mutex<SearchIndex>,
    // Mai letto direttamente: il campo esiste per tenere in vita il watcher
    // (viene fermato quando droppato) e per poterlo sostituire quando il
    // vault path cambia a runtime.
    #[allow(dead_code)]
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    /// Path relativi scritti da un command dell'app (write_page, open_today,
    /// open_page) con il momento della scrittura — il watcher li confronta
    /// per non scambiare il proprio salvataggio per una modifica esterna
    /// (altrimenti ogni battitura mostrerebbe il banner "file cambiato",
    /// vedi bug segnalato dall'utente).
    pub recent_writes: Mutex<HashMap<String, Instant>>,
    /// Esito dell'ultimo pull/push tentato — `Syncing`/`Conflict`/`Offline`
    /// non sono ricavabili ispezionando solo il repository su disco (vedi
    /// `ramus_core::git_sync::status`), vivono qui come stato di sessione.
    pub sync_network_state: Mutex<SyncState>,
}

fn mark_self_write(state: &State<AppState>, relative_path: &str) {
    if let Ok(mut recent) = state.recent_writes.lock() {
        recent.insert(relative_path.to_string(), Instant::now());
    }
}

fn set_network_state(state: &State<AppState>, new_state: SyncState) {
    if let Ok(mut guard) = state.sync_network_state.lock() {
        *guard = new_state;
    }
}

fn current_network_state(state: &State<AppState>) -> SyncState {
    state
        .sync_network_state
        .lock()
        .map(|guard| *guard)
        .unwrap_or(SyncState::Idle)
}

fn lock_config<'a>(state: &'a State<AppState>) -> Result<MutexGuard<'a, Config>, CoreError> {
    state
        .config
        .lock()
        .map_err(|_| CoreError::PoisonedConfigLock)
}

fn lock_index<'a>(state: &'a State<AppState>) -> Result<MutexGuard<'a, Index>, CoreError> {
    state.index.lock().map_err(|_| CoreError::PoisonedIndexLock)
}

fn lock_search_index<'a>(
    state: &'a State<AppState>,
) -> Result<MutexGuard<'a, SearchIndex>, CoreError> {
    state
        .search_index
        .lock()
        .map_err(|_| CoreError::PoisonedSearchIndexLock)
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

        let Some(state) = app_handle.try_state::<AppState>() else {
            return;
        };

        // Il salvataggio dell'app stessa tocca lo stesso file che il
        // watcher osserva: senza questo controllo, ogni scrittura propria
        // verrebbe scambiata per una modifica esterna (falso positivo del
        // banner "file cambiato" a ogni battitura).
        let is_self_write = state
            .recent_writes
            .lock()
            .ok()
            .and_then(|recent| recent.get(&relative_path).copied())
            .is_some_and(|written_at| written_at.elapsed() < SELF_WRITE_WINDOW);
        if is_self_write {
            return;
        }

        // Tiene entrambi gli indici allineati per modifiche esterne vere
        // (non passate dai command dell'app). Un file rimosso resta stale
        // fino al prossimo `sync` completo (apertura vault): coerente con
        // specs/M2/2026-09-02-indice-sqlite.DONE.md, non vale la complessità
        // di gestirlo anche qui.
        if let (Ok(config), Ok(index), Ok(search_index)) = (
            state.config.lock(),
            state.index.lock(),
            state.search_index.lock(),
        ) {
            let vault = Vault::new(config.vault_path.clone());
            if vault
                .resolve(&relative_path)
                .map(|abs| abs.exists())
                .unwrap_or(false)
            {
                let _ = index.refresh_page(&vault, &relative_path);
                let _ = search_index.refresh_page(&vault, &relative_path);
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
        .map_err(|_| CoreError::PoisonedWatcherLock)?;
    *watcher_guard = Some(new_watcher);

    // Gli indici erano per il vault precedente: se ne aprono di nuovi per
    // la cartella appena scelta e li si allinea subito al suo contenuto.
    // L'indice di ricerca è "dumb" (vedi specs/M2/2026-09-02-ricerca-full-text.DONE.md):
    // riceve esattamente i path che l'indice SQLite ha rilevato come
    // nuovi/cambiati/rimossi, nessuna logica di staleness propria.
    let new_index = Index::open(&vault.root)?;
    let outcome = new_index.sync(&vault)?;
    let new_search_index = SearchIndex::open(&vault.root)?;
    for path in &outcome.refreshed {
        new_search_index.refresh_page(&vault, path)?;
    }
    for path in &outcome.removed {
        new_search_index.remove_page(path)?;
    }

    let mut index_guard = lock_index(&state)?;
    *index_guard = new_index;
    let mut search_index_guard = lock_search_index(&state)?;
    *search_index_guard = new_search_index;

    Ok(config.clone())
}

/// Sposta i task `[ ] ` non fatti rimasti nella finestra configurata
/// verso oggi (M4). Chiamata dal frontend prima di aprire il journal di
/// oggi (avvio, o rollover di mezzanotte a app aperta) — no-op se
/// `task_rollover_enabled` è `false`. Nessun `mark_self_write` qui: le
/// pagine sorgente toccate devono restare rilevabili dal file watcher
/// come una modifica "esterna", per riusare lo stesso percorso già
/// gestito in `App.tsx` (ricarica silenziosa se non dirty, avviso se
/// dirty) invece di duplicare quella logica qui.
#[tauri::command]
pub fn roll_over_unfinished_tasks(state: State<AppState>) -> Result<RolloverOutcome, CoreError> {
    let config = lock_config(&state)?;
    if !config.task_rollover_enabled {
        return Ok(RolloverOutcome { moved_count: 0 });
    }
    let vault = Vault::new(config.vault_path.clone());
    rollover::roll_over_unfinished_tasks(&vault, config.task_rollover_days)
}

#[tauri::command]
pub fn open_today(state: State<AppState>) -> Result<Page, CoreError> {
    let config = lock_config(&state)?;
    let vault = Vault::new(config.vault_path.clone());
    vault.ensure_exists()?;
    let page = vault.open_today()?;
    let relative_path = page.path.to_string_lossy();
    mark_self_write(&state, &relative_path);
    lock_index(&state)?.refresh_page(&vault, &relative_path)?;
    lock_search_index(&state)?.refresh_page(&vault, &relative_path)?;
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
    mark_self_write(&state, &path);
    lock_index(&state)?.refresh_page(&vault, &path)?;
    lock_search_index(&state)?.refresh_page(&vault, &path)
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
pub fn set_locale(locale: Locale, state: State<AppState>) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_locale(locale)?;
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
    let relative_path = page.path.to_string_lossy();
    mark_self_write(&state, &relative_path);
    lock_index(&state)?.refresh_page(&vault, &relative_path)?;
    lock_search_index(&state)?.refresh_page(&vault, &relative_path)?;
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

#[tauri::command]
pub fn list_open_tasks(state: State<AppState>) -> Result<Vec<TaskHit>, CoreError> {
    lock_index(&state)?.list_open_tasks()
}

#[tauri::command]
pub fn search(query: String, state: State<AppState>) -> Result<Vec<SearchHit>, CoreError> {
    lock_search_index(&state)?.search(&query)
}

#[tauri::command]
pub fn set_shortcut(
    action_id: String,
    shortcut: String,
    state: State<AppState>,
) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_shortcut(action_id, shortcut)?;
    Ok(config.clone())
}

/// Crea il repository (idempotente) e committa subito lo stato attuale del
/// vault — l'utente non deve aspettare il prossimo tick per vedere il primo
/// commit dopo aver attivato la sync.
#[tauri::command]
pub fn init_git_sync(state: State<AppState>) -> Result<SyncStatus, CoreError> {
    let config = lock_config(&state)?;
    git_sync::init_repo(&config.vault_path)?;
    git_sync::commit_if_dirty(&config.vault_path)?;
    git_sync::status(&config.vault_path, current_network_state(&state))
}

#[tauri::command]
pub fn get_sync_status(state: State<AppState>) -> Result<SyncStatus, CoreError> {
    let config = lock_config(&state)?;
    git_sync::status(&config.vault_path, current_network_state(&state))
}

#[tauri::command]
pub fn set_git_sync_interval(minutes: u32, state: State<AppState>) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_git_sync_interval_minutes(minutes)?;
    Ok(config.clone())
}

#[tauri::command]
pub fn set_task_rollover(
    enabled: bool,
    days: u32,
    state: State<AppState>,
) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_task_rollover(enabled, days)?;
    Ok(config.clone())
}

#[tauri::command]
pub fn set_mcp_enabled(enabled: bool, state: State<AppState>) -> Result<Config, CoreError> {
    let mut config = lock_config(&state)?;
    config.set_mcp_enabled(enabled)?;
    Ok(config.clone())
}

#[derive(Serialize)]
pub struct McpInfo {
    pub enabled: bool,
    pub binary_found: bool,
    /// Snippet JSON pronto da incollare in `.mcp.json`/
    /// `claude_desktop_config.json`, `None` se `binary_found` è `false`.
    pub config_snippet: Option<String>,
}

/// `ramus-mcp` e il binario dell'app Tauri sono membri dello stesso
/// workspace Cargo: `cargo build`/`cargo tauri dev` li compila nella
/// stessa cartella — file fratelli, primo tentativo sotto (nome
/// semplice). In una build pacchettizzata (`tauri build`), `ramus-mcp`
/// è incluso come sidecar (`bundle.externalBin` in `tauri.conf.json`)
/// e Tauri lo posiziona comunque accanto al binario principale, ma con
/// il nome suffissato dal target triple corrente (convenzione Tauri
/// per gli external bin) — secondo tentativo. `TARGET_TRIPLE` è
/// iniettato a tempo di compilazione da `build.rs`: Rust non espone
/// il target triple completo a runtime altrimenti. Vedi
/// specs/release/2026-09-03-packaging-mcp.DONE.md.
fn find_mcp_binary() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let dir = current.parent()?;
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let candidates = [
        format!("ramus-mcp{suffix}"),
        format!("ramus-mcp-{}{suffix}", env!("TARGET_TRIPLE")),
    ];
    candidates
        .into_iter()
        .map(|name| dir.join(name))
        .find(|candidate| {
            // Non solo esiste: build.rs crea un segnaposto vuoto allo stesso
            // nome suffissato per non rompere `cargo check`/`cargo test`
            // ordinari (vedi build.rs) — un file a lunghezza zero non è mai
            // il binario vero, anche se dovesse finire per sbaglio in una
            // build pacchettizzata.
            candidate
                .metadata()
                .map(|meta| meta.len() > 0)
                .unwrap_or(false)
        })
}

#[tauri::command]
pub fn get_mcp_info(state: State<AppState>) -> Result<McpInfo, CoreError> {
    let enabled = lock_config(&state)?.mcp_enabled;
    let binary = find_mcp_binary();
    let config_snippet = binary.as_ref().map(|path| {
        let snippet = serde_json::json!({
            "mcpServers": {
                "ramus": {
                    "command": path.to_string_lossy(),
                }
            }
        });
        serde_json::to_string_pretty(&snippet).unwrap_or_default()
    });
    Ok(McpInfo {
        enabled,
        binary_found: binary.is_some(),
        config_snippet,
    })
}

/// Imposta (o aggiorna) il remote `origin` e prova subito un pull — stessa
/// logica di `init_git_sync`, l'utente non deve aspettare il prossimo tick
/// per vedere l'esito del primo collegamento al remote.
#[tauri::command]
pub fn set_git_remote(url: String, state: State<AppState>) -> Result<SyncStatus, CoreError> {
    let vault_path = lock_config(&state)?.vault_path.clone();
    git_sync::set_remote(&vault_path, &url)?;

    match git_sync::pull(&vault_path) {
        Ok(outcome) if outcome.conflict => set_network_state(&state, SyncState::Conflict),
        Ok(_) => set_network_state(&state, SyncState::Idle),
        // Un pull fallito qui è quasi sempre di rete/autenticazione: si
        // riflette nello stato invece di far fallire il command — l'utente
        // ha comunque impostato il remote con successo, riprova al
        // prossimo tick.
        Err(_) => set_network_state(&state, SyncState::Offline),
    }

    git_sync::status(&vault_path, current_network_state(&state))
}

/// Un ciclo di sync completo: pull (se c'è un remote) — i file che cambia
/// arrivano al frontend tramite lo stesso file watcher già usato per
/// qualunque modifica esterna (`checkout_head` scrive su disco come
/// qualunque altro processo, nessuna notifica separata da costruire qui) —
/// poi, se non in conflitto, commit locale se dovuto e push se c'è stato un
/// commit. Chiamato sia una volta all'avvio (pull immediato, non bloccante)
/// sia a ogni tick del task periodico: stessa logica, nessuna duplicazione.
pub(crate) fn run_sync_cycle(app_handle: &AppHandle) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    let Ok(config) = state.config.lock() else {
        return;
    };
    let vault_path = config.vault_path.clone();
    let interval_minutes = config.git_sync_interval_minutes;
    drop(config);

    if !git_sync::is_git_repo(&vault_path) {
        return;
    }

    if git_sync::has_remote(&vault_path) {
        set_network_state(&state, SyncState::Syncing);
        match git_sync::pull(&vault_path) {
            Ok(outcome) if outcome.conflict => {
                set_network_state(&state, SyncState::Conflict);
                return;
            }
            Ok(_) => set_network_state(&state, SyncState::Idle),
            Err(_) => set_network_state(&state, SyncState::Offline),
        }
    }

    if current_network_state(&state) == SyncState::Conflict {
        return;
    }

    let Ok(status) = git_sync::status(&vault_path, SyncState::Idle) else {
        return;
    };
    if !git_sync::is_due(&status, interval_minutes, now_epoch_secs()) {
        return;
    }
    if let Ok(true) = git_sync::commit_if_dirty(&vault_path) {
        if git_sync::has_remote(&vault_path) {
            match git_sync::push(&vault_path) {
                Ok(()) => set_network_state(&state, SyncState::Idle),
                Err(_) => set_network_state(&state, SyncState::Offline),
            }
        }
    }
}

fn now_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Apre la dialog nativa "scegli cartella". `None` se l'utente annulla.
/// Puro I/O di sistema delegato al plugin: nessuna decisione qui.
///
/// Su mobile non esiste un selettore di cartella nell'API del plugin
/// dialog (`blocking_pick_folder` è desktop-only) — scelta già presa in
/// `specs/M6/2026-09-03-supporto-mobile-fondamenta.DONE.md`, "il vault
/// vive in un percorso fisso". Il comando resta registrato su entrambe
/// le piattaforme (nessuna modifica a `generate_handler!`/al frontend,
/// che già gestisce `None` come "annullato, nessuna modifica"): su
/// mobile si comporta sempre come un annullamento invece di provare a
/// compilare una chiamata che lì non esiste.
#[tauri::command]
pub fn pick_vault_folder(app: AppHandle) -> Result<Option<String>, CoreError> {
    #[cfg(desktop)]
    {
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
    #[cfg(mobile)]
    {
        let _ = app;
        Ok(None)
    }
}
