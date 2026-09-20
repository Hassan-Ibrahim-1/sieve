import type { GraphNode } from "../api/types";

export const palette = {
  group: "#20a978",
  theorem: "#20a978",
  definition: "#7182ee",
  proof: "#d18d45",
  technical: "#83908b",
  edge: "#82918b",
  accent: "#20a978",
};

export function nodeColor(node: GraphNode) {
  if (node.technical) return palette.technical;
  if (node.nodeKind === "group") return palette.group;
  return node.declarationKind === "theorem" ? palette.theorem : palette.definition;
}

export function nodeSize(node: GraphNode, mostUsed: boolean, maximum: number) {
  if (node.nodeKind === "group") {
    const packingRadius = 24 + Math.ceil(Math.sqrt(node.memberCount)) * 11;
    if (!mostUsed) return packingRadius;
    const normalized = maximum > 0 ? Math.sqrt((node.metrics.directDependents ?? 0) / maximum) : 0;
    return packingRadius + normalized * 24;
  }
  if (!mostUsed) return 7;
  const normalized = maximum > 0 ? Math.sqrt((node.metrics.directDependents ?? 0) / maximum) : 0;
  return 6 + normalized * 15;
}
