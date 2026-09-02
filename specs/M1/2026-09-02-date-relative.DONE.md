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

## Formato data "carino" (giorno + mese, anno solo se diverso)

Aggiunta dopo il primo giro di implementazione: la parte data
dell'header non è più la stringa ISO grezza (`2026-09-02`) ma un
formato leggibile — `"2 settembre"` nell'anno corrente,
`"15 marzo 2025"` quando l'anno è diverso da quello corrente (l'anno è
rumore quando è ovvio, utile quando non lo è). Risultato:
`"Oggi 2 settembre"`, `"3 giorni fa 30 agosto"`,
`"Mercoledì 19 agosto"`, `"Sabato 15 marzo 2025"`.

```ts
const PRETTY_DATE_FORMATTER = new Intl.DateTimeFormat("it-IT", { day: "numeric", month: "long" });
const PRETTY_DATE_WITH_YEAR_FORMATTER = new Intl.DateTimeFormat("it-IT", {
  day: "numeric",
  month: "long",
  year: "numeric",
});

function formatPrettyDate(iso: string): string {
  const date = parseIsoDate(iso);
  const isCurrentYear = date.getFullYear() === new Date().getFullYear();
  return (isCurrentYear ? PRETTY_DATE_FORMATTER : PRETTY_DATE_WITH_YEAR_FORMATTER).format(date);
}
```

**Effetto collaterale corretto insieme**: i nomi dei mesi in italiano
sono minuscoli per convenzione ("settembre", non "Settembre"). La CSS
`.journal-section-date { text-transform: capitalize }` esistente
avrebbe capitalizzato anche il mese a metà stringa (risultato scorretto
tipo "2 Settembre"). Rimossa dal CSS: la maiuscola sulla prima lettera
si fa ora in `formatJournalHeader` stesso, una volta sola, sull'intera
stringa già composta (`capitalizeFirst`), non parola per parola.

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
