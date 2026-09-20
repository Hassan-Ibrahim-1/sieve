import type { Breadcrumb, GraphResponse } from "../api/types";

export function GraphHeader({ data, onNavigate }: { data: GraphResponse; onNavigate: (crumb: Breadcrumb) => void }) {
  return <div className="graph-header">
    <nav aria-label="Graph location">
      {data.scope.breadcrumbs.map((crumb, index) => <span key={`${crumb.level}:${crumb.id ?? "root"}`}>
        {index > 0 && <i>/</i>}
        <button disabled={index === data.scope.breadcrumbs.length - 1} onClick={() => onNavigate(crumb)}>{crumb.label}</button>
      </span>)}
    </nav>
    <div className="visible-count"><strong>{data.totals.returned.toLocaleString()}</strong><span>/</span>{data.totals.matching.toLocaleString()}</div>
  </div>;
}
