# Supporto mobile (Android/iOS) — fondamenta

Stato: implementata **solo per la sezione 1** (refactor
`Config::default_vault_path`/`config_file_path`, incluso il ramo
`#[cfg(mobile)]` — vedi "Cosa è stato implementato" sotto per il
dettaglio di cosa è verificato e cosa no). Il resto della spec (init
Tauri per Android/iOS, build reale, test su device/emulatore) resta
non implementato — richiede Android Studio/Xcode, non disponibili in
questo sandbox, esplicitamente rimandato a quando si costruisce
davvero il mobile.

Le due "Domande aperte" sono state confermate **con una scelta
diversa dal default proposto** per la prima — vedi sotto. Prima spec
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
le piattaforme incluso Android. **Aggiornamento**: verificato più a
fondo in fase di implementazione — `tauri::path` ha in realtà solo
due moduli, non tre come scritto sopra: `android.rs` (dedicato) e
`desktop.rs`, che copre **sia** desktop **sia** iOS (`#[cfg(not(
target_os = "android"))] mod desktop`, letto nel sorgente di
`tauri 2.11.5`) — nessun `ios.rs` a sé. Irrilevante per come Ramus
distingue i due casi (resta `cfg(desktop)`/`cfg(mobile)`, lo stesso
confine comportamentale — niente selettore di cartella, vault fisso —
vale per Android e iOS insieme), ma la spec originale sovrastimava la
struttura interna del resolver. `app_config_dir()`/`app_data_dir()`
esistono con la stessa firma (`fn(&self) -> Result<PathBuf>`) in
entrambi i moduli — l'API che Ramus consuma è identica sulle due
piattaforme.

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

### Cosa è stato implementato (e cosa no)

Le due funzioni statiche `Config::default_vault_path()`/
`config_file_path()` **spariscono del tutto** da `ramus-core` (non
solo cambiano firma): `Config::load_or_init(config_path: &Path,
default_vault_path: PathBuf)` li riceve entrambi già calcolati.
`Config` guadagna un campo privato `config_path: PathBuf` (`#[serde(
skip)]`, mai scritto nel file — un percorso salvato dentro se stesso
diventerebbe stale se il file venisse spostato), popolato da
`load`/`load_or_init` e riletto da tutti i setter esistenti
(`set_theme`, `set_vault_path`, ...) per sapere dove salvare, senza
dover cambiare anche la loro firma pubblica.

`src-tauri/src/lib.rs` guadagna
`resolve_default_vault_path`/`resolve_config_path`, ciascuna in due
varianti `#[cfg(desktop)]` (invariata, stesso `dirs` di sempre — path
esistenti sul disco degli utenti attuali restano identici, nessuna
migrazione richiesta) e `#[cfg(mobile)]` (nuova, `app.path()`).
`crates/ramus-mcp/src/main.rs` (desktop-only per costruzione, M5)
duplica solo il ramo desktop — stesso principio "copiato non
condiviso" già usato per la sequenza di sync degli indici. `dirs`
esce da `ramus-core/Cargo.toml`, entra in `src-tauri/Cargo.toml` e
`crates/ramus-mcp/Cargo.toml`.

**Non implementato, non verificabile in questo sandbox**: il ramo
`#[cfg(mobile)]` non è mai stato compilato per davvero — nessun
target Android/iOS installato qui, e `#[cfg(mobile)]` esclude quel
codice da qualunque build desktop (`cargo check`/`clippy` non lo
vedono nemmeno). Scritto seguendo l'API verificata nel sorgente di
`tauri 2.11.5` (nomi di metodo, firme, moduli — vedi sopra), non
seguendo un esempio compilato. `tauri android init`/`tauri ios init`,
`tauri.conf.json` con gli identificativi bundle, e build reale su
device/emulatore: non toccati, restano com'erano nel testo originale
("sviluppiamo poi").

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

### 3. Dove vive il vault su mobile: `app_data_dir()` (confermato,
non la proposta originale)

Due opzioni offerte dal resolver di Tauri: `app_data_dir()` (sandbox
privata dell'app, invisibile ovunque altro) o `document_dir()`
(porzione "documenti" dell'app, visibile nell'app File su iOS se si
abilita `UIFileSharingEnabled`/`LSSupportsOpeningDocumentsInPlace`
nell'Info.plist, o nello storage condiviso su Android). Il testo
originale proponeva `document_dir()` per restare coerenti con SPEC.md
principio 4 ("il formato su disco è compatibile con Obsidian") anche
su mobile — **in conferma, scelto invece `app_data_dir()`**: più
semplice (nessun entitlement/permesso da configurare e mantenere per
due piattaforme), al costo esplicitamente accettato di isolare il
vault da altri editor markdown mobile sullo stesso device — la
compatibilità con Obsidian resta piena su desktop, non si estende a
mobile in questa iterazione. Implementato in
`src-tauri/src/lib.rs::resolve_default_vault_path` (ramo
`#[cfg(mobile)]`).

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

Nessuna: entrambe risolte — 1) `app_data_dir()` scelto invece della
proposta originale `document_dir()` (vedi sezione 3, "Cosa è stato
implementato" per il perché); 2) `app_config_dir()` confermato come
proposto per `config.json` su mobile.

## Test da scrivere (core)

Tre test nuovi in `crates/ramus-core/src/config.rs` (oltre a quelli
già esistenti, tutti aggiornati per la nuova firma):

- `load_or_init` su un file mancante usa il `default_vault_path`
  iniettato così com'è, senza logica di piattaforma residua dentro
  `ramus-core`.
- `load_or_init` su un file già esistente **ignora** il default
  iniettato (il valore su disco vince, coerente col comportamento
  pre-refactor).
- Un setter (`set_theme`) persiste sullo stesso `config_path` da cui
  la configurazione è stata caricata — verifica che il campo
  `#[serde(skip)]` sopravviva al giro `load_or_init` → mutazione →
  rilettura.

Rimosso `default_vault_path_is_under_home` (testava la funzione
statica ora rimossa). Nessun test possibile per il comportamento
reale di `app.path()`/`notify` su Android/iOS in questo sandbox
(richiede un device o un emulatore) — il ramo `#[cfg(mobile)]` non è
mai stato compilato qui, vedi sopra.

## Verifica

`cargo test` sull'intero workspace (117 core + 15 mcp, tutti puliti),
`cargo clippy --all-targets -D warnings`, `cargo fmt --check`, `npm
run typecheck` — solo il ramo `#[cfg(desktop)]` è stato
effettivamente compilato/testato (`cfg(mobile)` è escluso a priori da
una build desktop, non un test che ha "fallito silenziosamente": non
è mai stato eseguito). Il resto — `tauri android init`/`tauri ios
init`, build reale, avvio su emulatore o device — non è verificabile
in questo sandbox: richiede Android Studio/Xcode installati e
configurati, previsto per quando si implementa davvero.
