# Serif di sistema per il corpo dell'editor, chrome invariata

Stato: implementata. Nona spec della fase di refinement, seguito
diretto di `specs/refinement/2026-09-03-palette-lettura.DONE.md`
("ideale per la lettura a schermo e rilassante" copriva anche il
font, non solo il colore).

## Motivazione

Tutta l'app usava lo stesso stack sans-serif di sistema, bottoni e
testo scritto/letto compresi. Un serif pensato per la lettura lunga
rende il corpo del journal più "da libro", meno "da UI" — confrontato
in un artifact (sans vs serif, stessa palette morbida già in
produzione, entrambi i temi) prima di implementare:
https://claude.ai/code/artifact/0df88b89-4e4e-40d2-bef0-76aa49222066

## Modifiche

`src/index.css`, `.ramus-editor .ProseMirror` (unico punto:
`Editor.tsx` applica questa classe sia al journal sia alle pagine,
`Editor.tsx:98`): aggiunto

```css
font-family: "Iowan Old Style", Georgia, "Times New Roman", serif;
```

`"Iowan Old Style"` è il serif di lettura di Apple (Apple Books),
presente solo su macOS/iOS; su Windows scivola su Georgia (molto
diffuso), altrimenti sul serif generico del sistema — tutti font già
presenti, nessuna dipendenza nuova (CLAUDE.md, "nessuna dipendenza
nuova senza una riga di motivazione nel commit"). Il rendering non è
identico su ogni piattaforma: limite intrinseco dell'usare font di
sistema invece di scaricarne uno, accettato per restare offline-first
e senza il peso di un font web.

**Nessun'altra modifica**: header del giorno (`.journal-section-date`),
bottoni, Impostazioni, command palette — tutto il resto resta nello
stack sans-serif di `body` (invariato), che eredita per default. Solo
il testo che si scrive e si legge cambia.

## Fuori scope

- Font-size/line-height dell'editor: `line-height: 1.6` già adeguato
  (stessa regola, non toccata), nessuna richiesta sul corpo dimensione.
- Font della UI di sistema in generale (header, Impostazioni): restano
  sans-serif, mai in discussione.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust, un
solo file CSS). Confronto visivo già fatto prima dell'implementazione
(artifact sopra), non da ripetere. Verifica finale nell'app reale
lasciata all'utente in `npm run tauri dev` (hot reload), journal e
almeno una pagina, entrambi i temi.
