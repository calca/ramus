// Lista fissa di azioni dell'app eseguibili dalla Command Palette (M4).
// Non configurabile, deliberatamente poche voci: solo ciò a cui serve
// davvero un accesso rapido da tastiera.
//
// Non un componente React: le etichette si risolvono con l'API imperativa
// i18next.t() (vedi src/i18n/index.ts) dentro buildActions(), non con
// useTranslation() — e non in un array costruito una volta sola a livello
// di modulo, altrimenti un cambio di lingua non aggiornerebbe queste
// etichette (buildActions() viene richiamata ad ogni render di App.tsx
// mentre la palette è aperta, quindi resta comunque reattiva).

import i18n from "../i18n";

export interface PaletteAction {
  id: string;
  label: string;
  run: () => void;
}

export interface PaletteActionContext {
  viewKind: "journal" | "page";
  isCompact: boolean;
  onToday: () => void;
  onReturnToJournal: () => void;
  onToggleCompact: () => void;
  onOpenSettings: () => void;
  onShowAbout: () => void;
  onShowCheatsheet: () => void;
  onShowOpenTasks: () => void;
}

export function buildActions(ctx: PaletteActionContext): PaletteAction[] {
  const actions: PaletteAction[] = [];
  if (ctx.viewKind === "journal") {
    actions.push({ id: "today", label: i18n.t("palette.action.today"), run: ctx.onToday });
  } else {
    actions.push({
      id: "return-journal",
      label: i18n.t("palette.action.returnToJournal"),
      run: ctx.onReturnToJournal,
    });
  }
  actions.push({
    id: "toggle-compact",
    label: ctx.isCompact ? i18n.t("app.compact.expand") : i18n.t("app.compact.collapse"),
    run: ctx.onToggleCompact,
  });
  actions.push({ id: "settings", label: i18n.t("settings.title"), run: ctx.onOpenSettings });
  actions.push({ id: "about", label: i18n.t("palette.action.about"), run: ctx.onShowAbout });
  actions.push({
    id: "cheatsheet",
    label: i18n.t("actions.cheatsheet.label"),
    run: ctx.onShowCheatsheet,
  });
  // Stesso id di SHORTCUT_ACTIONS ("open_tasks", non "open-tasks" come le
  // altre azioni qui): getShortcut fa un confronto esatto sull'id, serve
  // per mostrare la scorciatoia accanto all'etichetta nella palette
  // (CommandPalette.tsx, stesso meccanismo già usato per "cheatsheet").
  actions.push({ id: "open_tasks", label: i18n.t("tasks.title"), run: ctx.onShowOpenTasks });
  return actions;
}
