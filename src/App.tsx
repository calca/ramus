import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import faviconUrl from "../assets/favicon.svg";
import { DayNav } from "./components/DayNav";
import { Editor, type EditorHandle } from "./components/Editor";
import { isPageNotFoundError, openToday, readPage } from "./lib/commands";
import { journalRelativePath } from "./lib/journal";
import type { Page } from "./lib/types";

function emptyPage(relativePath: string): Page {
  return { path: relativePath, blocks: [] };
}

function App() {
  const [page, setPage] = useState<Page | null>(null);
  const [currentDate, setCurrentDate] = useState<Date>(new Date());
  const [dirty, setDirty] = useState(false);
  const [externalChangeWarning, setExternalChangeWarning] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const editorRef = useRef<EditorHandle | null>(null);
  const pageRef = useRef<Page | null>(null);
  const dirtyRef = useRef(false);

  useEffect(() => {
    pageRef.current = page;
  }, [page]);

  useEffect(() => {
    dirtyRef.current = dirty;
  }, [dirty]);

  // Apertura automatica del journal di oggi, senza schermate intermedie.
  useEffect(() => {
    void (async () => {
      try {
        const today = await openToday();
        setPage(today);
        setCurrentDate(new Date());
      } catch (error) {
        setLoadError(String(error));
      }
    })();
  }, []);

  // File watcher: ricarica silenziosamente se non ci sono modifiche
  // pendenti, altrimenti avvisa senza sovrascrivere nulla.
  useEffect(() => {
    const unlistenPromise = listen<string>("vault://file-changed", (event) => {
      const current = pageRef.current;
      if (!current || event.payload !== current.path) {
        return;
      }
      if (dirtyRef.current) {
        setExternalChangeWarning(true);
        return;
      }
      void readPage(current.path)
        .then((fresh) => setPage(fresh))
        .catch(() => {
          // Il file è stato rimosso esternamente: si lascia lo stato
          // attuale, l'utente lo scoprirà al prossimo salvataggio.
        });
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Flush su chiusura della finestra: non si perde nessuna modifica pendente.
  useEffect(() => {
    let allowClose = false;
    const win = getCurrentWindow();
    const unlistenPromise = win.onCloseRequested(async (event) => {
      if (allowClose) {
        return;
      }
      event.preventDefault();
      await editorRef.current?.flush();
      allowClose = true;
      await win.close();
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const navigateTo = useCallback(async (nextDate: Date) => {
    await editorRef.current?.flush();
    const relativePath = journalRelativePath(nextDate);
    try {
      const nextPage = await readPage(relativePath);
      setPage(nextPage);
    } catch (error) {
      if (isPageNotFoundError(error)) {
        setPage(emptyPage(relativePath));
      } else {
        setLoadError(String(error));
        return;
      }
    }
    setCurrentDate(nextDate);
    setDirty(false);
    setExternalChangeWarning(false);
  }, []);

  return (
    <div className="app">
      <header className="app-header">
        <img src={faviconUrl} alt="" className="app-logo" width={20} height={20} />
        <span className="app-title">Ramus</span>
        <DayNav currentDate={currentDate} onNavigate={(date) => void navigateTo(date)} />
      </header>

      {externalChangeWarning && (
        <div className="banner banner-warning">
          Questo file è cambiato su disco. Ci sono modifiche non salvate: non è stato ricaricato per non
          perderle.
        </div>
      )}
      {loadError && <div className="banner banner-error">{loadError}</div>}

      <main className="app-body">
        {page ? <Editor key={page.path} ref={editorRef} page={page} onDirtyChange={setDirty} /> : null}
      </main>
    </div>
  );
}

export default App;
