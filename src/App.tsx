import { useCallback, useEffect, useRef, useState } from "react";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import faviconUrl from "../assets/favicon.svg";
import { AboutPanel } from "./components/AboutPanel";
import { Cheatsheet } from "./components/Cheatsheet";
import { CommandPalette } from "./components/CommandPalette";
import type { PaletteItem } from "./components/CommandPalette";
import type { EditorHandle } from "./components/Editor";
import { JournalControls } from "./components/JournalControls";
import { JournalSection } from "./components/JournalSection";
import { PageView } from "./components/PageView";
import { SettingsPanel } from "./components/SettingsPanel";
import {
  getConfig,
  getSyncStatus,
  listJournals,
  openPage,
  openToday,
  readPage,
  rollOverUnfinishedTasks,
} from "./lib/commands";
import { formatIsoDate, journalDateFromPath } from "./lib/journal";
import { buildActions } from "./lib/paletteActions";
import { loadRecentPages, pushRecentPage } from "./lib/recentPages";
import { getShortcut, matchesShortcut } from "./lib/shortcut";
import { applyTheme } from "./lib/theme";
import type { Config, Page, SyncState } from "./lib/types";

const BATCH_SIZE = 14;
const COMPACT_WIDTH = 420;

/** Stesso intervallo del polling già usato da SettingsPanel quando il
 * pannello Sync è aperto — qui gira sempre, non solo a pannello aperto,
 * perché il badge nell'header deve aggiornarsi anche a Impostazioni
 * chiuse. */
const SYNC_STATE_POLL_MS = 30_000;

const SYNC_BADGE_LABELS: Partial<Record<SyncState, string>> = {
  noremote: "Sync locale attiva, nessun remote collegato",
  syncing: "Sincronizzazione in corso…",
  conflict: "Conflitto: sync automatica ferma, serve intervento manuale",
  offline: "Rete non raggiungibile, riprovo al prossimo giro",
};

/** Da chiamare prima di aprire/riaprire "oggi" (avvio, o rollover di
 * mezzanotte): sposta i task non fatti rimasti indietro, se l'utente ha
 * l'opzione attiva (Config.task_rollover_enabled — il command stesso è un
 * no-op se disattivata). Un fallimento qui non deve mai impedire
 * l'apertura del journal ("zero attrito all'avvio", SPEC.md) — errore
 * inghiottito in silenzio, non un banner per una comodità automatica di
 * sfondo che l'utente non ha richiesto esplicitamente in quel momento. */
async function tryRollOverUnfinishedTasks(): Promise<void> {
  try {
    await rollOverUnfinishedTasks();
  } catch {
    // Vedi commento sopra: si prosegue comunque ad aprire oggi.
  }
}

type View = { kind: "journal" } | { kind: "page"; page: Page };

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [activePanel, setActivePanel] = useState<
    "settings" | "about" | "palette" | "cheatsheet" | null
  >(null);
  const [syncState, setSyncState] = useState<SyncState | null>(null);
  const [isCompact, setIsCompact] = useState(false);
  const [isFocusMode, setIsFocusMode] = useState(false);
  const [view, setView] = useState<View>({ kind: "journal" });
  const [vaultVersion, setVaultVersion] = useState(0);
  const [pages, setPages] = useState<Page[]>([]);
  const [recentPages, setRecentPages] = useState<string[]>([]);
  const [dirtyPaths, setDirtyPaths] = useState<Set<string>>(new Set());
  const [externalWarnings, setExternalWarnings] = useState<Set<string>>(new Set());
  const [hasMore, setHasMore] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  const pagesRef = useRef<Page[]>([]);
  const dirtyRef = useRef<Set<string>>(new Set());
  const hasMoreRef = useRef(true);
  const loadingRef = useRef(false);
  const editorHandles = useRef(new Map<string, EditorHandle>());
  const sectionElements = useRef(new Map<string, HTMLElement>());
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const preCompactSizeRef = useRef<{ width: number; height: number } | null>(null);
  const viewRef = useRef<View>({ kind: "journal" });
  /** Path del giorno di journal col fuoco, per le scorciatoie di
   * navigazione fra giorni — un ref (non solo lo state) perché il
   * listener keydown globale ha bisogno del valore corrente senza dover
   * ricreare l'effetto ad ogni cambio di fuoco (stesso pattern già usato
   * per pagesRef/dirtyRef/hasMoreRef). */
  const focusedPathRef = useRef<string | null>(null);

  useEffect(() => {
    viewRef.current = view;
  }, [view]);

  useEffect(() => {
    pagesRef.current = pages;
  }, [pages]);

  useEffect(() => {
    dirtyRef.current = dirtyPaths;
  }, [dirtyPaths]);

  useEffect(() => {
    hasMoreRef.current = hasMore;
  }, [hasMore]);

  const registerEditorHandle = useCallback((path: string, handle: EditorHandle | null) => {
    if (handle) {
      editorHandles.current.set(path, handle);
    } else {
      editorHandles.current.delete(path);
    }
  }, []);

  const registerElement = useCallback((path: string, element: HTMLElement | null) => {
    if (element) {
      sectionElements.current.set(path, element);
    } else {
      sectionElements.current.delete(path);
    }
  }, []);

  const setDirty = useCallback((path: string, dirty: boolean) => {
    setDirtyPaths((prev) => {
      const next = new Set(prev);
      if (dirty) {
        next.add(path);
      } else {
        next.delete(path);
      }
      return next;
    });
  }, []);

  /** Richiede il blocco di giorni immediatamente precedenti all'ultimo
   * caricato, lo accoda, e aggiorna `hasMore`. Primitiva condivisa da
   * scroll infinito e salto a data. */
  const fetchNextBatch = useCallback(async (): Promise<Page[]> => {
    const current = pagesRef.current;
    const last = current[current.length - 1];
    if (!last) {
      return [];
    }
    const batch = await listJournals(journalDateFromPath(last.path), BATCH_SIZE);
    if (batch.length < BATCH_SIZE) {
      hasMoreRef.current = false;
      setHasMore(false);
    }
    if (batch.length > 0) {
      const next = [...current, ...batch];
      pagesRef.current = next;
      setPages(next);
    }
    return batch;
  }, []);

  /** Apre oggi e il primo blocco di giorni precedenti da zero: usato sia
   * all'avvio sia dopo un cambio vault (che può avere contenuto del tutto
   * diverso, non è una copia). */
  const resetAndLoadJournal = useCallback(async () => {
    try {
      await tryRollOverUnfinishedTasks();
      const today = await openToday();
      pagesRef.current = [today];
      setPages([today]);
      hasMoreRef.current = true;
      setHasMore(true);
      await fetchNextBatch();
    } catch (error) {
      setLoadError(String(error));
    }
  }, [fetchNextBatch]);

  const loadMore = useCallback(async () => {
    if (loadingRef.current || !hasMoreRef.current) {
      return;
    }
    loadingRef.current = true;
    try {
      await fetchNextBatch();
    } catch (error) {
      setLoadError(String(error));
    } finally {
      loadingRef.current = false;
    }
  }, [fetchNextBatch]);

  const scrollToPath = useCallback((path: string) => {
    sectionElements.current.get(path)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

  /** `true` se il primo giorno caricato non è più oggi (l'app è rimasta
   * aperta a cavallo di mezzanotte). */
  const needsNewDay = useCallback(() => {
    const first = pagesRef.current[0];
    return !first || journalDateFromPath(first.path) !== formatIsoDate(new Date());
  }, []);

  /** Se serve, apre il nuovo giorno (idempotente, crea il file se manca)
   * e lo antepone alla lista — nessuno scroll, nessun tocco alle sezioni
   * esistenti: il vecchio "oggi" resta dov'è, perde solo `isToday`
   * (derivato da `index === 0`, si sposta da solo col nuovo ordine). */
  const ensureToday = useCallback(async () => {
    if (!needsNewDay()) {
      return;
    }
    await tryRollOverUnfinishedTasks();
    const today = await openToday();
    const next = [today, ...pagesRef.current];
    pagesRef.current = next;
    setPages(next);
  }, [needsNewDay]);

  const scrollToToday = useCallback(async () => {
    await ensureToday();
    const today = pagesRef.current[0];
    if (today) {
      scrollToPath(today.path);
    }
  }, [ensureToday, scrollToPath]);

  const jumpToDate = useCallback(
    async (target: string) => {
      if (loadingRef.current) {
        return;
      }
      loadingRef.current = true;
      try {
        let found = pagesRef.current.find((p) => journalDateFromPath(p.path) <= target);
        while (!found && hasMoreRef.current) {
          const batch = await fetchNextBatch();
          if (batch.length === 0) {
            break;
          }
          found = batch.find((p) => journalDateFromPath(p.path) <= target);
        }
        const targetPage = found ?? pagesRef.current[pagesRef.current.length - 1];
        if (targetPage) {
          requestAnimationFrame(() => scrollToPath(targetPage.path));
        }
      } catch (error) {
        setLoadError(String(error));
      } finally {
        loadingRef.current = false;
      }
    },
    [fetchNextBatch, scrollToPath],
  );

  /** Scorciatoie journal_prev_day/journal_next_day: sposta scroll e fuoco
   * al giorno adiacente al giorno col fuoco attuale (o al primo giorno
   * caricato se nessun editor ce l'ha). "prev"/"next" nel senso della
   * lista — verso l'alto è il giorno più recente, coerente con "oggi in
   * cima" (M1). */
  const navigateJournalDay = useCallback(
    async (direction: "prev" | "next") => {
      const current = pagesRef.current;
      const first = current[0];
      if (!first) {
        return;
      }
      const activePath = focusedPathRef.current ?? first.path;
      let index = current.findIndex((p) => p.path === activePath);
      if (index === -1) {
        index = 0;
      }
      if (direction === "prev") {
        if (index === 0) {
          return;
        }
        index -= 1;
      } else {
        index += 1;
        if (index >= pagesRef.current.length) {
          if (!hasMoreRef.current) {
            return;
          }
          const batch = await fetchNextBatch();
          if (batch.length === 0) {
            return;
          }
        }
      }
      const target = pagesRef.current[index];
      if (!target) {
        return;
      }
      requestAnimationFrame(() => {
        scrollToPath(target.path);
        editorHandles.current.get(target.path)?.focus();
      });
    },
    [fetchNextBatch, scrollToPath],
  );

  // Config (tema incluso) e apertura automatica del journal di oggi,
  // senza schermate intermedie.
  useEffect(() => {
    void (async () => {
      try {
        const cfg = await getConfig();
        setConfig(cfg);
        applyTheme(cfg.theme);
      } catch (error) {
        setLoadError(String(error));
      }
      await resetAndLoadJournal();
    })();
  }, [resetAndLoadJournal]);

  // Pagine aperte di recente (Command Palette): persistite in
  // localStorage per vault, ricaricate quando il vault attivo cambia.
  useEffect(() => {
    if (!config) {
      return;
    }
    setRecentPages(loadRecentPages(config.vault_path));
  }, [config?.vault_path]);

  // Rollover a mezzanotte: due trigger. Il ritorno di focus/visibilità
  // copre il caso comune (si chiude il laptop la sera, si riapre il
  // giorno dopo); un controllo ogni 60s copre il caso raro in cui la
  // finestra resti a fuoco ininterrottamente attraverso la mezzanotte
  // (schermo sempre acceso) — costo trascurabile, solo un confronto di
  // stringhe quando non serve fare nulla.
  useEffect(() => {
    const onFocusOrVisible = () => {
      void ensureToday();
    };
    window.addEventListener("focus", onFocusOrVisible);
    document.addEventListener("visibilitychange", onFocusOrVisible);
    const interval = setInterval(() => void ensureToday(), 60_000);
    return () => {
      window.removeEventListener("focus", onFocusOrVisible);
      document.removeEventListener("visibilitychange", onFocusOrVisible);
      clearInterval(interval);
    };
  }, [ensureToday]);

  // Scroll infinito: quando la sentinella in fondo alla lista entra in
  // viewport, si carica il blocco successivo.
  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel) {
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          void loadMore();
        }
      },
      { rootMargin: "400px" },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loadMore]);

  // File watcher: ricarica silenziosamente la sezione toccata se non ha
  // modifiche pendenti, altrimenti avvisa senza sovrascrivere nulla.
  useEffect(() => {
    const unlistenPromise = listen<string>("vault://file-changed", (event) => {
      const path = event.payload;
      const inJournal = pagesRef.current.some((p) => p.path === path);
      const inPageView = viewRef.current.kind === "page" && viewRef.current.page.path === path;
      if (!inJournal && !inPageView) {
        return;
      }
      if (dirtyRef.current.has(path)) {
        setExternalWarnings((prev) => new Set(prev).add(path));
        return;
      }
      void readPage(path)
        .then((fresh) => {
          setPages((prev) => prev.map((p) => (p.path === path ? fresh : p)));
          if (viewRef.current.kind === "page" && viewRef.current.page.path === path) {
            setView({ kind: "page", page: fresh });
          }
          setExternalWarnings((prev) => {
            if (!prev.has(path)) {
              return prev;
            }
            const next = new Set(prev);
            next.delete(path);
            return next;
          });
        })
        .catch(() => {
          // Il file è stato rimosso esternamente: si lascia lo stato
          // attuale, l'utente lo scoprirà al prossimo salvataggio.
        });
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Traccia quale giorno di journal ha il fuoco (per journal_prev_day/
  // journal_next_day sotto) — focusin/focusout bubbling, delegato su
  // window invece di un listener per sezione: ogni JournalSection porta
  // già data-path sul proprio <section>, nessuna mappa element->path da
  // mantenere a parte. Solo focusin serve: focusout non aggiunge nulla,
  // l'ultimo giorno con fuoco resta il riferimento più utile anche se il
  // fuoco è temporaneamente altrove (es. un pannello aperto).
  useEffect(() => {
    const onFocusIn = (event: FocusEvent) => {
      const target = event.target as HTMLElement | null;
      const path = target?.closest<HTMLElement>(".journal-section")?.dataset.path;
      if (path) {
        focusedPathRef.current = path;
      }
    };
    window.addEventListener("focusin", onFocusIn);
    return () => window.removeEventListener("focusin", onFocusIn);
  }, []);

  // Scorciatoie app-level configurabili (Impostazioni, vedi
  // src/lib/shortcut.ts): un listener globale, non legato al focus di un
  // elemento specifico. Ogni azione del registro apre il proprio pannello
  // o esegue la propria azione.
  useEffect(() => {
    if (!config) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (matchesShortcut(event, getShortcut(config.shortcuts, "command_palette"))) {
        event.preventDefault();
        setActivePanel("palette");
      } else if (matchesShortcut(event, getShortcut(config.shortcuts, "cheatsheet"))) {
        event.preventDefault();
        setActivePanel("cheatsheet");
      } else if (matchesShortcut(event, getShortcut(config.shortcuts, "focus_mode"))) {
        event.preventDefault();
        setIsFocusMode((prev) => !prev);
      } else if (
        viewRef.current.kind === "journal" &&
        matchesShortcut(event, getShortcut(config.shortcuts, "journal_prev_day"))
      ) {
        event.preventDefault();
        void navigateJournalDay("prev");
      } else if (
        viewRef.current.kind === "journal" &&
        matchesShortcut(event, getShortcut(config.shortcuts, "journal_next_day"))
      ) {
        event.preventDefault();
        void navigateJournalDay("next");
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [config, navigateJournalDay]);

  // Badge di stato sync (M3): visibile solo quando c'è qualcosa da dire
  // (mai per "disabled"/"idle", vedi render sotto) — un polling leggero,
  // stesso intervallo già usato da SettingsPanel per lo stesso comando.
  useEffect(() => {
    const refresh = () => {
      void getSyncStatus()
        .then((status) => setSyncState(status.state))
        .catch(() => {
          // Nessun banner per un polling di sfondo che fallisce: il badge
          // semplicemente non si aggiorna questo giro, riprova al prossimo.
        });
    };
    refresh();
    const interval = setInterval(refresh, SYNC_STATE_POLL_MS);
    return () => clearInterval(interval);
  }, []);

  // Flush su chiusura della finestra: nessuna sezione aperta perde modifiche.
  useEffect(() => {
    let allowClose = false;
    const win = getCurrentWindow();
    const unlistenPromise = win.onCloseRequested(async (event) => {
      if (allowClose) {
        return;
      }
      event.preventDefault();
      await Promise.all(Array.from(editorHandles.current.values(), (handle) => handle.flush()));
      allowClose = true;
      await win.close();
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const handleVaultChanged = useCallback(
    (nextConfig: Config) => {
      setConfig(nextConfig);
      setActivePanel(null);
      // La pagina eventualmente aperta appartiene al vault vecchio: non ha
      // più senso restare a guardarla.
      setView({ kind: "journal" });
      // Il vault nuovo può avere un contenuto completamente diverso (non è
      // una copia): si scarta tutto lo stato della vista journal e si
      // ricomincia da capo. vaultVersion forza il remount delle sezioni
      // anche per il giorno di oggi, il cui path relativo (dipende solo
      // dalla data) resterebbe altrimenti identico fra un vault e l'altro.
      setVaultVersion((v) => v + 1);
      setDirtyPaths(new Set());
      dirtyRef.current = new Set();
      setExternalWarnings(new Set());
      editorHandles.current.clear();
      sectionElements.current.clear();
      void resetAndLoadJournal();
    },
    [resetAndLoadJournal],
  );

  const handleThemeChanged = useCallback((nextConfig: Config) => {
    setConfig(nextConfig);
  }, []);

  const handleShortcutChanged = useCallback((nextConfig: Config) => {
    setConfig(nextConfig);
  }, []);

  const handleGitSyncIntervalChanged = useCallback((nextConfig: Config) => {
    setConfig(nextConfig);
  }, []);

  const handleTaskRolloverChanged = useCallback((nextConfig: Config) => {
    setConfig(nextConfig);
  }, []);

  const handleMcpEnabledChanged = useCallback((nextConfig: Config) => {
    setConfig(nextConfig);
  }, []);

  /** Apre (creando se manca) la pagina cliccata e ci passa la vista. Flush
   * di tutti gli editor montati prima di navigare, stesso Promise.all già
   * usato alla chiusura finestra. */
  const navigateToPage = useCallback(
    async (title: string) => {
      await Promise.all(Array.from(editorHandles.current.values(), (handle) => handle.flush()));
      try {
        const page = await openPage(title);
        setView({ kind: "page", page });
        if (config) {
          setRecentPages(pushRecentPage(config.vault_path, page.title ?? title));
        }
      } catch (error) {
        setLoadError(String(error));
      }
    },
    [config],
  );

  const returnToJournal = useCallback(async () => {
    await Promise.all(Array.from(editorHandles.current.values(), (handle) => handle.flush()));
    setView({ kind: "journal" });
  }, []);

  /** Selezione di una voce della Command Palette: un'azione si esegue e
   * basta; una pagina (recente, risultato o "crea") naviga come un
   * [[link]] cliccato (stessa navigateToPage); un giorno di journal usa
   * la stessa jumpToDate già usata da "salta a data" — il giorno esiste
   * per certo (viene dall'indice o è già aperto), il match è sempre
   * esatto. */
  const handlePaletteSelect = useCallback(
    async (item: PaletteItem) => {
      setActivePanel(null);
      if (item.kind === "action") {
        item.action.run();
        return;
      }
      if (item.kind === "hit" && item.hit.kind === "journal") {
        if (viewRef.current.kind === "page") {
          await returnToJournal();
        }
        await jumpToDate(journalDateFromPath(item.hit.path));
        return;
      }
      const title =
        item.kind === "hit" ? (item.hit.title ?? item.hit.path.replace(/^pages\//, "").replace(/\.md$/, "")) : item.title;
      await navigateToPage(title);
    },
    [navigateToPage, returnToJournal, jumpToDate],
  );

  /** Restringe la finestra a COMPACT_WIDTH per affiancarla a un'altra
   * finestra (note, appunti), memorizzando la dimensione attuale per
   * ripristinarla esattamente all'uscita — non un default fisso. Solo
   * la larghezza cambia, l'altezza resta quella dell'utente. Il bordo
   * destro resta fisso in entrambe le direzioni (si sposta la X): comprimere
   * si restringe verso l'interno, espandere cresce verso l'interno dello
   * schermo invece di uscire dal bordo destro del monitor. Non persistita:
   * è una preferenza di sessione. */
  const toggleCompact = useCallback(async () => {
    const win = getCurrentWindow();
    const scale = await win.scaleFactor();
    const currentSize = (await win.outerSize()).toLogical(scale);
    const currentPos = (await win.outerPosition()).toLogical(scale);
    const rightEdge = currentPos.x + currentSize.width;

    if (!isCompact) {
      preCompactSizeRef.current = { width: currentSize.width, height: currentSize.height };
      await win.setSize(new LogicalSize(COMPACT_WIDTH, currentSize.height));
      await win.setPosition(new LogicalPosition(rightEdge - COMPACT_WIDTH, currentPos.y));
      setIsCompact(true);
    } else {
      const restore = preCompactSizeRef.current;
      if (restore) {
        await win.setSize(new LogicalSize(restore.width, restore.height));
        await win.setPosition(new LogicalPosition(rightEdge - restore.width, currentPos.y));
      }
      setIsCompact(false);
    }
  }, [isCompact]);

  return (
    <div className={isFocusMode ? "app is-focus" : "app"}>
      <header
        className={isCompact ? "app-header is-compact" : "app-header"}
        data-tauri-drag-region="true"
      >
        <img src={faviconUrl} alt="" className="app-logo" width={20} height={20} />
        <span className="app-title">Ramus</span>
        <button
          type="button"
          className="settings-button compact-toggle"
          aria-label={isCompact ? "Espandi finestra" : "Comprimi finestra"}
          title={isCompact ? "Espandi finestra" : "Comprimi finestra"}
          onClick={() => void toggleCompact()}
        >
          {isCompact ? "«" : "»"}
        </button>
        {config && (
          <button
            type="button"
            className="settings-button"
            aria-label="Comandi"
            onClick={() => setActivePanel("palette")}
          >
            ⚡
          </button>
        )}
        {config && (
          <button
            type="button"
            className="settings-button"
            aria-label="Impostazioni"
            onClick={() => setActivePanel("settings")}
          >
            ⚙
          </button>
        )}
      </header>

      {loadError && <div className="banner banner-error">{loadError}</div>}

      <main className="app-body" style={view.kind === "journal" ? undefined : { display: "none" }}>
        {pages.map((page, index) => (
          <JournalSection
            key={`${vaultVersion}:${page.path}`}
            page={page}
            isToday={index === 0}
            externalChangeWarning={externalWarnings.has(page.path)}
            onDirtyChange={setDirty}
            onLinkClick={(title) => void navigateToPage(title)}
            registerElement={registerElement}
            registerEditorHandle={registerEditorHandle}
          />
        ))}
        {hasMore && <div ref={sentinelRef} className="journal-sentinel" />}
      </main>

      {view.kind === "page" && (
        <PageView
          page={view.page}
          onDirtyChange={(dirty) => setDirty(view.page.path, dirty)}
          onLinkClick={(title) => void navigateToPage(title)}
          onBack={() => void returnToJournal()}
          registerEditorHandle={(handle) => registerEditorHandle(view.page.path, handle)}
        />
      )}

      <div className={isCompact ? "app-statusbar is-compact" : "app-statusbar"}>
        {view.kind === "journal" && (
          <JournalControls
            onToday={() => void scrollToToday()}
            onJumpToDate={(iso) => void jumpToDate(iso)}
          />
        )}
        {syncState && !["disabled", "idle"].includes(syncState) && (
          <button
            type="button"
            className={
              syncState === "conflict" ? "sync-badge is-conflict" : "sync-badge"
            }
            aria-label={SYNC_BADGE_LABELS[syncState]}
            title={SYNC_BADGE_LABELS[syncState]}
            onClick={() => setActivePanel("settings")}
          >
            {syncState === "conflict" ? "⚠" : "⇄"}
          </button>
        )}
      </div>

      {activePanel === "settings" && config && (
        <SettingsPanel
          config={config}
          onClose={() => setActivePanel(null)}
          onVaultChanged={handleVaultChanged}
          onThemeChanged={handleThemeChanged}
          onShortcutChanged={handleShortcutChanged}
          onGitSyncIntervalChanged={handleGitSyncIntervalChanged}
          onTaskRolloverChanged={handleTaskRolloverChanged}
          onMcpEnabledChanged={handleMcpEnabledChanged}
          onShowAbout={() => setActivePanel("about")}
        />
      )}
      {activePanel === "about" && <AboutPanel onClose={() => setActivePanel(null)} />}
      {activePanel === "palette" && (
        <CommandPalette
          actions={buildActions({
            viewKind: view.kind,
            isCompact,
            onToday: () => void scrollToToday(),
            onReturnToJournal: () => void returnToJournal(),
            onToggleCompact: () => void toggleCompact(),
            onOpenSettings: () => setActivePanel("settings"),
            onShowAbout: () => setActivePanel("about"),
            onShowCheatsheet: () => setActivePanel("cheatsheet"),
          })}
          recentPages={recentPages}
          onClose={() => setActivePanel(null)}
          onSelect={(item) => void handlePaletteSelect(item)}
        />
      )}
      {activePanel === "cheatsheet" && config && (
        <Cheatsheet config={config} onClose={() => setActivePanel(null)} />
      )}
    </div>
  );
}

export default App;
