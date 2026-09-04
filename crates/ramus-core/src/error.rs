use std::path::PathBuf;

use serde::Serialize;
use serde_json::json;
use thiserror::Error;

/// Errore del core. Implementa `Serialize` a mano (non richiede Tauri:
/// `serde` è già una dipendenza del crate) così i command possono
/// restituirlo direttamente al frontend.
///
/// `Display`/`to_string()` (i messaggi `#[error("...")]` sotto) restano in
/// italiano: sono usati per i log Rust lato server e da `ramus-mcp` (i cui
/// errori tornano a un client MCP, non a una persona con una lingua
/// preferita — vedi `specs/M7/2026-09-04-i18n-errori.DONE.md`, "Cosa resta
/// in italiano"). `impl Serialize`, invece, è quello che raggiunge i
/// command Tauri consumati dalla GUI: emette `{"code", "params"}` invece
/// di una stringa già composta, così il frontend può tradurre (vedi
/// `src/lib/errors.ts`).
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

    /// Messaggio genuinamente dinamico (serde_json, notify, tauri::Error,
    /// conversioni di path) — non riconducibile a un template fisso, a
    /// differenza dei quattro varianti `Poisoned*Lock` sotto (le uniche
    /// costruzioni di `Config` rimaste dopo
    /// `specs/M7/2026-09-04-i18n-errori.DONE.md` sono tutte di questo tipo).
    #[error("errore di configurazione: {0}")]
    Config(String),

    #[error("stato di configurazione corrotto")]
    PoisonedConfigLock,

    #[error("stato dell'indice corrotto")]
    PoisonedIndexLock,

    #[error("stato dell'indice di ricerca corrotto")]
    PoisonedSearchIndexLock,

    #[error("stato del watcher corrotto")]
    PoisonedWatcherLock,

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
        use serde::ser::SerializeMap;

        let (code, params): (&str, serde_json::Value) = match self {
            CoreError::InvalidPath(path) => ("invalid_path", json!({ "path": path })),
            CoreError::PageNotFound(path) => (
                "page_not_found",
                json!({ "path": path.display().to_string() }),
            ),
            CoreError::InvalidDate(date) => ("invalid_date", json!({ "date": date })),
            CoreError::MalformedBlock { line, reason } => {
                ("malformed_block", json!({ "line": line, "reason": reason }))
            }
            CoreError::Io { path, source } => (
                "io",
                json!({ "path": path.display().to_string(), "detail": source.to_string() }),
            ),
            CoreError::Config(detail) => ("config_error", json!({ "detail": detail })),
            CoreError::PoisonedConfigLock => ("poisoned_config_lock", json!({})),
            CoreError::PoisonedIndexLock => ("poisoned_index_lock", json!({})),
            CoreError::PoisonedSearchIndexLock => ("poisoned_search_index_lock", json!({})),
            CoreError::PoisonedWatcherLock => ("poisoned_watcher_lock", json!({})),
            CoreError::Index(e) => ("index_error", json!({ "detail": e.to_string() })),
            CoreError::Search(e) => ("search_error", json!({ "detail": e.to_string() })),
            CoreError::Git(e) => ("git_error", json!({ "detail": e.to_string() })),
        };

        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", code)?;
        map.serialize_entry("params", &params)?;
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_and_params(err: &CoreError) -> (String, serde_json::Value) {
        let value = serde_json::to_value(err).expect("CoreError deve serializzarsi");
        let code = value["code"]
            .as_str()
            .expect("code deve essere una stringa")
            .to_string();
        (code, value["params"].clone())
    }

    #[test]
    fn invalid_path_has_path_param() {
        let err = CoreError::InvalidPath("../fuori".to_string());
        let (code, params) = code_and_params(&err);
        assert_eq!(code, "invalid_path");
        assert_eq!(params, json!({ "path": "../fuori" }));
    }

    #[test]
    fn page_not_found_has_path_param() {
        let err = CoreError::PageNotFound(PathBuf::from("2026-09-04.md"));
        let (code, params) = code_and_params(&err);
        assert_eq!(code, "page_not_found");
        assert_eq!(params, json!({ "path": "2026-09-04.md" }));
    }

    #[test]
    fn invalid_date_has_date_param() {
        let err = CoreError::InvalidDate("non-una-data".to_string());
        let (code, params) = code_and_params(&err);
        assert_eq!(code, "invalid_date");
        assert_eq!(params, json!({ "date": "non-una-data" }));
    }

    #[test]
    fn malformed_block_has_line_and_reason_params() {
        let err = CoreError::MalformedBlock {
            line: 3,
            reason: "prefisso mancante".to_string(),
        };
        let (code, params) = code_and_params(&err);
        assert_eq!(code, "malformed_block");
        assert_eq!(params, json!({ "line": 3, "reason": "prefisso mancante" }));
    }

    #[test]
    fn io_has_path_and_detail_params() {
        let err = CoreError::Io {
            path: PathBuf::from("/tmp/vault/nota.md"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file non trovato"),
        };
        let (code, params) = code_and_params(&err);
        assert_eq!(code, "io");
        assert_eq!(params["path"], json!("/tmp/vault/nota.md"));
        assert!(params["detail"]
            .as_str()
            .unwrap()
            .contains("file non trovato"));
    }

    #[test]
    fn config_error_has_detail_param() {
        let err = CoreError::Config("messaggio dinamico da serde_json".to_string());
        let (code, params) = code_and_params(&err);
        assert_eq!(code, "config_error");
        assert_eq!(
            params,
            json!({ "detail": "messaggio dinamico da serde_json" })
        );
    }

    #[test]
    fn poisoned_lock_variants_have_empty_params() {
        for (err, expected_code) in [
            (CoreError::PoisonedConfigLock, "poisoned_config_lock"),
            (CoreError::PoisonedIndexLock, "poisoned_index_lock"),
            (
                CoreError::PoisonedSearchIndexLock,
                "poisoned_search_index_lock",
            ),
            (CoreError::PoisonedWatcherLock, "poisoned_watcher_lock"),
        ] {
            let (code, params) = code_and_params(&err);
            assert_eq!(code, expected_code);
            assert_eq!(params, json!({}));
        }
    }
}
