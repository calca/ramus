# Pagina "Informazioni su Ramus" con la mascotte

Stato: implementata. `getVersion()`/`openUrl()` hanno funzionato senza
modifiche alle capability, come previsto (verificato lanciando l'app:
nessun errore di permesso nei log).

## Motivazione

SPEC.md riserva la mascotte Stecco a "schermate vuote, stati di errore,
onboarding, README e changelog" — mai dentro l'area di scrittura. Una
pagina "Informazioni su Ramus" è la stessa categoria di momento
(presentazione del brand, non uno strumento di lavoro) e non esiste
ancora: oggi non c'è alcun posto nell'app che mostri la mascotte.

## Contenuto

- Mascotte (`assets/mascotte.svg`), a dimensione generosa — SPEC.md:
  "sotto i 64px non è leggibile". Qui ha spazio dedicato, non un vincolo
  di layout stretto: propongo ~128px.
- Nome "Ramus".
- Versione dell'app (vedi sotto).
- Tagline, la stessa riga di apertura di `SPEC.md`: "App desktop di
  journaling, outliner a blocchi su file markdown locali." — niente
  copy nuovo da inventare, si riusa quello già scritto.
- Link "Codice sorgente" → apre `https://github.com/calca/ramus` nel
  browser di sistema.

## Versione: nessun nuovo command

`@tauri-apps/api/app` espone già `getVersion()` (legge la versione da
`tauri.conf.json`/`Cargo.toml`, la stessa che builda l'app) — **non
serve un command Tauri custom**. Permesso richiesto: `core:app`
include `allow-version` nel proprio set di default, e `core:default`
(già in `capabilities/default.json`) copre i default di tutti i
sotto-moduli core (`app`, `window`, `event`, ...) — stesso motivo per
cui `get_config`/`open_today` funzionano oggi senza permessi
aggiuntivi. Da verificare comunque lanciando l'app (come già successo
col drag-region: un permesso mancante si manifesta subito nei log,
prima di dare per scontato che sia coperto).

## Link al repository: nessun nuovo command

`openUrl` da `@tauri-apps/plugin-opener` (già installato e registrato,
usato per "Apri nel file manager" nelle Impostazioni) apre un URL nel
browser di default. Il permesso `opener:default` (già presente)
include `allow-open-url`. Nessuna dipendenza nuova, nessun comando
nuovo.

## UI: dove vive

Non dentro il pannello Impostazioni esistente (quello resta compatto e
orientato all'azione: vault, tema). Un pannello separato,
`AboutPanel.tsx`, raggiungibile da un link in fondo alle Impostazioni
("Informazioni su Ramus"). Un solo pannello attivo alla volta — niente
modali impilati:

- `App.tsx`: lo stato booleano `settingsOpen` diventa
  `activePanel: "settings" | "about" | null`.
- Il gear button apre `"settings"`.
- Un link/bottone in fondo a `SettingsPanel` chiama `onShowAbout` →
  `activePanel = "about"` (sostituisce Impostazioni, non si accumula).
- `AboutPanel` chiude tornando a `activePanel = null` (non torna
  indietro a Impostazioni: stesso comportamento "chiudi e basta" degli
  About-box tipici).

### Refactoring piccolo: `Modal.tsx` condiviso

`SettingsPanel` e il nuovo `AboutPanel` condividono esattamente la
stessa meccanica (backdrop, click fuori per chiudere, Escape per
chiudere, `stopPropagation` sul contenuto) — non speculativa, è
duplicazione reale fra due componenti che esistono entrambi ora.
Estratto un `Modal.tsx`:

```tsx
interface ModalProps {
  onClose: () => void;
  ariaLabel: string;
  children: ReactNode;
}

export function Modal({ onClose, ariaLabel, children }: ModalProps) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="settings-backdrop" onClick={onClose}>
      <div
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        onClick={(event) => event.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
```

`SettingsPanel` viene alleggerito per usarlo (rimuove il proprio
`useEffect` di Escape e il markup di backdrop), `AboutPanel` lo usa
identico. Stile CSS invariato: `.settings-backdrop`/`.settings-panel`
restano nomi generici, non specifici delle Impostazioni — già lo erano.

## Fuori scope

- Changelog/release notes nella pagina About: non richiesto, SPEC.md
  cita "changelog" come altro contesto ammesso per la mascotte, ma è
  un file separato (se e quando esisterà), non questa pagina.
- Link "controlla aggiornamenti" o simili: nessun meccanismo di update
  automatico nel progetto, fuori scope.
- Licenza/crediti terze parti: non richiesto, si aggiunge se serve in
  futuro.

## Verifica

Non testabile con `cargo test` (nessun codice Rust toccato). Da
verificare con `npm run typecheck` e a vista in `npm run tauri dev`:

- Il link "Informazioni su Ramus" in fondo alle Impostazioni apre il
  pannello About, chiudendo quello delle Impostazioni.
- La mascotte è visibile e leggibile (non schiacciata sotto 64px).
- La versione mostrata combacia con quella in `tauri.conf.json`.
- Il link "Codice sorgente" apre il repository nel browser di sistema,
  non dentro la finestra dell'app.
- Escape e click sul backdrop chiudono sia Impostazioni sia About.
