# Auto-update

Stato: proposta. Hosting deciso (GitHub Releases), ma resta bloccata:
dipende dalla firma del codice
(`specs/release/2026-09-03-firma-notarizzazione.TODO.md`), a sua volta
bloccata sulla registrazione di un Apple Developer ID.

## Motivazione

Senza un meccanismo di aggiornamento, ogni nuova versione richiede
all'utente di scaricare e reinstallare manualmente — accettabile per
uso personale, un ostacolo reale se altri iniziano a usare l'app.
Tauri offre `tauri-plugin-updater`, un plugin ufficiale (stesso
principio di `plugin-opener`/`plugin-dialog` già in uso, non una
libreria di terze parti scelta ad hoc).

## Come funziona (per chi non lo conosce)

L'app, avviandosi (o su richiesta), scarica un piccolo file JSON
("manifest") da un URL fisso, controlla se la versione lì dentro è
più recente di quella installata, e se sì scarica/installa
l'aggiornamento — firmato con una chiave separata da quella di
code-signing (una coppia di chiavi generata da Tauri stesso,
`tauri signer generate`). Il manifest va **ripubblicato a ogni
release**, non è statico.

## Domande aperte (bloccanti)

1. ~~Dove ospitare il manifest di aggiornamento?~~ **Risposto: GitHub
   Releases** — nessun hosting nuovo da gestire, ma richiede che ogni
   release sia pubblicata lì con gli asset giusti (coerente con
   `specs/release/2026-09-03-ci.TODO.md`, che dovrebbe generarli
   quando `release.yml` verrà scritto).
2. **Resta bloccata sulla firma del codice** (spec
   `2026-09-03-firma-notarizzazione.TODO.md`, a sua volta ferma
   sull'Apple Developer ID): un binario non firmato che si scarica e
   sostituisce se stesso è un rischio di sicurezza più serio di uno
   semplicemente non firmato all'installazione — va fatto insieme, non
   prima. Nessuna implementazione possibile finché quella catena non
   si sblocca.

## Modifiche (una volta risolte le domande)

- `npm install @tauri-apps/plugin-updater @tauri-apps/plugin-dialog`
  (quest'ultimo già presente) — nuova dipendenza, motivata da: plugin
  ufficiale Tauri, stessa famiglia di quelli già in uso.
- `src-tauri/Cargo.toml`: `tauri-plugin-updater`.
- `tauri.conf.json` → `plugins.updater`: endpoint del manifest, chiave
  pubblica di verifica.
- `src-tauri/src/lib.rs`: registrazione del plugin (stesso pattern di
  `plugin-opener`/`plugin-dialog` già presenti).
- UI minima: un controllo automatico all'avvio (silenzioso se
  nessun aggiornamento) + un banner/notifica quando ce n'è uno,
  coerente con lo stile già in uso per gli altri banner
  (`.banner-warning`/`.banner-error`) — nessun dialog invasivo.

## Fuori scope

- Aggiornamenti differenziali/delta: il plugin scarica l'intero
  pacchetto, sufficiente per un'app di queste dimensioni.
- Canale beta/nightly separato dal canale stabile: un solo canale per
  ora.

## Verifica

Non applicabile finché la spec resta bloccata. Una volta implementata:
pubblicare una versione `0.1.1` fittizia, verificare che una build
`0.1.0` locale rilevi e installi l'aggiornamento.
