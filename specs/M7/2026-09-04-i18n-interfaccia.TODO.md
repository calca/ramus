# Multilingua: interfaccia (react-i18next, italiano + inglese)

Stato: proposta, da implementare. Prima delle due spec i18n (questa:
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

## Verifica

`cargo test`, `cargo clippy --all-targets -D warnings`, `cargo fmt
--check`, `npm run typecheck`, `npm run test` — tutti puliti prima di
chiudere. Verifica manuale in `npm run tauri dev`: cambiare lingua da
Impostazioni, confermare che testo/date cambino subito senza riavvio;
avviare con `LANG`/locale di sistema in inglese (o forzare
temporaneamente `resolveSystemLocale`) per verificare il rilevamento
automatico.
