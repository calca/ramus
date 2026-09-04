# Firma del codice e notarizzazione (macOS + Windows)

Stato: proposta, bloccata. Risposta dell'utente alla prima domanda:
non ha ancora un Apple Developer ID — richiede registrazione/pagamento
presso Apple, fuori dal controllo di questa sessione. Le altre due
domande restano aperte, non ha senso deciderle prima della prima.

## Motivazione

`tauri.conf.json` → `bundle` non ha nessuna configurazione di firma.
Una build `.dmg`/`.app` non firmata viene bloccata o marcata "sviluppatore
non identificato" da Gatekeeper su macOS; un `.msi`/`.exe` non firmato
attiva l'avviso SmartScreen su Windows. Per una distribuzione reale
(anche solo a pochi utenti fidati fuori dalla propria macchina) questo
è il primo ostacolo che si incontra.

## Cosa serve, per piattaforma

**macOS**: un Apple Developer ID (account a pagamento, 99$/anno),
un certificato "Developer ID Application" generato da quell'account,
più le credenziali per la notarizzazione (Apple ID + app-specific
password, o una API key). Tauri firma e notarizza in automatico nel
bundler se queste sono presenti come variabili d'ambiente
(`APPLE_CERTIFICATE`, `APPLE_ID`, ecc. — vedi
https://v2.tauri.app/distribute/sign/macos/).

**Windows**: un certificato di code-signing (Standard o EV, da una CA
riconosciuta — DigiCert, Sectigo, ecc., a pagamento, EV richiede
verifica aziendale). Senza, SmartScreen resta un avviso "attenuabile"
solo con abbastanza reputazione accumulata nel tempo (Microsoft
SmartScreen si basa anche su volume di download, non solo sulla
firma) — per un progetto nuovo, la firma è l'unico modo di ridurlo da
subito.

**Linux**: nessun equivalente di Gatekeeper/SmartScreen per i formati
target di Tauri (`.deb`/`.AppImage`/`.rpm`) — nessuna azione
necessaria qui.

## Domande aperte (bloccanti)

1. ~~Hai già un Apple Developer ID?~~ **Risposto: no, non ancora.**
   Serve registrarsi (99$/anno) prima di poter procedere — quando
   fatto, questa spec riprende da qui.
2. **Vuoi firmare anche per Windows fin da subito**, o accettare
   l'avviso SmartScreen per ora (molti progetti open source piccoli
   partono così, lo tolgono quando hanno budget/utenti)? — non ancora
   chiesta: non ha senso deciderla prima della domanda 1.
3. **Chi/dove ospita le credenziali di firma?** (GitHub Actions
   secrets è l'opzione naturale, coerente con la CI già scritta in
   `specs/release/2026-09-03-ci.TODO.md`) — stessa cosa, in sospeso.

## Fuori scope

- Notarizzazione/firma per iOS/Android: `SPEC.md` esclude
  esplicitamente client mobile dagli obiettivi del progetto.
- Qualunque automazione finché le domande sopra non hanno risposta.

## Verifica

Non applicabile finché la spec resta bloccata sulle domande aperte.
Una volta risolte: build di release firmata, `spctl --assess` (macOS)
o verifica della firma in Explorer (Windows) confermano l'identità.
