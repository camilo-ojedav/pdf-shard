import { S } from "../strings/es";
import { fileTitle, useDocuments } from "../state/document";

interface TabBarProps {
  onOpenAnother: () => void;
}

export function TabBar({ onOpenAnother }: TabBarProps) {
  const tabs = useDocuments((s) => s.tabs);
  const activeTabId = useDocuments((s) => s.activeTabId);
  const setActiveTab = useDocuments((s) => s.setActiveTab);
  const closeTab = useDocuments((s) => s.closeTab);

  if (tabs.length === 0) return null;

  return (
    <div className="tab-bar">
      {tabs.map((t) => (
        <div
          key={t.id}
          className={t.id === activeTabId ? "tab tab-active" : "tab"}
          onClick={() => setActiveTab(t.id)}
          title={t.path}
        >
          <span className="tab-title">{fileTitle(t.path)}</span>
          <button
            className="tab-close"
            title={S.visor.cerrarPestana}
            onClick={(e) => {
              e.stopPropagation();
              void closeTab(t.id);
            }}
          >
            ×
          </button>
        </div>
      ))}
      <button className="tab-add" title={S.visor.nuevaPestana} onClick={onOpenAnother}>
        +
      </button>
    </div>
  );
}
