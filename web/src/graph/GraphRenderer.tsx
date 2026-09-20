import { useEffect, useMemo } from "react";
import { useLoadGraph, useRegisterEvents, useSigma } from "@react-sigma/core";
import { MultiDirectedGraph } from "graphology";
import type { GraphNode, GraphResponse } from "../api/types";
import type { Theme } from "../theme/useTheme";
import { nodeColor, nodeSize, palette } from "./graphStyles";

const layoutCache = new Map<string, Record<string, { x: number; y: number }>>();
const cameraCache = new Map<string, { x: number; y: number; angle: number; ratio: number }>();

interface Props {
  data: GraphResponse;
  mostUsed: boolean;
  selected?: string;
  selectedEdge?: string;
  pins: string[];
  search: string;
  theme: Theme;
  onSelect: (id?: string) => void;
  onSelectEdge: (id?: string) => void;
  onOpen: (id: string) => void;
  onOpenEdge: (id: string) => void;
}

export function GraphRenderer(props: Props) {
  const { data, mostUsed, selected, selectedEdge, pins, search, theme, onSelect, onSelectEdge, onOpen, onOpenEdge } = props;
  const loadGraph = useLoadGraph();
  const registerEvents = useRegisterEvents();
  const sigma = useSigma();
  const nodeSignature = useMemo(() => data.nodes.map((node) => node.id).join("|"), [data.nodes]);
  const layoutKey = `${data.corpusFingerprint}:${mostUsed ? "usage" : "default"}:${nodeSignature}`;
  const maximum = useMemo(
    () => Math.max(1, ...data.nodes.map((node) => node.metrics.directDependents ?? 0)),
    [data.nodes],
  );

  useEffect(() => {
    const graph = new MultiDirectedGraph();
    const cached = layoutCache.get(layoutKey);
    for (const node of data.nodes) {
      const position = cached?.[node.id] ?? node.position;
      graph.addNode(node.id, {
        x: position.x,
        y: position.y,
        size: nodeSize(node, mostUsed, maximum),
        label: compactGraphLabel(node.displayStatement, node.nodeKind === "group" ? 46 : 38),
        labelTheme: theme,
        color: node.nodeKind === "group" ? groupMaskColor(theme) : nodeColor(node),
        nodeKind: node.nodeKind,
        zIndex: node.nodeKind === "group" ? 0 : 1,
      });
    }
    for (const edge of data.edges) {
      if (!graph.hasNode(edge.source) || !graph.hasNode(edge.target)) continue;
      graph.addEdgeWithKey(edge.id, edge.source, edge.target, {
        size: Math.min(4, 0.45 + Math.log2(edge.aggregateCount + 1) * 0.55),
        color: palette.edge,
        type: "arrow",
      });
    }
    loadGraph(graph);
    if (!cached && data.nodes.length > 1) {
      const worker = new Worker(new URL("./LayoutWorker.ts", import.meta.url), { type: "module" });
      worker.postMessage({
        kind: "grouped-force",
        mostUsed,
        nodes: data.nodes.map((node) => ({
          id: node.id,
          ...node.position,
          groupId: node.groupId,
          nodeKind: node.nodeKind,
          size: nodeSize(node, mostUsed, maximum),
          usage: node.metrics.directDependents ?? 0,
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
  }, [data, layoutKey, loadGraph, maximum, mostUsed, sigma, theme]);

  useEffect(() => {
    const layerId = "equivalence-groups";
    if (sigma.getCanvases()[layerId]) sigma.killLayer(layerId);
    const groups = data.nodes.filter((node) => node.nodeKind === "group");
    if (groups.length === 0) return;

    const canvas = sigma.createCanvas(layerId, { beforeLayer: "labels", style: { pointerEvents: "none" } });
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
      for (const group of groups) {
        const point = sigma.getNodeDisplayData(group.id);
        if (!point) continue;
        context.beginPath();
        context.arc(point.x, point.y, point.size, 0, Math.PI * 2);
        context.fillStyle = theme === "dark" ? "rgba(66, 199, 147, .075)" : "rgba(24, 128, 92, .065)";
        context.fill();
        context.strokeStyle = theme === "dark" ? "rgba(98, 219, 171, .5)" : "rgba(20, 112, 81, .42)";
        context.lineWidth = group.id === selected ? 2.5 : 1.5;
        context.stroke();
      }
    };

    sigma.on("afterRender", draw);
    draw();
    return () => {
      sigma.off("afterRender", draw);
      if (sigma.getCanvases()[layerId]) sigma.killLayer(layerId);
    };
  }, [data.nodes, selected, sigma, theme]);

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
    return () => { cameraCache.set(layoutKey, sigma.getCamera().getState()); };
  }, [layoutKey, sigma]);

  useEffect(() => {
    if (!selected || !sigma.getGraph().hasNode(selected)) return;
    const position = sigma.getNodeDisplayData(selected);
    if (position) sigma.getCamera().animate(position, { duration: 300 });
  }, [selected, sigma]);

  useEffect(() => {
    const normalized = search.trim().toLowerCase();
    const byId = new Map(data.nodes.map((node) => [node.id, node]));
    const matchingGroups = new Set(data.nodes
      .filter((node) => node.nodeKind === "declaration" && node.groupId && matches(node, normalized))
      .map((node) => node.groupId!));
    const topLabels = new Set(data.nodes
      .filter((node) => node.nodeKind === "declaration")
      .slice().sort((a, b) => (b.metrics.directDependents ?? 0) - (a.metrics.directDependents ?? 0))
      .slice(0, 5).map((node) => node.id));
    const selectedNode = selected ? byId.get(selected) : undefined;
    const selectedEndpoint = selectedNode?.groupId ?? selected;
    sigma.setSetting("nodeReducer", (node, attributes) => {
      const source = byId.get(node);
      const match = !normalized || !!source && (
        matches(source, normalized)
        || matchingGroups.has(source.id)
        || !!source.groupId && matchingGroups.has(source.groupId)
      );
      const graph = sigma.getGraph();
      const sameGroup = selectedNode?.nodeKind === "group"
        ? source?.groupId === selectedNode.id
        : !!selectedNode?.groupId && (source?.groupId === selectedNode.groupId || source?.id === selectedNode.groupId);
      const relevant = !selectedEndpoint || !graph.hasNode(selectedEndpoint) || node === selectedEndpoint || sameGroup || graph.neighbors(selectedEndpoint).includes(node);
      return {
        ...attributes,
        color: source?.nodeKind === "group" ? groupMaskColor(theme) : !match || !relevant ? "#9aa29f" : attributes.color,
        hidden: false,
        highlighted: node === selected || pins.includes(node),
        forceLabel: node === selected || pins.includes(node) || topLabels.has(node),
        labelTheme: theme,
        zIndex: node === selected || pins.includes(node) ? 3 : source?.nodeKind === "group" ? 0 : 1,
      };
    });
    sigma.setSetting("edgeReducer", (edge, attributes) => {
      const extremities = sigma.getGraph().extremities(edge);
      const relevant = !selectedEndpoint || extremities.includes(selectedEndpoint);
      return { ...attributes, hidden: !relevant, color: edge === selectedEdge ? palette.accent : attributes.color, size: edge === selectedEdge ? 3 : attributes.size };
    });
    sigma.refresh();
  }, [data.nodes, pins, search, selected, selectedEdge, sigma, theme]);
  return null;
}

function matches(node: GraphNode, normalized: string) {
  return !normalized || `${node.statement} ${node.leanName ?? ""}`.toLowerCase().includes(normalized);
}

function groupMaskColor(theme: Theme) {
  return theme === "dark" ? "#0b0e0d" : "#f3f5f2";
}

export function compactGraphLabel(value: string, maximum = 38) {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= maximum) return normalized;
  return `${normalized.slice(0, maximum - 1).trimEnd()}…`;
}
