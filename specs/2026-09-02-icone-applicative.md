# Icone applicative (bundle desktop)

Stato: implementata. Icone rigenerate, checksum confermati diversi dai
placeholder originali, `cargo build --workspace` e il rebuild
automatico di `tauri dev` (che ha rilevato i file cambiati da solo)
funzionano senza errori.

## Motivazione

SPEC.md — "Identità visiva" — già descrive questo lavoro, mai fatto
durante l'implementazione di M1: `npm run tauri icon assets/logo.svg`
con un margine del 12% attorno al segno, perché "la maschera di macOS
taglia il marchio contro il bordo". Oggi l'app usa ancora le icone
placeholder generiche del template `create-tauri-app` (`src-tauri/icons/`).

## Sorgente: nuovo file, non si tocca `logo.svg`

`assets/logo.svg` ha viewBox `0 0 96 96` e il segno (path + 4 cerchi)
già disegnato con un po' di margine interno rispetto ai bordi del
canvas, ma non i 12% prescritti da SPEC.md, e quel file è usato anche
altrove nell'interfaccia (tabella asset di SPEC.md: "marchio pieno,
sopra i 32px") — non va alterato per un vincolo specifico della
generazione icone.

Nuovo file `assets/icon-source.svg`: stesso disegno di `logo.svg`,
avvolto in un canvas più grande con un margine uniforme, invece di
modificare le coordinate del disegno stesso:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">
  <g transform="translate(16, 16)">
    <!-- contenuto di logo.svg (path + cerchi), invariato -->
  </g>
</svg>
```

Calcolo: canvas 96×96 originale trattato come cornice del segno (ha già
margine interno, quindi il margine reale sull'inchiostro è ancora più
ampio — meglio abbondare che tagliare). Nuovo canvas 128×128, margine
16px per lato → `16 / 128 = 12.5%`, praticamente i 12% richiesti, con
numeri tondi e facili da verificare. Non si copia il blocco di metadata
C2PA di `logo.svg` (attestazione di provenienza specifica a quel file,
non applicabile a un file nuovo derivato) — irrilevante per l'icona,
fuori scope toccarlo.

## Comando

```bash
npm run tauri icon assets/icon-source.svg
```

Verificato con `npx tauri icon --help`: accetta SVG direttamente
("Path to the source icon (squared PNG or SVG file with transparency)"),
scrive in `icons/` accanto a `tauri.conf.json` per default — cioè
`src-tauri/icons/`, sovrascrivendo i file placeholder in posto.

Nessuna modifica a `tauri.conf.json`: l'array `bundle.icon` esistente
(`icons/32x32.png`, `icons/128x128.png`, `icons/128x128@2x.png`,
`icons/icon.icns`, `icons/icon.ico`) usa già gli stessi nomi file che
il generatore produce di default — il comando li sovrascrive con
contenuto nuovo, i riferimenti restano validi senza toccare la config.

### Nota: genera anche cartelle Android/iOS

Il comando produce sempre il set completo multipiattaforma, incluse
sottocartelle `icons/android/` e `icons/ios/` con le loro varianti,
anche se il progetto non ha `tauri android init`/`tauri ios init`
(non c'è `src-tauri/gen/` per mobile). È il comportamento normale del
tool, non specifico di questo progetto — restano semplicemente inutilizzate
finché (se mai) si aggiungerà un target mobile. Non richiede pulizia.

### Bug scoperti dopo il primo giro di implementazione

1. **Sfondo trasparente**: `icon-source.svg` non aveva uno sfondo — solo
   il segno su tela trasparente, come `logo.svg` (pensato per essere
   disegnato sopra lo sfondo dell'app, non per essere un'icona di
   sistema a sé stante). Nel Dock risultava un segno "fluttuante" senza
   silhouette. Corretto aggiungendo `<rect width="128" height="128"
   fill="#F5F1E8"/>` (paper) dietro al segno — un quadrato pieno, non
   arrotondato: la maschera (squircle su macOS, ecc.) la applica il
   sistema operativo, non serve pre-arrotondare l'SVG sorgente.

2. **Angoli vivi invece che arrotondati**: in dev mode su macOS l'icona
   del Dock viene impostata con `NSApplication.setApplicationIconImage`,
   chiamata diretta che mostra l'immagine così com'è — **non** applica
   la maschera arrotondata automatica che macOS riserva alle app
   installate/pacchettizzate via Finder/LaunchServices. Il rettangolo di
   sfondo va quindi arrotondato direttamente nel sorgente, non lasciato
   al sistema operativo: `rx="23" ry="23"` su un canvas 128×128
   (~18%, la proporzione usata nei template ufficiali Apple per le
   icone macOS).

3. **Le icone cambiate non facevano scattare una ricompilazione**:
   `tauri-build` dichiara `cargo:rerun-if-changed` solo per
   `tauri.conf.json`, non per i file dentro `icons/`. Sostituire le
   icone non bastava perché Cargo non vedeva nessun input cambiato e
   riusava il binario già compilato — con i vecchi byte icona ancora
   incorporati (su macOS in dev mode l'icona del Dock viene incorporata
   nel binario a tempo di compilazione). Corretto in `src-tauri/build.rs`
   con `println!("cargo:rerun-if-changed=icons")`. Verificato: dopo
   questa correzione, rigenerare le icone fa comparire "Compiling ramus
   v0.1.0" nel log di `tauri dev` (ricompilazione vera), non solo un
   relink a costo zero.

## Fuori scope

- Favicon web (`assets/favicon.svg`, usato per la finestra/tab, non fa
  parte del bundle nativo): non toccato, resta la versione a tre nodi
  già esistente.
- Icona tray/system tray: nessuna funzionalità di tray nel progetto,
  non applicabile.
- `logo-mono.svg`/`mascotte.svg`: non coinvolti nella generazione icone
  applicative.

## Verifica

Non testabile con `cargo test`/`npm run typecheck` (asset binari, non
codice). Dopo aver lanciato il comando:

- `src-tauri/icons/icon.png` e gli altri file elencati in `bundle.icon`
  risultano diversi (dimensione/contenuto) dai placeholder originali
  del template.
- `cargo build --workspace` e `npm run tauri dev` continuano a
  funzionare senza errori (le icone non influenzano la compilazione,
  solo il bundling finale).
- A vista: l'icona nel dock/Finder (macOS) mostra il marchio Ramus
  centrato, senza il segno tagliato contro il bordo arrotondato della
  maschera — verificabile solo dall'utente, non in questo sandbox.
