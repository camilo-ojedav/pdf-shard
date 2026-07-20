import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { S } from "./strings/es";
import { tileUrl } from "./lib/protocol";
import { useDocument } from "./state/document";

interface DocumentInfo {
  id: number;
  path: string;
  page_count: number;
}

const ANCHO_BASE_PX = 900;

export function App() {
  const doc = useDocument();

  async function abrirDocumento() {
    const seleccion = await open({
      multiple: false,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof seleccion !== "string") return;
    try {
      const info = await invoke<DocumentInfo>("open_document", { path: seleccion });
      doc.opened(info.id, info.path, info.page_count);
    } catch (e) {
      doc.fail(`${S.visor.errorAbrir}: ${String(e)}`);
    }
  }

  const anchoTile = Math.round(ANCHO_BASE_PX * doc.zoom);

  return (
    <div className="app">
      <header className="toolbar">
        <span className="brand">{S.app.titulo}</span>
        <button onClick={abrirDocumento}>{S.visor.abrir}</button>
        {doc.docId !== null && (
          <>
            <span className="sep" />
            <button
              onClick={() => doc.setPage(doc.page - 1)}
              disabled={doc.page <= 1}
              title={S.visor.anterior}
            >
              ◀
            </button>
            <span className="page-indicator">
              {S.visor.paginaDe(doc.page, doc.pageCount)}
            </span>
            <button
              onClick={() => doc.setPage(doc.page + 1)}
              disabled={doc.page >= doc.pageCount}
              title={S.visor.siguiente}
            >
              ▶
            </button>
            <span className="sep" />
            <button onClick={doc.zoomOut} title={S.visor.alejar}>
              −
            </button>
            <span className="page-indicator">{S.visor.zoomPorciento(doc.zoom)}</span>
            <button onClick={doc.zoomIn} title={S.visor.acercar}>
              +
            </button>
          </>
        )}
      </header>

      <main className="canvas-area">
        {doc.error && <p className="error">{doc.error}</p>}
        {doc.docId === null && !doc.error && (
          <p className="empty">{S.visor.sinDocumento}</p>
        )}
        {doc.docId !== null && (
          <img
            className="page"
            src={tileUrl(doc.docId, doc.page - 1, anchoTile)}
            alt={S.visor.paginaDe(doc.page, doc.pageCount)}
          />
        )}
      </main>
    </div>
  );
}
