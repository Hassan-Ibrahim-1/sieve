import { useEffect, useMemo } from "react";
import { useLoadGraph, useRegisterEvents, useSigma } from "@react-sigma/core";
import { MultiDirectedGraph } from "graphology";
import type { GraphResponse, Metric } from "../api/types";
import type { Theme } from "../theme/useTheme";
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
  layout: "force" | "layered" | "clustered";
  theme: Theme;
  onSelect: (id?: string) => void;
  onSelectEdge: (id?: string) => void;
  onOpen: (id: string) => void;
  onOpenEdge: (id: string) => void;
}

export function GraphRenderer(props: Props) {
  const { data, metric, selected, selectedEdge, pins, search, layout, theme, onSelect, onSelectEdge, onOpen, onOpenEdge } = props;
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
        labelTheme: theme,
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
          familyId: node.familyId,
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

  useEffect(() => {
    const layerId = "family-rings";
    if (sigma.getCanvases()[layerId]) sigma.killLayer(layerId);
    if (data.scope.level !== "corpus") return;

    const families = new Map<string, string[]>();
    for (const node of data.nodes) {
      if (!node.familyId) continue;
      const members = families.get(node.familyId) ?? [];
      members.push(node.id);
      families.set(node.familyId, members);
    }
    if (families.size === 0) return;

    const canvas = sigma.createCanvas(layerId, {
      beforeLayer: "edges",
      style: { pointerEvents: "none" },
    });
    const context = canvas.getContext("2d");
    if (!context) return () => sigma.killLayer(layerId);

    const draw = () => {
      const { width, height } = sigma.getDimensions();
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      const pixelWidth = Math.round(width * ratio);
      const pixelHeight = Math.round(height * ratio);
      if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
        canvas.width = pixelWidth;
        canvas.height = pixelHeight;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;
      }
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, width, height);

      for (const members of families.values()) {
        const points = members
          .map((id) => sigma.getNodeDisplayData(id))
          .filter((point): point is NonNullable<typeof point> => !!point);
        if (points.length === 0) continue;
        const centerX = points.reduce((sum, point) => sum + point.x, 0) / points.length;
        const centerY = points.reduce((sum, point) => sum + point.y, 0) / points.length;
        const radius = Math.max(
          28,
          ...points.map((point) => Math.hypot(point.x - centerX, point.y - centerY) + point.size + 18),
        );
        context.beginPath();
        context.arc(centerX, centerY, radius, 0, Math.PI * 2);
        context.fillStyle = theme === "dark" ? "rgba(66, 199, 147, .045)" : "rgba(24, 128, 92, .035)";
        context.fill();
        context.strokeStyle = theme === "dark" ? "rgba(98, 219, 171, .42)" : "rgba(20, 112, 81, .34)";
        context.lineWidth = 1.5;
        context.setLineDash([5, 5]);
        context.stroke();
        context.setLineDash([]);
      }
    };

    sigma.on("afterRender", draw);
    draw();
    return () => {
      sigma.off("afterRender", draw);
      if (sigma.getCanvases()[layerId]) sigma.killLayer(layerId);
    };
  }, [data.nodes, data.scope.level, sigma, theme]);

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
        labelTheme: theme,
        zIndex: node === selected || pins.includes(node) ? 2 : 1,
      };
    });
    sigma.setSetting("edgeReducer", (edge, attributes) => {
      const extremities = sigma.getGraph().extremities(edge);
      const relevant = !selected || extremities.includes(selected);
      return { ...attributes, hidden: !relevant, color: edge === selectedEdge ? palette.accent : attributes.color, size: edge === selectedEdge ? 3 : attributes.size };
    });
    sigma.refresh();
  }, [data.nodes, metric, pins, search, selected, selectedEdge, sigma, theme]);
  return null;
}

export function compactGraphLabel(value: string, maximum = 38) {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= maximum) return normalized;
  return `${normalized.slice(0, maximum - 1).trimEnd()}…`;
}
