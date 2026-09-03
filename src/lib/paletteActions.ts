// Lista fissa di azioni dell'app eseguibili dalla Command Palette (M4).
// Non configurabile, cinque voci deliberatamente poche: solo ciò a cui
// serve davvero un accesso rapido da tastiera.

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
}

export function buildActions(ctx: PaletteActionContext): PaletteAction[] {
  const actions: PaletteAction[] = [];
  if (ctx.viewKind === "journal") {
    actions.push({ id: "today", label: "Vai a oggi", run: ctx.onToday });
  } else {
    actions.push({ id: "return-journal", label: "Torna al journal", run: ctx.onReturnToJournal });
  }
  actions.push({
    id: "toggle-compact",
    label: ctx.isCompact ? "Espandi finestra" : "Comprimi finestra",
    run: ctx.onToggleCompact,
  });
  actions.push({ id: "settings", label: "Impostazioni", run: ctx.onOpenSettings });
  actions.push({ id: "about", label: "Informazioni su Ramus", run: ctx.onShowAbout });
  actions.push({ id: "cheatsheet", label: "Mostra scorciatoie", run: ctx.onShowCheatsheet });
  return actions;
}
