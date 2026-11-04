import { useCallback, useEffect, useState } from "react";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PasswordDialog } from "./components/PasswordDialog";
import { PageList } from "./components/PageList";
import { SearchBar } from "./components/SearchBar";
import { Sidebar } from "./components/Sidebar";
import { StartScreen } from "./components/StartScreen";
import { TabBar } from "./components/TabBar";
import { Toolbar } from "./components/Toolbar";
import { activeTab, fileTitle, useDocuments, type SidebarPanel } from "./state/document";
import { S } from "./strings/es";
import { useTheme } from "./state/theme";

export function App() {
  const tab = useDocuments(activeTab);
  const tabs = useDocuments((s) => s.tabs);
  const openPath = useDocuments((s) => s.openPath);
  const closeTab = useDocuments((s) => s.closeTab);
  const setActiveTab = useDocuments((s) => s.setActiveTab);
  const goToPage = useDocuments((s) => s.goToPage);
  const zoomIn = useDocuments((s) => s.zoomIn);
  const zoomOut = useDocuments((s) => s.zoomOut);
  const setZoomMode = useDocuments((s) => s.setZoomMode);
  const openSearch = useDocuments((s) => s.openSearch);
  const closeSearch = useDocuments((s) => s.closeSearch);
  const saveTab = useDocuments((s) => s.saveTab);
  const tool = useDocuments((s) => s.tool);
  const setTool = useDocuments((s) => s.setTool);
  const theme = useTheme((s) => s.theme);
  const [sidebar, setSidebar] = useState<SidebarPanel>(null);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  const abrirDocumento = useCallback(async () => {
    const seleccion = await open({
      multiple: false,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof seleccion !== "string") return;
    try {
      await openPath(seleccion);
    } catch {
      // El error ya quedó reflejado en el estado del tab o en el prompt de
      // contraseña; no hay nada más que hacer aquí.
    }
  }, [openPath]);

  const guardar = useCallback(async () => {
    if (tab?.dirty) await saveTab(tab.id);
  }, [tab, saveTab]);

  const guardarComo = useCallback(async () => {
    if (!tab) return;
    const destino = await save({
      defaultPath: fileTitle(tab.path),
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (destino) await saveTab(tab.id, destino);
  }, [tab, saveTab]);

  // Al cerrar la ventana con documentos modificados, se pide confirmación.
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async (event) => {
      const sucios = useDocuments.getState().tabs.filter((t) => t.dirty);
      if (sucios.length === 0) return;
      const descartar = await ask(
        sucios.length === 1
          ? S.sinGuardar.mensaje(fileTitle(sucios[0].path))
          : S.sinGuardar.mensajeVarios(sucios.length),
        {
          title: S.sinGuardar.titulo,
          kind: "warning",
          okLabel: S.sinGuardar.descartar,
          cancelLabel: S.sinGuardar.cancelar,
        },
      );
      if (!descartar) event.preventDefault();
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, []);

  function toggleSidebarPanel(panel: Exclude<SidebarPanel, null>) {
    setSidebar((cur) => (cur === panel ? null : panel));
  }

  // Atajos de teclado (M1: Ctrl+O/F/W, Ctrl+Tab, +/-, Ctrl+0, Ctrl+C, Av/RePág;
  // M2: Ctrl+S guardar, Ctrl+Shift+S guardar como, Esc sale de la herramienta nota).
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const mod = e.ctrlKey || e.metaKey;
      const activeEl = document.activeElement;
      const typing = activeEl instanceof HTMLInputElement || activeEl instanceof HTMLTextAreaElement;

      if (mod && e.key.toLowerCase() === "o") {
        e.preventDefault();
        void abrirDocumento();
        return;
      }
      if (!tab) return;
      if (mod && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void (e.shiftKey ? guardarComo() : guardar());
        return;
      }
      if (mod && e.key.toLowerCase() === "f") {
        e.preventDefault();
        openSearch(tab.id);
        return;
      }
      if (mod && e.key.toLowerCase() === "w") {
        e.preventDefault();
        void closeTab(tab.id);
        return;
      }
      if (mod && e.key === "Tab") {
        e.preventDefault();
        const idx = tabs.findIndex((t) => t.id === tab.id);
        const next = tabs[(idx + 1) % tabs.length];
        if (next) setActiveTab(next.id);
        return;
      }
      if (typing) {
        if (e.key === "Escape" && tab.search.open) closeSearch(tab.id);
        return;
      }
      if (e.key === "Escape" && tab.search.open) {
        closeSearch(tab.id);
        return;
      }
      if (e.key === "Escape" && tool !== "select") {
        setTool("select");
        return;
      }
      if (mod && e.key.toLowerCase() === "c" && tab.selection) {
        void navigator.clipboard.writeText(tab.selection.text);
        return;
      }
      if ((mod && (e.key === "+" || e.key === "=")) || e.key === "+") {
        e.preventDefault();
        zoomIn(tab.id, tab.effectiveZoom);
        return;
      }
      if (mod && e.key === "0") {
        e.preventDefault();
        setZoomMode(tab.id, { kind: "fit-width" });
        return;
      }
      if (e.key === "-") {
        e.preventDefault();
        zoomOut(tab.id, tab.effectiveZoom);
        return;
      }
      if (e.key === "PageDown") {
        goToPage(tab.id, tab.currentPage + 1);
      } else if (e.key === "PageUp") {
        goToPage(tab.id, tab.currentPage - 1);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    tab,
    tabs,
    abrirDocumento,
    closeTab,
    setActiveTab,
    openSearch,
    closeSearch,
    zoomIn,
    zoomOut,
    setZoomMode,
    goToPage,
    guardar,
    guardarComo,
    tool,
    setTool,
  ]);

  return (
    <div className="app">
      <TabBar onOpenAnother={abrirDocumento} />
      <Toolbar
        tab={tab}
        sidebar={sidebar}
        onToggleSidebar={toggleSidebarPanel}
        onOpen={abrirDocumento}
        onSave={() => void guardar()}
        onSaveAs={() => void guardarComo()}
      />
      {tab && <SearchBar tab={tab} />}
      <div className="body-row">
        {tab && <Sidebar tab={tab} panel={sidebar} onClose={() => setSidebar(null)} />}
        <main className="main-area">
          {tab ? (
            <>
              {tab.error && <p className="error">{tab.error}</p>}
              <PageList tab={tab} />
            </>
          ) : (
            <StartScreen onOpen={abrirDocumento} />
          )}
        </main>
      </div>
      <PasswordDialog />
    </div>
  );
}
