import { useEffect } from "react";
import { useLoadGraph, useRegisterEvents, useSigma } from "@react-sigma/core";
import { MultiDirectedGraph } from "graphology";
import type { ProofOutlineNode, ProofOutlineResponse } from "../api/types";
import { compactGraphLabel } from "../graph/GraphRenderer";
import { nodeLabelsAreVisible } from "../graph/graphZoom";
import { palette } from "../graph/graphStyles";
import type { Theme } from "../theme/useTheme";
import { aggregateProofEdges, proofLayout, proofNodeId, proofStepFromNodeId } from "./proofGraphModel";

const cameraCache = new Map<string, { x: number; y: number; angle: number; ratio: number }>();
const LARGE_PROOF_THRESHOLD = 120;

export function ProofGraphRenderer({ data, selected, showLabels, theme, onSelect }: {
  data: ProofOutlineResponse;
  selected?: number;
  showLabels: boolean;
  theme: Theme;
  onSelect: (step?: number) => void;
}) {
  const loadGraph = useLoadGraph();
  const registerEvents = useRegisterEvents();
  const sigma = useSigma();
  const conclusion = data.outline.conclusionStep;

  useEffect(() => {
    const graph = new MultiDirectedGraph();
    const layout = proofLayout(data.outline.retainedNodes, data.outline.condensedEdges);
    for (const node of layout.nodes) {
      graph.addNode(proofNodeId(node.rawStepId), {
        x: node.x,
        y: node.y,
        size: node.rawStepId === conclusion ? 12 : node.kind === "branchConclusion" ? 9 : 8,
        label: `#${node.rawStepId} ${compactGraphLabel(node.proposition, 16)}`,
        color: proofNodeColor(node, conclusion),
        proofKind: node.kind,
        type: "circle",
        zIndex: node.rawStepId === conclusion ? 2 : 1,
      });
    }
    for (const edge of aggregateProofEdges(data.outline.condensedEdges)) {
      const source = proofNodeId(edge.source);
      const target = proofNodeId(edge.target);
      if (!graph.hasNode(source) || !graph.hasNode(target)) continue;
      graph.addEdgeWithKey(`proof-edge:${edge.source}:${edge.target}`, source, target, {
        size: Math.min(3, 1 + Math.log2(edge.pathCount + 1) * 0.6),
        color: edge.maximumRawPathLength > 2 ? palette.violet : palette.edge,
        type: "arrow",
      });
    }
    loadGraph(graph);
    // The proof canvas is mounted as the view changes. Let layout settle before
    // measuring and fitting; fitting a transient zero-sized canvas leaves Sigma
    // with a valid graph that appears completely blank.
    const frame = requestAnimationFrame(() => {
      sigma.resize();
      sigma.refresh();
      const saved = cameraCache.get(data.declaration);
      if (saved) sigma.getCamera().setState(saved);
      else if (layout.nodes.length >= LARGE_PROOF_THRESHOLD) {
        // A whole-graph fit makes a large proof look like one dense dot. Open
        // on a legible neighborhood around the conclusion instead; the rest is
        // still available by panning or zooming out.
        const conclusionId = conclusion === null ? undefined : proofNodeId(conclusion);
        const position = conclusionId ? sigma.getNodeDisplayData(conclusionId) : undefined;
        const ratio = Math.max(0.18, Math.min(0.55, Math.sqrt(80 / layout.nodes.length)));
        const target = position ? { x: position.x, y: position.y, ratio } : { ratio };
        sigma.getCamera().animate(target, {
          duration: matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 240,
        });
      } else {
        sigma.getCamera().animatedReset({ duration: matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 240 });
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [conclusion, data, loadGraph, sigma]);

  useEffect(() => registerEvents({
    clickNode: ({ node }) => onSelect(proofStepFromNodeId(node)),
    clickStage: () => onSelect(undefined),
  }), [onSelect, registerEvents]);

  useEffect(() => {
    return () => { cameraCache.set(data.declaration, sigma.getCamera().getState()); };
  }, [data.declaration, sigma]);

  useEffect(() => {
    const camera = sigma.getCamera();
    let labelsVisible: boolean | undefined;
    const syncLabelVisibility = (state: ReturnType<typeof camera.getState>) => {
      const nextLabelsVisible = showLabels && nodeLabelsAreVisible(state.ratio);
      if (nextLabelsVisible === labelsVisible) return;
      labelsVisible = nextLabelsVisible;
      sigma.setSetting("renderLabels", nextLabelsVisible);
    };
    syncLabelVisibility(camera.getState());
    camera.on("updated", syncLabelVisibility);
    return () => { camera.removeListener("updated", syncLabelVisibility); };
  }, [showLabels, sigma]);

  useEffect(() => {
    const selectedId = selected === undefined ? undefined : proofNodeId(selected);
    if (selectedId && sigma.getGraph().hasNode(selectedId)) {
      const position = sigma.getNodeDisplayData(selectedId);
      if (position) sigma.getCamera().animate(position, { duration: 300 });
    }
    const graph = sigma.getGraph();
    const neighborhood = selectedId && graph.hasNode(selectedId)
      ? new Set([selectedId, ...graph.neighbors(selectedId)])
      : undefined;
    sigma.setSetting("nodeReducer", (node, attributes) => {
      const relevant = !neighborhood || neighborhood.has(node);
      return {
        ...attributes,
        color: relevant ? attributes.color : "#9aa29f",
        highlighted: node === selectedId,
        forceLabel: node === selectedId || node === proofNodeId(conclusion ?? -1),
        labelTheme: theme,
        zIndex: node === selectedId ? 3 : attributes.zIndex,
      };
    });
    sigma.setSetting("edgeReducer", (edge, attributes) => {
      const relevant = !selectedId || !sigma.getGraph().hasNode(selectedId) || sigma.getGraph().extremities(edge).includes(selectedId);
      return { ...attributes, hidden: !relevant };
    });
    sigma.refresh();
  }, [conclusion, selected, sigma, theme]);

  return null;
}

function proofNodeColor(node: ProofOutlineNode, conclusion: number | null) {
  if (node.rawStepId === conclusion) return palette.accent;
  if (node.kind === "localFact") return palette.violet;
  if (node.kind === "branchConclusion") return palette.proof;
  return palette.theorem;
}
