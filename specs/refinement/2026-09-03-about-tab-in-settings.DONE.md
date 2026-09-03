# "Informazioni su Ramus" diventa un tab di Impostazioni

Stato: implementata. Seconda spec della fase di refinement, seguito
diretto di `specs/refinement/2026-09-03-settings-sidebar.DONE.md`
(sidebar di categorie in `SettingsPanel`). Entrambe le domande aperte
confermate come proposto: tab "Informazioni" in fondo alla sidebar,
uguale alle altre sei, nessun divisore; nessuna label di categoria
sopra il contenuto.

## Motivazione

`AboutPanel` è oggi un modal separato (mascotte, nome, versione,
tagline, link "Codice sorgente"), aperto da un bottone in fondo a
`SettingsPanel` o dall'azione "Informazioni su Ramus" nella command
palette. Ora che Impostazioni ha una sidebar di categorie stabile
(altezza fissa, `min(36rem, calc(100vh - 4rem))`), un secondo modal
separato solo per queste quattro righe di testo non ha più senso:
diventa una categoria in più nella stessa sidebar, un click invece di
un modal-sopra-il-modal.

## Modifiche

**`SettingsPanel.tsx`**:
- `SettingsSectionId` guadagna `"about"`; `SETTINGS_SECTIONS` guadagna
  `{ id: "about", label: "Informazioni" }` come ultima voce
  dell'elenco normale (vedi "Domande aperte" per posizione/divisore).
- Contenuto della sezione: il markup di `AboutPanel.tsx` (mascotte,
  nome, versione, tagline, bottone "Codice sorgente") spostato dentro
  `<section className="settings-section">`, invariato — stesse classi
  CSS (`about-content`, `about-mascotte`, `about-name`,
  `about-version`, `about-tagline`), quindi zero modifiche a
  `index.css` per questa parte.
- Nuovo stato locale `version` + lo stesso `useEffect(() =>
  void getVersion().then(setVersion).catch(() => {}), [])` di
  `AboutPanel.tsx`, copiato 1:1.
- Nuovi import: `getVersion` (`@tauri-apps/api/app`), `openUrl`
  (`@tauri-apps/plugin-opener`), `mascotteUrl`
  (`../../assets/mascotte.svg`), costante `REPO_URL`.
- Prop `onShowAbout` **rimossa** da `SettingsPanelProps`: non serve
  più un callback verso il genitore, il click sul tab "Informazioni"
  è un normale `setActiveSection("about")` interno, come ogni altro
  tab.
- Nuova prop opzionale `initialSection?: SettingsSectionId` (default
  `"vault"` se assente): permette di aprire il pannello già posizionato
  su un tab specifico — serve per "Informazioni su Ramus" dalla
  command palette, che deve continuare a portare dritto lì invece che
  su Vault.

**`App.tsx`**:
- Rimosso `import { AboutPanel } from "./components/AboutPanel"` e il
  ramo `{activePanel === "about" && <AboutPanel ... />}`.
- Il ramo `{activePanel === "settings" && ...}` diventa
  `{(activePanel === "settings" || activePanel === "about") && config
  && <SettingsPanel initialSection={activePanel === "about" ? "about"
  : "vault"} ... />}` — **il valore `"about"` di `activePanel` resta**
  (nessuna modifica a `paletteActions.ts`, all'azione "Informazioni su
  Ramus" della command palette, o al tipo dell'union): significa
  ancora "apri il pannello di info", solo che ora quel pannello è
  Impostazioni-posizionata-su-Informazioni invece di un modal a parte.
  Cambia solo cosa viene renderizzato, non l'API interna di `App.tsx`.
- `onShowAbout` non più passato a `SettingsPanel` (prop rimossa).

**File eliminato**: `src/components/AboutPanel.tsx` (contenuto
migrato, nessun altro punto lo importa dopo questa modifica — verificato
con grep prima di cancellare).

**`Modal.tsx`**: il commento "Condiviso da SettingsPanel e AboutPanel"
aggiornato (`AboutPanel` non esiste più) in "Condiviso da
SettingsPanel, CommandPalette e Cheatsheet".

**Nessuna modifica** a `paletteActions.ts`, a nessun command Tauri, o
a `ramus-core`/`src-tauri` — refactor di frontend puro, stesso
principio della spec sidebar.

## Domande aperte

Nessuna: entrambe confermate come proposto. Tab "Informazioni" in
fondo all'elenco, uguale alle altre sei voci, nessun separatore.
Nessuna label di categoria sopra mascotte/nome/versione — il nome
"Ramus" è già l'intestazione.

## Fuori scope per questa spec

- Cambiare il contenuto di "Informazioni" (testo, link, versione
  mostrata): solo un trasloco di posizione, zero modifiche al
  contenuto stesso.
- Ricordare l'ultimo tab aperto fra un'apertura e l'altra del pannello:
  stessa scelta già presa nella spec sidebar (sempre "Vault" di
  default, tranne l'apertura esplicita su "Informazioni" dalla command
  palette).

## Test da scrivere

Nessuno, zero modifiche Rust. Coerente con l'assenza di un runner JS
per componenti nel progetto (stessa scelta di tutte le altre spec di
`SettingsPanel`).

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust,
verificati comunque per la regola di CLAUDE.md). `grep` di conferma:
nessun file importa più `AboutPanel` dopo la cancellazione;
`.settings-sidebar-divider` e `.settings-sidebar > .settings-about-link`
rimosse da `index.css` insieme al loro unico punto d'uso (diventate
dead CSS quando il bottone "Informazioni su Ramus" a parte è sparito
dalla sidebar). Non verificato in questa sessione con uno screenshot
reale: `npm run tauri dev` era già in esecuzione (hot reload via
Vite), verifica visiva lasciata all'utente nell'app già aperta.
