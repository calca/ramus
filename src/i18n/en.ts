// English dictionary — mirrors the shape of it.ts key for key (i18next
// falls back to this when a key is missing from the active language, and
// vitest's t() calls resolve against whichever language is active).

const en = {
  common: {
    close: "Close",
    loading: "Loading…",
    createTitled: 'Create "{{title}}"',
  },
  app: {
    commands: "Commands",
    compact: {
      expand: "Expand window",
      collapse: "Collapse window",
    },
    sync: {
      noremote: "Local sync active, no remote connected",
      syncing: "Syncing…",
      conflict: "Conflict: automatic sync stopped, manual action needed",
      offline: "Network unreachable, will retry next cycle",
    },
  },
  actions: {
    commandPalette: { label: "Open command palette" },
    cheatsheet: { label: "Show shortcuts" },
    focusMode: { label: "Focus mode (hide everything but the editor)" },
    journalPrevDay: { label: "Previous journal day" },
    journalNextDay: { label: "Next journal day" },
  },
  settings: {
    title: "Settings",
    sections: {
      vault: "Vault",
      theme: "Theme",
      locale: "Language",
      shortcuts: "Shortcuts",
      task: "Tasks",
      mcp: "MCP",
      sync: "Sync",
      about: "About",
    },
    vault: {
      confirmChange: "This will open the vault at {{path}}. Continue?",
      change: "Change",
      openInFileManager: "Open in file manager",
      stats: "{{journals}} journals, {{pages}} pages",
    },
    theme: {
      light: "Light",
      dark: "Dark",
      system: "System",
    },
    locale: {
      it: "Italiano",
      en: "English",
      system: "System",
    },
    shortcuts: {
      recording: "Press a combination…",
    },
    task: {
      rolloverToggle: "Automatically move unfinished tasks left behind to today",
      rolloverDaysLabel: "Look back",
      days_one: "{{count}} day",
      days_other: "{{count}} days",
    },
    mcp: {
      enable: "Enable MCP server",
      help: {
        pasteSnippet:
          "Paste it into <0>.mcp.json</0> (Claude Code) or <1>claude_desktop_config.json</1> (Claude Desktop). Restart the client after a change.",
        notFound:
          "<0>ramus-mcp</0> binary not found — run <1>cargo build -p ramus-mcp</1> and reopen this section.",
        disabled: "The MCP server refuses to start until you re-enable it here.",
      },
    },
    sync: {
      intro:
        "Version the vault with Git — even just locally, without a remote repository, it protects against a bad write: every change becomes a recoverable commit. Leave the field empty for this (no account, no external service), or paste a repository URL to sync it across devices too.",
      conflictBanner:
        "The local and remote vault have diverged, manual action is needed: open a terminal in the vault and resolve it with git.",
      status: {
        conflict: "Conflict: automatic sync stopped",
        offline: "Network unreachable, will retry next cycle",
        syncing: "Syncing…",
        dirty: "Changes waiting for the next automatic commit",
        clean: "Everything is synced",
      },
      lastCommit: " — last commit {{datetime}}",
      remotePlaceholder: "git@github.com:user/vault.git (optional)",
      action: {
        activate: "Enable sync",
        connect: "Connect remote",
        update: "Update remote",
      },
      help: 'On GitHub, GitLab or Bitbucket: open the repository, press "Code" (or "Clone"), copy the SSH URL (recommended, requires a key already added to your account) or HTTPS, and paste it above.',
      intervalLabel: "Sync interval",
      minutes_one: "{{count}} minute",
      minutes_other: "{{count}} minutes",
    },
    about: {
      mascotteAlt: "Stecco, Ramus's mascot",
      tagline: "Desktop journaling app, block outliner on local markdown files.",
      sourceCode: "Source code",
      version: "v{{version}}",
    },
  },
  palette: {
    searchPlaceholder: "Search, create, or run a command…",
    section: {
      actions: "Actions",
      recent: "Recent",
      results: "Results",
      create: "Create",
      date: "Date",
    },
    goToDate: "Go to {{date}}",
    noResults: 'No results for "{{query}}"',
    action: {
      today: "Go to today",
      returnToJournal: "Back to journal",
      about: "About Ramus",
    },
  },
  cheatsheet: {
    title: "Shortcuts",
    section: {
      app: "App",
      editor: "Editor",
    },
    editor: {
      newBlock: "New block",
      indent: "Indent",
      outdent: "Outdent",
      exitLevel: "Exit one level (empty block)",
      moveUp: "Move block up",
      moveDown: "Move block down",
      cycleTask: "Cycle task (normal → to do → done)",
    },
  },
  tasks: {
    title: "Open tasks",
    empty: "No open tasks.",
  },
  journal: {
    externalChangeWarning:
      "This file changed on disk. There are unsaved changes: it was not reloaded to avoid losing them.",
    today: "Today",
    yesterday: "Yesterday",
    daysAgo_one: "{{count}} day ago",
    daysAgo_other: "{{count}} days ago",
  },
  pageView: {
    backToJournal: "← Journal",
  },
  backlinks: {
    title: "Backlinks",
  },
};

export default en;
