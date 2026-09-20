import { useEffect, useMemo } from "react";
import { useLoadGraph, useRegisterEvents, useSigma } from "@react-sigma/core";
import { MultiDirectedGraph } from "graphology";
import type { GraphResponse, Metric } from "../api/types";
import { nodeColor, nodeSize, palette } from "./graphStyles";

const layoutCache = new Map<string, Record<string, { x: number; y: number }>>();
const cameraCache = new Map<string, { x: number; y: number; angle: number; ratio: number }>();

interface Props {
  data: GraphResponse;
  metric: Metric;
  selected?: string;
  selectedEdge?: string;
  pins: string[];
  search: string;
  layout: "force" | "layered";
  onSelect: (id?: string) => void;
  onSelectEdge: (id?: string) => void;
  onOpen: (id: string) => void;
  onOpenEdge: (id: string) => void;
}

export function GraphRenderer(props: Props) {
  const { data, metric, selected, selectedEdge, pins, search, layout, onSelect, onSelectEdge, onOpen, onOpenEdge } = props;
  const loadGraph = useLoadGraph();
  const registerEvents = useRegisterEvents();
  const sigma = useSigma();
  const layoutKey = `${data.corpusFingerprint}:${layout}:${data.scope.level}:${data.scope.id ?? "root"}`;
  const maximum = useMemo(() => Math.max(1, ...data.nodes.map((node) => node.metrics[metric] ?? node.metrics.recommended ?? 1)), [data.nodes, metric]);

  useEffect(() => {
    const graph = new MultiDirectedGraph();
    const cached = layoutCache.get(layoutKey);
    for (const node of data.nodes) {
      const position = cached?.[node.id] ?? node.position;
      graph.addNode(node.id, {
        x: position.x,
        y: position.y,
        size: nodeSize(node, metric, maximum),
        label: compactGraphLabel(node.displayStatement, node.nodeKind === "family" ? 46 : 38),
        color: nodeColor(node),
        nodeKind: node.nodeKind,
      });
    }
    for (const edge of data.edges) {
      if (!graph.hasNode(edge.source) || !graph.hasNode(edge.target)) continue;
      graph.addEdgeWithKey(edge.id, edge.source, edge.target, {
        size: Math.min(4, 0.45 + Math.log2(edge.aggregateCount + 1) * 0.55),
        color: palette.edge,
        type: edge.directed ? "arrow" : "line",
      });
    }
    loadGraph(graph);
    if (!cached && data.nodes.length > 1) {
      const worker = new Worker(new URL("./LayoutWorker.ts", import.meta.url), { type: "module" });
      worker.postMessage({
        kind: layout,
        nodes: data.nodes.map((node) => ({
          id: node.id,
          ...node.position,
          size: nodeSize(node, metric, maximum),
          width: Math.min(260, 48 + node.displayStatement.length * 3.8),
        })),
        edges: data.edges,
      });
      worker.onmessage = (event: MessageEvent<Record<string, { x: number; y: number }>>) => {
        layoutCache.set(layoutKey, event.data);
        for (const [id, position] of Object.entries(event.data)) {
          if (graph.hasNode(id)) graph.mergeNodeAttributes(id, position);
        }
        sigma.refresh();
        sigma.getCamera().animatedReset({ duration: matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 240 });
        worker.terminate();
      };
      return () => worker.terminate();
    }
  }, [data, layout, layoutKey, loadGraph, maximum, metric, sigma]);

  useEffect(() => registerEvents({
    clickNode: ({ node }) => onSelect(node),
    doubleClickNode: ({ node, event }) => { event.preventSigmaDefault(); onOpen(node); },
    clickEdge: ({ edge }) => onSelectEdge(edge),
    doubleClickEdge: ({ edge, event }) => { event.preventSigmaDefault(); onOpenEdge(edge); },
    clickStage: () => { onSelect(undefined); onSelectEdge(undefined); },
  }), [onOpen, onOpenEdge, onSelect, onSelectEdge, registerEvents]);

  useEffect(() => {
    const saved = cameraCache.get(layoutKey);
    if (saved) sigma.getCamera().setState(saved);
    return () => {
      cameraCache.set(layoutKey, sigma.getCamera().getState());
    };
  }, [layoutKey, sigma]);

  useEffect(() => {
    const normalized = search.trim().toLowerCase();
    const topLabels = new Set(data.nodes
      .slice().sort((a, b) => (b.metrics[metric] ?? 0) - (a.metrics[metric] ?? 0)).slice(0, 3).map((node) => node.id));
    sigma.setSetting("nodeReducer", (node, attributes) => {
      const source = data.nodes.find((item) => item.id === node);
      const match = !normalized || !!source && `${source.statement} ${source.leanName ?? ""}`.toLowerCase().includes(normalized);
      const graph = sigma.getGraph();
      const relevant = !selected || !graph.hasNode(selected) || node === selected || graph.neighbors(selected).includes(node);
      return {
        ...attributes,
        color: !match || !relevant ? "#9aa29f" : attributes.color,
        hidden: false,
        highlighted: node === selected || pins.includes(node),
        forceLabel: source?.nodeKind === "family" || node === selected || pins.includes(node) || topLabels.has(node),
        zIndex: node === selected || pins.includes(node) ? 2 : 1,
      };
    });
    sigma.setSetting("edgeReducer", (edge, attributes) => {
      const extremities = sigma.getGraph().extremities(edge);
      const relevant = !selected || extremities.includes(selected);
      return { ...attributes, hidden: !relevant, color: edge === selectedEdge ? palette.accent : attributes.color, size: edge === selectedEdge ? 3 : attributes.size };
    });
    sigma.refresh();
  }, [data.nodes, metric, pins, search, selected, selectedEdge, sigma]);
  return null;
}

export function compactGraphLabel(value: string, maximum = 38) {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= maximum) return normalized;
  return `${normalized.slice(0, maximum - 1).trimEnd()}…`;
}
