import { useEffect, useRef } from "react";
import { S } from "../strings/es";
import { useDocuments, type DocumentTab } from "../state/document";

interface SearchBarProps {
  tab: DocumentTab;
}

export function SearchBar({ tab }: SearchBarProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const setSearchQuery = useDocuments((s) => s.setSearchQuery);
  const runSearch = useDocuments((s) => s.runSearch);
  const nextResult = useDocuments((s) => s.nextResult);
  const prevResult = useDocuments((s) => s.prevResult);
  const closeSearch = useDocuments((s) => s.closeSearch);

  useEffect(() => {
    if (tab.search.open) inputRef.current?.focus();
  }, [tab.search.open]);

  useEffect(() => {
    const handle = setTimeout(() => {
      void runSearch(tab.id);
    }, 250);
    return () => clearTimeout(handle);
  }, [tab.search.query, tab.search.matchCase, tab.search.wholeWord, tab.id, runSearch]);

  if (!tab.search.open) return null;

  const total = tab.search.results?.hits.length ?? 0;

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) prevResult(tab.id);
      else nextResult(tab.id);
    } else if (e.key === "Escape") {
      closeSearch(tab.id);
    }
  }

  return (
    <div className="search-bar">
      <input
        ref={inputRef}
        type="text"
        placeholder={S.buscar.placeholder}
        value={tab.search.query}
        onChange={(e) => setSearchQuery(tab.id, e.target.value)}
        onKeyDown={onKeyDown}
      />
      <span className="search-count">
        {tab.search.query.trim().length === 0
          ? ""
          : total === 0
            ? S.buscar.sinResultados
            : S.buscar.contador(tab.search.activeHit + 1, total)}
      </span>
      <button onClick={() => prevResult(tab.id)} disabled={total === 0} title={S.buscar.anteriorResultado}>
        ▲
      </button>
      <button onClick={() => nextResult(tab.id)} disabled={total === 0} title={S.buscar.siguienteResultado}>
        ▼
      </button>
      {tab.search.results?.truncated && <span className="search-truncated">{S.buscar.truncado}</span>}
      <button onClick={() => closeSearch(tab.id)} title={S.buscar.cerrar}>
        ×
      </button>
    </div>
  );
}
