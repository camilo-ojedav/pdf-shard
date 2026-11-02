import { create } from "zustand";
import { ask } from "@tauri-apps/plugin-dialog";
import {
  clearRecentDocuments,
  closeDocument,
  createAnnotation,
  deleteAnnotation,
  exportAnnotationsXfdf,
  getOutline,
  getRecentDocuments,
  importAnnotationsXfdf,
  isOpenError,
  listAnnotations,
  openDocument,
  saveDocument,
  searchText,
  setAnnotationContents,
  type Annotation,
  type MarkupKind,
  type OpenErrorKind,
  type OutlineNode,
  type PageSize,
  type RecentEntry,
  type SearchResults,
  type TextSelectionResult,
} from "../lib/tauri";
import { S } from "../strings/es";

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 5;
export const ZOOM_STEP = 1.25;

export type ZoomMode =
  | { kind: "fixed"; value: number }
  | { kind: "fit-width" }
  | { kind: "fit-page" };

export type SidebarPanel = "outline" | "thumbnails" | "annotations" | null;

/** Herramienta activa sobre el canvas: seleccionar texto o colocar una nota. */
export type CanvasTool = "select" | "note";

/** Lado (en puntos PDF) del ícono de una nota nueva, como en Acrobat. */
export const NOTE_SIZE_PT = 20;

export interface SearchState {
  open: boolean;
  query: string;
  results: SearchResults | null;
  activeHit: number;
  matchCase: boolean;
  wholeWord: boolean;
}

const EMPTY_SEARCH: SearchState = {
  open: false,
  query: "",
  results: null,
  activeHit: 0,
  matchCase: false,
  wholeWord: false,
};

export interface DocumentTab {
  id: number;
  path: string;
  pageCount: number;
  pageSizes: PageSize[];
  /** Página actual (base 1), sincronizada con el scroll. */
  currentPage: number;
  /** Señal efímera para pedirle a la lista virtualizada que salte a una página. */
  scrollToPage: number | null;
  zoomMode: ZoomMode;
  /** Zoom efectivo (0..n) de la página actual, calculado por PageList a
   * partir de zoomMode + ancho del contenedor: lo necesita el Toolbar para
   * mostrar el porcentaje y como base de zoomIn/zoomOut. */
  effectiveZoom: number;
  outline: OutlineNode[] | null;
  search: SearchState;
  selection: TextSelectionResult | null;
  /** Anotaciones del documento (`null` = todavía no se pidieron). */
  annotations: Annotation[] | null;
  /** Hay cambios en memoria sin guardar en el archivo. */
  dirty: boolean;
  /** Sube con cada mutación: invalida la caché de tiles del webview. */
  revision: number;
  error: string | null;
}

const S_UNSAVED = S.sinGuardar;

function fileTitle(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export interface PasswordPrompt {
  path: string;
  kind: OpenErrorKind;
}

interface DocumentStoreState {
  tabs: DocumentTab[];
  activeTabId: number | null;
  recents: RecentEntry[];
  passwordPrompt: PasswordPrompt | null;
  tool: CanvasTool;

  openPath: (path: string, password?: string) => Promise<void>;
  /** Cierra la pestaña; si tiene cambios sin guardar pide confirmación
   * (salvo `force`). Devuelve `false` si el usuario canceló. */
  closeTab: (id: number, force?: boolean) => Promise<boolean>;
  setActiveTab: (id: number) => void;
  cancelPasswordPrompt: () => void;

  setZoomMode: (id: number, mode: ZoomMode) => void;
  zoomIn: (id: number, effectiveZoom: number) => void;
  zoomOut: (id: number, effectiveZoom: number) => void;

  setCurrentPage: (id: number, page: number) => void;
  goToPage: (id: number, page: number) => void;
  consumeScrollRequest: (id: number) => void;
  setEffectiveZoom: (id: number, value: number) => void;

  loadOutline: (id: number) => Promise<void>;

  openSearch: (id: number) => void;
  closeSearch: (id: number) => void;
  setSearchQuery: (id: number, query: string) => void;
  runSearch: (id: number) => Promise<void>;
  nextResult: (id: number) => void;
  prevResult: (id: number) => void;

  setSelection: (id: number, selection: TextSelectionResult | null) => void;

  setTool: (tool: CanvasTool) => void;
  loadAnnotations: (id: number) => Promise<void>;
  annotateSelection: (id: number, kind: MarkupKind) => Promise<void>;
  addNote: (id: number, pageIndex: number, x: number, y: number, contents: string) => Promise<void>;
  editAnnotation: (id: number, annotationId: string, contents: string) => Promise<void>;
  removeAnnotation: (id: number, annotationId: string) => Promise<void>;
  importXfdf: (id: number, path: string) => Promise<void>;
  exportXfdf: (id: number, path: string) => Promise<void>;
  /** Guarda (en `path` = "Guardar como"). Devuelve `false` si falló. */
  saveTab: (id: number, path?: string) => Promise<boolean>;

  refreshRecents: () => Promise<void>;
  clearRecents: () => Promise<void>;
}

function updateTab(
  tabs: DocumentTab[],
  id: number,
  patch: Partial<DocumentTab> | ((t: DocumentTab) => Partial<DocumentTab>),
): DocumentTab[] {
  return tabs.map((t) => {
    if (t.id !== id) return t;
    const p = typeof patch === "function" ? patch(t) : patch;
    return { ...t, ...p };
  });
}

function errorMessage(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

export const useDocuments = create<DocumentStoreState>((set, get) => {
  /** Aplica el resultado de una mutación de anotaciones: lista nueva,
   * documento modificado y tiles a re-renderizar. */
  function applyAnnotations(id: number, annotations: Annotation[], extra?: Partial<DocumentTab>) {
    set((s) => ({
      tabs: updateTab(s.tabs, id, (t) => ({
        annotations,
        dirty: true,
        revision: t.revision + 1,
        error: null,
        ...extra,
      })),
    }));
  }

  /** Ejecuta una mutación y deja el error visible en la pestaña si falla. */
  async function mutate(id: number, run: () => Promise<Annotation[]>, extra?: Partial<DocumentTab>) {
    try {
      applyAnnotations(id, await run(), extra);
    } catch (e) {
      set((s) => ({ tabs: updateTab(s.tabs, id, { error: errorMessage(e) }) }));
    }
  }

  return {
    tabs: [],
    activeTabId: null,
    recents: [],
    passwordPrompt: null,
    tool: "select",

    openPath: async (path, password) => {
      const existing = get().tabs.find((t) => t.path === path);
      if (existing && !password) {
        set({ activeTabId: existing.id, passwordPrompt: null });
        return;
      }
      try {
        const info = await openDocument(path, password);
        set((s) => {
          const withoutDup = s.tabs.filter((t) => t.id !== info.id);
          const tab: DocumentTab = {
            id: info.id,
            path: info.path,
            pageCount: info.page_count,
            pageSizes: info.page_sizes,
            currentPage: 1,
            scrollToPage: null,
            zoomMode: { kind: "fit-width" },
            effectiveZoom: 1,
            outline: null,
            search: EMPTY_SEARCH,
            selection: null,
            annotations: null,
            dirty: false,
            revision: 0,
            error: null,
          };
          return {
            tabs: [...withoutDup, tab],
            activeTabId: info.id,
            passwordPrompt: null,
          };
        });
        void get().refreshRecents();
      } catch (e) {
        if (isOpenError(e) && (e.kind === "password_required" || e.kind === "wrong_password")) {
          set({ passwordPrompt: { path, kind: e.kind } });
          return;
        }
        const message = isOpenError(e) ? e.message : String(e);
        set((s) => ({
          passwordPrompt: null,
          tabs: s.tabs.some((t) => t.path === path)
            ? updateTab(s.tabs, s.tabs.find((t) => t.path === path)!.id, { error: message })
            : s.tabs,
        }));
        throw e;
      }
    },

    closeTab: async (id, force = false) => {
      const tab = get().tabs.find((t) => t.id === id);
      if (tab?.dirty && !force) {
        const discard = await ask(S_UNSAVED.mensaje(fileTitle(tab.path)), {
          title: S_UNSAVED.titulo,
          kind: "warning",
          okLabel: S_UNSAVED.descartar,
          cancelLabel: S_UNSAVED.cancelar,
        });
        if (!discard) return false;
      }
      await closeDocument(id).catch(() => undefined);
      set((s) => {
        const tabs = s.tabs.filter((t) => t.id !== id);
        const activeTabId =
          s.activeTabId === id ? (tabs.length > 0 ? tabs[tabs.length - 1].id : null) : s.activeTabId;
        return { tabs, activeTabId };
      });
      return true;
    },

    setActiveTab: (id) => set({ activeTabId: id }),

    cancelPasswordPrompt: () => set({ passwordPrompt: null }),

    setZoomMode: (id, mode) => set((s) => ({ tabs: updateTab(s.tabs, id, { zoomMode: mode }) })),

    zoomIn: (id, effectiveZoom) =>
      set((s) => ({
        tabs: updateTab(s.tabs, id, {
          zoomMode: { kind: "fixed", value: Math.min(ZOOM_MAX, effectiveZoom * ZOOM_STEP) },
        }),
      })),

    zoomOut: (id, effectiveZoom) =>
      set((s) => ({
        tabs: updateTab(s.tabs, id, {
          zoomMode: { kind: "fixed", value: Math.max(ZOOM_MIN, effectiveZoom / ZOOM_STEP) },
        }),
      })),

    setCurrentPage: (id, page) => set((s) => ({ tabs: updateTab(s.tabs, id, { currentPage: page }) })),

    goToPage: (id, page) =>
      set((s) => ({
        tabs: updateTab(s.tabs, id, (t) => ({
          scrollToPage: Math.min(Math.max(1, page), Math.max(1, t.pageCount)),
        })),
      })),

    consumeScrollRequest: (id) => set((s) => ({ tabs: updateTab(s.tabs, id, { scrollToPage: null }) })),

    setEffectiveZoom: (id, value) =>
      set((s) => ({ tabs: updateTab(s.tabs, id, { effectiveZoom: value }) })),

    loadOutline: async (id) => {
      const tab = get().tabs.find((t) => t.id === id);
      if (!tab || tab.outline !== null) return;
      try {
        const outline = await getOutline(id);
        set((s) => ({ tabs: updateTab(s.tabs, id, { outline }) }));
      } catch {
        set((s) => ({ tabs: updateTab(s.tabs, id, { outline: [] }) }));
      }
    },

    openSearch: (id) =>
      set((s) => ({ tabs: updateTab(s.tabs, id, (t) => ({ search: { ...t.search, open: true } })) })),

    closeSearch: (id) =>
      set((s) => ({
        tabs: updateTab(s.tabs, id, (t) => ({ search: { ...t.search, open: false } })),
      })),

    setSearchQuery: (id, query) =>
      set((s) => ({ tabs: updateTab(s.tabs, id, (t) => ({ search: { ...t.search, query } })) })),

    runSearch: async (id) => {
      const tab = get().tabs.find((t) => t.id === id);
      if (!tab || tab.search.query.trim().length === 0) {
        set((s) => ({
          tabs: updateTab(s.tabs, id, (t) => ({ search: { ...t.search, results: null, activeHit: 0 } })),
        }));
        return;
      }
      const results = await searchText(id, tab.search.query, tab.search.matchCase, tab.search.wholeWord);
      set((s) => ({
        tabs: updateTab(s.tabs, id, (t) => ({ search: { ...t.search, results, activeHit: 0 } })),
      }));
      const first = results.hits[0];
      if (first) {
        get().goToPage(id, first.page_index + 1);
      }
    },

    nextResult: (id) => {
      const tab = get().tabs.find((t) => t.id === id);
      const total = tab?.search.results?.hits.length ?? 0;
      if (!tab || total === 0) return;
      const activeHit = (tab.search.activeHit + 1) % total;
      set((s) => ({ tabs: updateTab(s.tabs, id, (t) => ({ search: { ...t.search, activeHit } })) }));
      const hit = tab.search.results!.hits[activeHit];
      get().goToPage(id, hit.page_index + 1);
    },

    prevResult: (id) => {
      const tab = get().tabs.find((t) => t.id === id);
      const total = tab?.search.results?.hits.length ?? 0;
      if (!tab || total === 0) return;
      const activeHit = (tab.search.activeHit - 1 + total) % total;
      set((s) => ({ tabs: updateTab(s.tabs, id, (t) => ({ search: { ...t.search, activeHit } })) }));
      const hit = tab.search.results!.hits[activeHit];
      get().goToPage(id, hit.page_index + 1);
    },

    setSelection: (id, selection) =>
      set((s) => ({ tabs: updateTab(s.tabs, id, { selection }) })),

    setTool: (tool) => set({ tool }),

    loadAnnotations: async (id) => {
      try {
        const annotations = await listAnnotations(id);
        set((s) => ({ tabs: updateTab(s.tabs, id, { annotations }) }));
      } catch (e) {
        set((s) => ({ tabs: updateTab(s.tabs, id, { annotations: [], error: errorMessage(e) }) }));
      }
    },

    annotateSelection: async (id, kind) => {
      const selection = get().tabs.find((t) => t.id === id)?.selection;
      if (!selection || selection.rects.length === 0) return;
      await mutate(
        id,
        () => createAnnotation(id, selection.page_index, kind, selection.rects),
        { selection: null },
      );
    },

    addNote: async (id, pageIndex, x, y, contents) => {
      const tab = get().tabs.find((t) => t.id === id);
      const size = tab?.pageSizes[pageIndex];
      if (!size) return;
      // El ícono se ancla con su esquina superior izquierda en el clic, sin
      // salirse de la página.
      const left = Math.min(Math.max(0, x), Math.max(0, size.width_pt - NOTE_SIZE_PT));
      const top = Math.min(Math.max(0, y), Math.max(0, size.height_pt - NOTE_SIZE_PT));
      await mutate(id, () =>
        createAnnotation(id, pageIndex, "text", [
          { left, top, right: left + NOTE_SIZE_PT, bottom: top + NOTE_SIZE_PT },
        ], contents),
      );
    },

    editAnnotation: async (id, annotationId, contents) => {
      await mutate(id, () => setAnnotationContents(id, annotationId, contents));
    },

    removeAnnotation: async (id, annotationId) => {
      await mutate(id, () => deleteAnnotation(id, annotationId));
    },

    importXfdf: async (id, path) => {
      await mutate(id, () => importAnnotationsXfdf(id, path));
    },

    exportXfdf: async (id, path) => {
      try {
        await exportAnnotationsXfdf(id, path);
      } catch (e) {
        set((s) => ({ tabs: updateTab(s.tabs, id, { error: errorMessage(e) }) }));
      }
    },

    saveTab: async (id, path) => {
      try {
        const saved = await saveDocument(id, path);
        set((s) => ({ tabs: updateTab(s.tabs, id, { path: saved, dirty: false, error: null }) }));
        if (path) void get().refreshRecents();
        return true;
      } catch (e) {
        set((s) => ({ tabs: updateTab(s.tabs, id, { error: errorMessage(e) }) }));
        return false;
      }
    },

    refreshRecents: async () => {
      const recents = await getRecentDocuments().catch(() => []);
      set({ recents });
    },

    clearRecents: async () => {
      await clearRecentDocuments().catch(() => undefined);
      set({ recents: [] });
    },
  };
});

export function activeTab(state: DocumentStoreState): DocumentTab | null {
  return state.tabs.find((t) => t.id === state.activeTabId) ?? null;
}

export { fileTitle };
