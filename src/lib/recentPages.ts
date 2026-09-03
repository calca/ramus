// Pagine aperte di recente nella Command Palette: persistite in
// localStorage (stato del webview, non del vault — CLAUDE.md regola 3
// riguarda il filesystem del vault, non lo storage del browser), chiave
// per vault path così un cambio vault non mostra titoli di un altro.

const MAX_RECENT_PAGES = 10;

function storageKey(vaultPath: string): string {
  return `ramus:recent-pages:${vaultPath}`;
}

export function loadRecentPages(vaultPath: string): string[] {
  try {
    const raw = localStorage.getItem(storageKey(vaultPath));
    if (!raw) {
      return [];
    }
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((title): title is string => typeof title === "string") : [];
  } catch {
    return [];
  }
}

/** Sposta `title` in cima (o lo inserisce), deduplicato, capped a
 * MAX_RECENT_PAGES. Ritorna la nuova lista per aggiornare lo stato React
 * senza dover rileggere subito da localStorage. */
export function pushRecentPage(vaultPath: string, title: string): string[] {
  const next = [title, ...loadRecentPages(vaultPath).filter((existing) => existing !== title)].slice(
    0,
    MAX_RECENT_PAGES,
  );
  try {
    localStorage.setItem(storageKey(vaultPath), JSON.stringify(next));
  } catch {
    // localStorage non disponibile: i "recenti" restano solo in memoria
    // per questa sessione, nessun impatto funzionale.
  }
  return next;
}
