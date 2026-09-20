/// Layout work stays off the interaction thread. ELK handles directed graphs;
/// a small deterministic force solver handles similarity maps.
import ELK from "elkjs/lib/elk.bundled.js";

interface LayoutRequest {
  kind: "force" | "layered" | "clustered";
  nodes: Array<{ id: string; x: number; y: number; size: number; width?: number; familyId?: string }>;
  edges: Array<{ source: string; target: string }>;
}

self.onmessage = async (event: MessageEvent<LayoutRequest>) => {
  const payload = event.data;
  if (payload.kind === "clustered") {
    const groups = new Map<string, LayoutRequest["nodes"]>();
    for (const node of payload.nodes) {
      const key = node.familyId ?? node.id;
      const members = groups.get(key) ?? [];
      members.push(node);
      groups.set(key, members);
    }
    const ordered = [...groups.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([familyId, members]) => [familyId, members.sort((left, right) => left.id.localeCompare(right.id))] as const);
    const outerRing = (count: number) => {
      let ring = 0;
      let capacity = 1;
      while (count > capacity) {
        ring += 1;
        capacity += ring * 6;
      }
      return ring;
    };
    const largestRadius = Math.max(50, ...ordered.map(([, members]) => outerRing(members.length) * 52 + 42));
    const spacing = largestRadius * 2 + 110;
    const columns = Math.max(1, Math.ceil(Math.sqrt(ordered.length)));
    const rows = Math.ceil(ordered.length / columns);
    const positions: Record<string, { x: number; y: number }> = {};

    ordered.forEach(([, members], groupIndex) => {
      const column = groupIndex % columns;
      const row = Math.floor(groupIndex / columns);
      const centerX = (column - (columns - 1) / 2) * spacing;
      const centerY = (row - (rows - 1) / 2) * spacing;
      members.forEach((node, memberIndex) => {
        if (memberIndex === 0) {
          positions[node.id] = { x: centerX, y: centerY };
          return;
        }
        let ring = 1;
        let offset = memberIndex - 1;
        while (offset >= ring * 6) {
          offset -= ring * 6;
          ring += 1;
        }
        const countOnRing = Math.min(ring * 6, members.length - (1 + 3 * (ring - 1) * ring));
        const angle = -Math.PI / 2 + (offset / countOnRing) * Math.PI * 2;
        positions[node.id] = {
          x: centerX + Math.cos(angle) * ring * 52,
          y: centerY + Math.sin(angle) * ring * 52,
        };
      });
    });
    self.postMessage(positions);
    return;
  }

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
