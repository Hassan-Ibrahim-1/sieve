import type { GraphNode, Metric } from "../api/types";

export const palette = {
  family: "#20a978",
  theorem: "#20a978",
  definition: "#7182ee",
  proof: "#d18d45",
  technical: "#83908b",
  edge: "#82918b",
  accent: "#20a978",
};

export function nodeColor(node: GraphNode) {
  if (node.technical) return palette.technical;
  if (node.nodeKind === "proofStep" || node.nodeKind === "rawProofStep") return palette.proof;
  if (node.nodeKind === "family") return palette.family;
  return node.declarationKind === "theorem" ? palette.theorem : palette.definition;
}

export function nodeSize(node: GraphNode, metric: Metric, maximum: number) {
  const value = Math.max(0, node.metrics[metric] ?? node.metrics.recommended ?? 1);
  const normalized = maximum > 0 ? Math.sqrt(value / maximum) : 0;
  return 5 + normalized * (node.nodeKind === "family" ? 20 : 13);
}
