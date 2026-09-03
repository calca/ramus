# Sync Git — parte remota (pull, push, conflitti)

Stato: implementata. Presuppone
`specs/M3/2026-09-02-sync-git-locale.DONE.md` (repo Git attivo, commit
automatico). Uno scostamento dal testo originale: `SyncState`/
`SyncStatus::state` non sono ricalcolati da `git_sync::status` a ogni
chiamata — `Syncing`/`Conflict`/`Offline` non sono ricavabili
ispezionando solo il repository su disco (se diverge dal remote lo si
scopre solo provando un fetch), quindi vivono come stato di sessione
in `AppState` (`sync_network_state`) e vengono passati a `status` come
parametro `network_hint`. `Disabled`/`NoRemote` restano invece sempre
ricalcolati dal repository, il hint non li sovrascrive mai.

## Motivazione

Secondo e ultimo pezzo di M3 (SPEC.md): "Pull all'avvio, push a
intervallo" e "Stato della sync visibile nella UI; in caso di
conflitto, stop e avviso esplicito, mai merge automatico silenzioso".

## Configurazione del remote

Stessa sezione "Sync" di `SettingsPanel` (dal pezzo locale), visibile
quando il repo è attivo: un campo per impostare l'URL del remote
`origin` (`git2::Repository::remote("origin", url)`, o
`remote_set_url` se già esiste). Nessun account o OAuth gestito da
Ramus (SPEC.md Fuori scope: "Sync proprietaria o account utente")
— l'utente fornisce un URL già pronto (es.
`git@github.com:utente/journal.git`), autenticazione delegata
interamente al sistema:

- URL SSH → `Cred::ssh_key_from_agent(username)`: usa l'agente SSH già
  configurato sul sistema, nessuna chiave gestita da Ramus.
- URL HTTPS → `Cred::credential_helper`: usa il credential helper Git
  di sistema (Keychain su macOS, Credential Manager su Windows, ecc.),
  già configurato dall'utente se ha mai fatto `git push` da terminale.

Se l'autenticazione fallisce, l'errore risultante viene mostrato nello
stato di sync (vedi sotto) — nessun form per inserire password/token
dentro Ramus.

## Pull all'avvio, non bloccante

L'app apre il journal di oggi immediatamente, invariato (principio
"zero attrito", M1) — il pull gira in un task async avviato in
`setup()` **dopo** `app.manage(...)`, se il repo ha un remote
configurato. Non blocca né ritarda l'apertura della finestra.

Se il pull porta modifiche (fast-forward pulito — vedi "Conflitti"
sotto), i file cambiati vengono ricaricati esattamente come una
modifica esterna già gestita dal file watcher: stesso evento
`vault://file-changed`, stesso meccanismo già presente in `App.tsx`
("ricarica silenziosa se non dirty, avviso se dirty, mai
sovrascrivere") — **riusato**, non duplicato. `refresh_page` sui due
indici (SQLite, tantivy) per i file cambiati, stesso schema già usato
per le modifiche del watcher in `commands.rs`.

## Push a intervallo

Nessun intervallo separato da quello del commit (pezzo locale): dopo
un `commit_if_dirty` riuscito, se un remote è configurato, push
immediato nello stesso tick — ha senso spingere un commit appena
fatto, e un secondo intervallo configurabile aggiungerebbe
un'impostazione senza un vero bisogno (KISS, coerente con
"Domande aperte" della spec precedente sullo stesso timer).

Se il push fallisce (rete assente, remote irraggiungibile), lo stato
di sync riflette `Offline` (vedi sotto) — non blocca nulla, si riprova
al tick successivo. Un fallimento di rete non è un conflitto: la
history locale resta valida, semplicemente non è ancora arrivata al
remote.

## Conflitti — mai merge automatico silenzioso

Prima di applicare un pull, `Repository::merge_analysis` classifica la
situazione:

- **`FAST_FORWARD`**: si applica il fast-forward (`Reference::set_target`
  sul branch corrente + checkout). Nessun conflitto possibile per
  definizione — è la strada felice, quella attesa nella grande
  maggioranza dei casi per un vault personale sincronizzato da un solo
  dispositivo alla volta.
- **`NORMAL`** (richiederebbe un merge vero, storie divergenti): **non
  si tenta alcun merge**. Auto-commit e auto-push si fermano (non ha
  senso continuare a scrivere sopra una situazione che richiede
  intervento manuale) finché lo stato non torna risolvibile. La UI
  mostra un avviso esplicito e persistente — non un toast che
  scompare — con il path del vault e un suggerimento minimo
  ("apri un terminale nel vault e risolvi con git"). **Mai** un
  pulsante "risolvi automaticamente" o "sovrascrivi": né qui né altrove
  nell'app.

Uscita dallo stato di conflitto: rilevata al prossimo pull/tick quando
`merge_analysis` torna a indicare `FAST_FORWARD` (l'utente ha risolto
manualmente fuori dall'app, es. da terminale, e pushato). Nessun
pulsante "riprova adesso" in questa spec — si affida al prossimo tick
naturale (fuori scope, vedi sotto).

## Stato di sync esteso

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    Disabled,   // niente .git (dal pezzo locale)
    NoRemote,   // .git c'è, nessun remote configurato
    Idle,       // sincronizzato, nessuna operazione in corso
    Syncing,    // pull/commit/push in corso in questo momento
    Conflict,   // merge_analysis NORMAL, auto-sync fermo
    Offline,    // ultimo pull/push fallito per rete, riprova al prossimo tick
}
```

`SyncStatus` (dal pezzo locale) guadagna `pub state: SyncState`.
`get_sync_status()` (command già esistente) lo restituisce senza
cambiare firma esterna (solo il campo in più).

## Badge di stato

**Aggiornamento**: `specs/M4/2026-09-02-header-status-bar.TODO.md`
(M4) sposta la navigazione del journal dall'header a una nuova status
bar in basso, riservando lì lo spazio per questo badge — se M4 è già
implementata quando si costruisce questo pezzo, il badge va nella
status bar, non nell'header. Il resto di questa sezione descrive il
comportamento del badge, indipendente da dove vive fisicamente.

Icona/testo visibile solo quando `state` non è `Disabled`/`Idle`
(nessun rumore visivo quando tutto va bene, coerente con la sobrietà
generale — lo stato "tutto sincronizzato" non ha bisogno di essere
annunciato). `Conflict` usa lo stesso rosso di `.banner-error` (non
l'amber, riservato altrove — qui non è comunque un caso ambiguo, un
conflitto Git è un errore vero e proprio, non una notifica).
`Syncing`/`Offline` usano `--ramus-stone` (neutro, informativo, non un
errore). Click sul badge: apre `SettingsPanel` direttamente sulla
sezione Sync (stesso pannello, non un secondo posto dove guardare lo
stato).

## Fuori scope per questa spec

- Risoluzione conflitti dentro l'app (editor di merge, diff viewer):
  richiederebbe una superficie UI sproporzionata per un caso limite di
  un'app a singolo utente/dispositivo primario.
- Pulsante "riprova adesso" per uscire da `Conflict`/`Offline` senza
  aspettare il prossimo tick: piccola estensione futura, non
  essenziale (il tick è al massimo l'intervallo configurato, minuti,
  non ore).
- Sync multi-remote o branch diversi da quello corrente: un solo
  remote `origin`, un solo branch — quello checked-out al momento
  della prima apertura del vault (che sia stato creato da Ramus o
  clonato dall'utente fuori dall'app).
- Notifiche di sistema (desktop notification) per "pull ha portato
  modifiche" o "conflitto rilevato": solo UI in-app.
- Rebase invece di merge/fast-forward: mai, aumenterebbe la
  complessità dei casi di conflitto senza un beneficio chiaro per
  questo caso d'uso.

## Domande aperte

Nessuna: tutte e quattro confermate come proposto — remote sempre
`origin`, un solo branch mai scambiato da Ramus, badge `Conflict` nel
rosso di `.banner-error`, pull all'avvio silenzioso senza banner
informativo.

## Test da scrivere (core)

- Configurare un remote su un repo esistente, verificare che
  `find_remote("origin")` lo trovi.
- Pull con fast-forward pulito (due repo locali di test, uno "remote"
  simulato via `file://`) → i file locali riflettono il contenuto del
  remote, indici aggiornati.
- Pull con storie divergenti → `SyncState::Conflict`, nessun file
  modificato, nessun commit/push successivo finché lo stato non cambia.
- `commit_if_dirty` + push non eseguono nulla se `SyncState::Conflict`.
- Push senza remote configurato → `SyncState::NoRemote`, non un errore
  bloccante.
- Push con remote irraggiungibile (URL non valido) → `SyncState::Offline`,
  la history locale resta intatta.
- Dopo la risoluzione di un conflitto (simulata: la storia locale
  diventa un discendente valido del remote), il prossimo
  `merge_analysis` torna `FAST_FORWARD` → lo stato esce da `Conflict`.

## Verifica

`cargo test` copre repository/merge/stato per quanto simulabile con
repository `file://` locali di test (niente rete reale in sandbox). Non
testabile in questo sandbox: pull/push contro un vero remote (GitHub o
simili), credenziali SSH/HTTPS reali, il badge nell'header e il flusso
completo in `SettingsPanel` — serve un giro manuale in
`npm run tauri dev` con un vault collegato a un remote Git reale.
