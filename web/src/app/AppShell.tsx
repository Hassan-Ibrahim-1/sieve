import type { ReactNode } from "react";
import type { Theme } from "../theme/useTheme";

export function AppShell({ theme, view, onTheme, onView, controls, children, inspector }: {
  theme: Theme;
  view: "sieve" | "proof";
  onTheme: () => void;
  onView: (view: "sieve" | "proof") => void;
  controls: ReactNode;
  children: ReactNode;
  inspector: ReactNode;
}) {
  return <div className="app-shell">
    <header className="topbar">
      <div className="brand-area">
        <div className="brand"><span className="sieve-mark" aria-hidden="true"><i /><i /><i /></span><span>Sieve</span></div>
        <label className="view-picker">
          <span className="sr-only">View</span>
          <select aria-label="View" value={view} onChange={(event) => onView(event.target.value as "sieve" | "proof")}>
            <option value="sieve">Dependency graph</option>
            <option value="proof">Proof steps</option>
          </select>
        </label>
      </div>
      <button className="icon-button theme-toggle" onClick={onTheme} aria-label={`Use ${theme === "dark" ? "light" : "dark"} mode`}>
        {theme === "dark" ? <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>
          : <svg viewBox="0 0 24 24"><path d="M20.5 15.2A8.5 8.5 0 0 1 8.8 3.5 8.5 8.5 0 1 0 20.5 15.2Z"/></svg>}
      </button>
    </header>
    <div className="workspace">{controls}<main className="canvas-column">{children}</main>{inspector}</div>
  </div>;
}
