import type { GraphResponse } from "../api/types";

export function GraphHeader({ data }: { data: GraphResponse }) {
  return <div className="graph-header">
    <strong className="graph-title">Dependencies and equivalent statements</strong>
    <div className="visible-count">
      <strong>{data.totals.declarations.toLocaleString()}</strong><span>declarations</span>
      <strong>{data.totals.groups.toLocaleString()}</strong><span>groups</span>
      <strong>{data.totals.connections.toLocaleString()}</strong><span>connections</span>
    </div>
  </div>;
}
