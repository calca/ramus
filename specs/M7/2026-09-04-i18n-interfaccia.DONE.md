# Multilingua: interfaccia (react-i18next, italiano + inglese)

Stato: implementata. Prima delle due spec i18n (questa:
l'interfaccia; la seconda,
`specs/M7/2026-09-04-i18n-errori.TODO.md`: i messaggi
d'errore, dipende da questa). Decisioni già confermate dall'utente:
libreria (react-i18next, non fatto in casa), italiano + inglese,
lingua di default che segue il sistema operativo.

## Motivazione

Il repository è pubblico (licenza MIT, `github.com/calca/ramus`), ma
ogni stringa dell'interfaccia è italiano scritto a mano, sparso in
almeno una decina di componenti (`App.tsx`, `SettingsPanel.tsx`,
`CommandPalette.tsx`, `Cheatsheet.tsx`, `OpenTasksPanel.tsx`,
`paletteActions.ts`, `shortcut.ts`) più la formattazione delle date
(`lib/journal.ts`, `Intl.DateTimeFormat("it-IT", ...)` fisso). Un
utente che non legge italiano oggi non può usare l'app.

## Perché una libreria (non fatto in casa)

Confermato dall'utente. `react-i18next` è lo standard de facto per
React, gestisce interpolazione e pluralizzazione in modo robusto e
testato — il progetto ne avrebbe comunque bisogno per casi come "N
giorni fa" (`lib/journal.ts`, oggi pluralizzazione scritta a mano) e
"{n} journal, {n} pagine" (`SettingsPanel.tsx`). Nuova dipendenza
(`i18next`, `react-i18next`), motivata qui: CLAUDE.md, "nessuna
dipendenza nuova senza una riga di motivazione nel commit".

**Non** `i18next-browser-languagedetector`: la rilevazione "segue il
sistema" si scrive a mano con `navigator.language`, stesso principio
già in uso in `lib/shortcut.ts` per `IS_MAC` — la preferenza di lingua
va comunque persistita in `config.json` (stesso meccanismo del
`theme`, non in `localStorage`), un pacchetto dedicato alla sola
rilevazione da browser storage non serve.

## Modifiche

### Config (Rust) — stesso pattern di `Theme`

**`crates/ramus-core/src/config.rs`**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    It,
    En,
    #[default]
    System,
}
```
`Config.locale: Locale` (`#[serde(default)]`, stessa migrazione
sicura già usata per `theme` — `config.json` scritti prima di questo
campo non hanno la chiave, il default evita che la deserializzazione
fallisca). `Config::set_locale(&mut self, locale: Locale) ->
Result<(), CoreError>`, identica a `set_theme`.

**`src-tauri/src/commands.rs`**: `#[tauri::command] pub fn
set_locale(locale: Locale, state) -> Result<Config, CoreError>` —
stesso wrapper di `set_theme`. **`src-tauri/src/lib.rs`**: registrato
in `generate_handler!`.

**`src/lib/types.ts`**: `type Locale = "it" | "en" | "system"`,
`Config.locale: Locale`. **`src/lib/commands.ts`**: `setLocale(locale:
Locale): Promise<Config>`.

### i18next — nuova cartella `src/i18n/`

- `src/i18n/it.ts`, `src/i18n/en.ts`: dizionari piatti chiave→stringa
  (un namespace solo, il progetto non è abbastanza grande da
  giustificarne più d'uno).
- `src/i18n/resolveSystemLocale.ts`: legge `navigator.language`,
  ritorna `"it"` se inizia per `"it"`, altrimenti `"en"` — fallback
  universale per chiunque non sia italiano, non un tentativo di
  coprire ogni locale possibile (solo due lingue supportate).
- `src/i18n/index.ts`: inizializza l'istanza `i18next` con le due
  risorse, lingua iniziale = `resolveSystemLocale()` se
  `Config.locale === "system"` altrimenti il valore esplicito. Va
  inizializzato **prima** del render (`main.tsx`), non dentro
  `App.tsx`: `useTranslation()` nei componenti assume che l'istanza
  esista già.
- Quando l'utente cambia lingua da Impostazioni: `i18next.changeLanguage(...)`
  oltre a `setLocale(...)` — la UI si aggiorna subito, non solo al
  prossimo avvio (stesso principio già seguito per `applyTheme`,
  chiamata insieme a `setTheme` in `SettingsPanel.tsx`).

### Estrazione delle stringhe

Ogni stringa letterale in JSX/attributi (`aria-label`, `placeholder`,
`title`) diventa una chiave in `it.ts`/`en.ts`, letta con `t("chiave")`
via `useTranslation()`. Fuori da componenti React (`paletteActions.ts`,
`lib/shortcut.ts`'s `SHORTCUT_ACTIONS[].label`) si usa l'API
imperativa `i18next.t(...)` (non richiede l'hook). Convenzione chiavi:
`namespace.sotto_area.nome` (es. `settings.sync.intro`,
`palette.action.today`) — non piatte a caso, per restare orientabili
in un dizionario che supererà probabilmente 100 voci.

**`lib/journal.ts`**: `formatJournalHeader`/`formatPrettyDate` usano
`Intl.DateTimeFormat("it-IT", ...)` fisso — diventa `Intl.DateTimeFormat(
currentLocale === "it" ? "it-IT" : "en-US", ...)`, con `currentLocale`
letto dall'istanza i18next corrente (`i18next.language`). Le etichette
relative (`"Oggi"`, `"Ieri"`, `"N giorni fa"`) diventano chiavi
tradotte con interpolazione/pluralizzazione react-i18next per il
conteggio giorni.

**Impostazioni**: nuova sezione o voce "Lingua" nella sidebar (o dentro
"Tema", da decidere in implementazione guardando lo spazio
disponibile — non un bivio di prodotto), stesso controllo radio
Italiano/English/Sistema già usato per Chiaro/Scuro/Sistema.

## Fuori scope (per questa spec)

- Messaggi d'errore (`CoreError`): spec separata,
  `2026-09-04-i18n-errori.TODO.md` — dipende da questa (serve
  l'infrastruttura i18next già pronta).
- Contenuto del vault dell'utente (il markdown che scrive): resta
  nella lingua che l'utente sceglie di scrivere, ovviamente mai
  tradotto.
- Traduzione di `README.md`/`SPEC.md`/`CLAUDE.md`: documentazione del
  progetto, non l'interfaccia — se serve un README in inglese, spec a
  parte (probabile, dato che il repo è pubblico, ma un lavoro diverso
  da questo).
- Altre lingue oltre italiano/inglese: confermato dall'utente, solo
  queste due per ora.
- Tradurre le stringhe di `ramus-mcp` (nomi/descrizioni degli
  strumenti MCP): il client di un server MCP è un agente, non una
  persona con una lingua preferita — non ha senso applicare questa
  spec lì.

## Domande aperte

Nessuna bloccante: le quattro decisioni principali (libreria, lingue,
ambito, default) sono già confermate. Un dettaglio lasciato
all'implementazione (non un bivio): dove esattamente posizionare il
controllo "Lingua" in Impostazioni (sezione propria o dentro "Tema").

## Test da scrivere

**Rust**: `Config` guadagna gli stessi test già esistenti per `theme`
(`config_without_locale_field_defaults_to_system` o simile — stesso
principio di `config_without_mcp_enabled_field_defaults_to_true`).
**Frontend**: `resolveSystemLocale` è una funzione pura — test
vitest analogo a `parseTypedDate`/`getShortcut` (stub di `navigator`,
stesso pattern già usato in `shortcut.test.ts` per evitare che il
risultato dipenda dalla macchina che esegue i test).

## Scoperte e deviazioni durante l'implementazione

**Le etichette derivate da array a livello di modulo non possono essere
stringhe già tradotte.** `SHORTCUT_ACTIONS` (`src/lib/shortcut.ts`) ed
`EDITOR_SHORTCUTS` (`Cheatsheet.tsx`) erano array costanti definiti al
caricamento del modulo, con `label` già in italiano. Se quel campo
diventasse `i18next.t("...")` chiamato una volta sola lì (lettura
letterale della spec, "si usa l'API imperativa i18next.t(...)"), il
risultato resterebbe congelato nella lingua attiva al primo import per
sempre — un cambio di lingua da Impostazioni non aggiornerebbe più
Cheatsheet/SettingsPanel finché l'app non viene riavviata, il che
contraddice esplicitamente "verifica manuale... confermare che
testo/date cambino subito senza riavvio" più sotto in questa stessa
spec. Soluzione: gli array tengono solo `labelKey` (una chiave, non una
stringa tradotta); `t(action.labelKey)` viene chiamato nel componente
che la mostra (`Cheatsheet.tsx`, `SettingsPanel.tsx`, via
`useTranslation()`), che react-i18next ri-renderizza da solo ad ogni
`changeLanguage()`. `paletteActions.ts` (non un componente) resta
sull'API imperativa come da spec, ma dentro `buildActions()` — chiamata
di nuovo ad ogni render di `App.tsx` mentre la palette è aperta, quindi
comunque reattiva — non in un array di modulo.

**Lingua iniziale di i18next vs `Config.locale` asincrono.** `Config`
si legge solo con un `invoke` Tauri asincrono (in `App.tsx`, dopo il
render), ma `src/i18n/index.ts` deve inizializzare l'istanza i18next
*prima* del render (`main.tsx`). `init()` parte quindi con
`resolveSystemLocale()` come lingua iniziale — corretto nel caso comune,
dato che anche `Config.locale` di default è `System` lato Rust — e
`applyLocale(cfg.locale)` la corregge nello stesso effetto di
`App.tsx` che già chiama `applyTheme(cfg.theme)`, non appena `Config`
torna dal backend. Per un utente con una preferenza esplicita diversa
dal sistema c'è quindi un breve istante (un giro di `invoke`, non
percepibile nella pratica) in cui l'interfaccia usa la lingua di
sistema prima di correggersi — stesso principio già in uso per il tema.

**"Lingua" ha una sezione propria nella sidebar** di Impostazioni,
subito dopo "Tema" — non annidata dentro "Tema": la sidebar è già una
lista piatta di categorie indipendenti (Vault, Tema, Scorciatoie, Task,
MCP, Sync, Informazioni), stesso trattamento per coerenza. Stesso
controllo a radio button già usato per Chiaro/Scuro/Sistema.

**I due paragrafi di aiuto MCP con `<code>` incorporato** ("Incollalo in
`.mcp.json`... o `claude_desktop_config.json`...", "Binario `ramus-mcp`
non trovato...") non si prestavano a un semplice `t()` con interpolazione
di stringa (avrebbero perso gli elementi `<code>`, o richiesto
`dangerouslySetInnerHTML`). Usato invece il componente `<Trans>` di
react-i18next con `components={[<code key="0" />, <code key="1" />]}` e
la sintassi `<0>...</0>`/`<1>...</1>` nel dizionario — l'unico punto del
progetto che usa `<Trans>` invece di `t()`.

**Test esistenti che assumevano l'italiano.** `journal.test.ts` e
`paletteActions.test.ts` asserivano stringhe italiane letterali
("Oggi", "Comprimi finestra", ...). La lingua iniziale di i18next segue
`navigator.language`, che in Node riflette il locale reale della
macchina (`LANG`/`LC_ALL`) — stesso problema già documentato per
`IS_MAC` in `shortcut.test.ts`. Entrambi i file ora forzano
`i18next.changeLanguage("it")` in un `beforeAll` prima delle asserzioni,
invece di affidarsi alla lingua rilevata. Aggiunto anche un blocco
`describe` in inglese a `journal.test.ts` (non richiesto esplicitamente
dalla spec, ma a costo quasi nullo) per verificare che sia le etichette
relative sia il locale `Intl.DateTimeFormat` seguano davvero
`i18next.language` a runtime.

**Pluralizzazione: solo dove il codice già trattava un conteggio.**
`settings.task.days`/`settings.sync.minutes`/`journal.daysAgo` usano le
convenzioni i18next `_one`/`_other` (erano già valori enumerati o un
conteggio esplicito nel codice originale). `settings.vault.stats`
("N journal, N pagine") non è stata pluralizzata: il codice originale
non gestiva il singolare nemmeno in italiano ("1 journal, 1 pagine" era
già il comportamento pre-esistente) — tradotta 1:1 senza introdurre un
comportamento nuovo non richiesto dalla spec.

**Nessuna deviazione sul resto**: `Locale` (Rust) rispecchia `Theme`
esattamente come descritto, `set_locale` rispecchia `set_theme`,
`src/i18n/` ha la forma descritta (dizionari annidati per area invece
che piatti — esplicitamente lasciato a scelta dalla spec), niente
toccato in `ramus-mcp` né in `CoreError`/messaggi d'errore Rust (fuori
scope, spec separata).

## Verifica

Eseguiti e tutti puliti, nell'ordine:

- `cargo test` — 123 test in `ramus-core` (inclusi i 2 nuovi di
  `Locale`: `config_without_locale_field_defaults_to_system`,
  `locale_serializes_lowercase`), 16 in `ramus-mcp` (invariati,
  `ramus-mcp` non tocca `Locale`), 0 in `ramus`/`ramus_lib` (nessun
  test unitario lì, invariato).
- `cargo clippy --all-targets -- -D warnings` — nessun warning.
- `cargo fmt --all -- --check` — pulito.
- `npm run typecheck` — pulito (`strict`, `noUnusedLocals`,
  `noUnusedParameters` tutti attivi, nessun `any` introdotto).
- `npm run test` — 86 test, tutti verdi (77 preesistenti + 9 nuovi:
  5 in `resolveSystemLocale.test.ts`, 4 aggiunti a `journal.test.ts`
  per il blocco inglese; `paletteActions.test.ts` invariato nel numero
  di test ma con lingua forzata esplicitamente).
- `npm run build` (non richiesto dalla checklist, eseguito comunque
  come controllo aggiuntivo) — build di produzione pulita, nessun
  errore da `tsc`/Vite legato a i18next/react-i18next/`<Trans>`.

**Conteggio chiavi**: 93 chiavi foglia in `it.ts`, 93 in `en.ts` —
verificato programmaticamente che i due insiemi di chiavi coincidano
esattamente (nessuna chiave mancante da un lato o dall'altro).

**Non verificato in questa sessione** (ambiente non interattivo, nessun
`npm run tauri dev` disponibile qui): il giro manuale reale — aprire
l'app, cambiare lingua da Impostazioni e vedere testo/date cambiare
subito senza riavvio; avviare con `LANG`/locale di sistema in inglese
per verificare il rilevamento automatico. Il codice è stato scritto e
ragionato esplicitamente per questo requisito (vedi "Scoperte" sopra,
in particolare il punto su `labelKey`/`useTranslation()`), ma la
verifica visiva/interattiva resta da fare da chi rivede questa spec
prima di committare.
