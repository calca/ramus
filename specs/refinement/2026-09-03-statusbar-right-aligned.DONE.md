# Status bar: solo a destra, non più a tutta larghezza

Stato: implementata. Sesta spec della fase di refinement, seguito di
`specs/refinement/2026-09-03-jump-to-date-command.DONE.md` (che ha
ridotto la status bar a soli due elementi: ⚙ Impostazioni e badge
sync). Una striscia a tutta larghezza con bordo divisore per due
piccoli bottoni non aveva più senso.

## Modifiche

`index.css`, `.app-statusbar`: `width: fit-content` +
`align-self: flex-end` (il genitore `.app` è una colonna flex, questo
è il modo esplicito di allineare un figlio all'estremità dell'asse
trasversale, invece di un fragile `margin-left: auto` implicito) al
posto di occupare l'intera larghezza. Rimosso `border-top`: il divisore
a tutta larghezza separava visivamente due sezioni distinte, ora è solo
un piccolo gruppo di bottoni che hanno già il proprio bordo — un bordo
esterno sarebbe stato ridondante. Rimossa anche la regola
`@media (max-width: 480px) .app-statusbar { flex-wrap: wrap; ... }`:
con `width: fit-content` e solo due icone non c'è più bisogno di andare
a capo nemmeno in finestre strette.

Nessuna modifica a `App.tsx`: stessa gerarchia, stesso posto nel flex
column, solo la larghezza/allineamento del box cambia.

## Fuori scope

Spostare la status bar altrove (es. in alto, o floating sopra il
contenuto invece che in normale flusso di documento): resta nella
stessa posizione in basso, solo più stretta.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust,
modifica solo CSS). Verifica visiva lasciata all'utente in `npm run
tauri dev` (hot reload).
