// Dizionario italiano — un solo namespace (default di i18next), il progetto
// non è abbastanza grande da giustificarne più d'uno. Convenzione chiavi:
// namespace.sotto_area.nome (es. settings.sync.intro, palette.action.today).
// Testo verbatim rispetto a quello che era hardcoded nei componenti prima
// di questa spec — nessuna riformulazione, solo estrazione.

const it = {
  common: {
    close: "Chiudi",
    loading: "Caricamento…",
    createTitled: "Crea «{{title}}»",
  },
  app: {
    commands: "Comandi",
    compact: {
      expand: "Espandi finestra",
      collapse: "Comprimi finestra",
    },
    sync: {
      noremote: "Sync locale attiva, nessun remote collegato",
      syncing: "Sincronizzazione in corso…",
      conflict: "Conflitto: sync automatica ferma, serve intervento manuale",
      offline: "Rete non raggiungibile, riprovo al prossimo giro",
    },
  },
  actions: {
    commandPalette: { label: "Apri command palette" },
    cheatsheet: { label: "Mostra scorciatoie" },
    focusMode: { label: "Focus mode (nascondi tutto tranne l'editor)" },
    journalPrevDay: { label: "Giorno precedente del journal" },
    journalNextDay: { label: "Giorno successivo del journal" },
  },
  settings: {
    title: "Impostazioni",
    sections: {
      vault: "Vault",
      theme: "Tema",
      locale: "Lingua",
      shortcuts: "Scorciatoie",
      task: "Task",
      mcp: "MCP",
      sync: "Sync",
      about: "Informazioni",
    },
    vault: {
      confirmChange: "Apro il vault in {{path}}. Procedere?",
      change: "Cambia",
      openInFileManager: "Apri nel file manager",
      stats: "{{journals}} journal, {{pages}} pagine",
    },
    theme: {
      light: "Chiaro",
      dark: "Scuro",
      system: "Sistema",
    },
    locale: {
      it: "Italiano",
      en: "English",
      system: "Sistema",
    },
    shortcuts: {
      recording: "Premi una combinazione…",
    },
    task: {
      rolloverToggle: "Sposta automaticamente a oggi i task non fatti rimasti indietro",
      rolloverDaysLabel: "Considera gli ultimi",
      days_one: "{{count}} giorno",
      days_other: "{{count}} giorni",
    },
    mcp: {
      enable: "Abilita server MCP",
      help: {
        pasteSnippet:
          "Incollalo in <0>.mcp.json</0> (Claude Code) o <1>claude_desktop_config.json</1> (Claude Desktop). Riavvia il client dopo una modifica.",
        notFound:
          "Binario <0>ramus-mcp</0> non trovato — esegui <1>cargo build -p ramus-mcp</1> e riapri questa sezione.",
        disabled: "Il server MCP si rifiuta di avviarsi finché non lo riattivi qui.",
      },
    },
    sync: {
      intro:
        "Versiona il vault con Git — anche solo in locale, senza un repository remoto, protegge da una scrittura andata male: ogni modifica diventa un commit recuperabile. Lascia il campo vuoto per questo (nessun account, nessun servizio esterno), oppure incolla l'URL di un repository per sincronizzarlo anche fra dispositivi.",
      conflictBanner:
        "Il vault locale e quello remoto sono divergenti, serve intervento manuale: apri un terminale nel vault e risolvi con git.",
      status: {
        conflict: "Conflitto: sync automatica ferma",
        offline: "Rete non raggiungibile, riprovo al prossimo giro",
        syncing: "Sincronizzazione in corso…",
        dirty: "Modifiche in attesa del prossimo commit automatico",
        clean: "Tutto sincronizzato",
      },
      lastCommit: " — ultimo commit {{datetime}}",
      remotePlaceholder: "git@github.com:utente/vault.git (opzionale)",
      action: {
        activate: "Attiva sync",
        connect: "Collega remote",
        update: "Aggiorna remote",
      },
      help: 'Su GitHub, GitLab o Bitbucket: apri il repository, premi "Code" (o "Clone"), copia l\'URL SSH (consigliato, richiede una chiave già aggiunta al tuo account) o HTTPS, e incollalo qui sopra.',
      intervalLabel: "Intervallo di sync",
      minutes_one: "{{count}} minuto",
      minutes_other: "{{count}} minuti",
    },
    about: {
      mascotteAlt: "Stecco, la mascotte di Ramus",
      tagline: "App desktop di journaling, outliner a blocchi su file markdown locali.",
      sourceCode: "Codice sorgente",
      version: "v{{version}}",
    },
  },
  palette: {
    searchPlaceholder: "Cerca, crea o esegui un comando…",
    section: {
      actions: "Azioni",
      recent: "Recenti",
      results: "Risultati",
      create: "Crea",
      date: "Data",
    },
    goToDate: "Vai al {{date}}",
    noResults: "Nessun risultato per «{{query}}»",
    action: {
      today: "Vai a oggi",
      returnToJournal: "Torna al journal",
      about: "Informazioni su Ramus",
    },
  },
  cheatsheet: {
    title: "Scorciatoie",
    section: {
      app: "App",
      editor: "Editor",
    },
    editor: {
      newBlock: "Nuovo blocco",
      indent: "Indenta",
      outdent: "Rimuovi indentazione",
      exitLevel: "Esci di un livello (blocco vuoto)",
      moveUp: "Sposta blocco su",
      moveDown: "Sposta blocco giù",
      cycleTask: "Ciclo task (normale → da fare → fatto)",
    },
  },
  tasks: {
    title: "Task aperti",
    empty: "Nessun task aperto.",
  },
  journal: {
    externalChangeWarning:
      "Questo file è cambiato su disco. Ci sono modifiche non salvate: non è stato ricaricato per non perderle.",
    today: "Oggi",
    yesterday: "Ieri",
    daysAgo_one: "{{count}} giorno fa",
    daysAgo_other: "{{count}} giorni fa",
  },
  pageView: {
    backToJournal: "← Journal",
  },
  backlinks: {
    title: "Backlink",
  },
  errors: {
    invalid_path: "Percorso non valido all'interno del vault: {{path}}",
    page_not_found: "Pagina non trovata: {{path}}",
    invalid_date: "Data non valida, atteso formato YYYY-MM-DD: {{date}}",
    malformed_block: "Riga malformata nel blocco {{line}}: {{reason}}",
    io: "Errore di I/O su {{path}}: {{detail}}",
    config_error: "Errore di configurazione: {{detail}}",
    poisoned_config_lock: "Stato di configurazione corrotto",
    poisoned_index_lock: "Stato dell'indice corrotto",
    poisoned_search_index_lock: "Stato dell'indice di ricerca corrotto",
    poisoned_watcher_lock: "Stato del watcher corrotto",
    index_error: "Errore di indice: {{detail}}",
    search_error: "Errore di ricerca: {{detail}}",
    git_error: "Errore Git: {{detail}}",
  },
};

export default it;
