import { create } from "zustand";
import {
  clearRecentDocuments,
  closeDocument,
  getOutline,
  getRecentDocuments,
  isOpenError,
  openDocument,
  searchText,
  type OpenErrorKind,
  type OutlineNode,
  type PageSize,
  type RecentEntry,
  type SearchResults,
  type TextSelectionResult,
} from "../lib/tauri";

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 5;
export const ZOOM_STEP = 1.25;

export type ZoomMode =
  | { kind: "fixed"; value: number }
  | { kind: "fit-width" }
  | { kind: "fit-page" };

export type SidebarPanel = "outline" | "thumbnails" | null;

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
  error: string | null;
}

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

  openPath: (path: string, password?: string) => Promise<void>;
  closeTab: (id: number) => Promise<void>;
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

export const useDocuments = create<DocumentStoreState>((set, get) => ({
  tabs: [],
  activeTabId: null,
  recents: [],
  passwordPrompt: null,

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

  closeTab: async (id) => {
    await closeDocument(id).catch(() => undefined);
    set((s) => {
      const tabs = s.tabs.filter((t) => t.id !== id);
      const activeTabId =
        s.activeTabId === id ? (tabs.length > 0 ? tabs[tabs.length - 1].id : null) : s.activeTabId;
      return { tabs, activeTabId };
    });
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

  refreshRecents: async () => {
    const recents = await getRecentDocuments().catch(() => []);
    set({ recents });
  },

  clearRecents: async () => {
    await clearRecentDocuments().catch(() => undefined);
    set({ recents: [] });
  },
}));

export function activeTab(state: DocumentStoreState): DocumentTab | null {
  return state.tabs.find((t) => t.id === state.activeTabId) ?? null;
}

export { fileTitle };
