# CLAUDE.md

Ramus — app desktop di journaling, outliner a blocchi su file markdown locali.
Tauri v2 + Rust + React/TypeScript.

La specifica completa è in `SPEC.md`. Leggila prima di modifiche non banali.
Questo file contiene solo le convenzioni operative.

## Comandi

```bash
npm install                 # dipendenze frontend
npm run tauri dev           # avvia l'app in sviluppo
npm run tauri build         # build di release
npm run typecheck           # tsc --noEmit
cargo test -p ramus-core    # test del core (i più importanti)
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

Prima di dichiarare finito un task: `cargo test`, `cargo clippy` e
`npm run typecheck` devono passare tutti e tre.

## Dove sta cosa

| Percorso | Contenuto |
| --- | --- |
| `crates/ramus-core/` | logica pura: parser, modello a blocchi, vault, indice |
| `src-tauri/src/commands.rs` | command Tauri — solo wrapper sottili sul core |
| `src/editor/` | Tiptap: estensioni, serializer, deserializer |
| `src/lib/` | wrapper tipizzati che chiamano i command |
| `assets/` | logo e palette |

## Regole non negoziabili

1. **`ramus-core` non dipende da Tauri.** Nessun `use tauri::` in quel crate,
   nessun tipo Tauri nelle sue firme. Deve compilare e testarsi da solo.
2. **La logica sta nel core, non nei command.** Un command che contiene più di
   una decisione va rifattorizzato spostando la logica in `ramus-core`.
3. **Il frontend non tocca il filesystem** e non conosce percorsi assoluti:
   passa solo path relativi al vault.
4. **I file markdown sono la source of truth.** Indice e cache sono derivati e
   rigenerabili: se il codice non funziona dopo aver cancellato l'indice, è rotto.
5. **Round-trip del parser garantito.** `parse(render(page)) == page`. Ogni
   modifica al parser o al serializer richiede un test che lo verifichi.
6. **Formato su disco fisso**: un blocco per riga, `- ` come prefisso,
   due spazi per livello di annidamento, newline finale. Vedi `SPEC.md`.
7. **Colori solo via variabili CSS** di `assets/palette.css`. Mai hex inline.
8. **Nessuna dipendenza nuova** senza una riga di motivazione nel commit.
   In particolare: niente librerie markdown per Tiptap, la serializzazione
   è scritta a mano di proposito.

## Stile di codice

- Rust: errori con `thiserror`, mai `unwrap()` o `expect()` nei command e nel
  codice di produzione del core. Nei test è ammesso.
- TypeScript: `strict` attivo, niente `any`. I tipi dei command stanno in
  `src/lib/types.ts` e devono rispecchiare le struct Rust.
- Test del parser e del serializer prima di collegare la UI, non dopo.
- Commit convenzionali: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`.

## Come lavorare sui task

- Rispetta le milestone di `SPEC.md`. Non anticipare funzionalità di M2 o M3
  mentre M1 è aperta, nemmeno come "predisposizione".
- La sezione "Fuori scope" dello spec è vincolante: se un task sembra
  richiedere qualcosa che è lì dentro, fermati e chiedi.
- Modifiche al formato su disco o al modello dati vanno proposte prima di
  essere implementate: rompono la compatibilità con i vault esistenti.
- Per un task non banale, scrivi prima una spec in `specs/`, aspetta
  conferma o correzioni, implementa solo dopo un via libera esplicito.
  Convenzioni di nome, cartella e stato: vedi `specs/README.md`.
