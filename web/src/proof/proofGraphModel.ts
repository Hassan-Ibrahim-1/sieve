import type { ProofOutlineEdge, ProofOutlineNode } from "../api/types";

const COLUMN_GAP = 170;
const ROW_GAP = 120;
const RANK_GAP = 150;

interface PositionedNode extends ProofOutlineNode { x: number; y: number }

export interface AggregatedProofEdge {
  source: number;
  target: number;
  pathCount: number;
  maximumRawPathLength: number;
}

export function aggregateProofEdges(edges: ProofOutlineEdge[]): AggregatedProofEdge[] {
  const aggregated = new Map<string, AggregatedProofEdge>();
  for (const edge of edges) {
    const key = `${edge.source}:${edge.target}`;
    const existing = aggregated.get(key);
    if (existing) {
      existing.pathCount += 1;
      existing.maximumRawPathLength = Math.max(existing.maximumRawPathLength, edge.rawStepPath.length);
    } else {
      aggregated.set(key, {
        source: edge.source,
        target: edge.target,
        pathCount: 1,
        maximumRawPathLength: edge.rawStepPath.length,
      });
    }
  }
  return [...aggregated.values()];
}

export function proofLayout(nodes: ProofOutlineNode[], edges: ProofOutlineEdge[]) {
  const nodeIds = new Set(nodes.map((node) => node.rawStepId));
  const incoming = new Map<number, number[]>();
  for (const edge of edges) {
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target)) continue;
    incoming.set(edge.target, [...(incoming.get(edge.target) ?? []), edge.source]);
  }
  const depth = new Map<number, number>();
  const visit = (id: number, visiting = new Set<number>()): number => {
    if (depth.has(id)) return depth.get(id)!;
    if (visiting.has(id)) return 0;
    const nextVisiting = new Set(visiting).add(id);
    const prerequisites = incoming.get(id) ?? [];
    const value = prerequisites.length === 0 ? 0 : 1 + Math.max(...prerequisites.map((source) => visit(source, nextVisiting)));
    depth.set(id, value);
    return value;
  };
  nodes.forEach((node) => visit(node.rawStepId));
  const rows = new Map<number, ProofOutlineNode[]>();
  for (const node of nodes) rows.set(depth.get(node.rawStepId)!, [...(rows.get(depth.get(node.rawStepId)!) ?? []), node]);
  for (const row of rows.values()) row.sort((left, right) => left.rawStepId - right.rawStepId);
  let maximumColumns = 1;
  const positioned: PositionedNode[] = [];
  let verticalOffset = 0;
  for (const [, row] of [...rows].sort(([left], [right]) => left - right)) {
    // A single very broad rank makes Sigma fit thousands of horizontal graph
    // units into the viewport, which can make every node look invisible. Wrap
    // broad ranks into a compact grid while keeping each rank in its own band.
    const columns = Math.max(1, Math.ceil(Math.sqrt(row.length * 1.6)));
    maximumColumns = Math.max(maximumColumns, columns);
    const subrows = Math.ceil(row.length / columns);
    row.forEach((node, index) => {
      const subrow = Math.floor(index / columns);
      const column = index % columns;
      const nodesInSubrow = Math.min(columns, row.length - subrow * columns);
      positioned.push({
        ...node,
        x: (column - (nodesInSubrow - 1) / 2) * COLUMN_GAP,
        // Sigma's graph coordinates run upward; negate depth so prerequisites
        // appear above the claims that use them.
        y: -(verticalOffset + subrow * ROW_GAP),
      });
    });
    verticalOffset += Math.max(0, subrows - 1) * ROW_GAP + RANK_GAP;
  }
  return { nodes: positioned, rowCount: Math.max(1, ...rows.keys()) + 1, maximumColumns };
}

export function proofNodeId(step: number) {
  return `proof-step:${step}`;
}

export function proofStepFromNodeId(id: string) {
  const match = /^proof-step:(\d+)$/.exec(id);
  return match ? Number(match[1]) : undefined;
}

export function humanize(value: string) {
  return value.replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
}
