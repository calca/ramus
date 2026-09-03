use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

/// Errore del core. Implementa `Serialize` (non richiede Tauri: `serde` è
/// già una dipendenza del crate) così i command possono restituirlo
/// direttamente al frontend come stringa.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("percorso non valido all'interno del vault: {0}")]
    InvalidPath(String),

    #[error("pagina non trovata: {0}")]
    PageNotFound(PathBuf),

    #[error("data non valida, atteso formato YYYY-MM-DD: {0}")]
    InvalidDate(String),

    #[error("riga malformata nel blocco {line}: {reason}")]
    MalformedBlock { line: usize, reason: String },

    #[error("errore di I/O su {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("errore di configurazione: {0}")]
    Config(String),

    #[error("errore di indice: {0}")]
    Index(#[from] rusqlite::Error),

    #[error("errore di ricerca: {0}")]
    Search(#[from] tantivy::TantivyError),

    #[error("errore Git: {0}")]
    Git(#[from] git2::Error),
}

impl Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
