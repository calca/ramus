//! Sync Git — parte locale (commit automatico su intervallo configurabile),
//! vedi specs/M3/2026-09-02-sync-git-locale.DONE.md. Funzioni libere, non
//! un oggetto persistente: il repository si apre e chiude a ogni
//! operazione, stesso principio già usato per `Vault::new(...)` costruito
//! ad-hoc in ogni command — nessuno stato Git da tenere in vita.

use std::fs;
use std::path::Path;

use git2::{IndexAddOption, Repository, StatusOptions};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Stato di sincronizzazione esposto alla UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    /// `false` se il vault non è (ancora) un repository Git.
    pub enabled: bool,
    /// Epoch secondi dell'ultimo commit, `None` se nessuno esiste ancora.
    pub last_commit_at: Option<i64>,
    /// `true` se ci sono modifiche non ancora committate in questo momento.
    pub dirty: bool,
}

pub fn is_git_repo(vault_root: &Path) -> bool {
    vault_root.join(".git").exists()
}

/// Crea il repository se non esiste già (idempotente) e garantisce che
/// `.gitignore` escluda `.ramus/` (indici derivati e rigenerabili,
/// SPEC.md principio 1 — versionarli produrrebbe solo conflitti di merge
/// rumorosi su file binari senza alcun beneficio).
pub fn init_repo(vault_root: &Path) -> Result<(), CoreError> {
    if !is_git_repo(vault_root) {
        Repository::init(vault_root)?;
    }
    ensure_gitignore(vault_root)
}

/// Aggiunge `.ramus/` a `.gitignore` se non già presente. Append a un file
/// esistente, mai sovrascrittura: altre regole dell'utente restano intatte.
fn ensure_gitignore(vault_root: &Path) -> Result<(), CoreError> {
    let path = vault_root.join(".gitignore");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if existing.lines().any(|line| line.trim() == ".ramus/") {
        return Ok(());
    }
    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(".ramus/\n");
    fs::write(&path, content).map_err(|source| CoreError::Io { path, source })
}

fn status_options() -> StatusOptions {
    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    options
}

/// Committa se ci sono modifiche non tracciate (working tree + untracked).
/// `Ok(false)` se il working tree è pulito — nessun commit vuoto, nessuna
/// firma richiesta in quel caso (evita un errore "user.name mancante"
/// quando non c'è comunque nulla da salvare).
pub fn commit_if_dirty(vault_root: &Path) -> Result<bool, CoreError> {
    let repo = Repository::open(vault_root)?;

    let statuses = repo.statuses(Some(&mut status_options()))?;
    if statuses.is_empty() {
        return Ok(false);
    }
    let file_count = statuses.len();

    let mut index = repo.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree = repo.find_tree(index.write_tree()?)?;

    let signature = repo.signature()?;
    let message = format!("Ramus: sync automatico — {file_count} file modificati");

    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<_> = parent.iter().collect();

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &message,
        &tree,
        &parents,
    )?;
    Ok(true)
}

/// Se, dato lo stato attuale e l'intervallo configurato, è il momento di
/// tentare un commit automatico: ci sono modifiche pendenti **e** (nessun
/// commit precedente esiste ancora, oppure è passato abbastanza tempo
/// dall'ultimo). Senza il primo criterio, una volta superato l'intervallo
/// resterebbe "dovuto" per sempre finché non cambia qualcosa, causando un
/// `commit_if_dirty` inutile a ogni tick successivo.
pub fn is_due(status: &SyncStatus, interval_minutes: u32, now_epoch_secs: i64) -> bool {
    if !status.dirty {
        return false;
    }
    match status.last_commit_at {
        None => true,
        Some(last) => now_epoch_secs.saturating_sub(last) >= i64::from(interval_minutes) * 60,
    }
}

/// Stato di sync per la UI. Su un vault senza `.git`, `enabled: false` e
/// nessun errore — non è un caso anomalo, è semplicemente sync disattivato.
pub fn status(vault_root: &Path) -> Result<SyncStatus, CoreError> {
    if !is_git_repo(vault_root) {
        return Ok(SyncStatus {
            enabled: false,
            last_commit_at: None,
            dirty: false,
        });
    }

    let repo = Repository::open(vault_root)?;
    let statuses = repo.statuses(Some(&mut status_options()))?;
    let dirty = !statuses.is_empty();
    let last_commit_at = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| commit.time().seconds());

    Ok(SyncStatus {
        enabled: true,
        last_commit_at,
        dirty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let mut path = std::env::temp_dir();
            let unique = format!(
                "ramus-core-git-sync-test-{label}-{}-{:?}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            path.push(unique);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Un `Repository::signature()` valido richiede `user.name`/
    /// `user.email` configurati — non garantito sulla macchina di test, si
    /// imposta a livello di repo per non dipendere da una config globale.
    fn configure_identity(dir: &Path) {
        let repo = Repository::open(dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }

    #[test]
    fn is_due_false_when_not_dirty() {
        let status = SyncStatus {
            enabled: true,
            last_commit_at: Some(0),
            dirty: false,
        };
        assert!(!is_due(&status, 10, 100_000));
    }

    #[test]
    fn is_due_true_when_dirty_and_never_committed() {
        let status = SyncStatus {
            enabled: true,
            last_commit_at: None,
            dirty: true,
        };
        assert!(is_due(&status, 10, 0));
    }

    #[test]
    fn is_due_false_when_dirty_but_interval_not_elapsed() {
        let status = SyncStatus {
            enabled: true,
            last_commit_at: Some(1_000),
            dirty: true,
        };
        // 5 minuti dopo l'ultimo commit, intervallo di 10 minuti.
        assert!(!is_due(&status, 10, 1_000 + 5 * 60));
    }

    #[test]
    fn is_due_true_when_dirty_and_interval_elapsed() {
        let status = SyncStatus {
            enabled: true,
            last_commit_at: Some(1_000),
            dirty: true,
        };
        assert!(is_due(&status, 10, 1_000 + 10 * 60));
    }

    #[test]
    fn is_git_repo_before_and_after_init() {
        let dir = TempDir::new("is-repo");
        assert!(!is_git_repo(dir.path()));
        init_repo(dir.path()).unwrap();
        assert!(is_git_repo(dir.path()));
    }

    #[test]
    fn init_repo_is_idempotent_and_does_not_duplicate_gitignore() {
        let dir = TempDir::new("init-idempotent");
        init_repo(dir.path()).unwrap();
        init_repo(dir.path()).unwrap();
        let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(gitignore.matches(".ramus/").count(), 1);
    }

    #[test]
    fn init_repo_on_existing_repo_only_adds_gitignore() {
        let dir = TempDir::new("init-existing-repo");
        Repository::init(dir.path()).unwrap();
        configure_identity(dir.path());
        let vault = Vault::new(dir.path().to_path_buf());
        vault.ensure_exists().unwrap();
        vault
            .write_page("pages/uno.md", &[crate::block::Block::new("x")])
            .unwrap();
        commit_if_dirty(dir.path()).unwrap();

        init_repo(dir.path()).unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        assert!(repo.head().is_ok(), "la history esistente non va toccata");
        assert!(dir.path().join(".gitignore").exists());
    }

    #[test]
    fn init_repo_preserves_existing_gitignore_rules() {
        let dir = TempDir::new("init-preserves-gitignore");
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "node_modules/\n").unwrap();
        init_repo(dir.path()).unwrap();
        let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains("node_modules/"));
        assert!(gitignore.contains(".ramus/"));
    }

    #[test]
    fn commit_if_dirty_on_clean_tree_is_noop() {
        let dir = TempDir::new("commit-clean");
        init_repo(dir.path()).unwrap();
        configure_identity(dir.path());
        // init_repo crea .gitignore, non tracciato: il primo commit_if_dirty
        // lo cattura. Solo il secondo, su un albero davvero pulito, è no-op.
        assert!(commit_if_dirty(dir.path()).unwrap());
        assert!(!commit_if_dirty(dir.path()).unwrap());
    }

    #[test]
    fn commit_if_dirty_after_write_creates_commit() {
        let dir = TempDir::new("commit-after-write");
        init_repo(dir.path()).unwrap();
        configure_identity(dir.path());
        let vault = Vault::new(dir.path().to_path_buf());
        vault.ensure_exists().unwrap();
        vault
            .write_page("pages/uno.md", &[crate::block::Block::new("ciao")])
            .unwrap();

        assert!(commit_if_dirty(dir.path()).unwrap());

        let repo = Repository::open(dir.path()).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert!(head.message().unwrap().contains("sync automatico"));
    }

    #[test]
    fn commit_if_dirty_called_twice_only_commits_once() {
        let dir = TempDir::new("commit-twice");
        init_repo(dir.path()).unwrap();
        configure_identity(dir.path());
        let vault = Vault::new(dir.path().to_path_buf());
        vault.ensure_exists().unwrap();
        vault
            .write_page("pages/uno.md", &[crate::block::Block::new("ciao")])
            .unwrap();

        assert!(commit_if_dirty(dir.path()).unwrap());
        assert!(!commit_if_dirty(dir.path()).unwrap());
    }

    #[test]
    fn status_reflects_dirty_and_last_commit_at() {
        let dir = TempDir::new("status-reflects");
        init_repo(dir.path()).unwrap();
        configure_identity(dir.path());
        // init_repo crea .gitignore, non tracciato: lo si cattura subito
        // così "before" qui sotto parte da un albero davvero pulito.
        commit_if_dirty(dir.path()).unwrap();

        let before = status(dir.path()).unwrap();
        assert!(before.enabled);
        assert!(!before.dirty);
        assert!(before.last_commit_at.is_some());

        let vault = Vault::new(dir.path().to_path_buf());
        vault.ensure_exists().unwrap();
        vault
            .write_page("pages/uno.md", &[crate::block::Block::new("ciao")])
            .unwrap();
        let dirty_status = status(dir.path()).unwrap();
        assert!(dirty_status.dirty);

        commit_if_dirty(dir.path()).unwrap();
        let after = status(dir.path()).unwrap();
        assert!(!after.dirty);
        assert!(after.last_commit_at.is_some());
        assert!(after.last_commit_at >= before.last_commit_at);
    }

    #[test]
    fn status_on_non_git_vault_is_disabled_not_error() {
        let dir = TempDir::new("status-non-git");
        let result = status(dir.path()).unwrap();
        assert_eq!(
            result,
            SyncStatus {
                enabled: false,
                last_commit_at: None,
                dirty: false,
            }
        );
    }
}
