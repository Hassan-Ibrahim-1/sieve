import type { ProofOutlineResponse } from "../api/types";
import { aggregateProofEdges, proofLayout, proofNodeId, proofStepFromNodeId } from "../proof/proofGraphModel";

const data: ProofOutlineResponse = {
  declaration: "Fixture.result",
  statement: "P → P",
  outline: {
    algorithm: "fixture",
    retentionRules: [],
    complete: true,
    truncationReason: null,
    visitedTerms: 2,
    conclusionStep: 1,
    rawStepCount: 2,
    rawEdgeCount: 1,
    candidateNodeCount: 2,
    retainedNodes: [
      { rawStepId: 0, kind: "namedApplication", proposition: "P", context: [], hypothesisReferences: [], namedReferences: ["Fixture.input"] },
      { rawStepId: 1, kind: "conclusion", proposition: "P → P", context: [], hypothesisReferences: [], namedReferences: [] },
    ],
    condensedEdges: [{ source: 0, target: 1, pathCount: 1, maximumRawPathLength: 2, rawStepPath: [0, 1] }],
  },
};

describe("ProofGraph", () => {
  it("lays prerequisites above their dependents in Sigma coordinates", () => {
    const layout = proofLayout(data.outline.retainedNodes, data.outline.condensedEdges);
    expect(layout.nodes.find((node) => node.rawStepId === 0)!.y)
      .toBeGreaterThan(layout.nodes.find((node) => node.rawStepId === 1)!.y);
  });

  it("round-trips proof steps through graph node ids", () => {
    expect(proofStepFromNodeId(proofNodeId(17))).toBe(17);
    expect(proofStepFromNodeId("declaration:17")).toBeUndefined();
  });

  it("wraps broad proof ranks instead of shrinking them into one huge row", () => {
    const prerequisites = Array.from({ length: 64 }, (_, rawStepId) => ({
      rawStepId,
      kind: "localFact",
      proposition: `P${rawStepId}`,
      context: [],
      hypothesisReferences: [],
      namedReferences: [],
    }));
    const conclusion = { ...prerequisites[0], rawStepId: 64, kind: "conclusion", proposition: "Result" };
    const edges = prerequisites.map((node) => ({
      source: node.rawStepId,
      target: 64,
      pathCount: 1,
      maximumRawPathLength: 2,
      rawStepPath: [node.rawStepId, 64],
    }));
    const layout = proofLayout([...prerequisites, conclusion], edges);
    const sourcePositions = layout.nodes.filter((node) => node.rawStepId < 64);
    const width = Math.max(...sourcePositions.map((node) => node.x)) - Math.min(...sourcePositions.map((node) => node.x));

    expect(width).toBeLessThan(2_000);
    expect(new Set(sourcePositions.map((node) => node.y)).size).toBeGreaterThan(1);
    expect(layout.nodes.find((node) => node.rawStepId === 64)!.y).toBeLessThan(Math.min(...sourcePositions.map((node) => node.y)));
  });

  it("aggregates parallel condensed paths for Sigma's simple graph", () => {
    const edges = aggregateProofEdges([
      { source: 0, target: 8, pathCount: 3, maximumRawPathLength: 3, rawStepPath: [0, 3, 8] },
      { source: 0, target: 8, pathCount: 2, maximumRawPathLength: 4, rawStepPath: [0, 4, 6, 8] },
      { source: 2, target: 8, pathCount: 1, maximumRawPathLength: 2, rawStepPath: [2, 8] },
    ]);

    expect(edges).toEqual([
      { source: 0, target: 8, pathCount: 5, maximumRawPathLength: 4 },
      { source: 2, target: 8, pathCount: 1, maximumRawPathLength: 2 },
    ]);
  });
});
