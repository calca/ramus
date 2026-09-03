# Supporto mobile (Android/iOS) — fondamenta

Stato: proposta, in attesa di conferma — **tranne la sezione 1**
(refactor `Config::default_vault_path`/`config_file_path`), deciso in
`specs/M6/2026-09-03-impatti-milestone-precedenti.TODO.md` come task
da fare prima di iniziare M3, non insieme al resto di M6. Prima spec
di M6. Il secondo documento,
`specs/M6/2026-09-03-impatti-milestone-precedenti.TODO.md`,
cataloga cosa nelle spec di M1-M5 va adattato — questa si concentra
sulle fondamenta tecniche condivise da tutto il resto.

## Motivazione

SPEC.md, principio 2, lo anticipava fin da M1: "esposto in futuro via
FFI a un client mobile". Tauri v2 (già lo shell scelto per il
desktop) ha supporto mobile nativo integrato — stesso frontend
React/TypeScript, stesso `ramus-core`, stesso modello a command,
niente da riscrivere da zero. Non serve un client FFI separato come
il principio 2 lasciava intendere come possibilità: Tauri stesso è
già quel client.

## Non un nuovo target da FFI: Tauri mobile

`npm run tauri android init` / `npm run tauri ios init` generano
`src-tauri/gen/android` e `src-tauri/gen/apple` — progetti nativi
(Gradle/Xcode) che avvolgono lo stesso binario Rust compilato per
`aarch64-linux-android`/`aarch64-apple-ios` e la stessa webview
(Android System WebView / WKWebView) che già mostra la UI su
desktop. `tauri.conf.json` guadagna gli identificativi bundle
per entrambe le piattaforme.

## Tre problemi verificati, non ipotetici

Prima di scrivere qualunque riga di codice per M6, ho controllato
direttamente il sorgente delle dipendenze già in uso — non sono
supposizioni:

### 1. `dirs` non supporta Android in modo affidabile

`Config::default_vault_path()` e `Config::config_file_path()`
(`crates/ramus-core/src/config.rs`) chiamano `dirs::home_dir()`/
`dirs::config_dir()`. Verificato nel sorgente di `dirs 5.0.1`: la
crate distingue esplicitamente Windows, macOS+iOS (stesso modulo
`mac`), e "tutto il resto" tramite un unico modulo `lin` — **Android
non ha un proprio ramo**, ricade in quello Linux (`$HOME`/
`getpwuid_r`), che su Android non riflette un percorso realmente
utilizzabile dalla sandbox dell'app. Su iOS invece `dirs` gestisce
esplicitamente il caso (stesso codice di macOS, funziona).

**Fix**: Tauri v2 ha un proprio resolver di path, corretto su tutte
le piattaforme incluso Android (verificato:
`tauri::path` ha moduli dedicati `desktop.rs`/`android.rs`/`ios.rs`
con `app_config_dir()`/`app_data_dir()`/`document_dir()` propri).
**Impatto architetturale reale**: questo resolver vive
sull'`AppHandle` (`app.path().app_data_dir()`), un tipo Tauri —
`ramus-core` non può chiamarlo direttamente senza violare CLAUDE.md
regola 1 ("nessun `use tauri::` in quel crate"). Si risolve **senza**
violare la regola: `Config::default_vault_path()`/`config_file_path()`
smettono di calcolare il percorso da sole (smettono di dipendere da
`dirs`) e lo ricevono **come parametro**, iniettato dal chiamante —
`src-tauri` calcola il percorso giusto per piattaforma (via `dirs` su
desktop per non cambiare comportamento a chi già usa l'app, via
`app.path()` su mobile) e lo passa a `ramus-core`. Il crate resta
Tauri-free, cambia solo la firma di due funzioni — non un
compromesso, un miglioramento (`ramus-core` perde una dipendenza
diretta da `dirs`, il calcolo del percorso torna dov'è già la
decisione "che piattaforma è questa", cioè nel guscio).

### 2. Niente selettore di cartella su mobile

`pickVaultFolder`/`SettingsPanel` "Cambia" (M1) usa
`tauri-plugin-dialog`'s `blocking_pick_folder()`. Verificato nel
sorgente (`tauri-plugin-dialog 2.7.3/src/mobile.rs`, letto per
intero): il modulo mobile implementa solo `pick_file`/`pick_files`/
`save_file`/`show_message_dialog` — **nessun `pick_folder` esiste per
mobile**, né come sviste, proprio assente dall'API. Android e iOS non
hanno un concetto di "scegli una cartella qualsiasi del filesystem"
paragonabile a quello desktop (sandboxing).

**Conseguenza**: su mobile il vault **non è scelto dall'utente**, vive
in un percorso fisso deciso dall'app (vedi sotto). Il bottone "Cambia"
in `SettingsPanel` va nascosto su build mobile (`#[cfg(desktop)]`/
equivalente frontend — vedi
`specs/M6/2026-09-03-impatti-milestone-precedenti.TODO.md` per il
dettaglio).

### 3. Dove vive il vault su mobile: `document_dir()`, non `app_data_dir()`

Due opzioni offerte dal resolver di Tauri: `app_data_dir()` (sandbox
privata dell'app, invisibile ovunque altro) o `document_dir()`
(porzione "documenti" dell'app, visibile nell'app File su iOS se si
abilita `UIFileSharingEnabled`/`LSSupportsOpeningDocumentsInPlace`
nell'Info.plist, o nello storage condiviso su Android). SPEC.md
principio 4 ("il formato su disco è compatibile con Obsidian",
"le stesse note devono restare apribili... da altri editor markdown")
vale anche su mobile solo se il vault è **raggiungibile** da altre
app — `app_data_dir()` lo renderebbe invisibile a un editor Obsidian
mobile installato sullo stesso device, `document_dir()` no. Proposta:
`document_dir()` con le entitlement/permessi necessari.

## `notify` su iOS: funziona, ma con un fallback

Verificato nel sorgente di `notify 6.1.1`: Android usa lo stesso
backend nativo di Linux (`inotify`, funziona pienamente). iOS **non**
compare fra le piattaforme con backend nativo — ricade su
`PollWatcher` (polling periodico, non eventi del kernel). Più
latenza, più consumo batteria a parità di frequenza di polling
rispetto al desktop. Non bloccante (il file watcher su mobile serve
comunque a molto meno, un'app in background su iOS viene sospesa
quasi subito — vedi impatti su M3), ma da tenere a mente se il
polling va configurato con un intervallo diverso da quello desktop.

## Fuori scope per questa spec

- Notifiche push, badge sull'icona, o altre integrazioni OS-level
  mobile: non richieste.
- Autenticazione biometrica per aprire l'app: non richiesta, non
  esiste nemmeno su desktop.
- Ottimizzazioni specifiche di bundle size/tempo di avvio per mobile:
  si affronta se e quando risulta un problema reale, non
  preventivamente.

## Domande aperte

1. `document_dir()` (proposto, per restare "compatibile con
   Obsidian" anche su mobile) vs `app_data_dir()` (più privato, ma
   isola il vault da altre app) — confermi la proposta?
2. Percorso di `config.json` su mobile: `app_config_dir()` (proposto,
   dati di configurazione separati dal vault, coerente con
   l'organizzazione desktop dove config e vault sono già cartelle
   diverse) — confermi?

## Test da scrivere (core)

- `Config::default_vault_path`/`config_file_path` (dopo il refactor a
  parametro iniettato): un test che passa un path arbitrario e
  verifica che venga usato così com'è, senza logica di piattaforma
  residua dentro `ramus-core`.
- Nessun test possibile per il comportamento reale di
  `app.path()`/`notify` su Android/iOS in questo sandbox (richiede un
  device o un emulatore).

## Verifica

`cargo test -p ramus-core` per il refactor dei path. Il resto —
`tauri android init`/`tauri ios init`, build reale, avvio su
emulatore o device — non è verificabile in questo sandbox: richiede
Android Studio/Xcode installati e configurati, previsto per quando si
implementa davvero ("sviluppiamo poi" — questa sessione è solo di
progettazione, in viaggio).
