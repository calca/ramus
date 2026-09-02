# Sync Git — parte locale (commit automatico)

Stato: proposta, in attesa di conferma.

## Motivazione

Primo pezzo di M3 (SPEC.md): "Commit automatico su intervallo
configurabile". Funziona interamente offline, senza remote configurato
— una cronologia locale delle modifiche è già utile di per sé (rete di
sicurezza contro un errore di modifica/cancellazione), ed è il
fondamento su cui si appoggia il secondo pezzo (pull/push, spec
separata `specs/2026-09-02-sync-git-remoto.md`).

## Nuova dipendenza: `git2`

```toml
git2 = { version = "0.21", features = ["vendored-libgit2", "vendored-openssl"] }
```

Verificato contro `gitoxide` (`gix` 0.87.1, l'alternativa pura Rust):
nel sorgente del crate `gix` non esiste un `fn push` che esegua
l'operazione — solo `push_url`/`push_url_without_url_rewrite`, builder
per **configurare** l'URL di push, nessuna esecuzione. Serve
un'implementazione completa di push per il secondo pezzo di M3: `git2`
(binding a libgit2) ha `Remote::fetch`/`Remote::push` completi,
gestione credenziali (`RemoteCallbacks::credentials`,
`Cred::ssh_key_from_agent`, `Cred::credential_helper`, tutti verificati
nel sorgente) e `Repository::merge_analysis` per distinguere
fast-forward da conflitto — tutto quello che serve per entrambi i
pezzi di M3, in un'unica libreria matura e ampiamente usata in
produzione (es. Cargo stesso la usa).

`vendored-libgit2`/`vendored-openssl`: libgit2 compilato in-process,
nessuna libreria di sistema da richiedere all'utente su
macOS/Windows/Linux — stesso principio di `rusqlite`'s `bundled`
(SPEC.md Stack).

## Rilevamento del repository, non init forzato

All'apertura/cambio vault, Ramus verifica se `<vault>/.git` esiste:

- **Sì**: sync attivo — il vault è già sotto controllo versione
  (creato da Ramus in una sessione precedente, o un repo clonato
  dall'utente da fuori l'app e poi selezionato come vault).
- **No**: sync disattivo, nessuna UI di sync visibile (niente
  badge/bottone fantasma per una funzione che non fa nulla).

Nessun `git init` automatico e silenzioso: un vault non è
necessariamente destinato a finire sotto Git solo perché l'app lo
supporta — imporlo violerebbe l'aspettativa "l'app fa solo quello che
le è stato chiesto". L'attivazione è un'azione esplicita:
`SettingsPanel` guadagna una sezione "Sync", visibile sempre; quando
`.git` non esiste ancora mostra un solo bottone "Inizializza
repository Git" (chiama `git2::Repository::init(vault_root)` — nessuna
domanda, nessun remote in questo momento, quello è il secondo pezzo).

## `.gitignore` per `.ramus/`

Quando il sync viene attivato (init esplicito, o rilevamento di un
repo già esistente all'apertura del vault), Ramus scrive/aggiorna
`<vault>/.gitignore` aggiungendo la riga `.ramus/` se non già presente
(append a un file esistente, non sovrascrittura — altre regole
dell'utente restano intatte). Motivazione: `.ramus/` contiene indici
derivati e rigenerabili (SPEC.md principio 1, già il criterio usato
per tenerlo fuori da `list_pages`/`list_journals`) — versionarli non
ha senso, e i file binari SQLite/tantivy produrrebbero solo conflitti
di merge rumorosi a ogni sync senza alcun beneficio.

## Cosa viene committato

`git add -A` (l'intero vault, `.gitignore` escluso `.ramus/`) seguito
da un commit con messaggio fisso: `"Ramus: sync automatico —
{numero} file modificati"` (il numero viene da `Repository::statuses`,
già necessario per decidere se c'è qualcosa da committare). Nessuna
selezione granulare né messaggio descrittivo scritto a mano: un commit
automatico non ha un autore umano che possa descriverlo, e la
cronologia serve da rete di sicurezza, non da changelog leggibile.

Autore del commit: `Repository::signature()`, che legge `user.name`/
`user.email` dalla configurazione Git esistente dell'utente (globale o
locale al repo) — Ramus non chiede né memorizza questi dati (coerente
con "niente account utente", SPEC.md Fuori scope). Se mancano,
`signature()` fallisce: l'errore risultante (`CoreError`) viene
mostrato così com'è nella sezione Sync, senza un form per impostarli
(fuori scope, si presume `git config --global` già fatto una volta
fuori dall'app).

## Intervallo configurabile, timer lato Rust

Nuovo campo `Config::git_sync_interval_minutes: u32` (default: 10,
`#[serde(default = ...)]` stesso trattamento di `theme`/
`search_shortcut` per compatibilità con `config.json` precedenti),
impostabile nella sezione "Sync" di `SettingsPanel` (uno `<select>` con
poche opzioni fisse: 5, 10, 30, 60 minuti — non un campo libero,
coerente con la sobrietà del resto delle Impostazioni).

Il ticking va lato Rust, non lato frontend: deve scattare anche se la
finestra perde focus o il frontend è inattivo per qualunque motivo, e
non deve dipendere da un componente React montato. Primo caso in
Ramus di un task di background autonomo (finora solo il file watcher,
basato su callback di eventi, non su un timer):

```rust
// in setup(), dopo app.manage(AppState { .. })
let app_handle = app.handle().clone();
tauri::async_runtime::spawn(async move {
    let mut ticker = tokio::time::interval(Duration::from_secs(60));
    loop {
        ticker.tick().await;
        // Rilegge l'intervallo configurato a ogni tick (non catturato
        // una volta sola): un cambio in Impostazioni si applica al
        // prossimo tick, senza riavviare l'app o il task.
        // ... confronta tempo trascorso dall'ultimo commit con
        // l'intervallo configurato attuale, committa se serve.
    }
});
```

Tick fisso di 60 secondi (non l'intervallo configurato): il timer
interno resta lo stesso indipendentemente dal valore scelto
dall'utente, che viene solo confrontato ogni minuto — più semplice che
ricreare il task quando l'intervallo cambia.

## Stato di sync esposto

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub enabled: bool,
    pub last_commit_at: Option<i64>, // epoch secondi
    pub dirty: bool,                 // modifiche non committate ora
}
```

Command `get_sync_status() -> Result<SyncStatus, CoreError>`. In
questo primo pezzo nessun badge nell'header — solo la sezione Sync
delle Impostazioni lo interroga (polling leggero ogni 30s mentre il
pannello è aperto, si ferma alla chiusura). Un indicatore permanente
nell'header arriva col secondo pezzo, dove lo stato (incluso
"conflitto") è più rilevante per l'utente anche a pannello chiuso.

## Modulo nuovo: `ramus-core/src/git_sync.rs`

Funzioni libere, non un oggetto persistente in `AppState`: il repo si
apre e chiude a ogni operazione (`git2::Repository::open`), stesso
pattern già usato per `Vault::new(...)` costruito ad-hoc in ogni
command — nessuno stato Git da tenere in vita fra una chiamata e
l'altra.

```rust
pub fn is_git_repo(vault_root: &Path) -> bool { .. }

/// Crea il repository e aggiorna .gitignore. Idempotente: se il repo
/// esiste già, aggiorna solo .gitignore.
pub fn init_repo(vault_root: &Path) -> Result<(), CoreError> { .. }

/// Committa se ci sono modifiche non tracciate. Ok(false) se il
/// working tree è pulito (nessun commit vuoto).
pub fn commit_if_dirty(vault_root: &Path) -> Result<bool, CoreError> { .. }

pub fn status(vault_root: &Path) -> Result<SyncStatus, CoreError> { .. }
```

## Command Tauri

```
init_git_sync() -> Result<(), CoreError>
get_sync_status() -> Result<SyncStatus, CoreError>
set_git_sync_interval(minutes: u32) -> Result<Config, CoreError>
```

## Fuori scope per questa spec

- Pull/push, configurazione di un remote, rilevamento conflitti:
  secondo pezzo di M3, spec separata.
- Form per impostare `user.name`/`user.email` di Git da dentro Ramus.
- Cronologia/log dei commit visibile in UI (pannello "storico").
- Undo/revert di un commit da UI.
- Firma GPG dei commit.
- Escludere altri file dal vault oltre `.ramus/` (es. un `.gitignore`
  personalizzato dall'utente resta rispettato, ma Ramus non offre UI
  per modificarlo oltre garantire la riga `.ramus/`).

## Domande aperte

1. Intervallo di default proposto: **10 minuti**. Va bene?
2. Timer lato Rust con un task async avviato in `setup()` — primo caso
   di background task autonomo nell'app. Confermi questa direzione
   invece di, per esempio, un timer lato frontend che invoca un
   command a intervalli (più semplice ma si ferma se la finestra è in
   background/minimizzata a seconda del sistema operativo)?
3. Messaggio di commit fisso `"Ramus: sync automatico — N file
   modificati"` — sufficiente, o preferisci un formato diverso?

## Test da scrivere (core)

- `is_git_repo` su una cartella senza `.git` → `false`; dopo
  `init_repo` → `true`.
- `init_repo` è idempotente: chiamata due volte non fallisce, non
  duplica righe in `.gitignore`.
- `init_repo` su un repo Git già esistente (creato fuori da Ramus)
  aggiunge solo `.gitignore`, non tocca la history esistente.
- `init_repo` preserva regole `.gitignore` già presenti, aggiungendo
  solo `.ramus/` se assente.
- `commit_if_dirty` su un repo pulito (nessuna modifica) → `Ok(false)`,
  nessun commit creato.
- `commit_if_dirty` dopo aver scritto una pagina → `Ok(true)`, un
  nuovo commit in testa con tutti i file modificati.
- `commit_if_dirty` chiamata due volte di fila (seconda volta senza
  modifiche nel mezzo) → solo un commit, la seconda chiamata no-op.
- `status` riflette correttamente `dirty`/`last_commit_at` prima e
  dopo un `commit_if_dirty`.
- `status` su un vault senza `.git` → `enabled: false`, niente errore.

## Verifica

`cargo test` copre repository/commit/status. Non testabile in questo
sandbox: il task periodico reale (serve aspettare un tick, o un giro
manuale con l'intervallo abbassato temporaneamente) e l'interazione in
`SettingsPanel` — serve un giro manuale in `npm run tauri dev`.
