import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { RectPt } from "../lib/tauri";
import { tileUrl } from "../lib/protocol";
import { S } from "../strings/es";
import { useDocuments, type DocumentTab } from "../state/document";
import { selectText } from "../lib/tauri";

const PAGE_GAP_PX = 16;
const SIDE_PADDING_PX = 24;
const OVERSCAN_PAGES = 2;
const MIN_DRAG_PX = 3;

interface PageLayout {
  index: number;
  top: number;
  height: number;
  width: number;
  zoom: number;
}

function computeLayout(
  tab: DocumentTab,
  containerWidth: number,
  containerHeight: number,
): PageLayout[] {
  let top = 0;
  const layouts: PageLayout[] = [];
  for (let i = 0; i < tab.pageSizes.length; i++) {
    const { width_pt, height_pt } = tab.pageSizes[i];
    let zoom: number;
    if (tab.zoomMode.kind === "fixed") {
      zoom = tab.zoomMode.value;
    } else if (tab.zoomMode.kind === "fit-width") {
      zoom = Math.max(0.05, (containerWidth - SIDE_PADDING_PX * 2) / width_pt);
    } else {
      const zw = (containerWidth - SIDE_PADDING_PX * 2) / width_pt;
      const zh = (containerHeight - SIDE_PADDING_PX * 2) / height_pt;
      zoom = Math.max(0.05, Math.min(zw, zh));
    }
    const width = width_pt * zoom;
    const height = height_pt * zoom;
    layouts.push({ index: i, top, height, width, zoom });
    top += height + PAGE_GAP_PX;
  }
  return layouts;
}

function rectStyle(r: RectPt, zoom: number): React.CSSProperties {
  return {
    left: r.left * zoom,
    top: r.top * zoom,
    width: Math.max(0, (r.right - r.left) * zoom),
    height: Math.max(0, (r.bottom - r.top) * zoom),
  };
}

interface PageProps {
  tab: DocumentTab;
  layout: PageLayout;
}

function Page({ tab, layout }: PageProps) {
  const overlayRef = useRef<HTMLDivElement>(null);
  const dragStart = useRef<{ x: number; y: number } | null>(null);
  const setSelection = useDocuments((s) => s.setSelection);
  const dpr = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
  const renderWidthPx = Math.round(layout.width * dpr);

  const toPagePoint = useCallback(
    (clientX: number, clientY: number) => {
      const rect = overlayRef.current?.getBoundingClientRect();
      if (!rect) return { x: 0, y: 0 };
      return {
        x: (clientX - rect.left) / layout.zoom,
        y: (clientY - rect.top) / layout.zoom,
      };
    },
    [layout.zoom],
  );

  function onPointerDown(e: React.PointerEvent<HTMLDivElement>) {
    if (e.button !== 0 && e.pointerType === "mouse") return;
    dragStart.current = { x: e.clientX, y: e.clientY };
  }

  async function onPointerUp(e: React.PointerEvent<HTMLDivElement>) {
    const start = dragStart.current;
    dragStart.current = null;
    if (!start) return;
    const dx = e.clientX - start.x;
    const dy = e.clientY - start.y;
    if (Math.hypot(dx, dy) < MIN_DRAG_PX) {
      setSelection(tab.id, null);
      return;
    }
    const p0 = toPagePoint(start.x, start.y);
    const p1 = toPagePoint(e.clientX, e.clientY);
    try {
      const result = await selectText(tab.id, layout.index, p0.x, p0.y, p1.x, p1.y);
      setSelection(tab.id, result);
    } catch {
      // Selección best-effort: si falla (documento cerrado, etc.) se ignora.
    }
  }

  const searchRects =
    tab.search.results?.hits
      .map((hit, hitIndex) => ({ hit, hitIndex }))
      .filter(({ hit }) => hit.page_index === layout.index) ?? [];

  return (
    <div className="page-wrap" style={{ position: "absolute", top: layout.top, height: layout.height }}>
      <div className="page-inner" style={{ width: layout.width, height: layout.height }}>
        <img
          className="page"
          style={{ width: layout.width, height: layout.height }}
          src={tileUrl(tab.id, layout.index, renderWidthPx)}
          alt={S.visor.paginaDe(layout.index + 1, tab.pageCount)}
          draggable={false}
        />
        <div
          ref={overlayRef}
          className="page-overlay"
          onPointerDown={onPointerDown}
          onPointerUp={onPointerUp}
        >
          {searchRects.map(({ hit, hitIndex }) =>
            hit.rects.map((r, i) => (
              <div
                key={`s${hitIndex}-${i}`}
                className={hitIndex === tab.search.activeHit ? "highlight highlight-active" : "highlight"}
                style={rectStyle(r, layout.zoom)}
              />
            )),
          )}
          {tab.selection?.page_index === layout.index &&
            tab.selection.rects.map((r, i) => (
              <div key={`sel${i}`} className="selection-rect" style={rectStyle(r, layout.zoom)} />
            ))}
        </div>
      </div>
    </div>
  );
}

interface PageListProps {
  tab: DocumentTab;
}

export function PageList({ tab }: PageListProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 800, height: 600 });
  const [scrollTop, setScrollTop] = useState(0);
  const setCurrentPage = useDocuments((s) => s.setCurrentPage);
  const consumeScrollRequest = useDocuments((s) => s.consumeScrollRequest);
  const setEffectiveZoom = useDocuments((s) => s.setEffectiveZoom);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      setSize({ width: entry.contentRect.width, height: entry.contentRect.height });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const layouts = useMemo(
    // Se recalcula cuando cambian tamaños de página, modo de zoom o el contenedor.
    () => computeLayout(tab, size.width, size.height),
    [tab.pageSizes, tab.zoomMode, size.width, size.height],
  );

  const totalHeight = layouts.length > 0 ? layouts[layouts.length - 1].top + layouts[layouts.length - 1].height : 0;

  const onScroll = useCallback(() => {
    const el = containerRef.current;
    if (el) setScrollTop(el.scrollTop);
  }, []);

  // Página actual = la primera cuyo borde inferior todavía no pasó el tope del viewport.
  useEffect(() => {
    const current = layouts.find((l) => l.top + l.height > scrollTop + 1);
    if (current) setCurrentPage(tab.id, current.index + 1);
  }, [layouts, scrollTop, setCurrentPage, tab.id]);

  // Zoom efectivo de la página actual, para que el toolbar muestre el % y
  // zoomIn/zoomOut tengan una base numérica aunque el modo sea "ajustar".
  useEffect(() => {
    const idx = Math.min(Math.max(0, tab.currentPage - 1), layouts.length - 1);
    const l = layouts[idx];
    if (l) setEffectiveZoom(tab.id, l.zoom);
  }, [layouts, tab.currentPage, tab.id, setEffectiveZoom]);

  // Salto de página pedido desde fuera (outline, búsqueda, indicador de página).
  useEffect(() => {
    if (tab.scrollToPage === null) return;
    const target = layouts[tab.scrollToPage - 1];
    const el = containerRef.current;
    if (target && el) {
      el.scrollTo({ top: target.top, behavior: "auto" });
    }
    consumeScrollRequest(tab.id);
  }, [tab.scrollToPage, tab.id, layouts, consumeScrollRequest]);

  const bufferPx = size.height;
  const visible = layouts.filter((l) => l.top + l.height >= scrollTop - bufferPx && l.top <= scrollTop + size.height + bufferPx);
  const overscanStart = Math.max(0, (visible[0]?.index ?? 0) - OVERSCAN_PAGES);
  const overscanEnd = Math.min(layouts.length - 1, (visible[visible.length - 1]?.index ?? -1) + OVERSCAN_PAGES);
  const rendered = overscanEnd >= overscanStart ? layouts.slice(overscanStart, overscanEnd + 1) : [];

  return (
    <div ref={containerRef} className="canvas-area" onScroll={onScroll}>
      <div className="page-column" style={{ height: totalHeight, width: "100%", position: "relative" }}>
        {rendered.map((l) => (
          <Page key={l.index} tab={tab} layout={l} />
        ))}
      </div>
    </div>
  );
}
