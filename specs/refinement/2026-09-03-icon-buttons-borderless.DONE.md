# Decluttering: bottoni icona senza bordo (status bar + header)

Stato: implementata. Settima spec della fase di refinement, seguito
di `specs/refinement/2026-09-03-statusbar-right-aligned.DONE.md`.
Ambito confermato dall'utente: sia status bar (⚙, badge sync) sia
header (⌘ comandi, toggle compatto «/»).

## Modifiche

`index.css`: `border` rimosso da `.statusbar-icon-button` e
`.settings-button` (quest'ultima usata sia dal bottone ⌘ sia dal
toggle compatto, che la eredita via `className="settings-button
compact-toggle"` — nessuna modifica separata necessaria per lui).
Aggiunto `:hover { background: color-mix(in srgb, var(--ramus-stone)
12%, transparent); }` su entrambe, stesso pattern già in uso per i
bottoni della sidebar di Impostazioni (`settings-sidebar
button:hover`) — senza bordo a riposo serve comunque un segnale che
sono cliccabili, e uno sfondo leggero all'hover lo dà senza
reintrodurre un bordo permanente.

`.sync-badge.is-conflict` perde `border-color` (non aveva più nulla da
colorare, il bordo di `.statusbar-icon-button` da cui ereditava è
sparito): resta solo il `color` rosso sull'icona ⚠, già sufficiente a
distinguere lo stato di conflitto.

## Fuori scope

Bottoni in altri contesti (Impostazioni, command palette, editor):
restano com'erano, non menzionati da questa richiesta.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust,
solo CSS). Verifica visiva lasciata all'utente in `npm run tauri dev`
(hot reload).
