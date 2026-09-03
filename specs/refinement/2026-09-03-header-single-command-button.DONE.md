# Header: un solo bottone icona (comandi), gear impostazioni rimosso

Stato: implementata. Terza spec della fase di refinement.

## Motivazione

L'header aveva tre bottoni icona: compact-toggle (»/«), comandi (⚡) e
impostazioni (⚙). Impostazioni è già raggiungibile dal badge di sync
nella status bar (`onClick={() => setActivePanel("settings")}`,
invariato) — un secondo punto d'accesso nell'header era ridondante.
In più ⚡ è un emoji a colori (il font di sistema lo renderizza sempre
arancio/giallo, `color` in CSS non lo tocca), diverso dagli altri
simboli di testo monocromi (« » ⚙) già in uso.

## Modifiche

`App.tsx`: rimosso il bottone "Impostazioni" (⚙) dall'header.
Sostituita l'icona del bottone comandi da ⚡ a ⌘ (simbolo di testo
monocromo, coerente con «/»/⚙, tematicamente più vicino a "comandi"
che a "energia"); aggiunto `title="Comandi"` per coerenza con
`compact-toggle` (unico altro bottone dell'header ad avere già un
tooltip). Header ora: logo, titolo, compact-toggle, un solo bottone
comandi.

Nessuna modifica CSS: `.settings-button` è già generica (usata da
tutti e tre i bottoni), e la regola
`.app-header.is-compact .settings-button:not(.compact-toggle)` che
nasconde i bottoni non-toggle in modalità compatta continua a
funzionare invariata con un solo bottone invece di due.

Impostazioni resta raggiungibile da: badge sync in status bar, azione
"Impostazioni" nella command palette (`onOpenSettings`, invariata).

## Fuori scope

Il commento sul polling di sync (`App.tsx`, "il badge nell'header
deve aggiornarsi...") è già impreciso da prima di questa spec (il
badge sync è nella status bar, non nell'header, dal refactor M4) — non
toccato, non introdotto da questa modifica.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust).
Non verificato con uno screenshot reale in questa sessione — verifica
visiva lasciata all'utente nell'app già aperta (`npm run tauri dev`
via hot reload).
