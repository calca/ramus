# Scorciatoie app configurabili + cheatsheet

Stato: implementata. Le due "Domande aperte" sono state confermate
come proposto — `Mod+/` per la cheatsheet, sezione Impostazioni
rinominata "Scorciatoie" (era "Comandi", nome scelto per la sola
command palette prima di questa spec). Uno scostamento dal testo
originale: `default_shortcuts()` popola sia `command_palette` che
`cheatsheet` fin da subito (il blocco di codice originale mostrava
solo `command_palette`, ma il testo sopra di esso già diceva che
questa spec doveva popolare entrambi — un'incoerenza nel testo
proposto, risolta a favore della versione scritta per esteso).

Fondamenta per le altre spec
dell'idea "keyboard focused, less UI": `specs/M4/2026-09-02-riordino-blocchi-tastiera.TODO.md`
e `specs/M4/2026-09-02-focus-mode-navigazione-giorni.TODO.md` aggiungono
voci a questo registro una volta implementate.

## Motivazione

Terzo pezzo di M4. Generalizza `Config::search_shortcut` (M2, oggi
un singolo campo per un solo shortcut) in un registro di più
scorciatoie configurabili — richiesto esplicitamente insieme alle
altre idee "keyboard focused" di questa sessione.

## Cosa resta fuori: le scorciatoie dell'editor

**Solo scorciatoie a livello finestra** (listener globale su
`window`) entrano nel registro configurabile. Tab, Shift-Tab, Invio,
Backspace (esistenti) e il nuovo Alt-ArrowUp/Down per il riordino
blocchi (`specs/M4/2026-09-02-riordino-blocchi-tastiera.TODO.md`)
**restano fisse**, non configurabili. Due motivi, non solo
preferenza:

1. Vivono in un sistema diverso — una keymap ProseMirror configurata
   alla creazione dell'editor (`createExtensions()`), non un listener
   `window.addEventListener("keydown", ...)`. Renderle configurabili a
   runtime richiederebbe ricreare l'editor (o un layer di indirection)
   ogni volta che l'utente cambia una scorciatoia — complessità reale,
   non giustificata per un caso d'uso raro.
2. Sono scorciatoie "strutturali" dell'outliner (indent, nuovo blocco,
   merge), non azioni app-level: cambiarle romperebbe muscolo-memoria
   costruita fin da M1, senza un beneficio chiaro.

La cheatsheet (sotto) le elenca comunque, insieme a quelle
configurabili — solo in sola lettura.

## Migrazione: `search_shortcut` → `shortcuts`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub vault_path: PathBuf,
    #[serde(default)]
    pub theme: Theme,
    /// Scorciatoie app-level configurabili, chiave = id azione
    /// stabile (es. "command_palette"), valore = stringa canonica
    /// ("Mod+K") — stesso formato di prima, ora per N azioni invece
    /// di una sola.
    #[serde(default = "default_shortcuts")]
    pub shortcuts: HashMap<String, String>,
}
```

Il campo `search_shortcut: String` **sparisce** dalla struct — è un
cambio di schema del file persistito (`config.json`), ma **non** del
formato del vault (SPEC.md principio "il formato su disco è
compatibile con Obsidian" riguarda le note, non la configurazione
locale dell'app): `config.json` non è mai stato pensato per essere
letto da altri strumenti, è puro stato applicativo.

Migrazione one-shot in `Config::load`: se il JSON contiene ancora
`search_shortcut` (config scritto da una versione precedente) e non
contiene `shortcuts`, il valore viene spostato sotto la chiave
`"command_palette"` nella nuova mappa, poi il file viene riscritto nel
nuovo formato. Nessuna perdita: la scorciatoia personalizzata
dell'utente sopravvive alla migrazione. Se **entrambi** i campi
mancano (config ancora più vecchio, pre-M2), si parte da
`default_shortcuts()`.

```rust
fn default_shortcuts() -> HashMap<String, String> {
    HashMap::from([("command_palette".to_string(), "Mod+K".to_string())])
}
```

(Le chiavi `cheatsheet`, e in seguito `focus_mode`/
`journal_next_day`/`journal_prev_day`, si aggiungono ai default man
mano che le rispettive spec vengono implementate — questa spec popola
solo `command_palette`, già esistente, e `cheatsheet`, nuova qui.)

## Command Tauri

`set_search_shortcut(shortcut: String)` → rinominato e generalizzato:

```
set_shortcut(action_id: String, shortcut: String) -> Result<Config, CoreError>
```

## Frontend

### Registro delle azioni configurabili

`src/lib/shortcut.ts` guadagna una piccola lista descrittiva (id,
etichetta, default) — non solo le funzioni di normalizzazione già
esistenti:

```ts
export interface ShortcutAction {
  id: string;
  label: string;
  default: string;
}

export const SHORTCUT_ACTIONS: ShortcutAction[] = [
  { id: "command_palette", label: "Apri command palette", default: "Mod+K" },
  { id: "cheatsheet", label: "Mostra scorciatoie", default: "Mod+/" },
];
```

`getShortcut(config, actionId)`: legge `config.shortcuts[actionId]`,
ricade sul `default` di `SHORTCUT_ACTIONS` se la chiave manca (stesso
tipo di robustezza già presente lato Rust per config vecchi — doppia
rete di sicurezza, economica).

### `SettingsPanel`

La sezione "Comandi" (rinominata da "Ricerca" nella spec della
command palette, un solo bottone) diventa "Scorciatoie": un elenco,
una riga con etichetta + bottone di cattura per ogni voce di
`SHORTCUT_ACTIONS` — stesso meccanismo di cattura già scritto per
`search_shortcut` (fase `capture` + `stopPropagation`, invariato),
solo ripetuto per riga invece che una singola istanza.

### Dispatch in `App.tsx`

Il listener globale `keydown` esistente (oggi confronta solo contro
`config.search_shortcut`) itera su `SHORTCUT_ACTIONS`, e per ognuna
confronta l'evento con `getShortcut(config, action.id)` via
`matchesShortcut` (invariato) — generalizzazione diretta, stessa
funzione di confronto.

## Cheatsheet

Nuovo pannello, `activePanel` guadagna `"cheatsheet"` — riusa `Modal`.
Apribile dal proprio shortcut (default `Mod+/`) o da un'azione nella
Command Palette (`specs/M4/2026-09-02-command-palette.DONE.md`, che
guadagna una sesta azione "Mostra scorciatoie").

Due sezioni:

- **Scorciatoie app**: `SHORTCUT_ACTIONS`, etichetta +
  `formatShortcut(getShortcut(config, id))` — riflette il valore
  attuale, personalizzato o di default.
- **Scorciatoie editor** (sola lettura, elenco statico hardcoded, non
  dal registro — vedi "Cosa resta fuori"): Tab (indent), Shift+Tab
  (outdent), Invio (nuovo blocco), Backspace su blocco vuoto (esci di
  un livello), più le voci che arriveranno da
  `specs/M4/2026-09-02-riordino-blocchi-tastiera.TODO.md` quando
  implementata (questa spec lista solo quelle di oggi).

## Fuori scope per questa spec

- Rendere configurabili le scorciatoie dell'editor (vedi sopra).
- Scorciatoie multiple per la stessa azione (oggi una sola stringa per
  id, coerente con la semplicità di `search_shortcut`).
- Import/export della configurazione delle scorciatoie.

## Domande aperte

Nessuna: entrambe confermate come proposto — `Mod+/` per la
cheatsheet, sezione Impostazioni rinominata "Scorciatoie".

## Test da scrivere (core)

- `Config::load` su un JSON con `search_shortcut` ma senza
  `shortcuts` → migra il valore sotto `"command_palette"`, il file
  su disco viene riscritto nel nuovo formato.
- `Config::load` su un JSON senza né `search_shortcut` né `shortcuts`
  → `default_shortcuts()`.
- `Config::load` su un JSON già nel nuovo formato → invariato, nessuna
  migrazione spuria.
- `set_shortcut` aggiorna solo la chiave indicata, lascia le altre
  invariate.

## Verifica

`cargo test` copre la migrazione (106 test totali). `cargo clippy`,
`cargo fmt --check` e `npm run typecheck` puliti. L'interazione
(cattura di più scorciatoie in sequenza, apertura della cheatsheet,
nessuna collisione fra le scorciatoie configurate) non è verificabile
in questo sandbox: serve un giro manuale in `npm run tauri dev`.
