use std::path::PathBuf;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::CoreError;

/// Un file `.md` del vault è stato creato, modificato o rimosso sul disco,
/// da un processo esterno all'app.
pub struct VaultChange {
    pub relative_path: PathBuf,
}

/// Osserva il vault e invoca `on_change` per ogni file markdown toccato
/// dall'esterno. Il chiamante (il guscio Tauri) decide se ricaricare o
/// avvisare l'utente: qui c'è solo la rilevazione del cambiamento.
/// Il watcher va tenuto in vita (es. in uno stato dell'app): viene fermato
/// quando viene droppato.
pub fn watch_vault<F>(root: PathBuf, mut on_change: F) -> Result<RecommendedWatcher, CoreError>
where
    F: FnMut(VaultChange) + Send + 'static,
{
    let watch_root = root.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else { return };
        if !matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) {
            return;
        }
        for path in event.paths {
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Ok(relative_path) = path.strip_prefix(&watch_root) {
                on_change(VaultChange {
                    relative_path: relative_path.to_path_buf(),
                });
            }
        }
    })
    .map_err(|e| CoreError::Config(e.to_string()))?;

    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| CoreError::Config(e.to_string()))?;

    Ok(watcher)
}
