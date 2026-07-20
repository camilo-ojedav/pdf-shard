import { useEffect } from "react";
import { S } from "../strings/es";
import { fileTitle, useDocuments } from "../state/document";

interface StartScreenProps {
  onOpen: () => void;
}

export function StartScreen({ onOpen }: StartScreenProps) {
  const recents = useDocuments((s) => s.recents);
  const openPath = useDocuments((s) => s.openPath);
  const clearRecents = useDocuments((s) => s.clearRecents);
  const refreshRecents = useDocuments((s) => s.refreshRecents);

  useEffect(() => {
    void refreshRecents();
  }, [refreshRecents]);

  return (
    <div className="start-screen">
      <p className="empty">{S.visor.sinDocumento}</p>
      <button onClick={onOpen}>{S.visor.abrir}</button>

      <div className="start-recents">
        <h3>{S.recientes.titulo}</h3>
        {recents.length === 0 && <p className="dropdown-empty">{S.recientes.vacio}</p>}
        <ul>
          {recents.map((r) => (
            <li key={r.path}>
              <button className="start-recent-item" onClick={() => void openPath(r.path)} title={r.path}>
                {fileTitle(r.path)}
              </button>
            </li>
          ))}
        </ul>
        {recents.length > 0 && (
          <button className="link-button" onClick={() => void clearRecents()}>
            {S.recientes.limpiar}
          </button>
        )}
      </div>
    </div>
  );
}
