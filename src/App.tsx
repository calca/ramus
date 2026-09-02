import { useCallback, useEffect, useRef, useState } from "react";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import faviconUrl from "../assets/favicon.svg";
import { AboutPanel } from "./components/AboutPanel";
import type { EditorHandle } from "./components/Editor";
import { JournalControls } from "./components/JournalControls";
import { JournalSection } from "./components/JournalSection";
import { PageView } from "./components/PageView";
import { SearchPanel } from "./components/SearchPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { getConfig, listJournals, openPage, openToday, readPage } from "./lib/commands";
import { journalDateFromPath } from "./lib/journal";
import { matchesShortcut } from "./lib/shortcut";
import { applyTheme } from "./lib/theme";
import type { Config, Page, SearchHit } from "./lib/types";

const BATCH_SIZE = 14;
const COMPACT_WIDTH = 420;

type View = { kind: "journal" } | { kind: "page"; page: Page };

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [activePanel, setActivePanel] = useState<"settings" | "about" | "search" | null>(null);
  const [isCompact, setIsCompact] = useState(false);
  const [view, setView] = useState<View>({ kind: "journal" });
  const [vaultVersion, setVaultVersion] = useState(0);
  const [pages, setPages] = useState<Page[]>([]);
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

  const scrollToToday = useCallback(() => {
    const today = pagesRef.current[0];
    if (today) {
      scrollToPath(today.path);
    }
  }, [scrollToPath]);

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

  // Scorciatoia per aprire il pannello di ricerca (configurabile in
  // Impostazioni, vedi src/lib/shortcut.ts): un listener globale, non
  // legato al focus di un elemento specifico.
  useEffect(() => {
    if (!config) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (matchesShortcut(event, config.search_shortcut)) {
        event.preventDefault();
        setActivePanel("search");
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [config]);

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

  const handleSearchShortcutChanged = useCallback((nextConfig: Config) => {
    setConfig(nextConfig);
  }, []);

  /** Apre (creando se manca) la pagina cliccata e ci passa la vista. Flush
   * di tutti gli editor montati prima di navigare, stesso Promise.all già
   * usato alla chiusura finestra. */
  const navigateToPage = useCallback(async (title: string) => {
    await Promise.all(Array.from(editorHandles.current.values(), (handle) => handle.flush()));
    try {
      const page = await openPage(title);
      setView({ kind: "page", page });
    } catch (error) {
      setLoadError(String(error));
    }
  }, []);

  const returnToJournal = useCallback(async () => {
    await Promise.all(Array.from(editorHandles.current.values(), (handle) => handle.flush()));
    setView({ kind: "journal" });
  }, []);

  /** Selezione di un risultato di ricerca: una pagina naviga come un
   * [[link]] cliccato (stessa navigateToPage); un giorno di journal usa
   * la stessa jumpToDate già usata da "salta a data" — il giorno esiste
   * per certo (viene dall'indice), il match è sempre esatto. */
  const handleSearchSelect = useCallback(
    async (hit: SearchHit) => {
      setActivePanel(null);
      if (hit.kind === "page") {
        const fallback = hit.path.replace(/^pages\//, "").replace(/\.md$/, "");
        await navigateToPage(hit.title ?? fallback);
      } else {
        if (viewRef.current.kind === "page") {
          await returnToJournal();
        }
        await jumpToDate(journalDateFromPath(hit.path));
      }
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
    <div className="app">
      <header
        className={isCompact ? "app-header is-compact" : "app-header"}
        data-tauri-drag-region="true"
      >
        <img src={faviconUrl} alt="" className="app-logo" width={20} height={20} />
        <span className="app-title">Ramus</span>
        {view.kind === "journal" && (
          <JournalControls onToday={scrollToToday} onJumpToDate={(iso) => void jumpToDate(iso)} />
        )}
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
            aria-label="Cerca"
            onClick={() => setActivePanel("search")}
          >
            🔍
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

      {activePanel === "settings" && config && (
        <SettingsPanel
          config={config}
          onClose={() => setActivePanel(null)}
          onVaultChanged={handleVaultChanged}
          onThemeChanged={handleThemeChanged}
          onSearchShortcutChanged={handleSearchShortcutChanged}
          onShowAbout={() => setActivePanel("about")}
        />
      )}
      {activePanel === "about" && <AboutPanel onClose={() => setActivePanel(null)} />}
      {activePanel === "search" && (
        <SearchPanel
          onClose={() => setActivePanel(null)}
          onSelect={(hit) => void handleSearchSelect(hit)}
        />
      )}
    </div>
  );
}

export default App;
