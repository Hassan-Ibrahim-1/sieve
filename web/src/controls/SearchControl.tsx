import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { GraphNode } from "../api/types";

export function SearchControl({ value, onChange, onSelect }: {
  value: string; onChange: (value: string) => void; onSelect: (id: string) => void;
}) {
  const [results, setResults] = useState<GraphNode[]>([]);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (value.trim().length < 2) { setResults([]); return; }
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      setBusy(true);
      api.search(value.trim(), controller.signal)
        .then((response) => setResults(response.results))
        .catch((error) => { if (error.name !== "AbortError") setResults([]); })
        .finally(() => setBusy(false));
    }, 180);
    return () => { controller.abort(); clearTimeout(timer); };
  }, [value]);
  return <section className="search-section">
    <label className="field-label" htmlFor="statement-search">Statement search</label>
    <div className="search-box">
      <svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5"/><path d="m13 13 4 4"/></svg>
      <input id="statement-search" type="search" placeholder="Formula or Lean name" value={value} onChange={(event) => onChange(event.target.value)} />
      {busy && <span className="search-pulse" />}
    </div>
    {results.length > 0 && <div className="search-results">
      {results.map((node) => <button key={node.id} onClick={() => { onSelect(node.id); onChange(""); }}>
        <span>{node.displayStatement}</span><small>{node.leanName}</small>
      </button>)}
    </div>}
  </section>;
}
