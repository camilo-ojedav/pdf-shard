// Envoltorio tipado de los comandos IPC del backend (apps/shard/src-tauri).
// Los nombres de argumento van en camelCase: Tauri convierte automáticamente
// los parámetros snake_case de Rust a camelCase para el lado JS.

import { invoke } from "@tauri-apps/api/core";

export interface PageSize {
  width_pt: number;
  height_pt: number;
}

export interface DocumentInfo {
  id: number;
  path: string;
  page_count: number;
  page_sizes: PageSize[];
}

export type OpenErrorKind = "password_required" | "wrong_password" | "other";

export interface OpenError {
  kind: OpenErrorKind;
  message: string;
}

export function isOpenError(e: unknown): e is OpenError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    "message" in e
  );
}

export interface OutlineNode {
  title: string;
  page_index: number | null;
  children: OutlineNode[];
}

export interface RectPt {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface SearchHit {
  page_index: number;
  rects: RectPt[];
}

export interface SearchResults {
  hits: SearchHit[];
  truncated: boolean;
}

export interface TextSelectionResult {
  page_index: number;
  text: string;
  rects: RectPt[];
}

export interface RecentEntry {
  path: string;
  last_opened_unix_ms: number;
}

export function openDocument(path: string, password?: string): Promise<DocumentInfo> {
  return invoke<DocumentInfo>("open_document", { path, password });
}

export function closeDocument(id: number): Promise<void> {
  return invoke("close_document", { id });
}

export function getOutline(id: number): Promise<OutlineNode[]> {
  return invoke<OutlineNode[]>("get_outline", { id });
}

export function searchText(
  id: number,
  query: string,
  matchCase: boolean,
  wholeWord: boolean,
): Promise<SearchResults> {
  return invoke<SearchResults>("search_text", { id, query, matchCase, wholeWord });
}

export function selectText(
  id: number,
  page: number,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
): Promise<TextSelectionResult | null> {
  return invoke<TextSelectionResult | null>("select_text", { id, page, x0, y0, x1, y1 });
}

export function getRecentDocuments(): Promise<RecentEntry[]> {
  return invoke<RecentEntry[]>("get_recent_documents");
}

export function clearRecentDocuments(): Promise<void> {
  return invoke("clear_recent_documents");
}

export function printDocument(path: string): Promise<void> {
  return invoke("print_document", { path });
}
