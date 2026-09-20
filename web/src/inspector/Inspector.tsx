import type { Comparison, GraphEdge, GraphNode, WitnessResponse } from "../api/types";
import { ComparisonView } from "./ComparisonView";
import { StatementView } from "./StatementView";
import { WitnessPathView } from "./WitnessPathView";

interface Props {
  node?: GraphNode;
  edge?: GraphEdge;
  pins: string[];
  comparison?: Comparison[];
  witnesses?: WitnessResponse;
  busy?: boolean;
  onPin: (id: string) => void;
  onCompare: () => void;
  onWitnesses: (edge: GraphEdge) => void;
  onCloseDetail: () => void;
}

export function Inspector(props: Props) {
  if (props.comparison) return <aside className="inspector" data-open="true"><ComparisonView data={props.comparison} onClose={props.onCloseDetail} /></aside>;
  if (props.witnesses) return <aside className="inspector" data-open="true"><WitnessPathView data={props.witnesses} onClose={props.onCloseDetail} /></aside>;
  const { node, edge } = props;
  return <aside className="inspector" data-open={Boolean(node || edge || props.pins.length)} aria-label="Selection inspector">
    {!node && !edge && <EmptyInspector pins={props.pins} busy={props.busy} onCompare={props.onCompare} />}
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
      {node.nodeKind === "declaration" && <button className="pin-button" data-active={props.pins.includes(node.id)} onClick={() => props.onPin(node.id)}>
        <span>{props.pins.includes(node.id) ? "Pinned" : "Pin for comparison"}</span><kbd>{props.pins.length}/2</kbd>
      </button>}
      {props.pins.length === 2 && <button className="secondary-action" onClick={props.onCompare}>Compare pinned statements</button>}
    </>}
  </aside>;
}

function EmptyInspector({ pins, busy, onCompare }: { pins: string[]; busy?: boolean; onCompare: () => void }) {
  return <div className="empty-inspector">
    {busy ? <><span className="loading-ring" /><p>Loading evidence</p></> : pins.length ? <>
      <div className="inspector-title"><span>Pinned statements</span><small>{pins.length}/2</small></div>
      {pins.map((pin) => <code key={pin}>{pin.replace(/^decl:/, "")}</code>)}
      {pins.length === 2 && <button className="primary-action" onClick={onCompare}>Compare statements <span>→</span></button>}
    </> : <><div className="selection-glyph">⌁</div><p>Select a node or edge</p></>}
  </div>;
}

function Fact({ label, value }: { label: string; value: string }) { return <div className="fact"><span>{label}</span><strong>{value}</strong></div>; }
