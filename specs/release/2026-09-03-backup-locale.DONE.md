# Backup locale senza Git sync

Stato: direzione **A** (spingere su Git sync) implementata. Direzione
**B** (meccanismo di backup locale indipendente da Git) esplicitamente
rimandata a una decisione successiva — non scartata, solo non presa
ora.

## Motivazione

Oggi l'unica protezione contro una scrittura sbagliata o un bug che
corrompe un file è la sync Git (M3), **opzionale e disattivata di
default**. Un utente che non l'ha attivata non ha alcuna rete di
sicurezza oltre a backup a livello di sistema operativo (Time
Machine, File History) — se esistono e sono configurati, cosa che
Ramus non può assumere. "I file markdown sono la source of truth"
(CLAUDE.md, regola 4) rende questo rischio diretto: un bug nel
codice di scrittura tocca l'unica copia.

## Due direzioni possibili, non equivalenti

**A. Spingere di più verso Git sync** — non un backup nuovo, rendere
più visibile/incoraggiato quello che già esiste. Costo quasi nullo
(un banner o un suggerimento in Impostazioni), ma **Git sync richiede
un repository remoto per essere utile davvero** (locale-soltanto,
menzionato nel testo di Impostazioni, protegge comunque da una singola
scrittura corrotta grazie alla cronologia dei commit — vale la pena
verificarlo esplicitamente, vedi "Verifica").

**B. Un meccanismo di backup locale indipendente** — es. una copia
timestampata del vault (o solo dei file modificati) in una cartella
separata, con rotazione (tenere le ultime N). Protegge anche chi non
vuole/non può configurare Git. Costo più alto: nuova logica in
`ramus-core` (dove altrimenti "i file markdown sono la source of
truth" resterebbe vero solo a metà — la copia di backup diventerebbe
una seconda fonte, con tutte le domande su quando/come si sincronizza
che questo comporta).

## Modifiche (direzione A)

`SettingsPanel.tsx`, testo introduttivo della sezione Sync: da
"Versiona il vault con Git. Lascia il campo vuoto per una cronologia
solo locale..." (neutro, un'opzione fra tante) a una frase che spiega
esplicitamente il beneficio — proteggere da una scrittura andata male
anche senza un repository remoto, nessun account/servizio esterno
richiesto. Verificato che l'affermazione sia vera prima di scriverla,
non solo plausibile: `commit_if_dirty` (`git_sync.rs`) gira a ogni
ciclo di sync indipendentemente da un remote configurato
(`src-tauri/src/commands.rs`, il ciclo in background chiama
`commit_if_dirty` sempre, `push` solo `if git_sync::has_remote(...)`)
— la cronologia locale si costruisce per davvero, non solo in teoria.

Nessuna modifica a `ramus-core`: il comportamento esisteva già (M3),
solo comunicato meglio.

## Domande aperte (rimandate, non bloccanti)

1. **Costruire anche B** (meccanismo di backup locale indipendente da
   Git)? Deciso di rimandare — non "no", solo non deciso ora.
2. Se in futuro si sceglie B: dove va la copia di backup (dentro il
   vault, es. `.ramus/backups/`, o fuori — vedi il confronto sotto in
   "Due direzioni possibili") e con quale rotazione (integrale o
   incrementale) — ancora aperte se/quando si riprende in mano.

## Fuori scope

- Un sistema di versionamento completo (stile Time Machine) dentro
  l'app: Git sync già copre quel bisogno per chi lo attiva; costruirne
  uno parallelo duplicherebbe la stessa funzione con un meccanismo
  diverso.
- Sync/backup su cloud proprietario: esplicitamente fuori scope in
  `SPEC.md` ("Sync proprietaria o account utente").

## Verifica

`npm run typecheck` pulito (solo JSX toccato). L'affermazione chiave
del nuovo testo — la sync locale-soltanto protegge comunque grazie
alla cronologia dei commit — verificata leggendo `git_sync.rs` e il
ciclo di sync in `src-tauri/src/commands.rs` (non un test nuovo: il
comportamento esisteva già, verificato che sia davvero quello prima
di scriverlo in UI). Verifica visiva del nuovo testo lasciata
all'utente in `npm run tauri dev` (hot reload), sezione Sync.
