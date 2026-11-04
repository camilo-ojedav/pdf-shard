import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Annotation } from "../lib/tauri";
import { S } from "../strings/es";
import { fileTitle, useDocuments, type DocumentTab } from "../state/document";

const XFDF_FILTER = [{ name: "XFDF", extensions: ["xfdf"] }];

function xfdfDefaultName(pdfPath: string): string {
  return fileTitle(pdfPath).replace(/\.pdf$/i, "") + ".xfdf";
}

function AnnotationItem({ tab, annotation }: { tab: DocumentTab; annotation: Annotation }) {
  const goToPage = useDocuments((s) => s.goToPage);
  const editAnnotation = useDocuments((s) => s.editAnnotation);
  const removeAnnotation = useDocuments((s) => s.removeAnnotation);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(annotation.contents ?? "");

  function startEditing() {
    setDraft(annotation.contents ?? "");
    setEditing(true);
  }

  return (
    <li className="annot-item">
      <button className="annot-summary" onClick={() => goToPage(tab.id, annotation.page_index + 1)}>
        <span
          className="annot-swatch"
          style={{ background: annotation.color ?? "var(--muted)" }}
          aria-hidden="true"
        />
        <span className="annot-kind">{S.anotar.tipo[annotation.kind]}</span>
        <span className="annot-page">{S.anotar.pagina(annotation.page_index + 1)}</span>
      </button>
      {editing ? (
        <form
          className="annot-edit"
          onSubmit={(e) => {
            e.preventDefault();
            setEditing(false);
            void editAnnotation(tab.id, annotation.id, draft);
          }}
        >
          <textarea
            autoFocus
            rows={3}
            value={draft}
            placeholder={S.anotar.notaPlaceholder}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setEditing(false);
            }}
          />
          <div className="annot-actions">
            <button type="button" onClick={() => setEditing(false)}>
              {S.anotar.cancelar}
            </button>
            <button type="submit">{S.anotar.guardarTexto}</button>
          </div>
        </form>
      ) : (
        <>
          <p className={annotation.contents ? "annot-text" : "annot-text annot-text-empty"}>
            {annotation.contents ?? S.anotar.sinTexto}
          </p>
          {annotation.author && <p className="annot-author">{annotation.author}</p>}
          <div className="annot-actions">
            <button onClick={startEditing}>{S.anotar.editar}</button>
            <button onClick={() => void removeAnnotation(tab.id, annotation.id)}>
              {S.anotar.eliminar}
            </button>
          </div>
        </>
      )}
    </li>
  );
}

/** Panel lateral con la lista de anotaciones del documento, su edición y el
 * intercambio por XFDF. */
export function AnnotationsPanel({ tab }: { tab: DocumentTab }) {
  const loadAnnotations = useDocuments((s) => s.loadAnnotations);
  const importXfdf = useDocuments((s) => s.importXfdf);
  const exportXfdf = useDocuments((s) => s.exportXfdf);

  useEffect(() => {
    if (tab.annotations === null) void loadAnnotations(tab.id);
  }, [tab.annotations, tab.id, loadAnnotations]);

  async function onImport() {
    const path = await open({ multiple: false, filters: XFDF_FILTER });
    if (typeof path === "string") await importXfdf(tab.id, path);
  }

  async function onExport() {
    const path = await save({ defaultPath: xfdfDefaultName(tab.path), filters: XFDF_FILTER });
    if (path) await exportXfdf(tab.id, path);
  }

  return (
    <div className="annot-panel">
      <div className="annot-xfdf">
        <button onClick={() => void onImport()}>{S.anotar.importarXfdf}</button>
        <button onClick={() => void onExport()} disabled={(tab.annotations?.length ?? 0) === 0}>
          {S.anotar.exportarXfdf}
        </button>
      </div>
      {tab.annotations !== null && tab.annotations.length === 0 && (
        <p className="sidebar-empty">{S.anotar.sinAnotaciones}</p>
      )}
      <ul className="annot-list">
        {tab.annotations?.map((a) => (
          // El id cambia tras cada mutación; la clave incluye el contenido
          // para que un ítem en edición no herede el borrador de otro.
          <AnnotationItem key={`${a.id}|${a.kind}|${a.contents ?? ""}`} tab={tab} annotation={a} />
        ))}
      </ul>
    </div>
  );
}
