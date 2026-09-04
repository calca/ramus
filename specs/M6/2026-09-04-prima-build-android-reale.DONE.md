# Prima build Android reale

Stato: implementata/verificata. Completa la parte di M6 esplicitamente
lasciata aperta da
`specs/M6/2026-09-03-supporto-mobile-fondamenta.DONE.md` ("build reale
... resta da fare") — la prima volta che `tauri android init` e una
build vera vengono eseguiti per Ramus, non solo progettati.

## Ambiente (per riferimento futuro, non da rifare)

Android SDK/NDK/Android Studio già installati sulla macchina
(`~/Library/Android/sdk`, NDK `28.2.13676358`, Gradle 8.5 via
Homebrew, Java 17). Aggiunti i quattro target Rust per Android:
```
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

## `tauri android init`

Eseguito senza modifiche al codice, ha generato
`src-tauri/gen/android/` (progetto Android Studio reale — Gradle,
Kotlin, manifest, risorse). **Committato**: il `.gitignore` che Tauri
scaffolda dentro `gen/android/` esclude già correttamente `build/`,
`.gradle/`, `local.properties`, `key.properties`/`keystore.properties`
(credenziali di firma) — solo 40 file/932 righe di sorgente entrano
nel repo, non gigabyte di build artifact. Icone del launcher ancora
quelle generiche di Tauri (`ic_launcher*.png`), non il logo Ramus —
lasciato così, fuori scope qui (vedi "Fuori scope").

## Bug reale trovato: `openssl-sys` non cross-compila con NDK 28

Primo tentativo di build fallito **non per codice Ramus**: `git2` (Git
sync, M3) porta `openssl-sys`, il cui build script cerca il binario
`aarch64-linux-android-ranlib` — convenzione GNU-binutils che NDK r23+
non fornisce più (solo `llvm-ar`/`llvm-ranlib` unversionati). Fix,
**nessuna modifica al codice o all'NDK di sistema**: variabili
d'ambiente che la crate `cc` (usata dal build script di `openssl-src`)
rispetta per-target, impostate a runtime prima della build:
```bash
export AR_aarch64_linux_android="$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
export RANLIB_aarch64_linux_android="$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"
# stesso pattern per armv7_linux_androideabi, i686_linux_android, x86_64_linux_android
```
Necessario per qualunque build Android futura (locale o CI) finché il
progetto userà NDK r23+ — da portare nella spec della pipeline CI,
non risolvibile una volta per tutte nel codice del progetto (sono
variabili d'ambiente della shell che invoca la build, non
configurazione di `Cargo.toml`).

**`tantivy` (ricerca full-text, M2)**: era segnato "da verificare, non
dato per scontato" in
`specs/M6/2026-09-03-impatti-milestone-precedenti.DONE.md` — **verificato
per davvero qui**: compila pulito per `aarch64-linux-android`, nessun
problema. **SQLite (`rusqlite` bundled)**: confermato anche lui,
nessuna sorpresa.

## Bug reale trovato: `pick_vault_folder` non esiste su mobile

Secondo (e ultimo) errore di compilazione, questo sì nel codice di
Ramus: `src-tauri/src/commands.rs`, `pick_vault_folder` chiamava
`app.dialog().file().blocking_pick_folder()` — quel metodo non esiste
nell'API del plugin dialog per target non-desktop (mobile ha solo
`blocking_pick_file`, niente selettore di cartelle). Non una sorpresa
di design: `specs/M6/2026-09-03-supporto-mobile-fondamenta.DONE.md`
aveva già deciso "nessun selettore di cartella su mobile, il vault
vive in un percorso fisso" — mancava solo di tradurlo in codice che
compila.

**Fix minimo, non un redesign della UI mobile**: il comando resta
registrato su entrambe le piattaforme (nessuna modifica a
`generate_handler!` né al frontend, che già tratta `None` come
"annullato, nessuna modifica" — vedi `SettingsPanel.tsx`), ma il corpo
si divide con `#[cfg(desktop)]`/`#[cfg(mobile)]`: su mobile ritorna
sempre `Ok(None)`, cioè si comporta come se l'utente avesse annullato.
Il bottone "Cambia" resta visibile e cliccabile su mobile ma non fa
nulla — **non** l'esperienza finale desiderata (andrebbe nascosto o
sostituito su mobile), ma sufficiente per una build che compila e
un'app che non crasha; la UI mobile del vault picker è lavoro a sé,
non fatto qui (vedi "Fuori scope"). `use tauri_plugin_dialog::DialogExt`
spostato dietro lo stesso `#[cfg(desktop)]` (altrimenti import inutilizzato
→ warning → fallimento di `clippy -D warnings` su mobile).

## Risultato

APK di debug reale prodotto e verificato (non solo "il comando è
uscito con 0" — controllato che il file esista, `unzip`/`file`
confermano un archivio zip valido):
```
src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```
via:
```bash
npx tauri android build --debug --target aarch64 --apk
```
(solo `aarch64`, non tutti e quattro gli ABI — sufficiente per
verificare che il codice compili e produca un artefatto reale; una
build "universale" per tutti gli ABI è più lenta e non aggiunge
informazione per questo scopo).

## Fuori scope

- UI mobile del vault picker (nascondere/sostituire il bottone
  "Cambia" su mobile invece di renderlo un no-op silenzioso): richiede
  un modo per il frontend di sapere "sono su mobile" (non esiste
  ancora, es. un campo in `Config` o `@tauri-apps/api/os`) — lavoro
  vero, non fatto qui.
- Firma reale dell'APK per una release (oggi solo `--debug`, chiave di
  debug automatica di Android): dipende dalla stessa decisione già
  aperta in `specs/release/2026-09-03-firma-notarizzazione.TODO.md`
  (nessun Apple Developer ID ancora — lì il discorso era su
  macOS/Windows, ma la stessa domanda "vuoi investire in certificati di
  firma" vale anche per il Play Store).
- Test su device/emulatore reale (solo build verificata, mai avviata):
  questo sandbox non ha un emulatore Android configurato né un device
  fisico collegato.
- Icone del launcher Ramus-brandizzate al posto del placeholder Tauri.
- iOS: non toccato in questa spec, stesso "resta da fare" di prima —
  richiede una macchina macOS con Xcode configurato per il code
  signing, non tentato qui.
- Pipeline CI per automatizzare questa build: prossimo passo naturale,
  spec a parte (la domanda originale dell'utente — "una pipeline per
  apk android?" — ha portato a questa spec prima, per avere una
  ricetta verificata da automatizzare invece di scriverne una a
  scatola chiusa).

## Verifica

`cargo test`, `cargo clippy --all-targets -D warnings`, `cargo fmt
--check`, `npm run typecheck`, `npm run test` — tutti puliti su
desktop dopo il fix di `pick_vault_folder` (verificato che non abbia
rotto nulla lì, `cargo check -p ramus` pulito separatamente prima
della checklist completa). Build Android **eseguita per davvero, non
solo letta**: due tentativi, il primo fallito su `openssl-sys` (causa
diagnosticata leggendo il sorgente di `openssl-src` nella cache
cargo, non indovinata), il secondo fallito su `pick_vault_folder`
(causa ovvia dal messaggio del compilatore), il terzo riuscito con
APK reale prodotto e ispezionato su disco.
