import type { Comparison, GraphEdge, GraphMode, GraphNode, WitnessResponse } from "../api/types";
import { ComparisonView } from "./ComparisonView";
import { StatementView } from "./StatementView";
import { WitnessPathView } from "./WitnessPathView";

interface Props {
  mode: GraphMode;
  node?: GraphNode;
  edge?: GraphEdge;
  pins: string[];
  comparison?: Comparison[];
  witnesses?: WitnessResponse;
  busy?: boolean;
  onPin: (id: string) => void;
  onPrimary: (node: GraphNode) => void;
  onCompare: () => void;
  onWitnesses: (edge: GraphEdge) => void;
  onOpenEdge: (edge: GraphEdge) => void;
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
      {edge.similarity !== undefined && <Fact label="Similarity" value={`${Math.round(edge.similarity * 100)}%`} />}
      {edge.witnessAvailable && <button className="primary-action" onClick={() => props.onWitnesses(edge)}>Show witness paths <span>→</span></button>}
      {!edge.witnessAvailable && edge.kind === "condensedProofPath" && (edge.collapsedStepCount ?? 0) > 0 &&
        <button className="primary-action" onClick={() => props.onOpenEdge(edge)}>Expand {edge.collapsedStepCount} steps <span>→</span></button>}
    </>}
    {node && <>
      <StatementView node={node} />
      <div className="facts">
        {node.nodeKind === "family" && <Fact label="Members" value={String(node.memberCount)} />}
        {node.metrics.directDependents !== undefined && <Fact label="Direct dependents" value={String(node.metrics.directDependents)} />}
        {node.metrics.reachableDependents !== undefined && <Fact label="Reachable" value={String(node.metrics.reachableDependents)} />}
        {node.metrics.bridgeEvidence !== undefined && props.mode === "connections" && <Fact label="Bridge evidence" value={String(node.metrics.bridgeEvidence)} />}
      </div>
      {node.nodeKind === "declaration" && <button className="pin-button" data-active={props.pins.includes(node.id)} onClick={() => props.onPin(node.id)}>
        <span>{props.pins.includes(node.id) ? "Pinned" : "Pin for comparison"}</span><kbd>{props.pins.length}/2</kbd>
      </button>}
      {(node.nodeKind === "family" || node.nodeKind === "declaration") &&
        <button className="primary-action" onClick={() => props.onPrimary(node)}>{primaryLabel(props.mode, node)} <span>→</span></button>}
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
function primaryLabel(mode: GraphMode, node: GraphNode) {
  if (node.nodeKind === "family") return "Expand family";
  if (mode === "similarity") return "Compare nearby statements";
  if (mode === "influence") return "Trace downstream use";
  if (mode === "connections") return "Show connections";
  return "Inspect proof claim";
}
