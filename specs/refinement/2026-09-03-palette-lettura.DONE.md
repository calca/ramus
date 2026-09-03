# Palette: ink/paper più morbidi per la lettura prolungata

Stato: implementata. Ottava spec della fase di refinement.

## Motivazione

Contrasto ink/paper originale: 15.4:1 (chiaro), 16.2:1 (scuro) —
entrambi ben oltre la soglia AAA (7:1). La letteratura tipografica
converge solo su due estremi da evitare (nero puro su bianco, bianco
quasi puro su nero — quest'ultimo produce un alone/halation percepito
più affaticante a parità di rapporto di contrasto), non su un unico
valore "ideale" oltre quello. Confrontate tre fasce (attuale/moderato/
morbido) in un artifact visivo con contenuto realistico (blocco
journal, link, tag, task) prima di scegliere — impossibile giudicare
in modo affidabile solo dai codici esadecimali in chat.

## Valori scelti: "morbido"

| Token | Chiaro (prima → dopo) | Scuro (prima → dopo) |
| --- | --- | --- |
| `--ramus-ink` | `#1C1A17` → `#3D372D` | `#F5F1E8` → `#D2CBB8` |
| `--ramus-paper` | `#F5F1E8` (invariato) | `#16150F` → `#241F17` |
| `--ramus-sap`, `--ramus-amber`, `--ramus-stone` | invariati | invariati |

Contrasto ink/paper risultante: **10.45:1** (chiaro), **10.11:1**
(scuro) — nella fascia comunemente citata come punto di equilibrio per
la lettura prolungata, ancora ben sopra AAA. Sap/amber/stone restano
agli stessi valori esadecimali: il loro contrasto scende leggermente
in scuro solo perché il fondo si è alzato (rispettivamente 4.89:1,
5.63:1, 7.62:1 contro il nuovo `--ramus-paper` scuro), tutti ben sopra
la soglia AA (4.5:1). Non si è scesi oltre i ~9:1 perché sotto quella
soglia il testo piccolo della UI (date, etichette, ~0.8rem) inizia a
perdere nitidezza — limite pratico, non arbitrario.

## Modifiche

`assets/palette.css`: aggiornati `--ramus-ink` (chiaro e scuro) e
`--ramus-paper` (solo scuro) nei tre punti dove sono definiti (`:root`
di base, `@media (prefers-color-scheme: dark)`, `:root[data-theme=
"dark"]`) — stessa struttura a tre blocchi già esistente, nessun nuovo
token, nessuna nuova regola. Nessun'altra modifica: essendo tutto il
resto dell'app già vincolato alle variabili CSS di `palette.css`
(CLAUDE.md, "colori solo via variabili CSS, mai hex inline"), l'intera
UI eredita i nuovi valori senza toccare altri file.

## Fuori scope

- Ritoccare `--ramus-sap`/`--ramus-amber`/`--ramus-stone`: nessuna
  richiesta in merito, restano gli stessi valori esadecimali.
- Dimensione/altezza di riga del testo (font-size, line-height): fuori
  dall'ambito di `palette.css`, la richiesta era sui temi colore.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust,
un solo file CSS toccato). Confronto visivo già fatto prima
dell'implementazione (artifact con tre fasce a confronto,
https://claude.ai/code/artifact/67e18b83-b689-4de8-acc4-5056ad37330e),
non da ripetere qui. Verifica finale nell'app reale lasciata
all'utente in `npm run tauri dev` (hot reload) su entrambi i temi.
