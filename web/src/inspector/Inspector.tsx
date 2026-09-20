import type { GraphEdge, GraphNode, WitnessResponse } from "../api/types";
import { StatementView } from "./StatementView";
import { WitnessPathView } from "./WitnessPathView";

interface Props {
  node?: GraphNode;
  edge?: GraphEdge;
  witnesses?: WitnessResponse;
  busy?: boolean;
  onWitnesses: (edge: GraphEdge) => void;
  onCloseDetail: () => void;
}

export function Inspector(props: Props) {
  if (props.witnesses) return <aside className="inspector" data-open="true"><WitnessPathView data={props.witnesses} onClose={props.onCloseDetail} /></aside>;
  const { node, edge } = props;
  return <aside className="inspector" data-open={Boolean(node || edge)} aria-label="Selection inspector">
    {!node && !edge && <EmptyInspector busy={props.busy} />}
    {edge && <>
      <div className="kind-row"><span>{edge.kind}</span><code>{edge.aggregateCount} connection{edge.aggregateCount === 1 ? "" : "s"}</code></div>
      <div className="edge-route"><span>{edge.source}</span><i>→</i><span>{edge.target}</span></div>
      {edge.statementCount > 0 && <Fact label="Statement references" value={String(edge.statementCount)} />}
      {edge.proofCount > 0 && <Fact label="Proof references" value={String(edge.proofCount)} />}
      {edge.witnessAvailable && <button className="primary-action" onClick={() => props.onWitnesses(edge)}>Show witness paths <span>→</span></button>}
    </>}
    {node && <>
      <StatementView node={node} />
      <div className="facts">
        {node.nodeKind === "group" && <Fact label="Equivalent declarations" value={String(node.memberCount)} />}
        {node.metrics.directDependents !== undefined && <Fact label="Direct dependents" value={String(node.metrics.directDependents)} />}
      </div>
      {node.nodeKind === "group" && <div className="group-members" aria-label="Equivalent declarations">
        {node.memberIds.map((member) => <code key={member}>{member.replace(/^decl:/, "")}</code>)}
      </div>}
    </>}
  </aside>;
}

function EmptyInspector({ busy }: { busy?: boolean }) {
  return <div className="empty-inspector">
    {busy ? <><span className="loading-ring" /><p>Loading evidence</p></> : <><div className="selection-glyph">⌁</div><p>Select a node or edge</p></>}
  </div>;
}

function Fact({ label, value }: { label: string; value: string }) { return <div className="fact"><span>{label}</span><strong>{value}</strong></div>; }
