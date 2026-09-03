//! Sync Git — parte locale (commit automatico) e remota (pull/push,
//! vedi specs/M3/2026-09-02-sync-git-locale.DONE.md e
//! specs/M3/2026-09-02-sync-git-remoto.DONE.md). Funzioni libere, non
//! un oggetto persistente: il repository si apre e chiude a ogni
//! operazione, stesso principio già usato per `Vault::new(...)` costruito
//! ad-hoc in ogni command — nessuno stato Git da tenere in vita.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use git2::build::CheckoutBuilder;
use git2::{
    Cred, CredentialType, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository,
    StatusOptions,
};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Stato della sync verso il remote. `Syncing`/`Conflict`/`Offline` non sono
/// ricavabili ispezionando solo il repository su disco (se diverge dal
/// remote lo si scopre solo provando un fetch, non a ogni interrogazione di
/// stato) — vivono come stato di sessione nel guscio (`AppState`) e vengono
/// passati a [`status`] come `network_hint` invece che ricalcolati qui.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncState {
    /// Niente `.git`.
    Disabled,
    /// `.git` c'è, nessun remote `origin` configurato.
    NoRemote,
    /// Sincronizzato, nessuna operazione in corso.
    Idle,
    /// Pull/commit/push in corso in questo momento.
    Syncing,
    /// `merge_analysis` ha rilevato storie divergenti: auto-sync fermo
    /// finché non si torna a un fast-forward possibile.
    Conflict,
    /// Ultimo pull/push fallito per rete, si riprova al prossimo tick.
    Offline,
}

/// Stato di sincronizzazione esposto alla UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    /// `false` se il vault non è (ancora) un repository Git.
    pub enabled: bool,
    /// Epoch secondi dell'ultimo commit, `None` se nessuno esiste ancora.
    pub last_commit_at: Option<i64>,
    /// `true` se ci sono modifiche non ancora committate in questo momento.
    pub dirty: bool,
    pub state: SyncState,
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
/// `network_hint`: lo stato dell'ultimo pull/push tentato (vedi
/// [`SyncState`]) — ignorato se non c'è un remote configurato (`NoRemote`
/// vince sempre) o se il repo non esiste (`Disabled` vince sempre).
pub fn status(vault_root: &Path, network_hint: SyncState) -> Result<SyncStatus, CoreError> {
    if !is_git_repo(vault_root) {
        return Ok(SyncStatus {
            enabled: false,
            last_commit_at: None,
            dirty: false,
            state: SyncState::Disabled,
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

    let state = if repo.find_remote("origin").is_err() {
        SyncState::NoRemote
    } else {
        network_hint
    };

    Ok(SyncStatus {
        enabled: true,
        last_commit_at,
        dirty,
        state,
    })
}

fn remote_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed| {
        if allowed.contains(CredentialType::SSH_KEY) {
            return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT)
            || allowed.contains(CredentialType::DEFAULT)
        {
            let config = git2::Config::open_default()?;
            return Cred::credential_helper(&config, url, username_from_url);
        }
        Err(git2::Error::from_str(
            "nessuna credenziale disponibile per questo remote",
        ))
    });
    callbacks
}

/// `true` se il repo ha un remote `origin` configurato.
pub fn has_remote(vault_root: &Path) -> bool {
    match Repository::open(vault_root) {
        Ok(repo) => repo.find_remote("origin").is_ok(),
        Err(_) => false,
    }
}

/// Imposta (o aggiorna se già presente) l'URL del remote `origin`. Nessun
/// altro remote è mai gestito da Ramus — vedi
/// specs/M3/2026-09-02-sync-git-remoto.DONE.md, "un solo remote origin".
pub fn set_remote(vault_root: &Path, url: &str) -> Result<(), CoreError> {
    let repo = Repository::open(vault_root)?;
    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", url)?;
    } else {
        repo.remote("origin", url)?;
    }
    Ok(())
}

/// Esito di un [`pull`]: i path `.md` che sono cambiati (solo se non
/// `conflict` — un pull in conflitto non tocca nessun file) e se le storie
/// sono risultate divergenti (mai un merge automatico in quel caso, vedi
/// [`SyncState::Conflict`]).
pub struct PullOutcome {
    pub changed_paths: Vec<String>,
    pub conflict: bool,
}

/// Fetch da `origin` seguito da un fast-forward se possibile. Storie
/// divergenti (`merge_analysis` normale, non fast-forward) non vengono mai
/// unite automaticamente: `PullOutcome::conflict` lo segnala, nessun file
/// viene toccato.
pub fn pull(vault_root: &Path) -> Result<PullOutcome, CoreError> {
    let repo = Repository::open(vault_root)?;
    let mut remote = repo.find_remote("origin")?;
    let branch_name = current_branch_name(&repo)?;

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(remote_callbacks());
    remote.fetch(&[branch_name.as_str()], Some(&mut fetch_options), None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_up_to_date() {
        return Ok(PullOutcome {
            changed_paths: Vec::new(),
            conflict: false,
        });
    }

    if !analysis.is_fast_forward() {
        // Storie divergenti (o altro caso non fast-forward): mai un merge
        // automatico silenzioso.
        return Ok(PullOutcome {
            changed_paths: Vec::new(),
            conflict: true,
        });
    }

    let old_tree = repo.head()?.peel_to_tree()?;
    let refname = format!("refs/heads/{branch_name}");
    let mut local_ref = repo.find_reference(&refname)?;
    let message = format!("Fast-forward: {refname} -> {}", fetch_commit.id());
    local_ref.set_target(fetch_commit.id(), &message)?;
    repo.set_head(&refname)?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;

    let new_tree = repo.find_commit(fetch_commit.id())?.tree()?;
    let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;
    let changed_paths = diff
        .deltas()
        .filter_map(|delta| delta.new_file().path().or_else(|| delta.old_file().path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .map(|path| path.to_string_lossy().to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Ok(PullOutcome {
        changed_paths,
        conflict: false,
    })
}

/// Push del branch corrente su `origin`.
pub fn push(vault_root: &Path) -> Result<(), CoreError> {
    let repo = Repository::open(vault_root)?;
    let mut remote = repo.find_remote("origin")?;
    let branch_name = current_branch_name(&repo)?;
    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(remote_callbacks());
    remote.push(&[refspec.as_str()], Some(&mut push_options))?;
    Ok(())
}

fn current_branch_name(repo: &Repository) -> Result<String, CoreError> {
    Ok(repo.head()?.shorthand()?.to_string())
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
            state: SyncState::Idle,
        };
        assert!(!is_due(&status, 10, 100_000));
    }

    #[test]
    fn is_due_true_when_dirty_and_never_committed() {
        let status = SyncStatus {
            enabled: true,
            last_commit_at: None,
            dirty: true,
            state: SyncState::Idle,
        };
        assert!(is_due(&status, 10, 0));
    }

    #[test]
    fn is_due_false_when_dirty_but_interval_not_elapsed() {
        let status = SyncStatus {
            enabled: true,
            last_commit_at: Some(1_000),
            dirty: true,
            state: SyncState::Idle,
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
            state: SyncState::Idle,
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

        let before = status(dir.path(), SyncState::Idle).unwrap();
        assert!(before.enabled);
        assert!(!before.dirty);
        assert!(before.last_commit_at.is_some());
        // Nessun remote configurato: NoRemote vince sempre sull'hint.
        assert_eq!(before.state, SyncState::NoRemote);

        let vault = Vault::new(dir.path().to_path_buf());
        vault.ensure_exists().unwrap();
        vault
            .write_page("pages/uno.md", &[crate::block::Block::new("ciao")])
            .unwrap();
        let dirty_status = status(dir.path(), SyncState::Idle).unwrap();
        assert!(dirty_status.dirty);

        commit_if_dirty(dir.path()).unwrap();
        let after = status(dir.path(), SyncState::Idle).unwrap();
        assert!(!after.dirty);
        assert!(after.last_commit_at.is_some());
        assert!(after.last_commit_at >= before.last_commit_at);
    }

    #[test]
    fn status_on_non_git_vault_is_disabled_not_error() {
        let dir = TempDir::new("status-non-git");
        let result = status(dir.path(), SyncState::Idle).unwrap();
        assert_eq!(
            result,
            SyncStatus {
                enabled: false,
                last_commit_at: None,
                dirty: false,
                state: SyncState::Disabled,
            }
        );
    }

    /// Un repo Git locale che fa da "remote" per i test — un path assoluto
    /// è un URL valido per git2 tanto quanto uno `file://`, più semplice.
    fn init_bare_remote(dir: &Path) {
        Repository::init_bare(dir).unwrap();
    }

    /// Crea un repo locale, lo collega a `remote_dir` come origin, e
    /// pusha un primo commit — punto di partenza comune per i test di
    /// pull/push/conflitto: due "dispositivi" partono dallo stesso stato.
    fn init_repo_with_remote(dir: &Path, remote_dir: &Path) {
        init_repo(dir).unwrap();
        configure_identity(dir);
        commit_if_dirty(dir).unwrap(); // cattura .gitignore
        set_remote(dir, &remote_dir.to_string_lossy()).unwrap();
        push(dir).unwrap();
    }

    /// Un vero `git clone` — simula il flusso reale descritto nella spec
    /// per un secondo dispositivo: "un repo clonato dall'utente da fuori
    /// l'app e poi selezionato come vault", non un secondo `init_repo`
    /// (che creerebbe una history scollegata, mai il caso normale).
    fn checkout_from_remote(dir: &Path, remote_dir: &Path) {
        std::fs::remove_dir(dir).unwrap(); // clone vuole creare lui la cartella
        Repository::clone(&remote_dir.to_string_lossy(), dir).unwrap();
        configure_identity(dir);
    }

    #[test]
    fn has_remote_before_and_after_set_remote() {
        let dir = TempDir::new("has-remote");
        let remote_dir = TempDir::new("has-remote-origin");
        init_bare_remote(remote_dir.path());
        init_repo(dir.path()).unwrap();
        assert!(!has_remote(dir.path()));
        set_remote(dir.path(), &remote_dir.path().to_string_lossy()).unwrap();
        assert!(has_remote(dir.path()));
    }

    #[test]
    fn set_remote_twice_updates_url_instead_of_erroring() {
        let dir = TempDir::new("set-remote-twice");
        let remote_a = TempDir::new("set-remote-twice-a");
        let remote_b = TempDir::new("set-remote-twice-b");
        init_bare_remote(remote_a.path());
        init_bare_remote(remote_b.path());
        init_repo(dir.path()).unwrap();

        set_remote(dir.path(), &remote_a.path().to_string_lossy()).unwrap();
        set_remote(dir.path(), &remote_b.path().to_string_lossy()).unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        let remote = repo.find_remote("origin").unwrap();
        assert_eq!(
            remote.url().unwrap(),
            remote_b.path().to_string_lossy().as_ref()
        );
    }

    #[test]
    fn pull_fast_forward_updates_local_files() {
        let remote_dir = TempDir::new("pull-ff-origin");
        init_bare_remote(remote_dir.path());

        let device_a = TempDir::new("pull-ff-a");
        init_repo_with_remote(device_a.path(), remote_dir.path());

        // device_b clona subito, prima che device_a scriva altro: parte
        // allineato, così il prossimo pull ha esattamente un file da
        // scaricare, non zero.
        let device_b = TempDir::new("pull-ff-b");
        checkout_from_remote(device_b.path(), remote_dir.path());

        let vault_a = Vault::new(device_a.path().to_path_buf());
        vault_a
            .write_page("pages/uno.md", &[crate::block::Block::new("da device A")])
            .unwrap();
        commit_if_dirty(device_a.path()).unwrap();
        push(device_a.path()).unwrap();

        let outcome = pull(device_b.path()).unwrap();
        assert!(!outcome.conflict);
        assert_eq!(outcome.changed_paths, vec!["pages/uno.md".to_string()]);

        let content = fs::read_to_string(device_b.path().join("pages/uno.md")).unwrap();
        assert!(content.contains("da device A"));
    }

    #[test]
    fn pull_with_diverging_histories_reports_conflict_without_touching_files() {
        let remote_dir = TempDir::new("pull-conflict-origin");
        init_bare_remote(remote_dir.path());

        let device_a = TempDir::new("pull-conflict-a");
        init_repo_with_remote(device_a.path(), remote_dir.path());

        let device_b = TempDir::new("pull-conflict-b");
        checkout_from_remote(device_b.path(), remote_dir.path());

        // device_a scrive e pusha.
        let vault_a = Vault::new(device_a.path().to_path_buf());
        vault_a
            .write_page("pages/uno.md", &[crate::block::Block::new("da A")])
            .unwrap();
        commit_if_dirty(device_a.path()).unwrap();
        push(device_a.path()).unwrap();

        // device_b scrive qualcosa di diverso, senza aver ancora scaricato
        // la modifica di A: le due storie divergono.
        let vault_b = Vault::new(device_b.path().to_path_buf());
        vault_b
            .write_page("pages/due.md", &[crate::block::Block::new("da B")])
            .unwrap();
        commit_if_dirty(device_b.path()).unwrap();

        let before = fs::read_to_string(device_b.path().join("pages/due.md")).unwrap();
        let outcome = pull(device_b.path()).unwrap();
        assert!(outcome.conflict);
        assert!(outcome.changed_paths.is_empty());
        let after = fs::read_to_string(device_b.path().join("pages/due.md")).unwrap();
        assert_eq!(before, after, "un conflitto non deve toccare i file locali");
    }

    #[test]
    fn pull_up_to_date_is_a_noop() {
        let remote_dir = TempDir::new("pull-up-to-date-origin");
        init_bare_remote(remote_dir.path());
        let device_a = TempDir::new("pull-up-to-date-a");
        init_repo_with_remote(device_a.path(), remote_dir.path());

        let outcome = pull(device_a.path()).unwrap();
        assert!(!outcome.conflict);
        assert!(outcome.changed_paths.is_empty());
    }

    #[test]
    fn push_without_remote_is_an_error_not_a_panic() {
        let dir = TempDir::new("push-no-remote");
        init_repo(dir.path()).unwrap();
        configure_identity(dir.path());
        assert!(push(dir.path()).is_err());
    }

    /// Simula quello che l'utente farebbe manualmente da terminale per
    /// risolvere un conflitto (`git merge`) — mai codice di produzione:
    /// `pull()` non deve mai fare un merge automatico, questo helper
    /// rappresenta solo l'intervento esterno che la spec presuppone.
    /// Richiede `FETCH_HEAD` già popolato da un `pull()` precedente.
    fn resolve_conflict_with_merge_commit(dir: &Path) {
        let repo = Repository::open(dir).unwrap();
        let fetch_head = repo.find_reference("FETCH_HEAD").unwrap();
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head).unwrap();
        let local_commit = repo.head().unwrap().peel_to_commit().unwrap();
        let remote_commit = repo.find_commit(fetch_commit.id()).unwrap();

        let ancestor_oid = repo
            .merge_base(local_commit.id(), remote_commit.id())
            .unwrap();
        let ancestor_tree = repo.find_commit(ancestor_oid).unwrap().tree().unwrap();
        let mut merged_index = repo
            .merge_trees(
                &ancestor_tree,
                &local_commit.tree().unwrap(),
                &remote_commit.tree().unwrap(),
                None,
            )
            .unwrap();
        assert!(
            !merged_index.has_conflicts(),
            "setup del test: i due file non dovrebbero confliggere fra loro"
        );
        let result_tree = repo
            .find_tree(merged_index.write_tree_to(&repo).unwrap())
            .unwrap();

        let signature = repo.signature().unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "merge di risoluzione (simula un intervento manuale da terminale)",
            &result_tree,
            &[&local_commit, &remote_commit],
        )
        .unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();
    }

    #[test]
    fn conflict_clears_after_manual_resolution_and_push() {
        let remote_dir = TempDir::new("conflict-resolve-origin");
        init_bare_remote(remote_dir.path());

        let device_a = TempDir::new("conflict-resolve-a");
        init_repo_with_remote(device_a.path(), remote_dir.path());
        let device_b = TempDir::new("conflict-resolve-b");
        checkout_from_remote(device_b.path(), remote_dir.path());

        let vault_a = Vault::new(device_a.path().to_path_buf());
        vault_a
            .write_page("pages/uno.md", &[crate::block::Block::new("da A")])
            .unwrap();
        commit_if_dirty(device_a.path()).unwrap();
        push(device_a.path()).unwrap();

        let vault_b = Vault::new(device_b.path().to_path_buf());
        vault_b
            .write_page("pages/due.md", &[crate::block::Block::new("da B")])
            .unwrap();
        commit_if_dirty(device_b.path()).unwrap();

        assert!(pull(device_b.path()).unwrap().conflict);

        // Risoluzione manuale (fuori da Ramus) + push del merge: da qui in
        // poi B è di nuovo sincronizzabile.
        resolve_conflict_with_merge_commit(device_b.path());
        push(device_b.path()).unwrap();

        let after_resolution = pull(device_b.path()).unwrap();
        assert!(
            !after_resolution.conflict,
            "B ha già la history risolta: il prossimo pull deve tornare pulito"
        );

        // A, rimasto indietro, ora scarica il merge come un fast-forward
        // pulito — la risoluzione di B è visibile e utilizzabile da un
        // altro dispositivo, non solo localmente.
        let a_outcome = pull(device_a.path()).unwrap();
        assert!(!a_outcome.conflict);
        let content = fs::read_to_string(device_a.path().join("pages/due.md")).unwrap();
        assert!(content.contains("da B"));
    }
}
