import type { GraphNode } from "../api/types";

export function StatementView({ node }: { node: GraphNode }) {
  return <>
    <div className="kind-row"><span>{formatKind(node)}</span><code>{node.id.replace(/^decl:/, "")}</code></div>
    <div className="statement-block">{node.statement}</div>
    {node.leanName && <button className="copy-name" onClick={() => navigator.clipboard.writeText(node.leanName!)} title="Copy Lean name">
      <span>{node.leanName}</span><svg viewBox="0 0 20 20" aria-hidden="true"><rect x="6" y="6" width="10" height="10" rx="2"/><path d="M4 13H3a2 2 0 0 1-2-2V3a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1"/></svg>
    </button>}
  </>;
}

function formatKind(node: GraphNode) {
  if (node.nodeKind === "group") return "Equivalent statements";
  return node.declarationKind ?? "Declaration";
}
