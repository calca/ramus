# Header compatto + status bar in basso

Stato: implementata. Le tre "Domande aperte" sono state confermate
come proposto — titolo "Ramus" mantenuto nell'header, status bar
~2rem/0.8rem/`--ramus-stone` come da proposta, nascosta interamente in
modalità compatta. Un solo scostamento: il badge di sync (M3, già
implementato quando si è arrivati a questa spec, non solo "riservato"
come previsto qui) è stato spostato nella status bar insieme a
`JournalControls`, non lasciato nell'header — coerente col resto della
spec ("tutto il resto si sposta in basso").

## Motivazione

Primo pezzo di M4 (SPEC.md, "UI" — rinominata da questa spec: era "AI",
spostata a M5, vedi sotto). Idea raccolta durante l'uso reale
dell'app: l'header oggi accumula troppi elementi eterogenei (branding,
navigazione del journal, azioni app-level) nella stessa striscia in
alto — meno "distraction free" di quanto l'editor sotto già sia.

## Rinumerazione milestone

`SPEC.md`: M4 diventa "UI" (rifinitura interfaccia, non legata a una
singola feature), l'AI (vecchia M4) diventa M5, contenuto invariato.

## Cosa c'è oggi nell'header

`App.tsx`, `.app-header`: logo, titolo "Ramus", `JournalControls`
(input data "salta a data" + bottone "Oggi", solo in vista journal),
bottone comprimi/espandi finestra, bottone cerca 🔍, bottone
impostazioni ⚙ — cinque elementi interattivi oltre al branding,
tutti nella stessa riga.

## Redesign proposto

### Header (in alto): solo 3 icone

Resta: logo, titolo "Ramus" (invariato — l'idea è "meno bottoni", il
titolo non è un bottone; vedi "Domande aperte" se si vuole toglierlo
comunque), e **esattamente tre bottoni icona**:

1. Comprimi/espandi finestra (`compact-toggle`, invariato)
2. Cerca 🔍 (invariato)
3. Impostazioni ⚙ (invariato)

Tutto il resto (navigazione del journal) si sposta nella status bar.

### Nuova status bar (in basso)

Striscia sottile, sempre presente, ancorata al fondo della finestra
(`.app` è già `display:flex; flex-direction:column`, si aggiunge un
terzo figlio dopo `main`/`PageView` — nessuna ristrutturazione, stesso
schema flex già in uso). Contenuto:

- `JournalControls` (input data + "Oggi"), **spostato qui di peso**,
  stessa logica di visibilità di oggi (solo `view.kind === "journal"`).
- Punto di aggancio per lo stato di sync di M3 (`specs/M3/2026-09-02-sync-git-remoto.DONE.md`,
  non ancora implementata): quella spec proponeva un badge
  nell'header — **si sposta qui** quando M3 verrà implementata (questa
  spec aggiorna già il riferimento nel testo di M3, vedi sotto). Non
  fa parte dell'implementazione di *questa* spec (M3 non è ancora
  costruita), solo dello spazio riservato nel layout.

Peso visivo ridotto rispetto all'header: testo piccolo, colore
`--ramus-stone` (metadati/secondario, stesso token già usato per date e
placeholder), separatore sottile in alto (`border-top`, stesso
pattern di `.journal-section + .journal-section`), altezza contenuta
(~2rem). Sempre visibile, nessun auto-hide/hover-to-reveal in questa
prima iterazione (vedi "Fuori scope") — il "distraction free" viene
dal peso visivo ridotto, non dallo sparire.

### CSS

```css
.app-statusbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.3rem 1.25rem;
  border-top: 1px solid color-mix(in srgb, var(--ramus-stone) 20%, transparent);
  font-size: 0.8rem;
  color: var(--ramus-stone);
}
```

`.journal-controls` dentro la status bar può aver bisogno di uno stile
più minimale di quello attuale (bottoni con bordo pieno, pensati per
l'header) — da rivedere in fase di implementazione, non è un blocco
per la spec.

### Modalità compatta

Oggi `.app-header.is-compact` nasconde `.journal-controls` e i
`.settings-button` tranne il toggle. Con `journal-controls` spostato
nella status bar, quella regola CSS diventa obsoleta e va aggiornata:
in modalità compatta la status bar **si nasconde interamente**
(stesso principio di oggi — la finestra compatta è per affiancare
un'altra app, meno chrome possibile), l'header resta con i suoi 3
bottoni invariati.

## Fuori scope per questa spec

- Auto-hide della status bar (nascondersi durante la digitazione,
  riapparire al passaggio del mouse): estensione possibile in futuro,
  aggiunge complessità (animazione, hover-tracking) non richiesta ora.
- Contenuto aggiuntivo nella status bar oltre a navigazione journal +
  aggancio per lo stato di sync (es. conteggio parole/blocchi,
  indicatore di salvataggio): non richiesto, si aggiunge solo se serve
  davvero.
- Rimozione del titolo "Ramus" dall'header: vedi "Domande aperte".
- Personalizzazione dell'ordine/contenuto di header e status bar da
  parte dell'utente: fissi, coerente con l'assenza di configurabilità
  estesa nel resto dell'app.

## Domande aperte

Nessuna: tutte e tre confermate come proposto.

## Verifica

`npm run typecheck` pulito. Comportamento visivo (aspetto della status
bar, sparizione in modalità compatta, wrap a finestra stretta) da
verificare con un giro manuale in `npm run tauri dev`.
