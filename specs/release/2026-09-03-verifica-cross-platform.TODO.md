# Verifica su Windows e Linux

Stato: proposta — non una spec di implementazione, un elenco di punti
a rischio noto da verificare quando una macchina Windows/Linux reale
(o una VM) è disponibile. Nessuna macchina del genere in questa
sessione (solo macOS): questa spec cataloga cosa controllare, non lo
risolve.

## Motivazione

Ogni riga di codice, ogni verifica manuale e ogni screenshot di questa
intera sessione di lavoro sono avvenuti su macOS. `cfg!(windows)`
esiste in un punto del codice (`find_mcp_binary`, nome del binario con
`.exe`), e almeno una modifica recente si affida esplicitamente al
comportamento di **entrambi** i motori nativi di Tauri assumendolo
identico senza averlo mai verificato su Windows:

## Punti a rischio noti (dal codice, non ipotetici)

1. **Selettori CSS `-webkit-datetime-edit-*`** — non standard, non
   più in uso dopo `specs/refinement/2026-09-03-jump-to-date-command.DONE.md`
   (il date-picker nativo è stato rimosso, sostituito dalla command
   palette) — **rischio già eliminato**, righe qui solo per
   completezza dell'audit.
2. **`Iowan Old Style` (font serif dell'editor)**:
   `specs/refinement/2026-09-03-font-lettura.DONE.md` documenta già
   che esiste solo su macOS/iOS — su Windows/Linux lo stack scivola su
   Georgia o sul serif generico del sistema. Non un bug, ma
   **mai visto renderizzato**: verificare che il fallback sia
   effettivamente leggibile, non solo teoricamente presente nello
   stack.
3. **`titleBarStyle: "Overlay"` + `hiddenTitle: true`**
   (`tauri.conf.json`) — comportamento specifico di macOS (i "pallini"
   overlay che l'header lascia spazio per, `padding-left: 84px` in
   `index.css`); su Windows/Linux questo stile di titlebar non esiste
   allo stesso modo — **rischio concreto di un header con spazio
   vuoto a sinistra senza motivo** su quelle piattaforme, mai
   verificato.
4. **`find_mcp_binary()`**: usa `cfg!(windows)` per il nome del
   binario (`.exe`) — logica scritta ma mai eseguita su Windows vero.
5. **Scorciatoie da tastiera** (`lib/shortcut.ts`): `IS_MAC` rilevato
   via `navigator.platform`/`userAgent`, con fallback a `Ctrl` invece
   di `Cmd` — logica corretta a lettura del codice, mai premuta su una
   tastiera Windows/Linux vera.
6. **Percorsi di default del vault/config**
   (`src-tauri/src/lib.rs`, `resolve_default_vault_path`) — usa
   `dirs` per risolvere le cartelle standard per piattaforma
   (`~/Documents` o simili su macOS, `%USERPROFILE%\Documents` su
   Windows, `~/.config` per la config) — la libreria è affidabile, ma
   il risultato finale (dove l'app effettivamente crea `~/Journal` la
   prima volta) non è mai stato visto su Windows/Linux.

## Cosa fare quando una macchina è disponibile

Non una checklist di modifiche — un giro manuale reale: primo avvio
(creazione automatica del vault, nessun prompt — stesso criterio già
usato per il primo giro su macOS), aspetto dell'header/titlebar,
editor con grassetto/corsivo, command palette, tutte le scorciatoie
di `SHORTCUT_ACTIONS`. Ogni problema trovato diventa il proprio bugfix
puntuale (probabilmente in `specs/refinement/`), non serve una spec
preventiva per correzioni che non si sa ancora se serviranno.

## Fuori scope

- CI multi-piattaforma per l'esecuzione dei test unitari: già escluso
  esplicitamente in `specs/release/2026-09-03-ci.TODO.md` (i test non
  hanno bisogno di una matrice, il comportamento a rischio qui è tutto
  visivo/interattivo, non coperto da `cargo test`).
- Build automatiche multi-piattaforma: quella è
  `specs/release/2026-09-03-ci.TODO.md` (`release.yml`), un problema
  diverso dal "verificare che l'app funzioni" trattato qui.

## Verifica

Non applicabile in questa sessione — richiede hardware/VM non
disponibili qui. Questa spec stessa **è** l'unico output possibile
finché quella condizione non cambia.
