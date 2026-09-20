/// Layout work stays off the interaction thread. ELK handles directed graphs;
/// a small deterministic force solver handles similarity maps.
import ELK from "elkjs/lib/elk.bundled.js";

interface LayoutRequest {
  kind: "force" | "layered";
  nodes: Array<{ id: string; x: number; y: number; size: number; width?: number }>;
  edges: Array<{ source: string; target: string }>;
}

self.onmessage = async (event: MessageEvent<LayoutRequest>) => {
  const payload = event.data;
  if (payload.kind === "layered") {
    const elk = new ELK();
    const result = await elk.layout({
      id: "root",
      layoutOptions: {
        "elk.algorithm": "layered",
        "elk.direction": "RIGHT",
        "elk.spacing.nodeNode": "58",
        "elk.layered.spacing.nodeNodeBetweenLayers": "105",
        "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
      },
      children: payload.nodes.map((node) => ({ id: node.id, width: node.width ?? 42 + node.size * 2, height: 42 + node.size * 2 })),
      edges: payload.edges.map((edge, index) => ({ id: `e${index}`, sources: [edge.source], targets: [edge.target] })),
    });
    self.postMessage(Object.fromEntries((result.children ?? []).map((node) => [node.id, { x: node.x ?? 0, y: node.y ?? 0 }])));
    return;
  }

  const points = new Map(payload.nodes.map((node) => [node.id, { x: node.x, y: node.y }]));
  const edgePairs = payload.edges.map((edge) => [points.get(edge.source)!, points.get(edge.target)!] as const).filter(([a, b]) => a && b);
  const area = Math.max(1, payload.nodes.length) * 2400;
  const ideal = Math.sqrt(area / Math.max(1, payload.nodes.length));
  for (let iteration = 0; iteration < 160; iteration++) {
    const movement = new Map(payload.nodes.map((node) => [node.id, { x: 0, y: 0 }]));
    for (let a = 0; a < payload.nodes.length; a++) {
      for (let b = a + 1; b < payload.nodes.length; b++) {
        const left = points.get(payload.nodes[a].id)!;
        const right = points.get(payload.nodes[b].id)!;
        const dx = left.x - right.x || 0.01;
        const dy = left.y - right.y || 0.01;
        const distance = Math.max(2, Math.hypot(dx, dy));
        const force = (ideal * ideal) / distance;
        const mx = (dx / distance) * force;
        const my = (dy / distance) * force;
        movement.get(payload.nodes[a].id)!.x += mx; movement.get(payload.nodes[a].id)!.y += my;
        movement.get(payload.nodes[b].id)!.x -= mx; movement.get(payload.nodes[b].id)!.y -= my;
      }
    }
    for (const [left, right] of edgePairs) {
      const dx = left.x - right.x;
      const dy = left.y - right.y;
      const distance = Math.max(2, Math.hypot(dx, dy));
      const force = (distance * distance) / ideal;
      const mx = (dx / distance) * force;
      const my = (dy / distance) * force;
      const leftMove = [...points].find(([, point]) => point === left)?.[0];
      const rightMove = [...points].find(([, point]) => point === right)?.[0];
      if (leftMove && rightMove) {
        movement.get(leftMove)!.x -= mx; movement.get(leftMove)!.y -= my;
        movement.get(rightMove)!.x += mx; movement.get(rightMove)!.y += my;
      }
    }
    const temperature = 12 * (1 - iteration / 160);
    for (const node of payload.nodes) {
      const point = points.get(node.id)!;
      const move = movement.get(node.id)!;
      const length = Math.max(1, Math.hypot(move.x, move.y));
      point.x += (move.x / length) * Math.min(length, temperature);
      point.y += (move.y / length) * Math.min(length, temperature);
    }
  }
  self.postMessage(Object.fromEntries(points));
};
