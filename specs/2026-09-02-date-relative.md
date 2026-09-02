# Formattazione data relativa (Oggi / Ieri / N giorni fa)

Stato: implementata.

## Motivazione

L'header di ogni sezione journal (`formatJournalHeader` in
`src/lib/journal.ts`) oggi mostra sempre `"<giorno della settimana>
<ISO>"`, es. "mercoledì 2026-09-02", anche per il giorno corrente. Una
fascia di date relative (Oggi, Ieri, N giorni fa) si legge più in
fretta e rinforza il senso "dove sono" già dato dall'header ambrato del
giorno corrente (SPEC.md — palette, "amber solo giorno corrente").

## Comportamento

- Oggi → `"Oggi"`.
- Ieri → `"Ieri"`.
- Da 2 a 6 giorni fa (il resto della settimana) → `"N giorni fa"`, es.
  `"3 giorni fa"`.
- 7 o più giorni fa → resta il formato attuale, invariato:
  `"<giorno della settimana> <ISO>"`.
- La data ISO resta sempre visibile accanto all'etichetta, anche per
  quelle relative: `"Oggi 2026-09-02"`, `"3 giorni fa 2026-08-30"` — non
  si perde l'informazione esplicita già data dall'header attuale.

## Implementazione

Solo `src/lib/journal.ts`, nessuna modifica a componenti React (già
chiamano `formatJournalHeader`, l'output cambia ma la firma no):

```ts
function daysBetween(fromIso: string, toIso: string): number {
  const msPerDay = 24 * 60 * 60 * 1000;
  const diff = parseIsoDate(toIso).getTime() - parseIsoDate(fromIso).getTime();
  // Math.round, non una divisione secca: un giorno di cambio ora
  // legale/solare dura 23 o 25 ore, non esattamente 24.
  return Math.round(diff / msPerDay);
}

function relativeLabel(iso: string): string | null {
  const days = daysBetween(iso, formatIsoDate(new Date()));
  if (days === 0) return "Oggi";
  if (days === 1) return "Ieri";
  if (days >= 2 && days <= 6) return `${days} giorni fa`;
  return null;
}

export function formatJournalHeader(iso: string): string {
  const label = relativeLabel(iso) ?? WEEKDAY_FORMATTER.format(parseIsoDate(iso));
  return `${label} ${iso}`;
}
```

Nessuna libreria di date, nessuna dipendenza nuova — solo aritmetica su
`Date` già costruiti con `parseIsoDate` (esistente, evita l'insidia
UTC di `new Date(iso)` già documentata nel file).

### Limite noto e accettato

Le etichette si calcolano al momento della chiamata (ogni render di
`JournalSection`, quindi restano ragionevolmente fresche durante l'uso
normale), ma non c'è un timer che forzi un aggiornamento esattamente a
mezzanotte: se l'app resta aperta e nessuna sezione si ri-renderizza
attorno alla mezzanotte, un'etichetta potrebbe restare quella di prima
per qualche istante dopo il cambio di giorno, finché qualcosa non causa
un nuovo render. Stesso limite già presente altrove nel codice (es.
`TODAY_ISO` in `JournalControls.tsx`, calcolato una volta sola al
caricamento del modulo) — non è una regressione, è coerente con
l'effort già investito nel resto dell'app su questo dettaglio.

## Fuori scope

- Localizzazione in altre lingue (l'app è in italiano ovunque altrove,
  le etichette restano hardcoded come il resto della UI).
- Etichette relative per date future (non applicabile: non esistono
  giorni futuri nella vista journal).

## Verifica

Non testabile con `cargo test` (nessun codice Rust toccato). Da
verificare con `npm run typecheck` (già previsto) e a vista in
`npm run tauri dev`: oggi mostra "Oggi <ISO>", ieri "Ieri <ISO>", i
cinque giorni prima "N giorni fa <ISO>", da una settimana fa in poi il
formato assoluto di prima.
