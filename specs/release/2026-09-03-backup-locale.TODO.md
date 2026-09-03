# Backup locale senza Git sync

Stato: proposta — bloccata su una decisione di prodotto (vedi
"Domande aperte"), non solo tecnica.

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

## Domande aperte (bloccanti)

1. **A, B, o entrambe?** Sono risposte diverse alla stessa domanda,
   non passi progressivi dello stesso lavoro.
2. Se B: **dove va la copia di backup** (dentro il vault stesso, es.
   `.ramus/backups/`, o fuori — `~/Library/Application
   Support/...`/equivalenti)? Dentro il vault rischia di essere
   sincronizzata anche lei se l'utente ha *anche* Git sync attivo
   (duplicazione), fuori è più pulito ma meno visibile/ispezionabile
   dall'utente.
3. Se B: **backup dell'intero vault o solo incrementale sui file
   toccati**? Un vault con anni di journal potrebbe diventare grande
   se ogni backup è una copia integrale.

## Fuori scope

- Un sistema di versionamento completo (stile Time Machine) dentro
  l'app: Git sync già copre quel bisogno per chi lo attiva; costruirne
  uno parallelo duplicherebbe la stessa funzione con un meccanismo
  diverso.
- Sync/backup su cloud proprietario: esplicitamente fuori scope in
  `SPEC.md` ("Sync proprietaria o account utente").

## Verifica

Non applicabile finché la spec resta bloccata. Nel frattempo, un
controllo economico e già possibile: verificare (lettura del codice
`git_sync.rs`, non un test nuovo) che la sync **locale-soltanto**
(nessun remote configurato) produca comunque una cronologia di commit
utilizzabile per recuperare una versione precedente di un file — se
sì, vale la pena dirlo esplicitamente nel testo di Impostazioni invece
di lasciarlo implicito, indipendentemente da come si risolvono le
domande sopra.
