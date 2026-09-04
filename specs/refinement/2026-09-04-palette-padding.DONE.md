# Command palette: padding mancante e input più moderno

Stato: implementata. Bug segnalato dall'utente via screenshot (il
campo di ricerca toccava i bordi del pannello) più la richiesta di
rendere l'input "innovativo".

## Causa del bug

Regressione da `specs/refinement/2026-09-03-settings-sidebar.DONE.md`:
prima di quella spec `.settings-panel` aveva il proprio padding
diretto; la sidebar l'ha spostato su `.settings-panel-header`/
`.settings-content`, entrambi usati solo da `SettingsPanel`.
`CommandPalette` non ha né header né sidebar — renderizza solo
`<input>` + `<ul>` direttamente dentro `.settings-panel` — quindi ha
perso ogni padding senza che nessuno se ne accorgesse finché non è
comparso uno screenshot.

Stessa spec aveva anche reso `.settings-panel` ad **altezza fissa**
(`height: min(36rem, ...)`, per evitare che Impostazioni "saltasse"
cambiando tab) — corretto per quel caso, ma applicato per errore anche
alla palette, che restava sempre alta 36rem lasciando spazio vuoto
sotto quando i risultati erano pochi (visibile nello screenshot
dell'utente, sotto "Mostra scorciatoie").

## Modifiche

- `Modal.tsx`: nuova prop opzionale `panelClassName`, aggiunta come
  classe in più su `.settings-panel` invece di introdurre un
  componente separato — stessa meccanica (backdrop, Escape, click
  fuori) condivisa, solo l'aspetto del pannello cambia.
- `CommandPalette.tsx`: `panelClassName="palette-panel"`.
- `index.css`, nuova regola `.palette-panel`: `height: auto` (non più
  fissa — si restringe/allarga col numero di risultati, come un
  command palette vero) con `max-height: min(28rem, calc(100vh -
  4rem))` come tetto di sicurezza, più il padding che mancava.
- `.palette-input` ridisegnato: da riquadro con bordo pieno a sola
  riga sotto (`border-bottom`), niente sfondo, font leggermente più
  grande — stessa direzione già presa per i bottoni icona di
  header/status bar in questa fase di refinement (via bordo
  eliminato), più vicino a Raycast/Spotlight di un input in scatola.
  Risposta alla parte aperta della richiesta ("qualcosa di
  innovativo") senza inventare funzionalità nuove, solo trattamento
  visivo.
- `.palette-empty`: rimosso il padding orizzontale proprio, ridondante
  ora che `.palette-panel` fornisce già l'inset.

## Scoperta collaterale, non toccata qui

`Cheatsheet.tsx` usa `.settings-section` **direttamente** dentro
`.settings-panel` (niente `.settings-content` di mezzo) — stesso
meccanismo che ha lasciato la palette senza padding potrebbe
riguardare anche lui (`.settings-section` non ha padding proprio,
solo `.settings-content` ce l'ha). Non confermato con uno screenshot
reale, non toccato in questa spec per non allargare il raggio
d'azione oltre al bug segnalato — se risultasse rotto anche lì, stessa
spec a parte.

## Fuori scope

- Icone per tipo di risultato (Azioni/Recenti/Risultati/Data/Crea) o
  una riga di suggerimenti tastiera (↑↓ · ↵ · esc) in fondo: idee
  scambiate ma non richieste esplicitamente — proposte per un giro
  successivo se interessano.
- Toccare `.settings-panel`/`.settings-content`/`.settings-section`
  condivise: il fix resta additivo (`.palette-panel` in più, non
  modifiche alle regole esistenti), zero rischio per Impostazioni già
  verificata.

## Rifinitura dopo la prima verifica

Confermato via screenshot: bene, tranne lo spazio fra la riga
dell'input e "RECENTI" troppo stretto. `.palette-results` da `margin:
0.75rem 0 0` a `1.25rem 0 0` (stesso ritmo verticale da 1.25rem già
in uso altrove nell'app).

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check`, `npm run test` (77 test) — tutti
puliti. Verifica visiva lasciata all'utente in `npm run tauri dev`
(hot reload, dev server già rilanciato in questa sessione).
