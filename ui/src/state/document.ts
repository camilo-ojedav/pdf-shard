import { create } from "zustand";

const ZOOM_MIN = 0.5;
const ZOOM_MAX = 4;
const ZOOM_PASO = 1.25;

export interface DocumentState {
  docId: number | null;
  path: string | null;
  pageCount: number;
  /** Página actual, base 1 (como la ve el usuario). */
  page: number;
  zoom: number;
  error: string | null;
  opened: (id: number, path: string, pageCount: number) => void;
  fail: (mensaje: string) => void;
  setPage: (p: number) => void;
  zoomIn: () => void;
  zoomOut: () => void;
}

export const useDocument = create<DocumentState>((set) => ({
  docId: null,
  path: null,
  pageCount: 0,
  page: 1,
  zoom: 1,
  error: null,
  opened: (id, path, pageCount) =>
    set({ docId: id, path, pageCount, page: 1, error: null }),
  fail: (mensaje) => set({ error: mensaje }),
  setPage: (p) =>
    set((s) => ({ page: Math.min(Math.max(1, p), Math.max(1, s.pageCount)) })),
  zoomIn: () => set((s) => ({ zoom: Math.min(ZOOM_MAX, s.zoom * ZOOM_PASO) })),
  zoomOut: () => set((s) => ({ zoom: Math.max(ZOOM_MIN, s.zoom / ZOOM_PASO) })),
}));
