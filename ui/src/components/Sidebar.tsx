import { useEffect } from "react";
import type { OutlineNode } from "../lib/tauri";
import { tileUrl } from "../lib/protocol";
import { S } from "../strings/es";
import { useDocuments, type DocumentTab, type SidebarPanel } from "../state/document";

const THUMB_WIDTH_PX = 120;

interface SidebarProps {
  tab: DocumentTab;
  panel: SidebarPanel;
  onClose: () => void;
}

function OutlineTree({ tab, nodes }: { tab: DocumentTab; nodes: OutlineNode[] }) {
  const goToPage = useDocuments((s) => s.goToPage);
  return (
    <ul className="outline-tree">
      {nodes.map((n, i) => (
        <li key={i}>
          <button
            className="outline-item"
            disabled={n.page_index === null}
            onClick={() => {
              if (n.page_index !== null) goToPage(tab.id, n.page_index + 1);
            }}
          >
            {n.title || "—"}
          </button>
          {n.children.length > 0 && <OutlineTree tab={tab} nodes={n.children} />}
        </li>
      ))}
    </ul>
  );
}

function Thumbnails({ tab }: { tab: DocumentTab }) {
  const goToPage = useDocuments((s) => s.goToPage);
  return (
    <div className="thumb-list">
      {tab.pageSizes.map((size, i) => {
        const aspect = size.height_pt / size.width_pt;
        return (
          <button
            key={i}
            className={i + 1 === tab.currentPage ? "thumb thumb-active" : "thumb"}
            onClick={() => goToPage(tab.id, i + 1)}
          >
            <img
              src={tileUrl(tab.id, i, THUMB_WIDTH_PX)}
              alt={S.visor.paginaDe(i + 1, tab.pageCount)}
              style={{ width: THUMB_WIDTH_PX, height: THUMB_WIDTH_PX * aspect }}
              loading="lazy"
            />
            <span className="thumb-number">{i + 1}</span>
          </button>
        );
      })}
    </div>
  );
}

export function Sidebar({ tab, panel, onClose }: SidebarProps) {
  const loadOutline = useDocuments((s) => s.loadOutline);

  useEffect(() => {
    if (panel === "outline") void loadOutline(tab.id);
  }, [panel, tab.id, loadOutline]);

  if (panel === null) return null;

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <span>{panel === "outline" ? S.panel.marcadores : S.panel.miniaturas}</span>
        <button onClick={onClose} title={S.panel.cerrarPanel}>
          ×
        </button>
      </div>
      <div className="sidebar-body">
        {panel === "outline" &&
          (tab.outline === null ? null : tab.outline.length === 0 ? (
            <p className="sidebar-empty">{S.panel.sinMarcadores}</p>
          ) : (
            <OutlineTree tab={tab} nodes={tab.outline} />
          ))}
        {panel === "thumbnails" && <Thumbnails tab={tab} />}
      </div>
    </aside>
  );
}
