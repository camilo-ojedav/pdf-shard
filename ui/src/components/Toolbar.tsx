import { useState } from "react";
import { S } from "../strings/es";
import { fileTitle, useDocuments, type DocumentTab, type SidebarPanel } from "../state/document";
import { useTheme } from "../state/theme";
import { isAndroid } from "../lib/platform";
import { printDocument } from "../lib/tauri";

interface ToolbarProps {
  tab: DocumentTab | null;
  sidebar: SidebarPanel;
  onToggleSidebar: (panel: Exclude<SidebarPanel, null>) => void;
  onOpen: () => void;
}

export function Toolbar({ tab, sidebar, onToggleSidebar, onOpen }: ToolbarProps) {
  const [recentOpen, setRecentOpen] = useState(false);
  const recents = useDocuments((s) => s.recents);
  const openPath = useDocuments((s) => s.openPath);
  const clearRecents = useDocuments((s) => s.clearRecents);
  const refreshRecents = useDocuments((s) => s.refreshRecents);
  const goToPage = useDocuments((s) => s.goToPage);
  const setZoomMode = useDocuments((s) => s.setZoomMode);
  const zoomIn = useDocuments((s) => s.zoomIn);
  const zoomOut = useDocuments((s) => s.zoomOut);
  const openSearch = useDocuments((s) => s.openSearch);
  const theme = useTheme((s) => s.theme);
  const toggleTheme = useTheme((s) => s.toggle);

  function toggleRecent() {
    if (!recentOpen) void refreshRecents();
    setRecentOpen((v) => !v);
  }

  async function onPrint() {
    if (!tab) return;
    try {
      await printDocument(tab.path);
    } catch {
      window.alert(S.imprimir.error);
    }
  }

  return (
    <header className="toolbar">
      <span className="brand">{S.app.titulo}</span>
      <button onClick={onOpen}>{S.visor.abrir}</button>

      <div className="dropdown-wrap">
        <button onClick={toggleRecent} aria-expanded={recentOpen}>
          {S.recientes.titulo}
        </button>
        {recentOpen && (
          <div className="dropdown">
            {recents.length === 0 && <p className="dropdown-empty">{S.recientes.vacio}</p>}
            {recents.map((r) => (
              <button
                key={r.path}
                className="dropdown-item"
                onClick={() => {
                  setRecentOpen(false);
                  void openPath(r.path);
                }}
                title={r.path}
              >
                {fileTitle(r.path)}
              </button>
            ))}
            {recents.length > 0 && (
              <button
                className="dropdown-item dropdown-clear"
                onClick={() => {
                  void clearRecents();
                }}
              >
                {S.recientes.limpiar}
              </button>
            )}
          </div>
        )}
      </div>

      {tab && (
        <>
          <span className="sep" />
          <button
            onClick={() => onToggleSidebar("outline")}
            className={sidebar === "outline" ? "active" : ""}
          >
            {S.panel.marcadores}
          </button>
          <button
            onClick={() => onToggleSidebar("thumbnails")}
            className={sidebar === "thumbnails" ? "active" : ""}
          >
            {S.panel.miniaturas}
          </button>

          <span className="sep" />
          <button
            onClick={() => goToPage(tab.id, tab.currentPage - 1)}
            disabled={tab.currentPage <= 1}
            title={S.visor.anterior}
          >
            ◀
          </button>
          <span className="page-indicator">{S.visor.paginaDe(tab.currentPage, tab.pageCount)}</span>
          <button
            onClick={() => goToPage(tab.id, tab.currentPage + 1)}
            disabled={tab.currentPage >= tab.pageCount}
            title={S.visor.siguiente}
          >
            ▶
          </button>

          <span className="sep" />
          <button onClick={() => zoomOut(tab.id, tab.effectiveZoom)} title={S.visor.alejar}>
            −
          </button>
          <span className="page-indicator">{S.visor.zoomPorciento(tab.effectiveZoom)}</span>
          <button
            onClick={() => zoomIn(tab.id, tab.effectiveZoom)}
            title={S.visor.acercar}
          >
            +
          </button>
          <button
            title={S.visor.ajustarAncho}
            className={tab.zoomMode.kind === "fit-width" ? "active" : ""}
            onClick={() => setZoomMode(tab.id, { kind: "fit-width" })}
          >
            ↔
          </button>
          <button
            title={S.visor.ajustarPagina}
            className={tab.zoomMode.kind === "fit-page" ? "active" : ""}
            onClick={() => setZoomMode(tab.id, { kind: "fit-page" })}
          >
            ⬚
          </button>

          <span className="sep" />
          <button onClick={() => openSearch(tab.id)} title={S.buscar.boton}>
            🔍
          </button>
          {!isAndroid() && (
            <button onClick={onPrint} title={S.imprimir.boton}>
              {S.imprimir.boton}
            </button>
          )}
        </>
      )}

      <span className="spacer" />
      <button onClick={toggleTheme} title={S.tema.cambiar}>
        {theme === "dark" ? S.tema.claro : S.tema.oscuro}
      </button>
    </header>
  );
}
