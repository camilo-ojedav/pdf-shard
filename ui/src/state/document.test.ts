import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Annotation, RectPt } from "../lib/tauri";

const tauri = vi.hoisted(() => ({
  createAnnotation: vi.fn(),
  closeDocument: vi.fn(),
  saveDocument: vi.fn(),
  getRecentDocuments: vi.fn(),
}));
const dialog = vi.hoisted(() => ({ ask: vi.fn() }));

vi.mock("../lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/tauri")>()),
  ...tauri,
}));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);

const { useDocuments, NOTE_SIZE_PT } = await import("./document");
const { tileUrl } = await import("../lib/protocol");

const RECT: RectPt = { left: 10, top: 20, right: 110, bottom: 32 };

function annotation(kind: Annotation["kind"], rects: RectPt[]): Annotation {
  return { id: "0:0", page_index: 0, kind, rects, contents: null, author: null, color: null };
}

function seedTab() {
  useDocuments.setState({
    tool: "select",
    activeTabId: 1,
    tabs: [
      {
        id: 1,
        path: "/tmp/doc.pdf",
        pageCount: 1,
        pageSizes: [{ width_pt: 612, height_pt: 792 }],
        currentPage: 1,
        scrollToPage: null,
        zoomMode: { kind: "fit-width" },
        effectiveZoom: 1,
        outline: null,
        search: { open: false, query: "", results: null, activeHit: 0, matchCase: false, wholeWord: false },
        selection: { page_index: 0, text: "hola", rects: [RECT] },
        annotations: null,
        dirty: false,
        revision: 0,
        error: null,
      },
    ],
  });
}

const tab = () => useDocuments.getState().tabs[0];

describe("anotaciones en el store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    seedTab();
  });

  it("marca la selección: crea la anotación, ensucia el documento y refresca tiles", async () => {
    tauri.createAnnotation.mockResolvedValue([annotation("highlight", [RECT])]);
    await useDocuments.getState().annotateSelection(1, "highlight");
    expect(tauri.createAnnotation).toHaveBeenCalledWith(1, 0, "highlight", [RECT]);
    expect(tab().annotations).toHaveLength(1);
    expect(tab().dirty).toBe(true);
    expect(tab().revision).toBe(1);
    expect(tab().selection).toBeNull();
  });

  it("deja el error en la pestaña si el backend falla", async () => {
    tauri.createAnnotation.mockRejectedValue("pdfium: fallo");
    await useDocuments.getState().annotateSelection(1, "underline");
    expect(tab().error).toBe("pdfium: fallo");
    expect(tab().dirty).toBe(false);
    expect(tab().revision).toBe(0);
  });

  it("la nota no se sale de la página", async () => {
    tauri.createAnnotation.mockResolvedValue([]);
    await useDocuments.getState().addNote(1, 0, 605, -4, "texto");
    expect(tauri.createAnnotation).toHaveBeenCalledWith(
      1,
      0,
      "text",
      [{ left: 612 - NOTE_SIZE_PT, top: 0, right: 612, bottom: NOTE_SIZE_PT }],
      "texto",
    );
  });

  it("guardar limpia el estado modificado y adopta la ruta nueva", async () => {
    useDocuments.setState((s) => ({ tabs: s.tabs.map((t) => ({ ...t, dirty: true })) }));
    tauri.saveDocument.mockResolvedValue("/tmp/copia.pdf");
    tauri.getRecentDocuments.mockResolvedValue([]);
    expect(await useDocuments.getState().saveTab(1, "/tmp/copia.pdf")).toBe(true);
    expect(tab().dirty).toBe(false);
    expect(tab().path).toBe("/tmp/copia.pdf");
  });

  it("cerrar con cambios pide confirmación y respeta el cancelar", async () => {
    useDocuments.setState((s) => ({ tabs: s.tabs.map((t) => ({ ...t, dirty: true })) }));
    dialog.ask.mockResolvedValue(false);
    expect(await useDocuments.getState().closeTab(1)).toBe(false);
    expect(tauri.closeDocument).not.toHaveBeenCalled();
    expect(useDocuments.getState().tabs).toHaveLength(1);

    dialog.ask.mockResolvedValue(true);
    tauri.closeDocument.mockResolvedValue(undefined);
    expect(await useDocuments.getState().closeTab(1)).toBe(true);
    expect(useDocuments.getState().tabs).toHaveLength(0);
  });
});

describe("tileUrl", () => {
  it("agrega la revisión como query solo cuando el documento mutó", () => {
    expect(tileUrl(1, 2, 300)).toMatch(/\/tile\/1\/2\/300$/);
    expect(tileUrl(1, 2, 300, 4)).toMatch(/\/tile\/1\/2\/300\?r=4$/);
  });
});
